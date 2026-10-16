use crate::{downloads, install_recovery, packages::PackageSpec};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub version: String,
    pub path: String,
    pub size_bytes: u64,
    pub files: Vec<String>,
    pub healthy: bool,
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn unique_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{timestamp}", std::process::id())
}

pub fn verify_model(model_path: &Path, spec: &PackageSpec) -> Result<ModelInfo, String> {
    if !model_path.is_dir() {
        return Err(format!(
            "Model directory is missing: {}",
            model_path.display()
        ));
    }
    let mut files = Vec::new();
    for artifact in spec.artifacts() {
        let path = model_path.join(&artifact.file_name);
        downloads::verify_artifact(&path, artifact)?;
        files.push(artifact.file_name.clone());
    }
    Ok(ModelInfo {
        id: spec.id.clone(),
        version: spec.version.clone(),
        path: path_string(model_path),
        size_bytes: spec.total_size(),
        files,
        healthy: true,
    })
}

pub fn install_model(
    download_path: &Path,
    models_root: &Path,
    spec: &PackageSpec,
) -> Result<ModelInfo, String> {
    verify_model(download_path, spec)?;
    let package_root = models_root.join(&spec.id);
    fs::create_dir_all(&package_root)
        .map_err(|error| format!("Failed to create model directory: {error}"))?;
    let target_path = package_root.join(&spec.version);
    install_recovery::recover_interrupted_install(&package_root, &target_path, &spec.version)?;
    if target_path.exists()
        && let Ok(info) = verify_model(&target_path, spec)
    {
        install_recovery::clear_backups(&package_root, &spec.version)?;
        return Ok(info);
    }

    let suffix = unique_suffix();
    let staging_path = package_root.join(format!(".{}.staging-{suffix}", spec.version));
    let backup_path = package_root.join(format!(".{}.backup-{suffix}", spec.version));
    fs::create_dir(&staging_path)
        .map_err(|error| format!("Failed to create model staging directory: {error}"))?;

    let prepare_result = (|| {
        for artifact in spec.artifacts() {
            let source = download_path.join(&artifact.file_name);
            let destination = staging_path.join(&artifact.file_name);
            fs::rename(&source, &destination).map_err(|error| {
                format!(
                    "Failed to move model file {} into staging: {error}",
                    artifact.file_name
                )
            })?;
        }
        verify_model(&staging_path, spec)
    })();
    if let Err(error) = prepare_result {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(error);
    }

    if target_path.exists() {
        fs::rename(&target_path, &backup_path)
            .map_err(|error| format!("Failed to preserve the previous model: {error}"))?;
    }
    if let Err(error) = fs::rename(&staging_path, &target_path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, &target_path);
        }
        return Err(format!("Failed to activate the model: {error}"));
    }

    match verify_model(&target_path, spec) {
        Ok(info) => {
            install_recovery::clear_backups(&package_root, &spec.version)?;
            let _ = fs::remove_dir(download_path);
            Ok(info)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&target_path);
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &target_path);
            }
            Err(format!("Installed model failed verification: {error}"))
        }
    }
}

pub fn remove_model_files(
    install_path: Option<&str>,
    artifact_path: Option<&str>,
    models_root: &Path,
    cache_root: &Path,
) -> Result<(), String> {
    let install = install_path.map(PathBuf::from);
    let artifact = artifact_path.map(PathBuf::from);
    if let Some(path) = install.as_deref()
        && path.exists()
    {
        ensure_managed_child(path, models_root, "model")?;
        fs::remove_dir_all(path)
            .map_err(|error| format!("Failed to remove model {}: {error}", path.display()))?;
    }
    if let Some(path) = artifact.as_deref()
        && path.exists()
        && install.as_deref() != Some(path)
    {
        ensure_managed_child(path, cache_root, "model download")?;
        fs::remove_dir_all(path).map_err(|error| {
            format!(
                "Failed to remove model download {}: {error}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn ensure_managed_child(path: &Path, root: &Path, label: &str) -> Result<(), String> {
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("Failed to resolve {label} path: {error}"))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("Failed to resolve managed {label} root: {error}"))?;
    if canonical_path == canonical_root || !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "Refusing to remove {label} outside its managed directory"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::{PackageArtifact, PackageKind};
    use sha2::{Digest, Sha256};

    fn temp_directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("beatforge-model-{name}-{}", unique_suffix()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn fixture_spec(data: &[u8]) -> PackageSpec {
        let hash: String = Sha256::digest(data)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        PackageSpec {
            id: "fixture-model".into(),
            kind: PackageKind::Model,
            name: "Fixture model".into(),
            description: "Fixture".into(),
            version: "1.0.0".into(),
            artifact: PackageArtifact {
                url: "https://example.test/model.safetensors".into(),
                size_bytes: data.len() as u64,
                sha256: hash,
                file_name: "model.safetensors".into(),
            },
            additional_artifacts: vec![],
            requirements: vec![],
            recommended: true,
        }
    }

    #[test]
    fn atomically_installs_and_verifies_model() {
        let root = temp_directory("install");
        let download = root.join("download");
        let models = root.join("models");
        fs::create_dir_all(&download).unwrap();
        fs::create_dir_all(&models).unwrap();
        let data = b"fixture model";
        fs::write(download.join("model.safetensors"), data).unwrap();
        let spec = fixture_spec(data);

        let info = install_model(&download, &models, &spec).unwrap();
        assert!(info.healthy);
        assert!(
            PathBuf::from(&info.path)
                .join("model.safetensors")
                .is_file()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_corrupt_model() {
        let root = temp_directory("corrupt");
        fs::write(root.join("model.safetensors"), b"wrong").unwrap();
        let error = verify_model(&root, &fixture_spec(b"expected")).unwrap_err();
        assert!(error.contains("mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_remove_model_outside_managed_roots() {
        let root = temp_directory("safe-remove");
        let managed_models = root.join("managed-models");
        let managed_cache = root.join("managed-cache");
        let outside = root.join("outside");
        fs::create_dir_all(&managed_models).unwrap();
        fs::create_dir_all(&managed_cache).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let error = remove_model_files(
            Some(outside.to_string_lossy().as_ref()),
            None,
            &managed_models,
            &managed_cache,
        )
        .unwrap_err();
        assert!(error.contains("outside its managed directory"));
        assert!(outside.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
