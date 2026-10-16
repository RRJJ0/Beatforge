use crate::{downloads, mapperatorinator_lock, packages::PackageArtifact};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tar::Archive;

const REQUIREMENTS: &str =
    include_str!("../resources/python/mapperatorinator-windows-py310.requirements.txt");
const TRAINING_IMPORTS: &str =
    "from .init_utils import *\nfrom .model_utils import *\nfrom .train_utils import *\n";
const INFERENCE_IMPORTS: &str = "from .init_utils import *\nfrom .model_utils import *\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallMarker {
    source_commit: String,
    requirements_sha256: String,
    slider_commit: String,
}

#[derive(Debug, Clone)]
pub struct InstallInfo {
    pub source_path: PathBuf,
    pub source_version: String,
    pub dependency_version: String,
}

fn unique_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{timestamp}", std::process::id())
}

fn run_python(python: &Path, args: &[&str], label: &str) -> Result<String, String> {
    let output = Command::new(python)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to start {label}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{label} exited with {}", output.status)
        } else {
            format!("{label} failed: {stderr}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn managed_source_path(runtime_root: &Path) -> Result<PathBuf, String> {
    let lock = mapperatorinator_lock::builtin()?;
    Ok(runtime_root
        .join("mapperatorinator")
        .join(&lock.source.commit))
}

pub fn inspect(python: &Path, runtime_root: &Path) -> Result<InstallInfo, String> {
    let lock = mapperatorinator_lock::builtin()?;
    let source_path = managed_source_path(runtime_root)?;
    let marker_path = source_path.join(".beatforge-install.json");
    let marker: InstallMarker = serde_json::from_slice(
        &fs::read(&marker_path)
            .map_err(|error| format!("Mapperatorinator install marker is missing: {error}"))?,
    )
    .map_err(|error| format!("Mapperatorinator install marker is invalid: {error}"))?;
    if marker.source_commit != lock.source.commit
        || marker.requirements_sha256 != lock.dependencies.sha256
        || marker.slider_commit != lock.slider.commit
        || !source_path.join("inference.py").is_file()
        || !source_path.join("LICENSE").is_file()
    {
        return Err("Mapperatorinator source and dependency lock do not match".into());
    }

    let dependency_script = format!(
        "import importlib.metadata as m; expected={{'accelerate':'1.12.0','hydra-core':'1.3.2','nnAudio':'0.3.4','pandas':'2.3.3','pydub':'0.25.1','PyYAML':'6.0.3','rosu-pp-py':'3.1.0','scipy':'1.15.3','slider':'0.8.2','transformers':'4.57.3'}}; bad=[f'{{name}}={{m.version(name)}}' for name,version in expected.items() if m.version(name)!=version]; assert not bad, ', '.join(bad); print('{}')",
        &lock.dependencies.sha256[..12]
    );
    let dependency_version = run_python(
        python,
        &["-I", "-c", &dependency_script],
        "Mapperatorinator dependency check",
    )?;
    run_python(
        python,
        &["-I", "-m", "pip", "check"],
        "Python dependency consistency check",
    )?;

    Ok(InstallInfo {
        source_path,
        source_version: lock.source.commit[..12].into(),
        dependency_version,
    })
}

fn extract_source(
    archive_path: &Path,
    staging_root: &Path,
    commit: &str,
) -> Result<PathBuf, String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("Failed to open Mapperatorinator source archive: {error}"))?;
    Archive::new(GzDecoder::new(file))
        .unpack(staging_root)
        .map_err(|error| format!("Failed to extract Mapperatorinator source: {error}"))?;
    let extracted = staging_root.join(format!("Mapperatorinator-{commit}"));
    if !extracted.join("inference.py").is_file() || !extracted.join("LICENSE").is_file() {
        return Err("Mapperatorinator archive has an unexpected layout".into());
    }
    Ok(extracted)
}

fn apply_inference_only_patch(source_path: &Path) -> Result<(), String> {
    let init_path = source_path
        .join("osuT5")
        .join("osuT5")
        .join("utils")
        .join("__init__.py");
    let current = fs::read_to_string(&init_path)
        .map_err(|error| format!("Failed to inspect upstream utility imports: {error}"))?;
    if current != TRAINING_IMPORTS {
        return Err("Pinned Mapperatorinator utility imports changed; refusing to patch".into());
    }
    fs::write(&init_path, INFERENCE_IMPORTS)
        .map_err(|error| format!("Failed to apply inference-only import patch: {error}"))
}

