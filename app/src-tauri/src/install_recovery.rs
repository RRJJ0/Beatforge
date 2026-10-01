use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

fn matching_directories(parent: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    if !parent.exists() {
        return Ok(Vec::new());
    }
    let mut matches = Vec::new();
    for entry in fs::read_dir(parent)
        .map_err(|error| format!("Failed to inspect {}: {error}", parent.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
            && name.to_string_lossy().starts_with(prefix)
        {
            matches.push(entry.path());
        }
    }
    Ok(matches)
}

pub fn recover_interrupted_install(
    parent: &Path,
    target: &Path,
    version: &str,
) -> Result<(), String> {
    let staging_prefix = format!(".{version}.staging-");
    for path in matching_directories(parent, &staging_prefix)? {
        fs::remove_dir_all(&path).map_err(|error| {
            format!(
                "Failed to remove stale staging directory {}: {error}",
                path.display()
            )
        })?;
    }
    if target.exists() {
        return Ok(());
    }

    let backup_prefix = format!(".{version}.backup-");
    let mut backups = matching_directories(parent, &backup_prefix)?;
    backups.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    if let Some(backup) = backups.pop() {
        fs::rename(&backup, target).map_err(|error| {
            format!(
                "Failed to restore interrupted installation from {}: {error}",
                backup.display()
            )
        })?;
    }
    Ok(())
}

pub fn clear_backups(parent: &Path, version: &str) -> Result<(), String> {
    let backup_prefix = format!(".{version}.backup-");
    for path in matching_directories(parent, &backup_prefix)? {
        fs::remove_dir_all(&path).map_err(|error| {
            format!("Failed to remove stale backup {}: {error}", path.display())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_directory() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("beatforge-recovery-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn removes_staging_and_restores_backup_when_target_is_missing() {
        let root = temp_directory();
        let staging = root.join(".1.0.staging-fixture");
        let backup = root.join(".1.0.backup-fixture");
        let target = root.join("1.0");
        fs::create_dir(&staging).unwrap();
        fs::create_dir(&backup).unwrap();
        fs::write(backup.join("marker"), b"healthy").unwrap();

        recover_interrupted_install(&root, &target, "1.0").unwrap();
        assert!(!staging.exists());
        assert!(target.join("marker").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
