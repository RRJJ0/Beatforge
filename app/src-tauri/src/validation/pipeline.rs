use crate::validation::{
    IssueSeverity, ValidationCode, ValidationIssue, ValidationLimits, ValidationReport,
    ValidationStatus,
    archive::extract_osz,
    limits::{RAW_DIRECTORY, VALIDATED_DIRECTORY, VALIDATION_REPORT_FILE, WORKING_DIRECTORY},
    osu::{SemanticValidationContext, parse_osu_file, validate_semantics},
    types::{VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION},
};
use chrono::{SecondsFormat, Utc};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

fn issue(code: ValidationCode, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        code,
        severity: IssueSeverity::Error,
        section: Some("Output".into()),
        line: None,
        message: message.into(),
        repairable: false,
    }
}

fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn relative_display(directory: &str, name: &str) -> String {
    format!("{directory}/{name}")
}

fn atomic_copy(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "Validated artifact has no parent directory".to_string())?;
    fs::create_dir(parent)
        .map_err(|error| format!("Could not create validated artifact directory: {error}"))?;
    let file_name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Validated artifact has an invalid filename".to_string())?;
    let partial = parent.join(format!(".{file_name}.partial"));
    let result = (|| {
        let mut input = fs::File::open(source)
            .map_err(|error| format!("Could not read validated artifact source: {error}"))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|error| format!("Could not create validated artifact: {error}"))?;
        std::io::copy(&mut input, &mut output)
            .map_err(|error| format!("Could not copy validated artifact: {error}"))?;
        output
            .sync_all()
            .map_err(|error| format!("Could not flush validated artifact: {error}"))?;
        drop(output);
        fs::rename(&partial, destination)
            .map_err(|error| format!("Could not publish validated artifact: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
        let _ = fs::remove_dir(parent);
    }
    result
}

fn write_report(job_root: &Path, report: &ValidationReport) -> Result<(), String> {
    let path = job_root.join(VALIDATION_REPORT_FILE);
    let partial = job_root.join(format!(".{VALIDATION_REPORT_FILE}.partial"));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("Could not serialize validation report: {error}"))?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|error| format!("Could not create validation report: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Could not write validation report: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not flush validation report: {error}"))?;
        drop(file);
        fs::rename(&partial, path)
            .map_err(|error| format!("Could not publish validation report: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(partial);
    }
    result
}

fn preserve_candidate(
    job_root: &Path,
    candidate: &Path,
) -> Result<(PathBuf, String), ValidationIssue> {
    if !candidate.exists() {
        return Err(issue(
            ValidationCode::OutputNotFound,
            "The worker output could not be found",
        ));
    }
    if !candidate.is_file() {
        return Err(issue(
            ValidationCode::UnsupportedOutputType,
            "The worker output is not a regular file",
        ));
    }
    let root = job_root.canonicalize().map_err(|_| {
        issue(
            ValidationCode::OutputOutsideJobRoot,
            "The managed job directory could not be resolved",
        )
    })?;
    let resolved = candidate.canonicalize().map_err(|_| {
        issue(
            ValidationCode::OutputNotFound,
            "The worker output could not be resolved",
        )
    })?;
    if !resolved.starts_with(&root) {
        return Err(issue(
            ValidationCode::OutputOutsideJobRoot,
            "The worker output is outside the managed job directory",
        ));
    }
    let extension = resolved
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "osu" | "osz") {
        return Err(issue(
            ValidationCode::UnsupportedOutputType,
            "Only .osu and .osz worker outputs are supported",
        ));
    }
    let name = resolved
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            issue(
                ValidationCode::UnsupportedOutputType,
                "The worker output filename is invalid",
            )
        })?
        .to_string();
    let raw_directory = root.join(RAW_DIRECTORY);
    fs::create_dir(&raw_directory).map_err(|_| {
        issue(
            ValidationCode::OutputOutsideJobRoot,
            "The immutable raw artifact directory could not be created",
        )
    })?;
    let raw_path = raw_directory.join(&name);
    fs::rename(&resolved, &raw_path).map_err(|_| {
        issue(
            ValidationCode::OutputNotFound,
            "The worker output could not be preserved as an immutable raw artifact",
        )
    })?;
    Ok((raw_path, name))
}

fn validate_preserved(
    job_root: &Path,
    raw_path: &Path,
    limits: ValidationLimits,
) -> Vec<ValidationIssue> {
    match raw_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("osu") => {
            let parsed = parse_osu_file(raw_path, limits);
            let mut issues = parsed.issues;
            if let Some(beatmap) = parsed.beatmap {
                issues.extend(
                    validate_semantics(&beatmap, SemanticValidationContext::default()).issues,
                );
            }
            issues
        }
        Some("osz") => {
            let working = job_root.join(WORKING_DIRECTORY);
            let extracted = working.join("extracted");
            let result = match fs::create_dir(&working) {
                Ok(()) => extract_osz(raw_path, &extracted, limits).issues,
                Err(error) => vec![issue(
                    ValidationCode::ArchiveInvalid,
                    format!("Could not create archive validation workspace: {error}"),
                )],
            };
            let _ = fs::remove_dir_all(&working);
            result
        }
        _ => vec![issue(
            ValidationCode::UnsupportedOutputType,
            "Only .osu and .osz worker outputs are supported",
        )],
    }
}

