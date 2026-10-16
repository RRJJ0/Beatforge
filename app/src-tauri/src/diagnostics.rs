use crate::{
    AppPaths, AppSettings, SETTINGS_SCHEMA_VERSION, SystemInfo, jobs, mapperatorinator_lock,
    packages,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const DIAGNOSTIC_SCHEMA_VERSION: u16 = 1;
const MAX_JOB_SUMMARIES: usize = 25;
const MAX_LOG_SUMMARIES: usize = 8;
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
static DIAGNOSTIC_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const PRIVACY_NOTICE: &str = "This diagnostic package contains allowlisted system, package, settings, job-state, validation-code, and log-summary metadata. It excludes songs, beatmaps, images, videos, generated artifacts, databases, raw log text, complete personal paths, usernames, song titles, artist names, and free-text error messages. Review the file list before sharing.";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticFilePreview {
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticPreview {
    pub schema_version: u16,
    pub diagnostic_id: String,
    pub files: Vec<DiagnosticFilePreview>,
    pub excluded: Vec<String>,
    pub privacy_notice: String,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticExportResult {
    pub path: String,
    pub diagnostic_id: String,
    pub files: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticManifestEntry {
    name: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticManifest {
    schema_version: u16,
    diagnostic_id: String,
    created_at: String,
    app_version: &'static str,
    files: Vec<DiagnosticManifestEntry>,
    excluded: Vec<String>,
    privacy_notice: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeSettings {
    schema_version: u16,
    device: &'static str,
    precision: &'static str,
    workers: u8,
    auto_import: bool,
    keep_intermediate: bool,
    repair_generated_output: bool,
    check_updates: bool,
    desktop_notifications: bool,
    locale: &'static str,
    path_fields: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SafePackage {
    id: String,
    kind: packages::PackageKind,
    target_version: String,
    installed_version: Option<String>,
    status: packages::PackageStatus,
    progress: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentSummary {
    app_version: &'static str,
    mapperatorinator_commit: String,
    model_revision: String,
    packages: Vec<SafePackage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeJob {
    alias: String,
    status: String,
    progress: u8,
    difficulty_count: usize,
    validation_status: Option<crate::validation::ValidationStatus>,
    validation_errors: usize,
    validation_warnings: usize,
    validation_codes: Vec<crate::validation::ValidationCode>,
    output_available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogSummary {
    alias: String,
    size_bytes: u64,
    lines: usize,
    contained_invalid_utf8: bool,
    known_signals: BTreeMap<&'static str, usize>,
    content_included: bool,
}

#[derive(Debug)]
struct BundleEntry {
    name: &'static str,
    description: &'static str,
    bytes: Vec<u8>,
}

fn diagnostic_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = DIAGNOSTIC_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("BF-DIAG-{millis}-{sequence}")
}

fn valid_diagnostic_id(value: &str) -> bool {
    value.strip_prefix("BF-DIAG-").is_some_and(|tail| {
        !tail.is_empty()
            && tail
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-')
    }) && value.len() <= 64
}

fn json_entry<T: Serialize>(
    name: &'static str,
    description: &'static str,
    value: &T,
) -> Result<BundleEntry, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(BundleEntry {
        name,
        description,
        bytes,
    })
}

fn safe_jobs(connection: &Connection) -> Result<Vec<SafeJob>, String> {
    jobs::list(connection).map(|records| {
        records
            .into_iter()
            .take(MAX_JOB_SUMMARIES)
            .enumerate()
            .map(|(index, job)| {
                let report = job.validation_report.as_ref();
                let validation_codes = report
                    .map(|report| {
                        let mut codes = report
                            .issues
                            .iter()
                            .map(|issue| issue.code)
                            .collect::<Vec<_>>();
                        codes.sort_by_key(|code| format!("{code:?}"));
                        codes.dedup();
                        codes
                    })
                    .unwrap_or_default();
                SafeJob {
                    alias: format!("job-{:02}", index + 1),
                    status: match job.status.as_str() {
                        "queued" | "running" | "validating" | "completed" | "validation_failed"
                        | "failed" | "cancelled" | "interrupted" => job.status,
                        _ => "unknown".into(),
                    },
                    progress: job.progress,
                    difficulty_count: job.difficulty_count,
                    validation_status: report.map(|report| report.status),
                    validation_errors: report
                        .map(|report| {
                            report
                                .issues
                                .iter()
                                .filter(|issue| {
                                    issue.severity == crate::validation::IssueSeverity::Error
                                })
                                .count()
                        })
                        .unwrap_or(0),
                    validation_warnings: report
                        .map(|report| {
                            report
                                .issues
                                .iter()
                                .filter(|issue| {
                                    issue.severity == crate::validation::IssueSeverity::Warning
                                })
                                .count()
                        })
                        .unwrap_or(0),
                    validation_codes,
                    output_available: job.output_available,
                }
            })
            .collect()
    })
}

fn safe_packages(connection: &Connection) -> Result<Vec<SafePackage>, String> {
    packages::list(connection).map(|records| {
        records
            .into_iter()
            .filter(|package| {
                matches!(
                    package.id.as_str(),
                    packages::PYTHON_RUNTIME_ID | packages::STANDARD_MODEL_ID
                )
            })
            .map(|package| SafePackage {
                id: package.id,
                kind: package.kind,
                target_version: safe_version(&package.target_version),
                installed_version: package.installed_version.map(|value| safe_version(&value)),
                status: package.status,
                progress: package.progress,
            })
            .collect()
    })
}

fn safe_version(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_+".contains(&byte))
    {
        value.into()
    } else {
        "redacted".into()
    }
}

fn log_summaries(logs_root: &Path) -> Vec<LogSummary> {
    let Ok(entries) = fs::read_dir(logs_root) else {
        return Vec::new();
    };
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).ok()?;
            let is_log = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("log"));
            (metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= MAX_LOG_BYTES
                && is_log)
                .then_some((path, metadata))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, metadata)| std::cmp::Reverse(metadata.modified().ok()));
    candidates
        .into_iter()
        .take(MAX_LOG_SUMMARIES)
        .enumerate()
        .filter_map(|(index, (path, metadata))| {
            let bytes = fs::read(path).ok()?;
            let text = String::from_utf8_lossy(&bytes);
            let signals = [
                "CUDA_OUT_OF_MEMORY",
                "WORKER_TIMEOUT",
                "WORKER_EXITED",
                "WORKER_PROTOCOL_ERROR",
                "VALIDATION_FAILED",
            ];
            Some(LogSummary {
                alias: format!("log-{:02}", index + 1),
                size_bytes: metadata.len(),
                lines: text.lines().count(),
                contained_invalid_utf8: matches!(text, std::borrow::Cow::Owned(_)),
                known_signals: signals
                    .into_iter()
                    .filter_map(|signal| {
                        let count = text.matches(signal).count();
                        (count > 0).then_some((signal, count))
                    })
                    .collect(),
                content_included: false,
            })
        })
        .collect()
}

fn build_entries(
    connection: &Connection,
    paths: &AppPaths,
    system: &SystemInfo,
    settings: &AppSettings,
) -> Result<Vec<BundleEntry>, String> {
    let lock = mapperatorinator_lock::builtin()?;
    let environment = EnvironmentSummary {
        app_version: env!("CARGO_PKG_VERSION"),
        mapperatorinator_commit: lock.source.commit,
        model_revision: lock.model.revision,
        packages: safe_packages(connection)?,
    };
    let safe_settings = SafeSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        device: match settings.device.as_str() {
            "cuda" => "cuda",
            "cpu" => "cpu",
            _ => "unknown",
        },
        precision: match settings.precision.as_str() {
            "bf16" => "bf16",
            "fp32" => "fp32",
            _ => "unknown",
        },
        workers: settings.workers,
        auto_import: settings.auto_import,
        keep_intermediate: settings.keep_intermediate,
        repair_generated_output: settings.repair_generated_output,
        check_updates: settings.check_updates,
        desktop_notifications: settings.desktop_notifications,
        locale: match settings.locale.as_str() {
            "en" => "en",
            "zh-TW" => "zh-TW",
            _ => "unknown",
        },
        path_fields: "excluded",
    };
    Ok(vec![
        BundleEntry {
            name: "README.txt",
            description: "Privacy notice and sharing guidance",
            bytes: format!("Beatforge diagnostic package\r\n\r\n{PRIVACY_NOTICE}\r\n").into_bytes(),
        },
        json_entry(
            "system.json",
            "Operating system and hardware summary",
            system,
        )?,
        json_entry(
            "environment.json",
            "Application, runtime, model and package state",
            &environment,
        )?,
        json_entry(
            "settings.json",
            "Non-path application settings",
            &safe_settings,
        )?,
        json_entry(
            "jobs.json",
            "Pseudonymous recent job and validation state",
            &safe_jobs(connection)?,
        )?,
        json_entry(
            "recent-logs.json",
            "Recent log metadata and allowlisted signal counts; no raw text",
            &log_summaries(Path::new(&paths.logs)),
        )?,
    ])
}

fn excluded() -> Vec<String> {
    [
        "songs and source audio",
        "beatmap .osu/.osz contents and generated artifacts",
        "background images and videos",
        "SQLite databases",
        "raw log text and tracebacks",
        "complete paths, usernames, titles, artists and free-text errors",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn manifest(id: &str, entries: &[BundleEntry]) -> DiagnosticManifest {
    DiagnosticManifest {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION,
        diagnostic_id: id.to_string(),
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        app_version: env!("CARGO_PKG_VERSION"),
        files: entries
            .iter()
            .map(|entry| DiagnosticManifestEntry {
                name: entry.name.to_string(),
                size_bytes: entry.bytes.len() as u64,
                sha256: sha256(&entry.bytes),
            })
            .collect(),
        excluded: excluded(),
        privacy_notice: PRIVACY_NOTICE,
    }
}

pub fn preview(
    connection: &Connection,
    paths: &AppPaths,
    system: &SystemInfo,
    settings: &AppSettings,
) -> Result<DiagnosticPreview, String> {
    let id = diagnostic_id();
    let entries = build_entries(connection, paths, system, settings)?;
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest(&id, &entries)).map_err(|error| error.to_string())?;
    let mut files = vec![DiagnosticFilePreview {
        name: "manifest.json".into(),
        description: "Versioned inventory and SHA-256 checksums".into(),
        size_bytes: manifest_bytes.len() as u64,
    }];
    files.extend(entries.iter().map(|entry| DiagnosticFilePreview {
        name: entry.name.into(),
        description: entry.description.into(),
        size_bytes: entry.bytes.len() as u64,
    }));
    Ok(DiagnosticPreview {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION,
        diagnostic_id: id,
        total_bytes: files.iter().map(|file| file.size_bytes).sum(),
        files,
        excluded: excluded(),
        privacy_notice: PRIVACY_NOTICE.into(),
    })
}

fn zip_bytes(id: &str, entries: &[BundleEntry]) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let manifest =
        serde_json::to_vec_pretty(&manifest(id, entries)).map_err(|error| error.to_string())?;
    archive
        .start_file("manifest.json", options)
        .map_err(|error| format!("Could not create diagnostic manifest: {error}"))?;
    archive
        .write_all(&manifest)
        .map_err(|error| format!("Could not write diagnostic manifest: {error}"))?;
    for entry in entries {
        archive
            .start_file(entry.name, options)
            .map_err(|error| format!("Could not create diagnostic entry: {error}"))?;
        archive
            .write_all(&entry.bytes)
            .map_err(|error| format!("Could not write diagnostic entry: {error}"))?;
    }
    archive
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| format!("Could not finish diagnostic package: {error}"))
}

pub fn export(
    connection: &Connection,
    paths: &AppPaths,
    system: &SystemInfo,
    settings: &AppSettings,
    target: &Path,
    id: &str,
) -> Result<DiagnosticExportResult, String> {
    if !valid_diagnostic_id(id) {
        return Err("Invalid diagnostic identity".into());
    }
    if !target
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
    {
        return Err("Diagnostic export must use a .zip file".into());
    }
    let parent = target
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| "Diagnostic export folder does not exist".to_string())?;
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Diagnostic export filename is invalid".to_string())?;
    let target = parent.join(file_name);
    if target.exists() {
        return Err("Diagnostic export already exists; choose a new filename".into());
    }
    let entries = build_entries(connection, paths, system, settings)?;
    let bytes = zip_bytes(id, &entries)?;
    let partial = parent.join(format!(".{file_name}.partial"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|error| format!("Could not create diagnostic export: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Could not write diagnostic export: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not flush diagnostic export: {error}"))?;
        drop(file);
        fs::rename(&partial, &target)
            .map_err(|error| format!("Could not publish diagnostic export: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result?;
    Ok(DiagnosticExportResult {
        path: target.to_string_lossy().into_owned(),
        diagnostic_id: id.into(),
        files: entries.len() + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GpuInfo,
        protocol::StartJobInput,
        validation::{
            IssueSeverity, ValidationCode, ValidationIssue, ValidationReport, ValidationStatus,
            types::{VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION},
        },
    };
    use std::{fs::File, io::Read};
    use zip::ZipArchive;

    fn root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/diagnostic-test-temp")
            .join(format!("bundle-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn paths(root: &Path) -> AppPaths {
        let value = |name: &str| root.join(name).to_string_lossy().into_owned();
        let paths = AppPaths {
            root: root.to_string_lossy().into_owned(),
            models: value("models"),
            runtime: value("runtime"),
            cache: value("cache"),
            logs: value("logs"),
            jobs: value("jobs"),
            outputs: value("outputs"),
            database: value("secret-user.db"),
        };
        for directory in [
            &paths.models,
            &paths.runtime,
            &paths.cache,
            &paths.logs,
            &paths.jobs,
            &paths.outputs,
        ] {
            fs::create_dir_all(directory).unwrap();
        }
        paths
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
        packages::migrate(&connection).unwrap();
        packages::sync_manifest(&connection, &packages::builtin_manifest()).unwrap();
        connection
    }

    fn settings(root: &Path) -> AppSettings {
        AppSettings {
            output_path: root
                .join("Secret Song Output")
                .to_string_lossy()
                .into_owned(),
            temp_path: root.join("Secret User Temp").to_string_lossy().into_owned(),
            osu_path: r"C:\Users\SecretUser\AppData\Local\osu!".into(),
            device: "cuda".into(),
            precision: "bf16".into(),
            workers: 2,
            auto_import: true,
            keep_intermediate: false,
            repair_generated_output: true,
            check_updates: true,
            desktop_notifications: true,
            locale: "zh-TW".into(),
        }
    }

    fn system() -> SystemInfo {
        let gpu = GpuInfo {
            name: "Test GPU".into(),
            memory_total_mb: Some(8_192),
            memory_free_mb: Some(4_096),
            driver_version: Some("999.1".into()),
            compute_capability: Some("8.9".into()),
            available: true,
        };
        let gpu_compatibility = crate::gpu_compatibility::assess(crate::gpu_facts(&gpu));
        SystemInfo {
            os_name: "Windows".into(),
            os_version: "Test".into(),
            architecture: "x86_64".into(),
            cpu_name: "Test CPU".into(),
            cpu_cores: 8,
            total_memory_mb: 16_384,
            available_memory_mb: 8_192,
            available_disk_gb: 100,
            gpu,
            gpu_compatibility,
        }
    }

    #[test]
    fn exported_bundle_is_allowlisted_and_excludes_private_content() {
        let root = root();
        let paths = paths(&root);
        let connection = database();
        let input = StartJobInput {
            title: "TOP SECRET SONG TITLE".into(),
            artist: "PRIVATE ARTIST".into(),
            audio_file: r"C:\Users\SecretUser\Music\private-song.flac".into(),
            model: packages::STANDARD_MODEL_ID.into(),
            difficulty: 5.0,
            difficulties: vec![],
            gamemode: 0,
            seed: 123,
            descriptors: vec!["secret descriptor".into()],
            negative_descriptors: vec![],
            reference_beatmap: Some(r"C:\Users\SecretUser\private.osu".into()),
            background_image: Some(r"C:\Users\SecretUser\private.png".into()),
            video_file: Some(r"C:\Users\SecretUser\private.mp4".into()),
            video_start_time_ms: 0,
            approach_rate: Some(9.0),
            circle_size: Some(4.0),
            hitsounded: true,
            scenario: "success".into(),
        };
        jobs::insert(&connection, "BF-PRIVATE-ID", &input).unwrap();
        let report = ValidationReport {
            schema_version: VALIDATION_SCHEMA_VERSION,
            validator_version: VALIDATOR_VERSION.into(),
            job_id: "BF-PRIVATE-ID".into(),
            status: ValidationStatus::Invalid,
            source_path: "raw/private-song.osz".into(),
            validated_path: None,
            issues: vec![ValidationIssue {
                code: ValidationCode::TimingInvalid,
                severity: IssueSeverity::Error,
                section: Some("TOP SECRET SECTION".into()),
                line: Some(10),
                message: "C:\\Users\\SecretUser\\Music\\private-song.flac broke".into(),
                repairable: false,
            }],
            statistics: None,
            repairs: vec![],
            started_at: "2026-10-15T00:00:00Z".into(),
            completed_at: "2026-10-15T00:00:01Z".into(),
        };
        jobs::finish_validation(&connection, "BF-PRIVATE-ID", &report, None).unwrap();
        fs::write(
            Path::new(&paths.logs).join("job-BF-PRIVATE-ID.log"),
            b"CUDA_OUT_OF_MEMORY C:\\Users\\SecretUser\\Music\\private-song.flac TOP SECRET SONG TITLE",
        )
        .unwrap();

        let settings = settings(&root);
        let preview = preview(&connection, &paths, &system(), &settings).unwrap();
        assert_eq!(preview.schema_version, DIAGNOSTIC_SCHEMA_VERSION);
        assert!(
            preview
                .files
                .iter()
                .any(|file| file.name == "manifest.json")
        );
        assert!(preview.excluded.iter().any(|item| item.contains("songs")));

        let target = root.join("diagnostic.zip");
        export(
            &connection,
            &paths,
            &system(),
            &settings,
            &target,
            &preview.diagnostic_id,
        )
        .unwrap();
        let mut archive = ZipArchive::new(File::open(&target).unwrap()).unwrap();
        let mut combined = Vec::new();
        let mut names = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            names.push(entry.name().to_string());
            entry.read_to_end(&mut combined).unwrap();
        }
        let text = String::from_utf8(combined).unwrap();
        for secret in [
            "SecretUser",
            "TOP SECRET SONG TITLE",
            "PRIVATE ARTIST",
            "private-song",
            "secret descriptor",
            "BF-PRIVATE-ID",
        ] {
            assert!(!text.contains(secret), "diagnostic leaked {secret}");
        }
        assert!(text.contains("CUDA_OUT_OF_MEMORY"));
        assert!(text.contains("TIMING_INVALID"));
        assert_eq!(
            names,
            vec![
                "manifest.json",
                "README.txt",
                "system.json",
                "environment.json",
                "settings.json",
                "jobs.json",
                "recent-logs.json",
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_rejects_invalid_identity_extension_and_overwrite() {
        let root = root();
        let paths = paths(&root);
        let connection = database();
        let settings = settings(&root);
        let system = system();
        assert!(
            export(
                &connection,
                &paths,
                &system,
                &settings,
                &root.join("diagnostic.zip"),
                "../invalid",
            )
            .is_err()
        );
        assert!(
            export(
                &connection,
                &paths,
                &system,
                &settings,
                &root.join("diagnostic.osz"),
                "BF-DIAG-1-0",
            )
            .is_err()
        );
        let target = root.join("diagnostic.zip");
        fs::write(&target, b"existing").unwrap();
        assert!(
            export(
                &connection,
                &paths,
                &system,
                &settings,
                &target,
                "BF-DIAG-1-0",
            )
            .is_err()
        );
        assert_eq!(fs::read(&target).unwrap(), b"existing");
        fs::remove_dir_all(root).unwrap();
    }
}