fn install_requirements(python: &Path, cache_root: &Path) -> Result<(), String> {
    let lock = mapperatorinator_lock::builtin()?;
    let lock_dir = cache_root.join("python-locks");
    fs::create_dir_all(&lock_dir)
        .map_err(|error| format!("Failed to create dependency lock directory: {error}"))?;
    let lock_path = lock_dir.join(&lock.dependencies.file_name);
    fs::write(&lock_path, REQUIREMENTS)
        .map_err(|error| format!("Failed to materialize dependency lock: {error}"))?;
    let lock_arg = lock_path.to_string_lossy().into_owned();
    run_python(
        python,
        &[
            "-I",
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--require-hashes",
            "--no-deps",
            "--no-build-isolation",
            "-r",
            &lock_arg,
        ],
        "Mapperatorinator dependency installation",
    )?;
    Ok(())
}

pub fn install<F>(
    python: &Path,
    runtime_root: &Path,
    cache_root: &Path,
    mut on_progress: F,
) -> Result<InstallInfo, String>
where
    F: FnMut(&str, u8, &str),
{
    let lock = mapperatorinator_lock::builtin()?;
    on_progress(
        "source_download",
        50,
        "Downloading pinned Mapperatorinator source",
    );
    let artifact = PackageArtifact {
        url: lock.source.archive.url.clone(),
        size_bytes: lock.source.archive.size_bytes,
        sha256: lock.source.archive.sha256.clone(),
        file_name: lock.source.archive.file_name.clone(),
    };
    let archive_path = downloads::download_artifact(
        "mapperatorinator-source",
        &artifact,
        &cache_root.join("sources"),
        |event| {
            let mapped = 50 + (event.progress as u16 * 12 / 100) as u8;
            on_progress(
                "source_download",
                mapped,
                "Downloading pinned Mapperatorinator source",
            );
            Ok(())
        },
    )?;

    on_progress(
        "python_dependencies",
        65,
        "Installing hash-locked inference dependencies",
    );
    install_requirements(python, cache_root)?;

    on_progress(
        "source_install",
        88,
        "Installing application-managed source",
    );
    let source_root = runtime_root.join("mapperatorinator");
    fs::create_dir_all(&source_root)
        .map_err(|error| format!("Failed to create Mapperatorinator runtime directory: {error}"))?;
    let target = source_root.join(&lock.source.commit);
    let suffix = unique_suffix();
    let staging = source_root.join(format!(".{}.staging-{suffix}", lock.source.commit));
    let backup = source_root.join(format!(".{}.backup-{suffix}", lock.source.commit));
    fs::create_dir(&staging)
        .map_err(|error| format!("Failed to create Mapperatorinator staging directory: {error}"))?;

    let prepare = (|| {
        let extracted = extract_source(&archive_path, &staging, &lock.source.commit)?;
        apply_inference_only_patch(&extracted)?;
        let marker = InstallMarker {
            source_commit: lock.source.commit.clone(),
            requirements_sha256: lock.dependencies.sha256.clone(),
            slider_commit: lock.slider.commit.clone(),
        };
        fs::write(
            extracted.join(".beatforge-install.json"),
            serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Failed to write Mapperatorinator install marker: {error}"))?;
        Ok::<_, String>(extracted)
    })();
    let extracted = match prepare {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| {
            format!("Failed to preserve previous Mapperatorinator source: {error}")
        })?;
    }
    if let Err(error) = fs::rename(&extracted, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "Failed to activate Mapperatorinator source: {error}"
        ));
    }
    let _ = fs::remove_dir_all(&staging);

    on_progress(
        "verifying",
        95,
        "Verifying source and Python dependency lock",
    );
    match inspect(python, runtime_root) {
        Ok(info) => {
            let _ = fs::remove_dir_all(&backup);
            on_progress("completed", 100, "Mapperatorinator environment is ready");
            Ok(info)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&target);
            if backup.exists() {
                let _ = fs::rename(&backup, &target);
            }
            Err(format!("Mapperatorinator verification failed: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_exclude_out_of_scope_packages() {
        for package in [
            "lightning==",
            "datasets==",
            "wandb==",
            "flask==",
            "pywebview==",
            "peft==",
            "torchcodec==",
        ] {
            assert!(!REQUIREMENTS.contains(package));
        }
        assert!(REQUIREMENTS.contains("--hash=sha256:"));
        assert!(REQUIREMENTS.contains("slider @ https://github.com/OliBomby/slider/archive/"));
    }

    #[test]
    fn inference_patch_only_removes_training_reexport() {
        assert!(TRAINING_IMPORTS.contains("train_utils"));
        assert!(!INFERENCE_IMPORTS.contains("train_utils"));
        assert!(INFERENCE_IMPORTS.contains("model_utils"));
    }
}
