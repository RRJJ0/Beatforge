import tempfile
import unittest
from pathlib import Path
import zipfile

import beatforge_worker as worker


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


if __name__ == "__main__":
    unittest.main()
