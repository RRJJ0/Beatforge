use crate::validation::{
    BeatmapStatistics, IssueSeverity, RepairManifest, RepairManifestStatus, RepairRecord,
    ValidationCode, ValidationIssue, ValidationLimits, ValidationReport, ValidationStatus,
    archive::{extract_osz, package_osz_from_directory},
    limits::{
        RAW_DIRECTORY, REPAIR_MANIFEST_FILE, VALIDATED_DIRECTORY, VALIDATION_REPORT_FILE,
        WORKING_DIRECTORY,
    },
    osu::{SemanticValidationContext, parse_osu_file, validate_semantics},
    repair::{normalize_osu_bytes, repair_extracted_archive},
    statistics::{calculate_archive_statistics, calculate_file_statistics},
    types::{REPAIR_MANIFEST_SCHEMA_VERSION, VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION},
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

/// For a generated multi-difficulty set, remove only members that fail
/// structural or gameplay validation before the final set-level validation.
/// The caller records the excluded member ids in provenance.
pub fn filter_invalid_set_difficulties(
    job_root: &Path,
    candidate: &Path,
    limits: ValidationLimits,
) -> Result<(PathBuf, Vec<(usize, Option<BeatmapStatistics>)>), String> {
    let work = job_root.join("set-filter-work");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("Could not reset set validation workspace: {error}"))?;
    }
    fs::create_dir(&work)
        .map_err(|error| format!("Could not create set validation workspace: {error}"))?;
    let extracted_path = work.join("extracted");
    let extracted = extract_osz(candidate, &extracted_path, limits);
    let Some(extracted) = extracted.value else {
        let _ = fs::remove_dir_all(&work);
        return Ok((candidate.to_path_buf(), Vec::new()));
    };

    let mut results = Vec::new();
    for beatmap_path in &extracted.inspection.beatmap_paths {
        let index = beatmap_path
            .strip_prefix("difficulty-")
            .and_then(|value| value.strip_suffix(".osu"))
            .and_then(|value| value.parse::<usize>().ok());
        if let Some(index) = index {
            results.push((
                index,
                calculate_file_statistics(&extracted.root.join(beatmap_path), limits),
            ));
        }
    }
    let excluded = results
        .iter()
        .filter_map(|(index, statistics)| statistics.is_none().then_some(*index))
        .collect::<Vec<_>>();

    if excluded.is_empty() || excluded.len() == extracted.inspection.beatmap_paths.len() {
        let _ = fs::remove_dir_all(&work);
        return Ok((candidate.to_path_buf(), results));
    }
    for index in &excluded {
        fs::remove_file(extracted.root.join(format!("difficulty-{index}.osu")))
            .map_err(|error| format!("Could not exclude invalid set difficulty: {error}"))?;
    }
    let filtered = job_root.join("beatmap-set-filtered.osz");
    if filtered.exists() {
        fs::remove_file(&filtered)
            .map_err(|error| format!("Could not replace filtered beatmap set: {error}"))?;
    }
    let packaged = package_osz_from_directory(&extracted.root, &filtered, limits);
    let _ = fs::remove_dir_all(&work);
    if packaged.value.is_none() || packaged.has_errors() {
        let details = packaged
            .issues
            .into_iter()
            .map(|issue| issue.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "Could not package valid set difficulties: {details}"
        ));
    }
    Ok((filtered, results))
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

