use crate::validation::{ValidationReport, limits::VALIDATED_DIRECTORY};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLaunch {
    pub method: String,
    pub application: Option<String>,
    pub artifact: String,
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn valid_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id != "."
        && job_id != ".."
        && !job_id.contains(['/', '\\'])
        && Path::new(job_id).components().count() == 1
}

pub fn resolve_validated_artifact(
    jobs_root: &Path,
    job_id: &str,
    status: &str,
    stored_output_path: Option<&str>,
    report: Option<&ValidationReport>,
) -> Result<PathBuf, String> {
    if !valid_job_id(job_id) {
        return Err("Invalid managed job identity".into());
    }
    if status != "completed" {
        return Err("Only a completed job can be opened or imported".into());
    }
    let report = report.ok_or_else(|| "The job has no validation report".to_string())?;
    if !report.can_complete() || report.job_id != job_id {
        return Err("The job validation report does not permit delivery".into());
    }
    let relative = report
        .validated_path
        .as_deref()
        .ok_or_else(|| "The validation report has no deliverable path".to_string())?;
    let components = Path::new(relative).components().collect::<Vec<_>>();
    if components.len() != 2
        || components[0] != Component::Normal(VALIDATED_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
    {
        return Err("The validation report contains an unsafe deliverable path".into());
    }
    let job_root = jobs_root
        .join(job_id)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed job directory: {error}"))?;
    let validated_root = job_root
        .join(VALIDATED_DIRECTORY)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed validated directory: {error}"))?;
    if validated_root.parent() != Some(job_root.as_path()) {
        return Err("The validated directory escaped the managed job directory".into());
    }
    let expected = job_root.join(relative);
    let metadata = fs::symlink_metadata(&expected)
        .map_err(|error| format!("Could not inspect the validated artifact: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The validated artifact is not a regular managed file".into());
    }
    let expected = expected
        .canonicalize()
        .map_err(|error| format!("Could not resolve the validated artifact: {error}"))?;
    if expected.parent() != Some(validated_root.as_path()) {
        return Err("The validated artifact escaped its managed directory".into());
    }
    let stored = stored_output_path
        .ok_or_else(|| "The completed job has no stored output path".to_string())?;
    let stored = Path::new(stored)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the stored output path: {error}"))?;
    if stored != expected {
        return Err("The stored output path does not match the validation report".into());
    }
    let extension = expected
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !matches!(extension.to_ascii_lowercase().as_str(), "osu" | "osz") {
        return Err("Only validated .osu and .osz files can be delivered".into());
    }
    Ok(expected)
}

fn executable_from_path(path: &Path) -> Option<PathBuf> {
    let candidate = if path.is_dir() {
        path.join("osu!.exe")
    } else {
        path.to_path_buf()
    };
    let name_is_osu = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("osu!.exe"));
    if name_is_osu && candidate.is_file() {
        candidate.canonicalize().ok()
    } else {
        None
    }
}

pub fn select_osu_executable(
    configured_path: Option<&str>,
    detected_paths: impl IntoIterator<Item = PathBuf>,
) -> Result<Option<PathBuf>, String> {
    if let Some(configured) = configured_path.filter(|value| !value.trim().is_empty()) {
        return executable_from_path(Path::new(configured))
            .map(Some)
            .ok_or_else(|| "The configured osu! path does not contain osu!.exe".into());
    }
    Ok(detected_paths
        .into_iter()
        .find_map(|path| executable_from_path(&path)))
}

