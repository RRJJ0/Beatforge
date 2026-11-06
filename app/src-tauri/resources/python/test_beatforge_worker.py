import tempfile
from types import SimpleNamespace
import unittest
from pathlib import Path
import zipfile

import beatforge_worker as worker


class ModelIdentityTests(unittest.TestCase):
    def test_each_pinned_model_routes_to_its_own_config(self):
        for name in ("v30", "v31", "v32-mini", "v32"):
            with self.subTest(name=name):
                command = {
                    "model_id": f"mapperatorinator-{name}-standard",
                    "model_version": f"{name.removeprefix('v')}.test",
                    "model_source": f"https://huggingface.co/OliBomby/Mapperatorinator-{name}/tree/test",
                }
                self.assertEqual(worker.resolve_model_identity(command)[-1], name)

    def test_rejects_mismatched_model_source_and_version(self):
        command = {
            "model_id": "mapperatorinator-v31-standard",
            "model_version": "30.test",
            "model_source": "https://huggingface.co/OliBomby/Mapperatorinator-v31/tree/test",
        }
        with self.assertRaisesRegex(ValueError, "model_version"):
            worker.resolve_model_identity(command)
        command["model_version"] = "31.test"
        command["model_source"] = "https://huggingface.co/OliBomby/Mapperatorinator-v30/tree/test"
        with self.assertRaisesRegex(ValueError, "model_source"):
            worker.resolve_model_identity(command)


class BeatmapMetadataTests(unittest.TestCase):
    def test_negative_descriptors_enable_guidance_without_changing_other_jobs(self):
        ordinary = SimpleNamespace(cfg_scale=1.0)
        worker.apply_descriptor_guidance(ordinary, [])
        self.assertEqual(ordinary.cfg_scale, 1.0)

        excluded = SimpleNamespace(cfg_scale=1.0)
        worker.apply_descriptor_guidance(excluded, ["skillset/streams"])
        self.assertGreater(excluded.cfg_scale, 1.0)
        self.assertEqual(excluded.cfg_scale, worker.NEGATIVE_DESCRIPTOR_CFG_SCALE)

    def test_public_tags_do_not_expose_private_generation_parameters(self):
        args = SimpleNamespace(tags=None)

        worker.set_public_beatmap_tags(args)

        self.assertEqual(args.tags, "Beatforge AI mapperatorinator")
        for private_fragment in ("model=", "seed=", "difficulty=", "\\", ":/"):
            self.assertNotIn(private_fragment, args.tags)

    def test_v020_fields_apply_before_inference_and_record_effective_values(self):
        args = SimpleNamespace(
            title="Song", artist="Artist", source="Reference source",
            hp_drain_rate=None, overall_difficulty=None,
            slider_multiplier=None, slider_tick_rate=None, tags=None,
        )
        config = {
            "request_schema_version": 2, "creator": "Human Mapper",
            "title_unicode": "原文", "artist_unicode": None, "source": None,
            "tags": ["electronic", "AI"], "hp_drain_rate": 6,
            "overall_difficulty": 8.5, "slider_multiplier": 1.6,
            "slider_tick_rate": None,
        }
        worker.apply_public_request_fields(args, config)
        self.assertEqual(args.creator, "Human Mapper")
        self.assertEqual(args.title_unicode, "原文")
        self.assertEqual(args.artist_unicode, "Artist")
        self.assertEqual(args.source, "")
        self.assertEqual(args.tags, "electronic Beatforge AI mapperatorinator")
        self.assertEqual(args.slider_multiplier, 1.6)
        args.slider_tick_rate = 1
        settings = worker.setting_resolution(config, args, has_reference=False)
        self.assertEqual(settings["slider_multiplier"]["origin"], "explicit")
        self.assertEqual(settings["slider_tick_rate"]["origin"], "upstream_default")
        self.assertEqual(settings["slider_tick_rate"]["applied"], 1)

    def test_legacy_reference_source_and_public_validation(self):
        args = SimpleNamespace(
            title="Song", artist="Artist", source=None, tags=None,
            hp_drain_rate=None, overall_difficulty=None,
            slider_multiplier=None, slider_tick_rate=None,
        )
        worker.apply_public_request_fields(args, {"request_schema_version": 1})
        self.assertIsNone(args.source)
        self.assertEqual(args.tags, "Beatforge AI mapperatorinator")
        for bad in (["two words"], ["seed=42"], ["same", "Same"]):
            with self.assertRaises(ValueError):
                worker.require_public_tags(bad)
        with self.assertRaises(ValueError):
            worker.require_optional_metadata_text("bad\nsource", "source")