fn write_json<T: serde::Serialize>(
    job_root: &Path,
    file_name: &str,
    value: &T,
) -> Result<(), String> {
    let path = job_root.join(file_name);
    let partial = job_root.join(format!(".{file_name}.partial"));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Could not serialize {file_name}: {error}"))?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|error| format!("Could not create {file_name}: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Could not write {file_name}: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not flush {file_name}: {error}"))?;
        drop(file);
        fs::rename(&partial, path)
            .map_err(|error| format!("Could not publish {file_name}: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(partial);
    }
    result
}

fn write_report(job_root: &Path, report: &ValidationReport) -> Result<(), String> {
    write_json(job_root, VALIDATION_REPORT_FILE, report)
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

fn validate_osu(path: &Path, limits: ValidationLimits) -> Vec<ValidationIssue> {
    let parsed = parse_osu_file(path, limits);
    let mut issues = parsed.issues;
    if let Some(beatmap) = parsed.beatmap {
        issues.extend(validate_semantics(&beatmap, SemanticValidationContext::default()).issues);
    }
    issues
}

struct ArtifactValidation {
    issues: Vec<ValidationIssue>,
    repairs: Vec<RepairRecord>,
    validated_path: Option<String>,
    repair_status: Option<RepairManifestStatus>,
    statistics: Option<BeatmapStatistics>,
}

fn contains_errors(issues: &[ValidationIssue]) -> bool {
    issues
        .iter()
        .any(|item| item.severity == IssueSeverity::Error)
}

fn validate_and_repair_osu(
    job_root: &Path,
    raw_path: &Path,
    name: &str,
    limits: ValidationLimits,
) -> Result<ArtifactValidation, String> {
    let original_issues = validate_osu(raw_path, limits);
    let source = fs::read(raw_path)
        .map_err(|error| format!("Could not read raw beatmap for repair: {error}"))?;
    let repaired = match normalize_osu_bytes(&source) {
        Ok(result) if !result.repairs.is_empty() => result,
        _ => {
            let mut validated_path = None;
            if !contains_errors(&original_issues) {
                let destination = job_root.join(VALIDATED_DIRECTORY).join(name);
                atomic_copy(raw_path, &destination)?;
                validated_path = Some(relative_display(VALIDATED_DIRECTORY, name));
            }
            return Ok(ArtifactValidation {
                statistics: (!contains_errors(&original_issues))
                    .then(|| calculate_file_statistics(raw_path, limits))
                    .flatten(),
                issues: original_issues,
                repairs: Vec::new(),
                validated_path,
                repair_status: None,
            });
        }
    };

    let working = job_root.join(WORKING_DIRECTORY);
    fs::create_dir(&working)
        .map_err(|error| format!("Could not create beatmap repair workspace: {error}"))?;
    let working_path = working.join(name);
    let result = (|| {
        fs::write(&working_path, repaired.bytes)
            .map_err(|error| format!("Could not write repaired working beatmap: {error}"))?;
        let issues = validate_osu(&working_path, limits);
        if contains_errors(&issues) {
            return Ok(ArtifactValidation {
                statistics: None,
                issues,
                repairs: repaired.repairs,
                validated_path: None,
                repair_status: Some(RepairManifestStatus::Failed),
            });
        }
        let destination = job_root.join(VALIDATED_DIRECTORY).join(name);
        atomic_copy(&working_path, &destination)?;
        Ok(ArtifactValidation {
            statistics: calculate_file_statistics(&working_path, limits),
            issues,
            repairs: repaired.repairs,
            validated_path: Some(relative_display(VALIDATED_DIRECTORY, name)),
            repair_status: Some(RepairManifestStatus::Applied),
        })
    })();
    let _ = fs::remove_dir_all(&working);
    result
}

fn validate_and_repair_osz(
    job_root: &Path,
    raw_path: &Path,
    name: &str,
    limits: ValidationLimits,
) -> Result<ArtifactValidation, String> {
    let working = job_root.join(WORKING_DIRECTORY);
    fs::create_dir(&working)
        .map_err(|error| format!("Could not create archive repair workspace: {error}"))?;
    let extracted_path = working.join("extracted");
    let extracted = extract_osz(raw_path, &extracted_path, limits);
    let original_issues = extracted.issues;
    let Some(extracted) = extracted.value else {
        let _ = fs::remove_dir_all(&working);
        return Ok(ArtifactValidation {
            statistics: None,
            issues: original_issues,
            repairs: Vec::new(),
            validated_path: None,
            repair_status: None,
        });
    };
    let repairs = match repair_extracted_archive(&extracted.root, &extracted.inspection) {
        Ok(repairs) => repairs,
        Err(_) => {
            let mut validated_path = None;
            let statistics = (!contains_errors(&original_issues))
                .then(|| {
                    calculate_archive_statistics(
                        &extracted.root,
                        &extracted.inspection.beatmap_paths,
                        limits,
                    )
                })
                .flatten();
            if !contains_errors(&original_issues) {
                let destination = job_root.join(VALIDATED_DIRECTORY).join(name);
                atomic_copy(raw_path, &destination)?;
                validated_path = Some(relative_display(VALIDATED_DIRECTORY, name));
            }
            let _ = fs::remove_dir_all(&working);
            return Ok(ArtifactValidation {
                statistics,
                issues: original_issues,
                repairs: Vec::new(),
                validated_path,
                repair_status: None,
            });
        }
    };
    if repairs.is_empty() {
        let mut validated_path = None;
        let statistics = (!contains_errors(&original_issues))
            .then(|| {
                calculate_archive_statistics(
                    &extracted.root,
                    &extracted.inspection.beatmap_paths,
                    limits,
                )
            })
            .flatten();
        if !contains_errors(&original_issues) {
            let destination = job_root.join(VALIDATED_DIRECTORY).join(name);
            atomic_copy(raw_path, &destination)?;
            validated_path = Some(relative_display(VALIDATED_DIRECTORY, name));
        }
        let _ = fs::remove_dir_all(&working);
        return Ok(ArtifactValidation {
            statistics,
            issues: original_issues,
            repairs,
            validated_path,
            repair_status: None,
        });
    }

    let validated_directory = job_root.join(VALIDATED_DIRECTORY);
    fs::create_dir(&validated_directory)
        .map_err(|error| format!("Could not create repaired artifact directory: {error}"))?;
    let destination = validated_directory.join(name);
    let statistics =
        calculate_archive_statistics(&extracted.root, &extracted.inspection.beatmap_paths, limits);
    let packaged = package_osz_from_directory(&extracted.root, &destination, limits);
    let _ = fs::remove_dir_all(&working);
    if packaged.has_errors() || packaged.value.is_none() {
        let _ = fs::remove_file(&destination);
        let _ = fs::remove_dir(&validated_directory);
        return Ok(ArtifactValidation {
            statistics: None,
            issues: packaged.issues,
            repairs,
            validated_path: None,
            repair_status: Some(RepairManifestStatus::Failed),
        });
    }
    Ok(ArtifactValidation {
        statistics,
        issues: packaged.issues,
        repairs,
        validated_path: Some(relative_display(VALIDATED_DIRECTORY, name)),
        repair_status: Some(RepairManifestStatus::Applied),
    })
}

fn validate_and_repair(
    job_root: &Path,
    raw_path: &Path,
    name: &str,
    limits: ValidationLimits,
) -> Result<ArtifactValidation, String> {
    match raw_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("osu") => validate_and_repair_osu(job_root, raw_path, name, limits),
        Some("osz") => validate_and_repair_osz(job_root, raw_path, name, limits),
        _ => Ok(ArtifactValidation {
            statistics: None,
            issues: vec![issue(
                ValidationCode::UnsupportedOutputType,
                "Only .osu and .osz worker outputs are supported",
            )],
            repairs: Vec::new(),
            validated_path: None,
            repair_status: None,
        }),
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
    let (name, mut artifact) = match preserved {
        Ok((path, name)) => {
            let artifact = validate_and_repair(job_root, &path, &name, limits)?;
            (name, artifact)
        }
        Err(error) => (
            candidate
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("candidate")
                .to_string(),
            ArtifactValidation {
                statistics: None,
                issues: vec![error],
                repairs: Vec::new(),
                validated_path: None,
                repair_status: None,
            },
        ),
    };

    artifact
        .issues
        .extend(worker_warnings.iter().map(|message| ValidationIssue {
            code: ValidationCode::WorkerWarning,
            severity: IssueSeverity::Warning,
            section: None,
            line: None,
            message: message.clone(),
            repairable: false,
        }));
    if artifact.repair_status == Some(RepairManifestStatus::Applied) {
        artifact.issues.push(ValidationIssue {
            code: ValidationCode::AutoRepairApplied,
            severity: IssueSeverity::Info,
            section: None,
            line: None,
            message: format!(
                "Applied {} allowlisted non-destructive repair(s)",
                artifact.repairs.len()
            ),
            repairable: false,
        });
    }
    let has_errors = artifact
        .issues
        .iter()
        .any(|item| item.severity == IssueSeverity::Error);
    let has_warnings = artifact
        .issues
        .iter()
        .any(|item| item.severity == IssueSeverity::Warning);
    if let Some(status) = artifact.repair_status {
        let manifest = RepairManifest {
            schema_version: REPAIR_MANIFEST_SCHEMA_VERSION,
            job_id: job_id.into(),
            status,
            source_path: relative_display(RAW_DIRECTORY, &name),
            validated_path: artifact.validated_path.clone(),
            repairs: artifact.repairs.clone(),
            completed_at: utc_now(),
        };
        if let Err(error) = write_json(job_root, REPAIR_MANIFEST_FILE, &manifest) {
            if let Some(relative) = artifact.validated_path.as_deref() {
                let _ = fs::remove_file(job_root.join(relative));
                let _ = fs::remove_dir(job_root.join(VALIDATED_DIRECTORY));
            }
            return Err(error);
        }
    }
    let report = ValidationReport {
        schema_version: VALIDATION_SCHEMA_VERSION,
        validator_version: VALIDATOR_VERSION.into(),
        job_id: job_id.into(),
        status: if has_errors {
            ValidationStatus::Invalid
        } else if artifact.repair_status == Some(RepairManifestStatus::Applied) {
            ValidationStatus::Repaired
        } else if has_warnings {
            ValidationStatus::ValidWithWarnings
        } else {
            ValidationStatus::Valid
        },
        source_path: relative_display(RAW_DIRECTORY, &name),
        validated_path: artifact.validated_path,
        issues: artifact.issues,
        statistics: if has_errors {
            None
        } else {
            artifact.statistics
        },
        repairs: artifact.repairs,
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
        let _ = fs::remove_file(job_root.join(REPAIR_MANIFEST_FILE));
        return Err(error);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

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

    fn write_osz(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn multi_difficulty_filter_excludes_only_invalid_members() {
        let root = temporary_job("set-filter");
        let candidate = root.join("beatmap-set.osz");
        let valid = fs::read(fixture("valid/minimal-standard.osu")).unwrap();
        let invalid = fs::read(fixture("invalid/invalid-timing.osu")).unwrap();
        write_osz(
            &candidate,
            &[
                ("difficulty-1.osu", &valid),
                ("difficulty-2.osu", &invalid),
                ("fixture.wav", b"audio"),
            ],
        );

        let (filtered, results) =
            filter_invalid_set_difficulties(&root, &candidate, ValidationLimits::default())
                .unwrap();

        assert_eq!(
            results
                .iter()
                .filter_map(|(index, statistics)| statistics.is_none().then_some(*index))
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(results[0].1.is_some());
        assert_ne!(filtered, candidate);
        let inspection = crate::validation::inspect_osz(&filtered, ValidationLimits::default())
            .value
            .unwrap();
        assert_eq!(inspection.beatmap_paths, vec!["difficulty-1.osu"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn packaged_background_reference_is_validated_as_a_shared_asset() {
        let root = temporary_job("background");
        let candidate = root.join("beatmap-set.osz");
        let map = fs::read_to_string(fixture("valid/minimal-standard.osu"))
            .unwrap()
            .replace(
                "//Background and Video events",
                "//Background and Video events\n0,0,\"beatforge-background.jpg\",0,0",
            );
        write_osz(
            &candidate,
            &[
                ("difficulty-1.osu", map.as_bytes()),
                ("fixture.wav", b"audio"),
                ("beatforge-background.jpg", b"background"),
            ],
        );

        let report = validate_candidate(
            "BF-BACKGROUND",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert!(report.can_complete());
        assert_eq!(report.statistics.unwrap().beatmap_count, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn packaged_video_reference_is_validated_as_a_shared_asset() {
        let root = temporary_job("video");
        let candidate = root.join("beatmap-set.osz");
        let map = fs::read_to_string(fixture("valid/minimal-standard.osu"))
            .unwrap()
            .replace(
                "//Background and Video events",
                "//Background and Video events\nVideo,1250,\"beatforge-video.mp4\",0,0",
            );
        write_osz(
            &candidate,
            &[
                ("difficulty-1.osu", map.as_bytes()),
                ("fixture.wav", b"audio"),
                ("beatforge-video.mp4", b"video"),
            ],
        );

        let report = validate_candidate(
            "BF-VIDEO",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert!(report.can_complete());
        assert_eq!(report.statistics.unwrap().beatmap_count, 1);
        fs::remove_dir_all(root).unwrap();
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
        let statistics = report.statistics.as_ref().unwrap();
        assert_eq!(statistics.beatmap_count, 1);
        assert_eq!(statistics.circle_count, 1);
        assert_eq!(statistics.slider_count, 1);
        assert_eq!(statistics.spinner_count, 1);
        assert_eq!(statistics.bpm_min, Some(120.0));
        assert_eq!(statistics.duration_ms, Some(2000));
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
        assert!(report.statistics.is_none());
        assert!(root.join("raw/generated.osu").is_file());
        assert!(!root.join(VALIDATED_DIRECTORY).exists());
        let persisted: ValidationReport =
            serde_json::from_slice(&fs::read(root.join(VALIDATION_REPORT_FILE)).unwrap()).unwrap();
        assert_eq!(persisted, report);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allowlisted_repairs_create_audited_copy_without_touching_raw() {
        let root = temporary_job("repair");
        let candidate = root.join("generated.osu");
        let fixture = fs::read_to_string(fixture("valid/minimal-standard.osu")).unwrap();
        let source = format!(
            "\u{feff}{}",
            fixture
                .trim_end_matches('\n')
                .replace("CircleSize:4", "CircleSize:04.000")
                .replace('\n', "\r\n")
        )
        .into_bytes();
        fs::write(&candidate, &source).unwrap();

        let report = validate_candidate(
            "BF-REPAIR",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Repaired);
        assert!(report.can_complete());
        assert_eq!(report.statistics.as_ref().unwrap().beatmap_count, 1);
        assert_eq!(fs::read(root.join("raw/generated.osu")).unwrap(), source);
        let validated = fs::read(root.join("validated/generated.osu")).unwrap();
        assert!(!validated.starts_with(b"\xef\xbb\xbf"));
        assert!(!validated.contains(&b'\r'));
        assert!(validated.ends_with(b"\n"));
        assert!(
            String::from_utf8(validated)
                .unwrap()
                .contains("CircleSize:4\n")
        );
        let manifest: RepairManifest =
            serde_json::from_slice(&fs::read(root.join(REPAIR_MANIFEST_FILE)).unwrap()).unwrap();
        assert_eq!(manifest.status, RepairManifestStatus::Applied);
        assert_eq!(manifest.repairs, report.repairs);
        assert!(!root.join(WORKING_DIRECTORY).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_repair_keeps_raw_and_remains_non_completable() {
        let root = temporary_job("repair-failed");
        let candidate = root.join("generated.osu");
        let source = fs::read_to_string(fixture("invalid/invalid-timing.osu"))
            .unwrap()
            .replace('\n', "\r\n");
        fs::write(&candidate, source).unwrap();

        let report = validate_candidate(
            "BF-REPAIR-FAILED",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Invalid);
        assert!(!report.can_complete());
        assert!(report.statistics.is_none());
        assert!(root.join("raw/generated.osu").is_file());
        assert!(!root.join(VALIDATED_DIRECTORY).exists());
        let manifest: RepairManifest =
            serde_json::from_slice(&fs::read(root.join(REPAIR_MANIFEST_FILE)).unwrap()).unwrap();
        assert_eq!(manifest.status, RepairManifestStatus::Failed);
        assert!(manifest.validated_path.is_none());
        assert!(!root.join(WORKING_DIRECTORY).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_repairs_are_repacked_and_fully_revalidated() {
        let root = temporary_job("repair-archive");
        let candidate = root.join("generated.osz");
        let map = fs::read_to_string(fixture("valid/minimal-standard.osu"))
            .unwrap()
            .replace('\n', "\r\n");
        write_osz(
            &candidate,
            &[
                ("folder\\map.osu", map.as_bytes()),
                ("folder\\fixture.wav", b"synthetic audio"),
            ],
        );

        let report = validate_candidate(
            "BF-REPAIR-ARCHIVE",
            &root,
            &candidate,
            &[],
            ValidationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.status, ValidationStatus::Repaired);
        assert!(report.can_complete());
        assert_eq!(report.statistics.as_ref().unwrap().beatmap_count, 1);
        assert!(root.join("raw/generated.osz").is_file());
        let validated = root.join("validated/generated.osz");
        assert!(validated.is_file());
        let inspection = crate::validation::inspect_osz(&validated, ValidationLimits::default());
        assert!(!inspection.has_errors(), "{:?}", inspection.issues);
        assert!(
            report
                .repairs
                .iter()
                .any(|repair| repair.code == "ARCHIVE_FILENAME_NORMALIZED")
        );
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
