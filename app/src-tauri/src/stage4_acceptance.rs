use crate::{
    downloads, models,
    packages::{self, PackageArtifact, PackageKind, PackageManifest, PackageSpec, PackageStatus},
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_directory(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("beatforge-stage4-{name}-{suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY);")
        .unwrap();
    packages::migrate(&connection).unwrap();
    connection
}

fn artifact(data: &[u8]) -> PackageArtifact {
    let sha256: String = Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    PackageArtifact {
        url: "https://example.test/model.safetensors".into(),
        size_bytes: data.len() as u64,
        sha256,
        file_name: "model.safetensors".into(),
    }
}

fn model_spec(version: &str, data: &[u8]) -> PackageSpec {
    PackageSpec {
        id: "acceptance-model".into(),
        kind: PackageKind::Model,
        name: "Acceptance model".into(),
        description: "Small local model used by stage 4 acceptance tests".into(),
        version: version.into(),
        artifact: artifact(data),
        additional_artifacts: vec![],
        requirements: vec![],
        recommended: true,
    }
}

fn sync(connection: &Connection, spec: PackageSpec) {
    packages::sync_manifest(
        connection,
        &PackageManifest {
            schema_version: packages::MANIFEST_SCHEMA_VERSION,
            packages: vec![spec],
        },
    )
    .unwrap();
}

#[test]
fn clean_profile_can_complete_model_lifecycle() {
    let root = temp_directory("clean-profile");
    let cache = root.join("cache");
    let model_root = root.join("models");
    let download = cache.join("download");
    fs::create_dir_all(&download).unwrap();
    fs::create_dir_all(&model_root).unwrap();
    let data = b"small verified model";
    let spec = model_spec("1.0.0", data);
    fs::write(download.join("model.safetensors"), data).unwrap();
    let connection = database();
    sync(&connection, spec.clone());
    let initial = packages::get(&connection, &spec.id).unwrap().unwrap();
    assert_eq!(initial.status, PackageStatus::NotInstalled);

    for status in [
        PackageStatus::Queued,
        PackageStatus::Downloading,
        PackageStatus::Verifying,
        PackageStatus::Downloaded,
        PackageStatus::Installing,
    ] {
        packages::transition(
            &connection,
            &spec.id,
            status,
            if status == PackageStatus::Queued {
                0
            } else {
                100
            },
            if status == PackageStatus::Queued {
                0
            } else {
                data.len() as u64
            },
            None,
            None,
            None,
        )
        .unwrap();
    }
    let info = models::install_model(&download, &model_root, &spec).unwrap();
    packages::transition(
        &connection,
        &spec.id,
        PackageStatus::Installed,
        100,
        data.len() as u64,
        Some(&spec.version),
        Some(&info.path),
        None,
    )
    .unwrap();
    models::verify_model(PathBuf::from(&info.path).as_path(), &spec).unwrap();

    packages::transition(
        &connection,
        &spec.id,
        PackageStatus::Removing,
        100,
        data.len() as u64,
        None,
        None,
        None,
    )
    .unwrap();
    models::remove_model_files(Some(&info.path), None, &model_root, &cache).unwrap();
    packages::finish_removal(&connection, &spec.id).unwrap();
    assert_eq!(
        packages::get(&connection, &spec.id)
            .unwrap()
            .unwrap()
            .status,
        PackageStatus::NotInstalled
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_download_is_marked_failed_and_can_resume() {
    let connection = database();
    let spec = model_spec("1.0.0", b"resume fixture");
    sync(&connection, spec.clone());
    packages::transition(
        &connection,
        &spec.id,
        PackageStatus::Queued,
        0,
        0,
        None,
        None,
        None,
    )
    .unwrap();
    packages::transition(
        &connection,
        &spec.id,
        PackageStatus::Downloading,
        40,
        5,
        None,
        None,
        None,
    )
    .unwrap();
    packages::mark_interrupted(&connection).unwrap();
    let interrupted = packages::get(&connection, &spec.id).unwrap().unwrap();
    assert_eq!(interrupted.status, PackageStatus::Failed);
    assert_eq!(interrupted.bytes_downloaded, 5);
    packages::transition(
        &connection,
        &spec.id,
        PackageStatus::Queued,
        interrupted.progress,
        interrupted.bytes_downloaded,
        None,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn corrupt_update_never_replaces_previous_model() {
    let root = temp_directory("rollback");
    let model_root = root.join("models");
    fs::create_dir_all(&model_root).unwrap();
    let old_download = root.join("old");
    fs::create_dir(&old_download).unwrap();
    fs::write(old_download.join("model.safetensors"), b"old model").unwrap();
    let old_spec = model_spec("1.0.0", b"old model");
    let old = models::install_model(&old_download, &model_root, &old_spec).unwrap();

    let corrupt_download = root.join("corrupt");
    fs::create_dir(&corrupt_download).unwrap();
    fs::write(corrupt_download.join("model.safetensors"), b"corrupt").unwrap();
    let new_spec = model_spec("2.0.0", b"expected new model");
    assert!(models::install_model(&corrupt_download, &model_root, &new_spec).is_err());
    downloads::verify_artifact(
        &PathBuf::from(&old.path).join("model.safetensors"),
        &old_spec.artifact,
    )
    .unwrap();
    fs::remove_dir_all(root).unwrap();
}