fn hidden(command: &mut Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

pub fn reveal_artifact(artifact: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut command = Command::new("explorer.exe");
        command.arg("/select,").arg(artifact);
        hidden(&mut command);
        command
            .spawn()
            .map_err(|error| format!("Could not open File Explorer: {error}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = artifact;
        Err("Opening the output folder is currently supported on Windows only".into())
    }
}

pub fn import_artifact(
    artifact: &Path,
    executable: Option<&Path>,
    allow_file_association: bool,
) -> Result<ImportLaunch, String> {
    #[cfg(windows)]
    {
        if let Some(executable) = executable {
            let mut command = Command::new(executable);
            command.arg(artifact);
            hidden(&mut command);
            command
                .spawn()
                .map_err(|error| format!("Could not start osu!: {error}"))?;
            return Ok(ImportLaunch {
                method: "executable".into(),
                application: Some(path_string(executable)),
                artifact: path_string(artifact),
            });
        }
        if !allow_file_association {
            return Err("No usable osu! executable was detected".into());
        }
        let mut command = Command::new("explorer.exe");
        command.arg(artifact);
        hidden(&mut command);
        command
            .spawn()
            .map_err(|error| format!("Could not open the osu! file association: {error}"))?;
        Ok(ImportLaunch {
            method: "file_association".into(),
            application: None,
            artifact: path_string(artifact),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (artifact, executable, allow_file_association);
        Err("Importing into osu! is currently supported on Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{
        ValidationStatus,
        types::{VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("beatforge-osu-integration-{name}-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn report(job_id: &str, relative: &str) -> ValidationReport {
        ValidationReport {
            schema_version: VALIDATION_SCHEMA_VERSION,
            validator_version: VALIDATOR_VERSION.into(),
            job_id: job_id.into(),
            status: ValidationStatus::Valid,
            source_path: "raw/result.osz".into(),
            validated_path: Some(relative.into()),
            issues: Vec::new(),
            statistics: None,
            repairs: Vec::new(),
            started_at: "2026-10-07T00:00:00Z".into(),
            completed_at: "2026-10-07T00:00:01Z".into(),
        }
    }

    #[test]
    fn resolves_only_the_reported_managed_validated_artifact() {
        let root = temporary_root("valid");
        let job = root.join("BF-TEST");
        fs::create_dir_all(job.join(VALIDATED_DIRECTORY)).unwrap();
        let artifact = job.join(VALIDATED_DIRECTORY).join("result.osz");
        fs::write(&artifact, b"fixture").unwrap();
        let resolved = resolve_validated_artifact(
            &root,
            "BF-TEST",
            "completed",
            Some(&path_string(&artifact)),
            Some(&report("BF-TEST", "validated/result.osz")),
        )
        .unwrap();
        assert_eq!(resolved, artifact.canonicalize().unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_raw_mismatched_and_non_completed_outputs() {
        let root = temporary_root("reject");
        let job = root.join("BF-TEST");
        fs::create_dir_all(job.join(VALIDATED_DIRECTORY)).unwrap();
        fs::create_dir_all(job.join("raw")).unwrap();
        let validated = job.join(VALIDATED_DIRECTORY).join("result.osz");
        let raw = job.join("raw/result.osz");
        fs::write(&validated, b"validated").unwrap();
        fs::write(&raw, b"raw").unwrap();
        assert!(
            resolve_validated_artifact(
                &root,
                "BF-TEST",
                "running",
                Some(&path_string(&validated)),
                Some(&report("BF-TEST", "validated/result.osz")),
            )
            .is_err()
        );
        assert!(
            resolve_validated_artifact(
                &root,
                "BF-TEST",
                "completed",
                Some(&path_string(&raw)),
                Some(&report("BF-TEST", "validated/result.osz")),
            )
            .is_err()
        );
        assert!(
            resolve_validated_artifact(
                &root,
                "BF-TEST",
                "completed",
                Some(&path_string(&raw)),
                Some(&report("BF-TEST", "raw/result.osz")),
            )
            .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configured_osu_path_must_resolve_to_named_executable() {
        let root = temporary_root("executable");
        fs::write(root.join("osu!.exe"), b"fixture").unwrap();
        fs::write(root.join("other.exe"), b"fixture").unwrap();
        assert_eq!(
            select_osu_executable(Some(root.to_str().unwrap()), Vec::new())
                .unwrap()
                .unwrap(),
            root.join("osu!.exe").canonicalize().unwrap()
        );
        assert!(
            select_osu_executable(Some(root.join("other.exe").to_str().unwrap()), Vec::new())
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