class BackgroundPackagingTests(unittest.TestCase):
    def test_one_background_is_referenced_by_every_difficulty(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            background = root / "beatforge-background.jpg"
            background.write_bytes(b"background fixture")
            audio = b"shared audio"
            completed = []
            for index, name in enumerate(("Normal", "Insane"), start=1):
                archive_path = root / f"source-{index}.osz"
                beatmap = (
                    "osu file format v14\n\n"
                    "[General]\nAudioFilename:audio.mp3\n\n"
                    f"[Metadata]\nVersion:Old {index}\n\n"
                    "[Events]\n//Background and Video events\n\n"
                    "[TimingPoints]\n0,500,4,2,1,70,1,0\n"
                ).encode("utf-8")
                with zipfile.ZipFile(archive_path, "w") as archive:
                    archive.writestr("map.osu", beatmap)
                    archive.writestr("audio.mp3", audio)
                completed.append(
                    (
                        {
                            "_set_index": index,
                            "difficulty_name": name,
                        },
                        archive_path,
                        {},
                    )
                )

            result = worker.merge_difficulty_outputs(root, completed, background)

            with zipfile.ZipFile(result) as archive:
                names = archive.namelist()
                self.assertEqual(names.count("beatforge-background.jpg"), 1)
                self.assertEqual(names.count("audio.mp3"), 1)
                for index, difficulty_name in enumerate(("Normal", "Insane"), start=1):
                    beatmap = archive.read(f"difficulty-{index}.osu").decode("utf-8")
                    self.assertIn(f"Version:{difficulty_name}", beatmap)
                    self.assertIn('0,0,"beatforge-background.jpg",0,0', beatmap)

    def test_one_video_is_referenced_by_every_difficulty_at_the_requested_time(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            video = root / "beatforge-video.mp4"
            video.write_bytes(b"video fixture")
            completed = []
            for index, name in enumerate(("Normal", "Insane"), start=1):
                archive_path = root / f"video-source-{index}.osz"
                beatmap = (
                    "osu file format v14\n\n"
                    "[General]\nAudioFilename:audio.mp3\n\n"
                    f"[Metadata]\nVersion:Old {index}\n\n"
                    "[Events]\nVideo,0,\"old.mp4\",0,0\n\n"
                    "[TimingPoints]\n0,500,4,2,1,70,1,0\n"
                ).encode("utf-8")
                with zipfile.ZipFile(archive_path, "w") as archive:
                    archive.writestr("map.osu", beatmap)
                    archive.writestr("audio.mp3", b"shared audio")
                completed.append(
                    ({"_set_index": index, "difficulty_name": name}, archive_path, {})
                )

            result = worker.merge_difficulty_outputs(
                root, completed, video_path=video, video_start_time_ms=1250
            )

            with zipfile.ZipFile(result) as archive:
                self.assertEqual(archive.namelist().count("beatforge-video.mp4"), 1)
                self.assertEqual(
                    archive.getinfo("beatforge-video.mp4").compress_type,
                    zipfile.ZIP_STORED,
                )
                for index in (1, 2):
                    beatmap = archive.read(f"difficulty-{index}.osu").decode("utf-8")
                    self.assertNotIn("old.mp4", beatmap)
                    self.assertIn('Video,1250,"beatforge-video.mp4",0,0', beatmap)


class OutputConsistencyTests(unittest.TestCase):
    def beatmap(self, name, hp):
        return (
            "osu file format v14\n[General]\nAudioFilename:audio.mp3\n"
            "[Metadata]\nTitle:Song\nArtist:Artist\nCreator:Mapper\n"
            f"Version:{name}\nTitleUnicode:原文\nArtistUnicode:Artist\n"
            "Source:\nTags:electronic Beatforge AI mapperatorinator\n"
            "[Difficulty]\nApproachRate:9\nCircleSize:4\n"
            f"HPDrainRate:{hp}\nOverallDifficulty:8\n"
            "SliderMultiplier:1.4\nSliderTickRate:1\n"
        ).encode("utf-8")

    def member(self, index, name, hp):
        config = {"_set_index": index, "difficulty_name": name}
        provenance = {"parameters": {
            "title": "Song", "artist": "Artist", "creator": "Mapper",
            "title_unicode": "原文", "artist_unicode": "Artist",
            "source": "", "tags": "electronic Beatforge AI mapperatorinator",
            "approach_rate": 9, "circle_size": 4, "hp_drain_rate": hp,
            "overall_difficulty": 8, "slider_multiplier": 1.4,
            "slider_tick_rate": 1,
        }}
        return config, Path("unused.osz"), provenance

    def test_single_and_multi_difficulty_match_their_own_effective_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "beatmap-set.osz"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("difficulty-1.osu", self.beatmap("Normal", 5))
                archive.writestr("difficulty-2.osu", self.beatmap("Hard", 7))
            first = self.member(1, "Normal", 5)
            second = self.member(2, "Hard", 7)
            worker.verify_output_fields(path, [first, second])
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("difficulty-1.osu", self.beatmap("Normal", 5))
            worker.verify_output_fields(path, [first])

    def test_mismatch_missing_member_and_invalid_number_fail_before_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "beatmap-set.osz"
            first = self.member(1, "Normal", 5)
            second = self.member(2, "Hard", 7)
            for bad_map in (
                self.beatmap("Normal", 6),
                self.beatmap("Normal", "nan"),
                self.beatmap("Normal", 5).replace(b"Creator:Mapper", b"Creator:Wrong"),
            ):
                with zipfile.ZipFile(path, "w") as archive:
                    archive.writestr("difficulty-1.osu", bad_map)
                with self.assertRaisesRegex(RuntimeError, "mismatched|invalid"):
                    worker.verify_output_fields(path, [first])
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("difficulty-1.osu", self.beatmap("Normal", 5))
            with self.assertRaisesRegex(RuntimeError, "difficulty count"):
                worker.verify_output_fields(path, [first, second])

    def test_partial_set_checks_only_completed_members_at_original_index(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "beatmap-set.osz"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("difficulty-2.osu", self.beatmap("Hard", 7))
            worker.verify_output_fields(path, [self.member(2, "Hard", 7)])

    def test_omitted_numeric_setting_checks_the_no_reference_default(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "beatmap-set.osz"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("difficulty-1.osu", self.beatmap("Normal", 5))
            config, source, provenance = self.member(1, "Normal", 5)
            provenance["parameters"]["slider_tick_rate"] = None
            worker.verify_output_fields(path, [(config, source, provenance)])
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr(
                    "difficulty-1.osu",
                    self.beatmap("Normal", 5).replace(
                        b"SliderTickRate:1", b"SliderTickRate:2"
                    ),
                )
            with self.assertRaisesRegex(RuntimeError, "SliderTickRate"):
                worker.verify_output_fields(path, [(config, source, provenance)])


if __name__ == "__main__":
    unittest.main()
