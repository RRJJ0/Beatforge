use crate::validation::{
    ValidationReport, ValidationStatus,
    limits::{RAW_DIRECTORY, VALIDATED_DIRECTORY},
};
use serde::Serialize;
use std::{
    fs,
    fs::OpenOptions,
    io,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

const IMPORT_HANDOFF_DIRECTORY: &str = "osu-import";
const IMPORT_HANDOFF_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

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

fn import_handoff_root(cache_root: &Path) -> Result<PathBuf, String> {
    let cache_root = cache_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed cache directory: {error}"))?;
    let handoff_root = cache_root.join(IMPORT_HANDOFF_DIRECTORY);
    fs::create_dir_all(&handoff_root)
        .map_err(|error| format!("Could not create the osu! import handoff directory: {error}"))?;
    let handoff_root = handoff_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve the osu! import handoff directory: {error}"))?;
    if handoff_root.parent() != Some(cache_root.as_path()) {
        return Err("The osu! import handoff directory escaped the managed cache".into());
    }
    Ok(handoff_root)
}

pub(crate) fn create_import_handoff(
    cache_root: &Path,
    job_id: &str,
    artifact: &Path,
) -> Result<PathBuf, String> {
    if !valid_job_id(job_id) {
        return Err("Invalid managed job identity".into());
    }
    let metadata = fs::symlink_metadata(artifact)
        .map_err(|error| format!("Could not inspect the import artifact: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The import artifact is not a regular managed file".into());
    }
    let extension = artifact
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "osu" | "osz") {
        return Err("Only managed .osu and .osz files can be imported".into());
    }

    let handoff_root = import_handoff_root(cache_root)?;
    let stem = artifact
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The import artifact filename is invalid".to_string())?;
    let mut destination = None;
    for ordinal in 1..=10_000 {
        let name = if ordinal == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({ordinal}).{extension}")
        };
        let candidate = handoff_root.join(name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut output) => {
                let copy_result = (|| {
                    let mut input = fs::File::open(artifact).map_err(|error| {
                        format!("Could not read the osu! import artifact: {error}")
                    })?;
                    io::copy(&mut input, &mut output).map_err(|error| {
                        format!("Could not copy the osu! import handoff: {error}")
                    })?;
                    output.sync_all().map_err(|error| {
                        format!("Could not flush the osu! import handoff: {error}")
                    })
                })();
                if let Err(error) = copy_result {
                    drop(output);
                    let _ = fs::remove_file(&candidate);
                    return Err(error);
                }
                destination = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("Could not create the osu! import handoff: {error}"));
            }
        }
    }
    let destination = destination
        .ok_or_else(|| "Could not allocate a unique osu! import handoff name".to_string())?;

    let copy_result = (|| {
        let copied = fs::symlink_metadata(&destination)
            .map_err(|error| format!("Could not inspect the osu! import handoff: {error}"))?;
        if !copied.is_file() || copied.file_type().is_symlink() || copied.len() != metadata.len() {
            return Err("The osu! import handoff copy is incomplete".into());
        }
        let destination = destination
            .canonicalize()
            .map_err(|error| format!("Could not resolve the osu! import handoff: {error}"))?;
        if destination.parent() != Some(handoff_root.as_path()) {
            return Err("The osu! import handoff escaped the managed cache".into());
        }
        Ok(destination)
    })();
    if copy_result.is_err() {
        let _ = fs::remove_file(&destination);
    }
    copy_result
}

