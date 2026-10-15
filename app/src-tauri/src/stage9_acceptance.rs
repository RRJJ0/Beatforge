use crate::{
    jobs, osu_integration, preview,
    protocol::{DifficultyRequest, StartJobInput},
    validation::{ValidationLimits, ValidationStatus, validate_candidate_with_delivery_name},
};
use rusqlite::Connection;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn temporary_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_root = std::env::var_os("BEATFORGE_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target/stage9-test-temp"));
    let root = test_root.join(format!("beatforge-stage9-{name}-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    root
}

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE jobs (id TEXT PRIMARY KEY, status TEXT NOT NULL, request_json TEXT NOT NULL, result_json TEXT,
               created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
             INSERT INTO schema_migrations(version) VALUES (1);",
        )
        .unwrap();
    jobs::migrate(&connection).unwrap();
    connection
}

fn input(audio: &Path) -> StartJobInput {
    StartJobInput {
        title: "Stage 9 Acceptance".into(),
        artist: "Beatforge".into(),
        audio_file: audio.to_string_lossy().into_owned(),
        model: crate::packages::STANDARD_MODEL_ID.into(),
        difficulty: 3.5,
        difficulties: vec![
            DifficultyRequest {
                id: "normal".into(),
                name: "Normal".into(),
                difficulty: 3.5,
                seed: 10,
                descriptors: vec![],
                negative_descriptors: vec![],
                approach_rate: Some(8.0),
                circle_size: Some(4.0),
                hitsounded: true,
            },
            DifficultyRequest {
                id: "hard".into(),
                name: "Hard".into(),
                difficulty: 5.0,
                seed: 20,
                descriptors: vec![],
                negative_descriptors: vec![],
                approach_rate: Some(9.2),
                circle_size: Some(4.2),
                hitsounded: true,
            },
        ],
        gamemode: 0,
        seed: 10,
        descriptors: vec![],
        negative_descriptors: vec![],
        reference_beatmap: None,
        background_image: None,
        video_file: None,
        video_start_time_ms: 0,
        approach_rate: Some(8.0),
        circle_size: Some(4.0),
        hitsounded: true,
        scenario: "success".into(),
    }
}

fn beatmap(version: &str, object_time: u64) -> Vec<u8> {
    format!(
        "osu file format v14\n[General]\nAudioFilename:fixture.wav\nMode:0\n[Metadata]\nTitle:Stage 9 Acceptance\nArtist:Beatforge\nCreator:Beatforge Tests\nVersion:{version}\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,1,70,1,0\n[HitObjects]\n256,192,{object_time},1,0,0:0:0:0:\n"
    )
    .into_bytes()
}

fn write_osz(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in [
        ("normal.osu", beatmap("Normal", 1_000)),
        ("hard.osu", beatmap("Hard", 1_500)),
        ("fixture.wav", b"test audio".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn multi_difficulty_job_reaches_preview_and_non_destructive_handoff() {
    let root = temporary_root("e2e");
    let jobs_root = root.join("jobs");
    let cache_root = root.join("cache");
    let job_id = "BF-STAGE9-E2E";
    let job_root = jobs_root.join(job_id);
    fs::create_dir_all(&job_root).unwrap();
    fs::create_dir_all(&cache_root).unwrap();
    let audio = root.join("fixture.wav");
    fs::write(&audio, b"test audio").unwrap();

    let connection = database();
    let created = jobs::insert(&connection, job_id, &input(&audio)).unwrap();
    assert_eq!(created.difficulty_count, 2);
    jobs::update_state(
        &connection,
        job_id,
        "validating",
        96,
        "Validating generated output",
        None,
        None,
        None,
    )
    .unwrap();

    let candidate = job_root.join("candidate.osz");
    write_osz(&candidate);
    let report = validate_candidate_with_delivery_name(
        job_id,
        &job_root,
        &candidate,
        &[],
        ValidationLimits::default(),
        Some("Beatforge - Stage 9 Acceptance.osz"),
    )
    .unwrap();
    assert_eq!(report.status, ValidationStatus::Valid);
    assert_eq!(report.statistics.as_ref().unwrap().beatmap_count, 2);
    let validated = job_root.join(report.validated_path.as_ref().unwrap());
    jobs::finish_validation(
        &connection,
        job_id,
        &report,
        Some(&validated.to_string_lossy()),
    )
    .unwrap();

    let completed = jobs::get(&connection, job_id).unwrap().unwrap();
    assert_eq!(completed.status, "completed");
    assert!(completed.output_available);
    let resolved = osu_integration::resolve_validated_artifact(
        &jobs_root,
        job_id,
        &completed.status,
        completed.output_path.as_deref(),
        completed.validation_report.as_ref(),
    )
    .unwrap();
    let difficulties = preview::build_difficulties(&resolved).unwrap();
    assert_eq!(difficulties.len(), 2);
    assert_eq!(
        difficulties
            .iter()
            .map(|difficulty| difficulty.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Hard", "Normal"]
    );

    let first = osu_integration::create_import_handoff(&cache_root, job_id, &resolved).unwrap();
    let second = osu_integration::create_import_handoff(&cache_root, job_id, &resolved).unwrap();
    assert_ne!(first, second);
    assert!(
        resolved.is_file(),
        "handoff must preserve the validated artifact"
    );
    assert_eq!(fs::read(&first).unwrap(), fs::read(&resolved).unwrap());
    assert_eq!(fs::read(&second).unwrap(), fs::read(&resolved).unwrap());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_and_retry_reconcile_sqlite_without_reusing_an_output() {
    let root = temporary_root("restart");
    let audio = root.join("fixture.wav");
    fs::write(&audio, b"test audio").unwrap();
    let connection = database();
    jobs::insert(&connection, "BF-RUNNING", &input(&audio)).unwrap();
    jobs::insert(&connection, "BF-QUEUED", &input(&audio)).unwrap();
    jobs::update_state(
        &connection,
        "BF-RUNNING",
        "running",
        50,
        "Generating",
        None,
        Some("stale-output.osz"),
        Some(1234),
    )
    .unwrap();

    jobs::mark_interrupted(&connection).unwrap();
    for id in ["BF-RUNNING", "BF-QUEUED"] {
        let interrupted = jobs::get(&connection, id).unwrap().unwrap();
        assert_eq!(interrupted.status, "interrupted");
        assert!(interrupted.error_message.is_some());
        jobs::reset_for_retry(&connection, id).unwrap();
        let retried = jobs::get(&connection, id).unwrap().unwrap();
        assert_eq!(retried.status, "queued");
        assert_eq!(retried.progress, 0);
        assert!(retried.output_path.is_none());
        assert!(retried.validation_report.is_none());
        assert!(retried.provenance.is_none());
    }
    assert_eq!(jobs::active_ids(&connection).unwrap().len(), 2);
    fs::remove_dir_all(root).unwrap();
}
