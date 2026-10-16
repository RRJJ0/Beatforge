use crate::install_recovery;
use flate2::read::GzDecoder;
use serde::Serialize;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tar::Archive;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub version: String,
    pub path: String,
    pub executable: String,
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

fn configure_python_runtime(runtime_path: &Path) -> Result<(), String> {
    fs::create_dir_all(runtime_path.join("Lib").join("site-packages"))
        .map_err(|error| format!("Failed to create site-packages: {error}"))
}

pub fn verify_python_runtime(
    runtime_path: &Path,
    expected_version: &str,
) -> Result<RuntimeInfo, String> {
    let executable = runtime_path.join("python.exe");
    if !executable.is_file() {
        return Err(format!(
            "Python executable is missing: {}",
            executable.display()
        ));
    }
    let output = Command::new(&executable)
        .args([
            "-I",
            "-c",
            "import sys; print(sys.version.split()[0]); print(sys.prefix)",
        ])
        .output()
        .map_err(|error| format!("Failed to launch private Python runtime: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Private Python runtime failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Private Python runtime returned invalid UTF-8".to_string())?;
    let mut lines = stdout.lines();
    let version = lines.next().unwrap_or_default().trim();
    if version != expected_version {
        return Err(format!(
            "Python version mismatch: expected {expected_version}, received {version}"
        ));
    }
    let reported_prefix = lines.next().unwrap_or_default().trim();
    if reported_prefix.is_empty() {
        return Err("Private Python runtime did not report its prefix".into());
    }
    Ok(RuntimeInfo {
        version: version.into(),
        path: path_string(runtime_path),
        executable: path_string(&executable),
        healthy: true,
    })
}

fn extract_runtime_archive(archive_path: &Path, staging_path: &Path) -> Result<PathBuf, String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("Failed to open {}: {error}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(staging_path)
        .map_err(|error| format!("Failed to extract Python runtime: {error}"))?;
    let extracted_path = staging_path.join("python");
    if !extracted_path.is_dir() {
        return Err("Python runtime archive does not contain the expected python directory".into());
    }
    Ok(extracted_path)
}

fn install_with_verifier<F>(
    archive_path: &Path,
    runtime_root: &Path,
    version: &str,
    verifier: F,
) -> Result<RuntimeInfo, String>
where
    F: Fn(&Path, &str) -> Result<RuntimeInfo, String>,
{
    let python_root = runtime_root.join("python");
    fs::create_dir_all(&python_root)
        .map_err(|error| format!("Failed to create runtime directory: {error}"))?;
    let target_path = python_root.join(version);
    install_recovery::recover_interrupted_install(&python_root, &target_path, version)?;
    if target_path.exists()
        && let Ok(info) = verifier(&target_path, version)
    {
        install_recovery::clear_backups(&python_root, version)?;
        return Ok(info);
    }

    let suffix = unique_suffix();
    let staging_path = python_root.join(format!(".{version}.staging-{suffix}"));
    let backup_path = python_root.join(format!(".{version}.backup-{suffix}"));
    fs::create_dir(&staging_path)
        .map_err(|error| format!("Failed to create runtime staging directory: {error}"))?;

    let prepare_result = (|| {
        let extracted_path = extract_runtime_archive(archive_path, &staging_path)?;
        configure_python_runtime(&extracted_path)?;
        verifier(&extracted_path, version)?;
        Ok::<PathBuf, String>(extracted_path)
    })();
    let extracted_path = match prepare_result {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_path);
            return Err(error);
        }
    };

    if target_path.exists() {
        fs::rename(&target_path, &backup_path)
            .map_err(|error| format!("Failed to preserve the previous runtime: {error}"))?;
    }
    if let Err(error) = fs::rename(&extracted_path, &target_path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, &target_path);
        }
        let _ = fs::remove_dir_all(&staging_path);
        return Err(format!("Failed to activate the Python runtime: {error}"));
    }
    let _ = fs::remove_dir_all(&staging_path);

    match verifier(&target_path, version) {
        Ok(info) => {
            install_recovery::clear_backups(&python_root, version)?;
            Ok(info)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&target_path);
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &target_path);
            }
            Err(format!(
                "Installed Python runtime failed verification: {error}"
            ))
        }
    }
}