fn cleanup_import_handoffs_older_than(
    cache_root: &Path,
    max_age: Duration,
) -> Result<usize, String> {
    let handoff_root = import_handoff_root(cache_root)?;
    let mut removed = 0;
    for entry in fs::read_dir(&handoff_root)
        .map_err(|error| format!("Could not inspect osu! import handoffs: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not inspect an import handoff: {error}"))?;
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not inspect an import handoff: {error}"))?;
        if !metadata.is_file() || entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
            continue;
        }
        let age = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok());
        if age.is_some_and(|age| age >= max_age) {
            fs::remove_file(entry.path())
                .map_err(|error| format!("Could not remove a stale import handoff: {error}"))?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn cleanup_stale_import_handoffs(cache_root: &Path) -> Result<usize, String> {
    cleanup_import_handoffs_older_than(cache_root, IMPORT_HANDOFF_MAX_AGE)
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
    let jobs_root = jobs_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed jobs directory: {error}"))?;
    let job_root = jobs_root
        .join(job_id)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed job directory: {error}"))?;
    if job_root.parent() != Some(jobs_root.as_path()) {
        return Err("The job directory escaped the managed jobs directory".into());
    }
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

pub fn resolve_failed_artifact(
    jobs_root: &Path,
    job_id: &str,
    status: &str,
    report: Option<&ValidationReport>,
) -> Result<PathBuf, String> {
    if !valid_job_id(job_id) {
        return Err("Invalid managed job identity".into());
    }
    if status != "validation_failed" {
        return Err("Only a validation-failed job can use Import anyway".into());
    }
    let report = report.ok_or_else(|| "The job has no validation report".to_string())?;
    if report.status != ValidationStatus::Invalid || report.job_id != job_id {
        return Err("The job validation report does not identify a failed artifact".into());
    }
    let components = Path::new(&report.source_path)
        .components()
        .collect::<Vec<_>>();
    if components.len() != 2
        || components[0] != Component::Normal(RAW_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
    {
        return Err("The validation report contains an unsafe raw artifact path".into());
    }
    let jobs_root = jobs_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed jobs directory: {error}"))?;
    let job_root = jobs_root
        .join(job_id)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed job directory: {error}"))?;
    if job_root.parent() != Some(jobs_root.as_path()) {
        return Err("The job directory escaped the managed jobs directory".into());
    }
    let raw_root = job_root
        .join(RAW_DIRECTORY)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed raw directory: {error}"))?;
    if raw_root.parent() != Some(job_root.as_path()) {
        return Err("The raw directory escaped the managed job directory".into());
    }
    let artifact = job_root.join(&report.source_path);
    let metadata = fs::symlink_metadata(&artifact)
        .map_err(|error| format!("Could not inspect the raw artifact: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The raw artifact is not a regular managed file".into());
    }
    let artifact = artifact
        .canonicalize()
        .map_err(|error| format!("Could not resolve the raw artifact: {error}"))?;
    if artifact.parent() != Some(raw_root.as_path()) {
        return Err("The raw artifact escaped its managed directory".into());
    }
    let extension = artifact
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "osu" | "osz") {
        return Err("Only managed .osu and .osz files can be imported".into());
    }
    Ok(artifact)
}

pub fn resolve_unfiltered_set_artifact(
    jobs_root: &Path,
    job_id: &str,
    status: &str,
    provenance: Option<&serde_json::Value>,
) -> Result<PathBuf, String> {
    if !valid_job_id(job_id) {
        return Err("Invalid managed job identity".into());
    }
    if status != "completed" {
        return Err("Only a completed beatmap set can import excluded difficulties".into());
    }
    let provenance = provenance.ok_or_else(|| "The job has no provenance record".to_string())?;
    let has_excluded = provenance
        .get("difficulties")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.get("status").and_then(serde_json::Value::as_str) == Some("validation_failed")
            })
        });
    if !has_excluded {
        return Err("The beatmap set has no validation-failed difficulty to import".into());
    }
    let name = provenance
        .get("unfiltered_output")
        .and_then(|value| value.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("beatmap-set.osz");
    if Path::new(name).components().count() != 1 || !name.to_ascii_lowercase().ends_with(".osz") {
        return Err("The provenance record contains an unsafe unfiltered artifact name".into());
    }
    let jobs_root = jobs_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed jobs directory: {error}"))?;
    let job_root = jobs_root
        .join(job_id)
        .canonicalize()
        .map_err(|error| format!("Could not resolve the managed job directory: {error}"))?;
    if job_root.parent() != Some(jobs_root.as_path()) {
        return Err("The job directory escaped the managed jobs directory".into());
    }
    let artifact = job_root.join(name);
    let metadata = fs::symlink_metadata(&artifact)
        .map_err(|error| format!("Could not inspect the unfiltered beatmap set: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The unfiltered beatmap set is not a regular managed file".into());
    }
    let artifact = artifact
        .canonicalize()
        .map_err(|error| format!("Could not resolve the unfiltered beatmap set: {error}"))?;
    if artifact.parent() != Some(job_root.as_path()) {
        return Err("The unfiltered beatmap set escaped its managed job directory".into());
    }
    Ok(artifact)
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
    cache_root: &Path,
    job_id: &str,
    executable: Option<&Path>,
    allow_file_association: bool,
) -> Result<ImportLaunch, String> {
    #[cfg(windows)]
    {
        let handoff = create_import_handoff(cache_root, job_id, artifact)?;
        if let Some(executable) = executable {
            let mut command = Command::new(executable);
            command.arg(&handoff);
            hidden(&mut command);
            if let Err(error) = command.spawn() {
                let _ = fs::remove_file(&handoff);
                return Err(format!("Could not start osu!: {error}"));
            }
            return Ok(ImportLaunch {
                method: "executable".into(),
                application: Some(path_string(executable)),
                artifact: path_string(&handoff),
            });
        }
        if !allow_file_association {
            let _ = fs::remove_file(&handoff);
            return Err("No usable osu! executable was detected".into());
        }
        let mut command = Command::new("explorer.exe");
        command.arg(&handoff);
        hidden(&mut command);
        if let Err(error) = command.spawn() {
            let _ = fs::remove_file(&handoff);
            return Err(format!("Could not open the osu! file association: {error}"));
        }
        Ok(ImportLaunch {
            method: "file_association".into(),
            application: None,
            artifact: path_string(&handoff),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (
            artifact,
            cache_root,
            job_id,
            executable,
            allow_file_association,
        );
        Err("Importing into osu! is currently supported on Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::types::{VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION};
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

    fn invalid_report(job_id: &str, source: &str) -> ValidationReport {
        ValidationReport {
            schema_version: VALIDATION_SCHEMA_VERSION,
            validator_version: VALIDATOR_VERSION.into(),
            job_id: job_id.into(),
            status: ValidationStatus::Invalid,
            source_path: source.into(),
            validated_path: None,
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
    fn resolves_only_the_reported_raw_artifact_for_a_validation_failure() {
        let root = temporary_root("failed");
        let job = root.join("BF-TEST");
        fs::create_dir_all(job.join(RAW_DIRECTORY)).unwrap();
        let artifact = job.join(RAW_DIRECTORY).join("result.osz");
        fs::write(&artifact, b"invalid but importable archive").unwrap();

        let resolved = resolve_failed_artifact(
            &root,
            "BF-TEST",
            "validation_failed",
            Some(&invalid_report("BF-TEST", "raw/result.osz")),
        )
        .unwrap();
        assert_eq!(resolved, artifact.canonicalize().unwrap());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_an_unfiltered_multi_difficulty_set_for_explicit_import() {
        let root = temporary_root("unfiltered-set");
        let job = root.join("BF-SET");
        fs::create_dir_all(&job).unwrap();
        let artifact = job.join("beatmap-set.osz");
        fs::write(&artifact, b"full set with an invalid difficulty").unwrap();
        let provenance = serde_json::json!({
            "difficulties": [
                {"id": "normal", "status": "validated"},
                {"id": "insane", "status": "validation_failed"}
            ]
        });

        let resolved =
            resolve_unfiltered_set_artifact(&root, "BF-SET", "completed", Some(&provenance))
                .unwrap();

        assert_eq!(resolved, artifact.canonicalize().unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unfiltered_set_import_requires_an_excluded_difficulty() {
        let root = temporary_root("unfiltered-set-reject");
        let job = root.join("BF-SET");
        fs::create_dir_all(&job).unwrap();
        fs::write(job.join("beatmap-set.osz"), b"set").unwrap();
        let provenance = serde_json::json!({
            "difficulties": [{"id": "normal", "status": "validated"}]
        });

        assert!(
            resolve_unfiltered_set_artifact(&root, "BF-SET", "completed", Some(&provenance),)
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_import_rejects_other_statuses_identities_and_paths() {
        let root = temporary_root("failed-reject");
        let job = root.join("BF-TEST");
        fs::create_dir_all(job.join(RAW_DIRECTORY)).unwrap();
        fs::write(job.join(RAW_DIRECTORY).join("result.osz"), b"fixture").unwrap();

        assert!(
            resolve_failed_artifact(
                &root,
                "BF-TEST",
                "completed",
                Some(&invalid_report("BF-TEST", "raw/result.osz")),
            )
            .is_err()
        );
        assert!(
            resolve_failed_artifact(
                &root,
                "BF-TEST",
                "validation_failed",
                Some(&invalid_report("BF-OTHER", "raw/result.osz")),
            )
            .is_err()
        );
        assert!(
            resolve_failed_artifact(
                &root,
                "BF-TEST",
                "validation_failed",
                Some(&invalid_report("BF-TEST", "../result.osz")),
            )
            .is_err()
        );
        assert!(
            resolve_failed_artifact(
                &root,
                "BF-TEST",
                "validation_failed",
                Some(&report("BF-TEST", "validated/result.osz")),
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

    #[test]
    fn import_handoff_is_a_managed_copy_and_preserves_the_validated_artifact() {
        let root = temporary_root("handoff");
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let validated = root.join("validated.osz");
        fs::write(&validated, b"validated beatmap").unwrap();

        let handoff = create_import_handoff(&cache, "BF-TEST", &validated).unwrap();
        let second = create_import_handoff(&cache, "BF-TEST", &validated).unwrap();
        assert!(validated.is_file());
        assert_eq!(fs::read(&validated).unwrap(), fs::read(&handoff).unwrap());
        assert_eq!(handoff.file_name().unwrap(), "validated.osz");
        assert_eq!(second.file_name().unwrap(), "validated (2).osz");
        assert_eq!(
            handoff.parent().unwrap(),
            cache.join(IMPORT_HANDOFF_DIRECTORY).canonicalize().unwrap()
        );
        assert_ne!(handoff, validated.canonicalize().unwrap());

        assert_eq!(
            cleanup_import_handoffs_older_than(&cache, Duration::ZERO).unwrap(),
            2
        );
        assert!(!handoff.exists());
        assert!(!second.exists());
        assert!(validated.is_file());
        assert_eq!(fs::read(&validated).unwrap(), b"validated beatmap");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_handoff_rejects_invalid_identity_and_file_type() {
        let root = temporary_root("handoff-reject");
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let unsupported = root.join("result.zip");
        fs::write(&unsupported, b"archive").unwrap();

        assert!(create_import_handoff(&cache, "../outside", &unsupported).is_err());
        assert!(create_import_handoff(&cache, "BF-TEST", &unsupported).is_err());

        fs::remove_dir_all(root).unwrap();
    }
}