pub fn validate_candidate(
    job_id: &str,
    job_root: &Path,
    candidate: &Path,
    worker_warnings: &[String],
    limits: ValidationLimits,
) -> Result<ValidationReport, String> {
    let started_at = utc_now();
    let preserved = preserve_candidate(job_root, candidate);
    let (raw_path, name, mut issues) = match preserved {
        Ok((path, name)) => (Some(path), name, Vec::new()),
        Err(error) => (
            None,
            candidate
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("candidate")
                .to_string(),
            vec![error],
        ),
    };

    if let Some(path) = raw_path.as_deref() {
        issues.extend(validate_preserved(job_root, path, limits));
    }
    issues.extend(worker_warnings.iter().map(|message| ValidationIssue {
        code: ValidationCode::WorkerWarning,
        severity: IssueSeverity::Warning,
        section: None,
        line: None,
        message: message.clone(),
        repairable: false,
    }));
    let has_errors = issues
        .iter()
        .any(|item| item.severity == IssueSeverity::Error);
    let has_warnings = issues
        .iter()
        .any(|item| item.severity == IssueSeverity::Warning);
    let mut validated_path = None;
    if !has_errors && let Some(raw_path) = raw_path.as_deref() {
        let destination = job_root.join(VALIDATED_DIRECTORY).join(&name);
        atomic_copy(raw_path, &destination)?;
        validated_path = Some(relative_display(VALIDATED_DIRECTORY, &name));
    }
    let report = ValidationReport {
        schema_version: VALIDATION_SCHEMA_VERSION,
        validator_version: VALIDATOR_VERSION.into(),
        job_id: job_id.into(),
        status: if has_errors {
            ValidationStatus::Invalid
        } else if has_warnings {
            ValidationStatus::ValidWithWarnings
        } else {
            ValidationStatus::Valid
        },
        source_path: relative_display(RAW_DIRECTORY, &name),
        validated_path,
        issues,
        statistics: None,
        repairs: Vec::new(),
        started_at,
        completed_at: utc_now(),
    };
    if let Err(error) = write_report(job_root, &report) {
        if let Some(relative) = report.validated_path.as_deref() {
            let output = job_root.join(relative);
            let _ = fs::remove_file(&output);
            if let Some(parent) = output.parent() {
                let _ = fs::remove_dir(parent);
            }
        }
        return Err(error);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_job(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("beatforge-pipeline-{name}-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/validation")
            .join(name)
    }

    #[test]
    fn valid_candidate_is_preserved_reported_and_published() {
        let root = temporary_job("valid");
        let candidate = root.join("generated.osu");
        fs::copy(fixture("valid/minimal-standard.osu"), &candidate).unwrap();

        let report = validate_candidate(
            "BF-PIPELINE",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert!(report.can_complete());
        assert!(!candidate.exists());
        assert!(root.join("raw/generated.osu").is_file());
        assert!(root.join("validated/generated.osu").is_file());
        assert!(root.join(VALIDATION_REPORT_FILE).is_file());
        assert!(!root.join(WORKING_DIRECTORY).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_candidate_keeps_raw_and_never_publishes() {
        let root = temporary_job("invalid");
        let candidate = root.join("generated.osu");
        fs::copy(fixture("invalid/invalid-timing.osu"), &candidate).unwrap();

        let report = validate_candidate(
            "BF-PIPELINE",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Invalid);
        assert!(!report.can_complete());
        assert!(root.join("raw/generated.osu").is_file());
        assert!(!root.join(VALIDATED_DIRECTORY).exists());
        let persisted: ValidationReport =
            serde_json::from_slice(&fs::read(root.join(VALIDATION_REPORT_FILE)).unwrap()).unwrap();
        assert_eq!(persisted, report);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn candidate_outside_job_root_is_rejected_without_moving_it() {
        let root = temporary_job("outside-root");
        let outside = temporary_job("outside-file").join("generated.osu");
        fs::copy(fixture("valid/minimal-standard.osu"), &outside).unwrap();

        let report = validate_candidate(
            "BF-PIPELINE",
            &root,
            &outside,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Invalid);
        assert!(outside.is_file());
        assert!(
            report
                .issues
                .iter()
                .any(|item| item.code == ValidationCode::OutputOutsideJobRoot)
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside.parent().unwrap()).unwrap();
    }
}