pub fn install_python_runtime(
    archive_path: &Path,
    runtime_root: &Path,
    version: &str,
) -> Result<RuntimeInfo, String> {
    install_with_verifier(archive_path, runtime_root, version, verify_python_runtime)
}

pub fn remove_runtime_files(
    install_path: Option<&str>,
    artifact_path: Option<&str>,
    runtime_root: &Path,
    cache_root: &Path,
) -> Result<(), String> {
    if let Some(path) = install_path {
        let path = PathBuf::from(path);
        if path.exists() {
            ensure_managed_child(&path, runtime_root, "runtime")?;
            fs::remove_dir_all(&path)
                .map_err(|error| format!("Failed to remove runtime {}: {error}", path.display()))?;
        }
    }
    if let Some(path) = artifact_path {
        let path = PathBuf::from(path);
        if path.exists() {
            ensure_managed_child(&path, cache_root, "runtime archive")?;
            fs::remove_file(&path).map_err(|error| {
                format!(
                    "Failed to remove runtime archive {}: {error}",
                    path.display()
                )
            })?;
        }
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
    use flate2::{Compression, write::GzEncoder};
    use std::io::Cursor;
    use tar::{Builder, Header};

    fn temp_directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("beatforge-runtime-{name}-{}", unique_suffix()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn fixture_archive(path: &Path) {
        let file = File::create(path).unwrap();
        let encoder = GzEncoder::new(file, Compression::fast());
        let mut archive = Builder::new(encoder);
        let data = b"fixture";
        let mut header = Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(&mut header, "python/python.exe", Cursor::new(data))
            .unwrap();
        archive.finish().unwrap();
    }

    fn fixture_verifier(path: &Path, version: &str) -> Result<RuntimeInfo, String> {
        if !path.join("python.exe").is_file() {
            return Err("fixture executable is missing".into());
        }
        if !path.join("Lib").join("site-packages").is_dir() {
            return Err("site-packages was not configured".into());
        }
        Ok(RuntimeInfo {
            version: version.into(),
            path: path_string(path),
            executable: path_string(&path.join("python.exe")),
            healthy: true,
        })
    }

    #[test]
    fn installs_runtime_through_a_staging_directory() {
        let root = temp_directory("install");
        let archive = root.join("python.tar.gz");
        fixture_archive(&archive);

        let info = install_with_verifier(&archive, &root, "3.10.21", fixture_verifier).unwrap();
        let installed = root.join("python").join("3.10.21");
        assert_eq!(PathBuf::from(info.path), installed);
        assert!(installed.join("Lib").join("site-packages").is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removes_only_the_managed_runtime_and_archive() {
        let root = temp_directory("remove");
        let runtime = root.join("runtime");
        let archive = root.join("python.tar.gz");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(&archive, b"archive").unwrap();

        remove_runtime_files(
            Some(runtime.to_string_lossy().as_ref()),
            Some(archive.to_string_lossy().as_ref()),
            &root,
            &root,
        )
        .unwrap();
        assert!(!runtime.exists());
        assert!(!archive.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_to_remove_paths_outside_managed_roots() {
        let root = temp_directory("safe-remove");
        let managed = root.join("managed");
        let outside = root.join("outside");
        fs::create_dir_all(&managed).unwrap();
        fs::create_dir_all(&outside).unwrap();

        let error = remove_runtime_files(
            Some(outside.to_string_lossy().as_ref()),
            None,
            &managed,
            &managed,
        )
        .unwrap_err();
        assert!(error.contains("outside its managed directory"));
        assert!(outside.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
