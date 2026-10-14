use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{Disks, System};
use tauri::{AppHandle, Emitter, Manager};

mod background;
mod downloads;
mod environment;
mod formal_worker;
mod install_recovery;
mod jobs;
mod mapperatorinator;
mod mapperatorinator_lock;
mod mock_worker;
mod models;
mod osu_integration;
mod packages;
mod preview;
mod protocol;
mod request_adapter;
mod runtime;
#[cfg(test)]
mod stage4_acceptance;
mod storage;
pub mod validation;
mod worker;

static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn run_mock_worker() -> i32 {
    mock_worker::run()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppPaths {
    root: String,
    models: String,
    runtime: String,
    cache: String,
    logs: String,
    jobs: String,
    outputs: String,
    database: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GpuInfo {
    name: String,
    memory_total_mb: Option<u64>,
    memory_free_mb: Option<u64>,
    driver_version: Option<String>,
    available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    os_name: String,
    os_version: String,
    architecture: String,
    cpu_name: String,
    cpu_cores: usize,
    total_memory_mb: u64,
    available_memory_mb: u64,
    available_disk_gb: u64,
    gpu: GpuInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OsuInstallation {
    kind: String,
    path: String,
    songs_path: Option<String>,
    executable: Option<String>,
}

const SETTINGS_SCHEMA_VERSION: u16 = 3;
const MAX_SETTINGS_IMPORT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    output_path: String,
    temp_path: String,
    osu_path: String,
    device: String,
    precision: String,
    workers: u8,
    auto_import: bool,
    keep_intermediate: bool,
    #[serde(default = "default_true")]
    repair_generated_output: bool,
    check_updates: bool,
    #[serde(default = "default_true")]
    desktop_notifications: bool,
    #[serde(default = "default_locale")]
    locale: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsDocument {
    schema_version: u16,
    settings: AppSettings,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PartialAppSettings {
    output_path: Option<String>,
    temp_path: Option<String>,
    osu_path: Option<String>,
    device: Option<String>,
    precision: Option<String>,
    workers: Option<u8>,
    auto_import: Option<bool>,
    keep_intermediate: Option<bool>,
    repair_generated_output: Option<bool>,
    check_updates: Option<bool>,
    desktop_notifications: Option<bool>,
    locale: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsImportItem {
    field: String,
    current: serde_json::Value,
    imported: Option<serde_json::Value>,
    result: serde_json::Value,
    action: String,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsImportPreview {
    schema_version: u16,
    source_path: String,
    items: Vec<SettingsImportItem>,
    resulting_settings: AppSettings,
}

fn default_true() -> bool {
    true
}

fn default_locale() -> String {
    "en".into()
}

impl AppSettings {
    fn default_for(paths: &AppPaths) -> Self {
        Self {
            output_path: paths.outputs.clone(),
            temp_path: paths.cache.clone(),
            osu_path: String::new(),
            device: "cuda".into(),
            precision: "bf16".into(),
            workers: 2,
            auto_import: true,
            keep_intermediate: false,
            repair_generated_output: true,
            check_updates: true,
            desktop_notifications: true,
            locale: default_locale(),
        }
    }
}

fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    for (label, value) in [
        ("Output path", settings.output_path.as_str()),
        ("Temporary path", settings.temp_path.as_str()),
        ("osu! path", settings.osu_path.as_str()),
    ] {
        if value.chars().count() > 4096 || value.chars().any(char::is_control) {
            return Err(format!("{label} is invalid"));
        }
    }
    if settings.output_path.trim().is_empty() || settings.temp_path.trim().is_empty() {
        return Err("Output and temporary paths cannot be empty".into());
    }
    if !matches!(settings.device.as_str(), "cuda" | "cpu" | "auto") {
        return Err("Unsupported compute device setting".into());
    }
    if !matches!(settings.precision.as_str(), "bf16" | "fp32") {
        return Err("Unsupported precision setting".into());
    }
    if !(1..=4).contains(&settings.workers) {
        return Err("Background workers must be between 1 and 4".into());
    }
    if !matches!(settings.locale.as_str(), "en" | "zh-TW") {
        return Err("Unsupported interface language setting".into());
    }
    Ok(())
}

fn migrate_legacy_settings(mut settings: AppSettings) -> AppSettings {
    if settings.precision == "fp16" {
        settings.precision = "bf16".into();
    }
    settings
}

fn settings_json(settings: &AppSettings) -> Result<String, String> {
    serde_json::to_string_pretty(&SettingsDocument {
        schema_version: SETTINGS_SCHEMA_VERSION,
        settings: settings.clone(),
    })
    .map_err(|error| error.to_string())
}

fn persist_settings(connection: &Connection, settings: &AppSettings) -> Result<(), String> {
    validate_settings(settings)?;
    let value = settings_json(settings)?;
    connection
        .execute(
            "INSERT INTO settings(id, value, updated_at) VALUES (1, ?1, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![value],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn parse_stored_settings(json: &str) -> Result<(AppSettings, bool), String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Stored settings are invalid: {error}"))?;
    if value.get("schemaVersion").is_some() {
        let document: SettingsDocument = serde_json::from_value(value)
            .map_err(|error| format!("Stored settings document is invalid: {error}"))?;
        if document.schema_version == 0 || document.schema_version > SETTINGS_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported settings schema version {}; newest supported version is {SETTINGS_SCHEMA_VERSION}",
                document.schema_version
            ));
        }
        let migrated = document.schema_version < SETTINGS_SCHEMA_VERSION
            || document.settings.precision == "fp16";
        let settings = migrate_legacy_settings(document.settings);
        validate_settings(&settings)?;
        Ok((settings, migrated))
    } else {
        let settings: AppSettings = serde_json::from_value(value)
            .map_err(|error| format!("Legacy settings are invalid: {error}"))?;
        let settings = migrate_legacy_settings(settings);
        validate_settings(&settings)?;
        Ok((settings, true))
    }
}

fn import_path_is_usable(field: &str, value: &str) -> bool {
    if field == "osuPath" && value.trim().is_empty() {
        return true;
    }
    let path = Path::new(value);
    if field == "osuPath" {
        path.is_dir()
            || (path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("osu!.exe")))
    } else {
        path.is_dir()
    }
}

fn preview_settings_json(
    json: &str,
    source_path: String,
    current: &AppSettings,
    defaults: &AppSettings,
) -> Result<SettingsImportPreview, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Settings import is not valid JSON: {error}"))?;
    let (schema_version, payload) = if let Some(version) = value.get("schemaVersion") {
        let version = version
            .as_u64()
            .ok_or_else(|| "Settings schemaVersion must be an integer".to_string())?;
        if version == 0 || version > SETTINGS_SCHEMA_VERSION as u64 {
            return Err(format!(
                "Unsupported settings schema version {version}; newest supported version is {SETTINGS_SCHEMA_VERSION}"
            ));
        }
        let object = value
            .as_object()
            .ok_or_else(|| "Settings document must be an object".to_string())?;
        if object
            .keys()
            .any(|key| key != "schemaVersion" && key != "settings")
        {
            return Err("Settings document contains unknown top-level fields".into());
        }
        (
            version as u16,
            value
                .get("settings")
                .cloned()
                .ok_or_else(|| "Settings document is missing settings".to_string())?,
        )
    } else {
        (0, value)
    };
    let mut partial: PartialAppSettings = serde_json::from_value(payload)
        .map_err(|error| format!("Settings import contains invalid or unknown fields: {error}"))?;
    if partial.precision.as_deref() == Some("fp16") {
        partial.precision = Some("bf16".into());
    }
    let mut result = current.clone();
    let mut items = Vec::new();

    macro_rules! portable_setting {
        ($field:literal, $member:ident, $imported:expr) => {{
            let imported = $imported;
            let current_value = serde_json::to_value(&current.$member).unwrap();
            let (value, action, reason) = match imported.as_ref() {
                Some(value) => {
                    let action = if value == &current.$member {
                        "unchanged"
                    } else {
                        "apply"
                    };
                    (value.clone(), action, "Imported portable setting")
                }
                None => (
                    defaults.$member.clone(),
                    "reset",
                    "Missing field uses the current schema default",
                ),
            };
            result.$member = value.clone();
            items.push(SettingsImportItem {
                field: $field.into(),
                current: current_value,
                imported: imported.map(|value| serde_json::to_value(value).unwrap()),
                result: serde_json::to_value(value).unwrap(),
                action: action.into(),
                reason: reason.into(),
            });
        }};
    }

    macro_rules! path_setting {
        ($field:literal, $member:ident, $imported:expr) => {{
            let imported = $imported;
            let current_value = current.$member.clone();
            let (value, action, reason) = match imported.as_ref() {
                Some(value) if import_path_is_usable($field, value) => (
                    value.clone(),
                    if value == &current_value {
                        "unchanged"
                    } else {
                        "apply"
                    },
                    "Imported path exists on this machine",
                ),
                Some(_) => (
                    current_value.clone(),
                    "ignored",
                    "Imported machine-specific path does not exist or is not usable",
                ),
                None => (
                    defaults.$member.clone(),
                    "reset",
                    "Missing field uses this machine's default path",
                ),
            };
            result.$member = value.clone();
            items.push(SettingsImportItem {
                field: $field.into(),
                current: serde_json::Value::String(current_value),
                imported: imported.map(serde_json::Value::String),
                result: serde_json::Value::String(value),
                action: action.into(),
                reason: reason.into(),
            });
        }};
    }

    path_setting!("outputPath", output_path, partial.output_path);
    path_setting!("tempPath", temp_path, partial.temp_path);
    path_setting!("osuPath", osu_path, partial.osu_path);
    portable_setting!("device", device, partial.device);
    portable_setting!("precision", precision, partial.precision);
    portable_setting!("workers", workers, partial.workers);
    portable_setting!("autoImport", auto_import, partial.auto_import);
    portable_setting!(
        "keepIntermediate",
        keep_intermediate,
        partial.keep_intermediate
    );
    portable_setting!(
        "repairGeneratedOutput",
        repair_generated_output,
        partial.repair_generated_output
    );
    portable_setting!("checkUpdates", check_updates, partial.check_updates);
    portable_setting!(
        "desktopNotifications",
        desktop_notifications,
        partial.desktop_notifications
    );
    portable_setting!("locale", locale, partial.locale);
    result = migrate_legacy_settings(result);
    validate_settings(&result)
        .map_err(|error| format!("Imported settings would be invalid: {error}"))?;

    Ok(SettingsImportPreview {
        schema_version,
        source_path,
        items,
        resulting_settings: result,
    })
}

#[cfg(test)]
mod settings_tests {
    use super::{
        AppSettings, SETTINGS_SCHEMA_VERSION, parse_stored_settings, preview_settings_json,
        settings_json,
    };

    fn settings(root: &str) -> AppSettings {
        AppSettings {
            output_path: root.into(),
            temp_path: root.into(),
            osu_path: String::new(),
            device: "cuda".into(),
            precision: "bf16".into(),
            workers: 2,
            auto_import: true,
            keep_intermediate: false,
            repair_generated_output: true,
            check_updates: true,
            desktop_notifications: true,
            locale: "en".into(),
        }
    }

    #[test]
    fn legacy_settings_enable_generated_output_repair() {
        let (settings, migrated) = parse_stored_settings(
            r#"{"outputPath":"out","tempPath":"temp","osuPath":"","device":"cuda","precision":"fp16","workers":2,"autoImport":true,"keepIntermediate":false,"checkUpdates":true}"#,
        )
        .unwrap();

        assert!(migrated);
        assert!(settings.repair_generated_output);
        assert!(settings.desktop_notifications);
        assert_eq!(settings.locale, "en");
        assert_eq!(settings.precision, "bf16");
        let exported = settings_json(&settings).unwrap();
        assert!(exported.contains(&format!(r#""schemaVersion": {SETTINGS_SCHEMA_VERSION}"#)));
    }

    #[test]
    fn import_preview_ignores_missing_machine_path_and_resets_missing_fields() {
        let root = std::env::temp_dir();
        let root_text = root.to_string_lossy();
        let current = settings(&root_text);
        let mut defaults = current.clone();
        defaults.device = "cuda".into();
        let json = format!(
            r#"{{"schemaVersion":1,"settings":{{"outputPath":"Z:\\missing-output","tempPath":{},"osuPath":"","device":"cpu","precision":"fp32","workers":1,"autoImport":false,"keepIntermediate":true,"checkUpdates":false}}}}"#,
            serde_json::to_string(root_text.as_ref()).unwrap()
        );

        let preview =
            preview_settings_json(&json, "settings.json".into(), &current, &defaults).unwrap();

        assert_eq!(preview.schema_version, 1);
        assert_eq!(preview.resulting_settings.output_path, current.output_path);
        assert_eq!(preview.resulting_settings.device, "cpu");
        assert!(preview.resulting_settings.repair_generated_output);
        assert!(preview.resulting_settings.desktop_notifications);
        assert!(
            preview
                .items
                .iter()
                .any(|item| item.field == "outputPath" && item.action == "ignored")
        );
        assert!(
            preview
                .items
                .iter()
                .any(|item| { item.field == "repairGeneratedOutput" && item.action == "reset" })
        );
    }

    #[test]
    fn version_one_settings_migrate_notification_preference() {
        let (settings, migrated) = parse_stored_settings(
            r#"{"schemaVersion":1,"settings":{"outputPath":"out","tempPath":"temp","osuPath":"","device":"cuda","precision":"bf16","workers":2,"autoImport":true,"keepIntermediate":false,"repairGeneratedOutput":true,"checkUpdates":true}}"#,
        )
        .unwrap();

        assert!(migrated);
        assert!(settings.desktop_notifications);
        assert!(
            settings_json(&settings)
                .unwrap()
                .contains(&format!(r#""schemaVersion": {SETTINGS_SCHEMA_VERSION}"#))
        );
    }

    #[test]
    fn newer_or_unknown_settings_documents_are_rejected() {
        let current = settings(".");
        assert!(
            preview_settings_json(
                r#"{"schemaVersion":99,"settings":{}}"#,
                "newer.json".into(),
                &current,
                &current,
            )
            .is_err()
        );
        assert!(
            preview_settings_json(
                r#"{"schemaVersion":1,"settings":{"surprise":true}}"#,
                "unknown.json".into(),
                &current,
                &current,
            )
            .is_err()
        );
    }

    #[test]
    fn exported_schema_round_trips_through_import_preview() {
        let root = std::env::temp_dir();
        let original = settings(&root.to_string_lossy());
        let json = settings_json(&original).unwrap();
        let preview =
            preview_settings_json(&json, "round-trip.json".into(), &original, &original).unwrap();

        assert_eq!(preview.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(preview.resulting_settings, original);
        assert!(preview.items.iter().all(|item| item.action == "unchanged"));
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn build_paths(app: &AppHandle) -> Result<AppPaths, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let paths = AppPaths {
        models: path_string(&root.join("models")),
        runtime: path_string(&root.join("runtime")),
        cache: path_string(&root.join("cache")),
        logs: path_string(&root.join("logs")),
        jobs: path_string(&root.join("jobs")),
        outputs: path_string(&root.join("outputs")),
        database: path_string(&root.join("beatforge.db")),
        root: path_string(&root),
    };
    for directory in [
        &paths.models,
        &paths.runtime,
        &paths.cache,
        &paths.logs,
        &paths.jobs,
        &paths.outputs,
    ] {
        fs::create_dir_all(directory)
            .map_err(|error| format!("Failed to create {directory}: {error}"))?;
    }
    Ok(paths)
}

pub(crate) fn open_database(app: &AppHandle) -> Result<(Connection, AppPaths), String> {
    let paths = build_paths(app)?;
    let connection = Connection::open(&paths.database).map_err(|error| error.to_string())?;
    connection.execute_batch(
    "PRAGMA journal_mode = WAL;
     PRAGMA foreign_keys = ON;
     CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
     CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id = 1), value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
     CREATE TABLE IF NOT EXISTS jobs (id TEXT PRIMARY KEY, status TEXT NOT NULL, request_json TEXT NOT NULL, result_json TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
     CREATE TABLE IF NOT EXISTS models (id TEXT PRIMARY KEY, version TEXT NOT NULL, status TEXT NOT NULL, manifest_json TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
     INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);",
  ).map_err(|error| error.to_string())?;
    jobs::migrate(&connection)?;
    packages::migrate(&connection)?;
    packages::sync_manifest(&connection, &packages::builtin_manifest())?;
    Ok((connection, paths))
}

fn detect_gpu() -> GpuInfo {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output();
    if let Ok(output) = output
        && output.status.success()
        && let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next()
    {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() >= 4 {
            return GpuInfo {
                name: fields[0].into(),
                memory_total_mb: fields[1].parse().ok(),
                memory_free_mb: fields[2].parse().ok(),
                driver_version: Some(fields[3].into()),
                available: true,
            };
        }
    }
    GpuInfo {
        name: "No NVIDIA GPU detected".into(),
        memory_total_mb: None,
        memory_free_mb: None,
        driver_version: None,
        available: false,
    }
}

#[tauri::command]
fn get_app_paths(app: AppHandle) -> Result<AppPaths, String> {
    build_paths(&app)
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut system = System::new_all();
    system.refresh_all();
    let disks = Disks::new_with_refreshed_list();
    let available_disk = disks
        .list()
        .iter()
        .map(|disk| disk.available_space())
        .max()
        .unwrap_or(0);
    let cpu_name = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".into());
    SystemInfo {
        os_name: System::name().unwrap_or_else(|| "Windows".into()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".into()),
        architecture: std::env::consts::ARCH.into(),
        cpu_name,
        cpu_cores: system.cpus().len(),
        total_memory_mb: system.total_memory() / 1024 / 1024,
        available_memory_mb: system.available_memory() / 1024 / 1024,
        available_disk_gb: available_disk / 1024 / 1024 / 1024,
        gpu: detect_gpu(),
    }
}

#[tauri::command]
fn detect_osu_installations() -> Vec<OsuInstallation> {
    let mut candidates: Vec<(String, PathBuf, Option<PathBuf>)> = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        candidates.push((
            "stable".into(),
            local.join("osu!"),
            Some(local.join("osu!").join("Songs")),
        ));
        candidates.push(("lazer".into(), local.join("osulazer").join("current"), None));
    }
    if let Some(roaming) = std::env::var_os("APPDATA").map(PathBuf::from) {
        candidates.push(("lazer-data".into(), roaming.join("osu"), None));
    }
    candidates
        .into_iter()
        .filter(|(_, path, _)| path.exists())
        .map(|(kind, path, songs)| OsuInstallation {
            kind,
            path: path_string(&path),
            songs_path: songs
                .filter(|candidate| candidate.exists())
                .map(|candidate| path_string(&candidate)),
            executable: {
                let candidate = path.join("osu!.exe");
                candidate.is_file().then(|| path_string(&candidate))
            },
        })
        .collect()
}

fn validated_job_artifact(app: &AppHandle, job_id: &str) -> Result<PathBuf, String> {
    let (connection, paths) = open_database(app)?;
    let job = jobs::get(&connection, job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    osu_integration::resolve_validated_artifact(
        Path::new(&paths.jobs),
        job_id,
        &job.status,
        job.output_path.as_deref(),
        job.validation_report.as_ref(),
    )
}

#[tauri::command]
fn get_job_preview(app: AppHandle, job_id: String) -> Result<preview::JobPreview, String> {
    let artifact = validated_job_artifact(&app, &job_id)?;
    let (connection, _) = open_database(&app)?;
    let job = jobs::get(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    let difficulties = preview::build_difficulties(&artifact)?;
    let (audio_path, audio_unavailable_reason, waveform_allowed) =
        preview::audio_details(Path::new(&job.audio_file));
    if let Some(path) = audio_path.as_ref() {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|error| format!("Could not authorize preview audio: {error}"))?;
    }
    Ok(preview::JobPreview {
        job_id,
        audio_path,
        audio_unavailable_reason,
        waveform_allowed,
        max_waveform_points: 320,
        difficulties,
    })
}

#[tauri::command]
fn authorize_background_preview(
    app: AppHandle,
    path: String,
) -> Result<background::BackgroundImageInfo, String> {
    let info = background::inspect_background_image(Path::new(&path))?;
    app.asset_protocol_scope()
        .allow_file(&info.path)
        .map_err(|error| format!("Could not authorize background preview: {error}"))?;
    Ok(info)
}

fn selected_osu_executable(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let settings = load_settings_value(app)?;
    let detected = detect_osu_installations()
        .into_iter()
        .filter_map(|installation| installation.executable.map(PathBuf::from));
    osu_integration::select_osu_executable(Some(&settings.osu_path), detected)
}

#[tauri::command]
fn reveal_job_output(app: AppHandle, job_id: String) -> Result<(), String> {
    let artifact = validated_job_artifact(&app, &job_id)?;
    osu_integration::reveal_artifact(&artifact)
}

#[tauri::command]
fn import_job_output(
    app: AppHandle,
    job_id: String,
) -> Result<osu_integration::ImportLaunch, String> {
    let artifact = validated_job_artifact(&app, &job_id)?;
    let executable = selected_osu_executable(&app)?;
    let paths = build_paths(&app)?;
    osu_integration::import_artifact(
        &artifact,
        Path::new(&paths.cache),
        &job_id,
        executable.as_deref(),
        true,
    )
}

#[tauri::command]
fn import_failed_job_output(
    app: AppHandle,
    job_id: String,
) -> Result<osu_integration::ImportLaunch, String> {
    let (connection, paths) = open_database(&app)?;
    let job = jobs::get(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    let artifact = osu_integration::resolve_failed_artifact(
        Path::new(&paths.jobs),
        &job_id,
        &job.status,
        job.validation_report.as_ref(),
    )?;
    let executable = selected_osu_executable(&app)?;
    osu_integration::import_artifact(
        &artifact,
        Path::new(&paths.cache),
        &job_id,
        executable.as_deref(),
        true,
    )
}

#[tauri::command]
fn import_unfiltered_set_output(
    app: AppHandle,
    job_id: String,
) -> Result<osu_integration::ImportLaunch, String> {
    let (connection, paths) = open_database(&app)?;
    let job = jobs::get(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    let artifact = osu_integration::resolve_unfiltered_set_artifact(
        Path::new(&paths.jobs),
        &job_id,
        &job.status,
        job.provenance.as_ref(),
    )?;
    let executable = selected_osu_executable(&app)?;
    osu_integration::import_artifact(
        &artifact,
        Path::new(&paths.cache),
        &job_id,
        executable.as_deref(),
        true,
    )
}

pub(crate) fn try_auto_import(app: &AppHandle, job_id: &str) -> Result<(), String> {
    let settings = load_settings_value(app)?;
    if !settings.auto_import {
        return Ok(());
    }
    let artifact = validated_job_artifact(app, job_id)?;
    let detected = detect_osu_installations()
        .into_iter()
        .filter_map(|installation| installation.executable.map(PathBuf::from));
    let executable = osu_integration::select_osu_executable(Some(&settings.osu_path), detected)?
        .ok_or_else(|| {
            "Automatic import skipped because no osu! executable was detected".to_string()
        })?;
    let paths = build_paths(app)?;
    osu_integration::import_artifact(
        &artifact,
        Path::new(&paths.cache),
        job_id,
        Some(&executable),
        false,
    )?;
    Ok(())
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    load_settings_value(&app)
}

fn load_settings_value(app: &AppHandle) -> Result<AppSettings, String> {
    let (connection, paths) = open_database(app)?;
    let value: Option<String> = connection
        .query_row("SELECT value FROM settings WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| error.to_string())?;
    match value {
        Some(json) => {
            let (settings, migrated) = parse_stored_settings(&json)?;
            if migrated {
                persist_settings(&connection, &settings)?;
            }
            Ok(settings)
        }
        None => Ok(AppSettings::default_for(&paths)),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    persist_settings(&connection, &migrate_legacy_settings(settings))
}

#[tauri::command]
fn export_settings(path: String, settings: AppSettings) -> Result<String, String> {
    let target = PathBuf::from(&path);
    if !target
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("json"))
    {
        return Err("Settings export must use a .json file".into());
    }
    let parent = target
        .parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| "Settings export folder does not exist".to_string())?;
    let target = parent.join(
        target
            .file_name()
            .ok_or_else(|| "Settings export filename is invalid".to_string())?,
    );
    validate_settings(&settings)?;
    fs::write(&target, settings_json(&settings)?)
        .map_err(|error| format!("Could not export settings: {error}"))?;
    Ok(path_string(&target))
}

#[tauri::command]
fn preview_settings_import(app: AppHandle, path: String) -> Result<SettingsImportPreview, String> {
    let source = PathBuf::from(&path);
    if !source
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("json"))
    {
        return Err("Settings import must use a .json file".into());
    }
    let metadata = source
        .metadata()
        .map_err(|error| format!("Could not inspect settings import: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_SETTINGS_IMPORT_BYTES {
        return Err("Settings import must be a regular JSON file no larger than 256 KiB".into());
    }
    let json = fs::read_to_string(&source)
        .map_err(|error| format!("Could not read settings import as UTF-8: {error}"))?;
    let current = load_settings_value(&app)?;
    let paths = build_paths(&app)?;
    preview_settings_json(
        &json,
        path_string(&source),
        &current,
        &AppSettings::default_for(&paths),
    )
}

#[tauri::command]
fn reset_settings(app: AppHandle, field: Option<String>) -> Result<AppSettings, String> {
    let (connection, paths) = open_database(&app)?;
    let defaults = AppSettings::default_for(&paths);
    let mut settings = load_settings_value(&app)?;
    match field.as_deref() {
        None => settings = defaults,
        Some("outputPath") => settings.output_path = defaults.output_path,
        Some("tempPath") => settings.temp_path = defaults.temp_path,
        Some("osuPath") => settings.osu_path = defaults.osu_path,
        Some("device") => settings.device = defaults.device,
        Some("precision") => settings.precision = defaults.precision,
        Some("workers") => settings.workers = defaults.workers,
        Some("autoImport") => settings.auto_import = defaults.auto_import,
        Some("keepIntermediate") => settings.keep_intermediate = defaults.keep_intermediate,
        Some("repairGeneratedOutput") => {
            settings.repair_generated_output = defaults.repair_generated_output
        }
        Some("checkUpdates") => settings.check_updates = defaults.check_updates,
        Some("desktopNotifications") => {
            settings.desktop_notifications = defaults.desktop_notifications
        }
        Some("locale") => settings.locale = defaults.locale,
        Some(_) => return Err("Unknown settings field".into()),
    }
    persist_settings(&connection, &settings)?;
    Ok(settings)
}

#[tauri::command]
async fn get_storage_usage(app: AppHandle) -> Result<storage::StorageUsageReport, String> {
    let (connection, paths) = open_database(&app)?;
    let active_jobs = jobs::active_ids(&connection)?;
    tauri::async_runtime::spawn_blocking(move || storage::usage(&paths, &active_jobs))
        .await
        .map_err(|error| format!("Storage scan task failed: {error}"))?
}

#[tauri::command]
async fn preview_storage_cleanup(
    app: AppHandle,
    categories: Vec<storage::StorageCategory>,
) -> Result<storage::StorageCleanupPreview, String> {
    let (connection, paths) = open_database(&app)?;
    let active_jobs = jobs::active_ids(&connection)?;
    tauri::async_runtime::spawn_blocking(move || {
        storage::preview_cleanup(&paths, &active_jobs, &categories)
    })
    .await
    .map_err(|error| format!("Storage preview task failed: {error}"))?
}

#[tauri::command]
async fn cleanup_storage(
    app: AppHandle,
    categories: Vec<storage::StorageCategory>,
    allow_protected: bool,
) -> Result<storage::StorageCleanupResult, String> {
    let (connection, paths) = open_database(&app)?;
    let active_jobs = jobs::active_ids(&connection)?;
    tauri::async_runtime::spawn_blocking(move || {
        storage::cleanup(&paths, &active_jobs, &categories, allow_protected)
    })
    .await
    .map_err(|error| format!("Storage cleanup task failed: {error}"))?
}

fn next_job_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("BF-{timestamp}-{sequence}")
}

fn validate_job_input(input: &protocol::StartJobInput) -> Result<(), String> {
    request_adapter::validate_domain(input)
}

fn build_production_request(
    app: &AppHandle,
    job_id: &str,
    input: &protocol::StartJobInput,
) -> Result<request_adapter::ProductionWorkerStartCommand, String> {
    let settings = load_settings_value(app)?;
    let (connection, paths) = open_database(app)?;
    let model = packages::get(&connection, packages::STANDARD_MODEL_ID)?
        .ok_or_else(|| "Managed V32 model package is unavailable".to_string())?;
    if model.status != packages::PackageStatus::Installed {
        return Err("Install the managed V32 standard model before generating".into());
    }
    let model_path = model
        .install_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| "Installed model has no managed path".to_string())?;
    let runtime_root = PathBuf::from(&paths.runtime);
    let gpu = detect_gpu();
    let source_path = mapperatorinator::managed_source_path(&runtime_root)?;
    let source_commit = mapperatorinator_lock::builtin()?.source.commit;
    let context = request_adapter::AdapterContext {
        runtime_root,
        models_root: PathBuf::from(paths.models),
        jobs_root: PathBuf::from(paths.jobs),
        source_path,
        source_commit,
        model_path,
        model_version: model.target_version,
        device: settings.device,
        precision: settings.precision,
        repair_generated_output: settings.repair_generated_output,
        gpu_name: gpu.available.then_some(gpu.name),
        gpu_memory_total_mb: gpu.memory_total_mb,
        driver_version: gpu.driver_version,
    };
    request_adapter::build(job_id, input, &context)
}

#[tauri::command]
fn create_job(
    app: AppHandle,
    manager: tauri::State<'_, worker::WorkerManager>,
    input: protocol::StartJobInput,
) -> Result<jobs::JobRecord, String> {
    validate_job_input(&input)?;
    let id = next_job_id();
    let (connection, _) = open_database(&app)?;
    let record = jobs::insert(&connection, &id, &input)?;
    manager.enqueue(id, input)?;
    Ok(record)
}

#[tauri::command]
fn list_jobs(app: AppHandle) -> Result<Vec<jobs::JobRecord>, String> {
    let (connection, _) = open_database(&app)?;
    jobs::list(&connection)
}

#[tauri::command]
fn list_managed_packages(app: AppHandle) -> Result<Vec<packages::ManagedPackage>, String> {
    let (connection, _) = open_database(&app)?;
    packages::list(&connection)
}

#[tauri::command]
async fn download_managed_package(
    app: AppHandle,
    package_id: String,
) -> Result<packages::ManagedPackage, String> {
    let (connection, paths) = open_database(&app)?;
    let spec = packages::get_spec(&connection, &package_id)?;
    let current = packages::get(&connection, &package_id)?
        .ok_or_else(|| format!("Unknown package: {package_id}"))?;
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Queued,
        current.progress,
        current.bytes_downloaded,
        None,
        None,
        None,
    )?;
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Downloading,
        current.progress,
        current.bytes_downloaded,
        None,
        None,
        None,
    )?;
    drop(connection);

    let destination = PathBuf::from(&paths.cache)
        .join("downloads")
        .join(&spec.id)
        .join(&spec.version);
    let database_path = paths.database.clone();
    let worker_database_path = database_path.clone();
    let worker_package_id = package_id.clone();
    let worker_app = app.clone();
    let total_size = spec.total_size();
    let artifacts: Vec<_> = spec.artifacts().cloned().collect();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let connection =
            Connection::open(&worker_database_path).map_err(|error| error.to_string())?;
        let mut completed_bytes = 0_u64;
        let mut first_path = None;
        for artifact in &artifacts {
            let path = downloads::download_artifact(
                &worker_package_id,
                artifact,
                &destination,
                |event| {
                    let bytes_downloaded = completed_bytes.saturating_add(event.bytes_downloaded);
                    let progress = bytes_downloaded
                        .saturating_mul(100)
                        .checked_div(total_size)
                        .unwrap_or(0)
                        .min(99) as u8;
                    let aggregate = downloads::PackageProgressEvent {
                        package_id: worker_package_id.clone(),
                        status: packages::PackageStatus::Downloading,
                        progress,
                        bytes_downloaded,
                        size_bytes: total_size,
                    };
                    packages::transition(
                        &connection,
                        &worker_package_id,
                        aggregate.status,
                        aggregate.progress,
                        aggregate.bytes_downloaded,
                        None,
                        None,
                        None,
                    )?;
                    worker_app
                        .emit("package-progress-event", &aggregate)
                        .map_err(|error| error.to_string())
                },
            )?;
            first_path.get_or_insert(path);
            completed_bytes = completed_bytes.saturating_add(artifact.size_bytes);
        }
        packages::transition(
            &connection,
            &worker_package_id,
            packages::PackageStatus::Verifying,
            100,
            total_size,
            None,
            None,
            None,
        )?;
        worker_app
            .emit(
                "package-progress-event",
                downloads::PackageProgressEvent {
                    package_id: worker_package_id,
                    status: packages::PackageStatus::Verifying,
                    progress: 100,
                    bytes_downloaded: total_size,
                    size_bytes: total_size,
                },
            )
            .map_err(|error| error.to_string())?;
        if artifacts.len() == 1 {
            first_path.ok_or_else(|| "Package has no artifact".to_string())
        } else {
            Ok(destination)
        }
    })
    .await
    .map_err(|error| format!("Download task failed: {error}"))?;

    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    match result {
        Ok(artifact_path) => {
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Downloaded,
                100,
                total_size,
                None,
                None,
                None,
            )?;
            packages::set_artifact_path(&connection, &package_id, &path_string(&artifact_path))?;
        }
        Err(error) => {
            let current = packages::get(&connection, &package_id)?
                .ok_or_else(|| format!("Unknown package: {package_id}"))?;
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Failed,
                current.progress,
                current.bytes_downloaded,
                None,
                None,
                Some(&error),
            )?;
            return Err(error);
        }
    }
    packages::get(&connection, &package_id)?.ok_or_else(|| format!("Unknown package: {package_id}"))
}

#[tauri::command]
async fn install_python_runtime(app: AppHandle) -> Result<packages::ManagedPackage, String> {
    let package_id = packages::PYTHON_RUNTIME_ID.to_string();
    let (connection, paths) = open_database(&app)?;
    let current = packages::get(&connection, &package_id)?
        .ok_or_else(|| "Python runtime package is missing from the manifest".to_string())?;
    if current.kind != packages::PackageKind::Runtime {
        return Err("Python runtime manifest entry has an invalid package kind".into());
    }
    if current.status == packages::PackageStatus::Installed {
        let install_path = current
            .install_path
            .as_deref()
            .ok_or_else(|| "Installed Python runtime has no install path".to_string())?;
        return match runtime::verify_python_runtime(
            Path::new(install_path),
            &current.target_version,
        ) {
            Ok(_) => Ok(current),
            Err(error) => {
                packages::transition(
                    &connection,
                    &package_id,
                    packages::PackageStatus::Failed,
                    current.progress,
                    current.bytes_downloaded,
                    None,
                    None,
                    Some(&error),
                )?;
                Err(error)
            }
        };
    }
    drop(connection);

    let downloaded = if current.status == packages::PackageStatus::Downloaded {
        current
    } else {
        download_managed_package(app.clone(), package_id.clone()).await?
    };
    let archive_path = downloaded
        .artifact_path
        .as_deref()
        .ok_or_else(|| "Downloaded Python runtime has no artifact path".to_string())?;
    let archive_path = PathBuf::from(archive_path);
    let runtime_root = PathBuf::from(&paths.runtime);
    let version = downloaded.target_version.clone();
    let database_path = paths.database.clone();
    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Installing,
        100,
        downloaded.size_bytes,
        None,
        None,
        None,
    )?;
    app.emit(
        "package-progress-event",
        downloads::PackageProgressEvent {
            package_id: package_id.clone(),
            status: packages::PackageStatus::Installing,
            progress: 100,
            bytes_downloaded: downloaded.size_bytes,
            size_bytes: downloaded.size_bytes,
        },
    )
    .map_err(|error| error.to_string())?;
    drop(connection);

    let worker_version = version.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        runtime::install_python_runtime(&archive_path, &runtime_root, &worker_version)
    })
    .await
    .map_err(|error| format!("Runtime installation task failed: {error}"))?;

    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    match result {
        Ok(info) => {
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Installed,
                100,
                downloaded.size_bytes,
                Some(&version),
                Some(&info.path),
                None,
            )?;
        }
        Err(error) => {
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Failed,
                100,
                downloaded.size_bytes,
                None,
                None,
                Some(&error),
            )?;
            return Err(error);
        }
    }
    packages::get(&connection, &package_id)?
        .ok_or_else(|| "Installed Python runtime could not be loaded".to_string())
}

#[tauri::command]
fn verify_python_runtime(app: AppHandle) -> Result<runtime::RuntimeInfo, String> {
    let (connection, _) = open_database(&app)?;
    let package = packages::get(&connection, packages::PYTHON_RUNTIME_ID)?
        .ok_or_else(|| "Python runtime package is missing from the manifest".to_string())?;
    if package.status != packages::PackageStatus::Installed {
        return Err("Python runtime is not installed".into());
    }
    let install_path = package
        .install_path
        .as_deref()
        .ok_or_else(|| "Installed Python runtime has no install path".to_string())?;
    match runtime::verify_python_runtime(Path::new(install_path), &package.target_version) {
        Ok(info) => Ok(info),
        Err(error) => {
            packages::transition(
                &connection,
                packages::PYTHON_RUNTIME_ID,
                packages::PackageStatus::Failed,
                package.progress,
                package.bytes_downloaded,
                None,
                None,
                Some(&error),
            )?;
            Err(error)
        }
    }
}

#[tauri::command]
async fn remove_python_runtime(app: AppHandle) -> Result<packages::ManagedPackage, String> {
    let package_id = packages::PYTHON_RUNTIME_ID.to_string();
    let (connection, paths) = open_database(&app)?;
    let current = packages::get(&connection, &package_id)?
        .ok_or_else(|| "Python runtime package is missing from the manifest".to_string())?;
    if current.status == packages::PackageStatus::NotInstalled {
        return Ok(current);
    }
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Removing,
        current.progress,
        current.bytes_downloaded,
        None,
        None,
        None,
    )?;
    app.emit(
        "package-progress-event",
        downloads::PackageProgressEvent {
            package_id: package_id.clone(),
            status: packages::PackageStatus::Removing,
            progress: current.progress,
            bytes_downloaded: current.bytes_downloaded,
            size_bytes: current.size_bytes,
        },
    )
    .map_err(|error| error.to_string())?;
    drop(connection);

    let install_path = current.install_path.clone();
    let artifact_path = current.artifact_path.clone();
    let runtime_root = PathBuf::from(paths.runtime);
    let cache_root = PathBuf::from(paths.cache);
    let result = tauri::async_runtime::spawn_blocking(move || {
        runtime::remove_runtime_files(
            install_path.as_deref(),
            artifact_path.as_deref(),
            &runtime_root,
            &cache_root,
        )
    })
    .await
    .map_err(|error| format!("Runtime removal task failed: {error}"))?;

    let (connection, _) = open_database(&app)?;
    if let Err(error) = result {
        packages::transition(
            &connection,
            &package_id,
            packages::PackageStatus::Failed,
            current.progress,
            current.bytes_downloaded,
            None,
            None,
            Some(&error),
        )?;
        return Err(error);
    }
    packages::finish_removal(&connection, &package_id)?;
    packages::get(&connection, &package_id)?
        .ok_or_else(|| "Removed Python runtime could not be loaded".to_string())
}

fn ensure_package_requirements(
    connection: &Connection,
    spec: &packages::PackageSpec,
) -> Result<(), String> {
    for requirement in &spec.requirements {
        let installed = packages::get(connection, &requirement.package_id)?
            .ok_or_else(|| format!("Required package {} is unavailable", requirement.package_id))?;
        if installed.status != packages::PackageStatus::Installed
            || installed.installed_version.as_deref() != Some(requirement.version.as_str())
        {
            return Err(format!(
                "{} {} must be installed first",
                installed.name, requirement.version
            ));
        }
    }
    Ok(())
}

#[tauri::command]
async fn install_model_package(
    app: AppHandle,
    package_id: String,
) -> Result<packages::ManagedPackage, String> {
    let (connection, paths) = open_database(&app)?;
    let spec = packages::get_spec(&connection, &package_id)?;
    if spec.kind != packages::PackageKind::Model {
        return Err("The selected package is not a model".into());
    }
    ensure_package_requirements(&connection, &spec)?;
    let current = packages::get(&connection, &package_id)?
        .ok_or_else(|| format!("Unknown package: {package_id}"))?;
    if current.status == packages::PackageStatus::Installed {
        let path = current
            .install_path
            .as_deref()
            .ok_or_else(|| "Installed model has no install path".to_string())?;
        return match models::verify_model(Path::new(path), &spec) {
            Ok(_) => Ok(current),
            Err(error) => {
                packages::transition(
                    &connection,
                    &package_id,
                    packages::PackageStatus::Failed,
                    current.progress,
                    current.bytes_downloaded,
                    None,
                    None,
                    Some(&error),
                )?;
                Err(error)
            }
        };
    }
    let previous_install_path = current.install_path.clone();
    drop(connection);

    let downloaded = if current.status == packages::PackageStatus::Downloaded {
        current
    } else {
        download_managed_package(app.clone(), package_id.clone()).await?
    };
    let download_path = downloaded
        .artifact_path
        .as_deref()
        .ok_or_else(|| "Downloaded model has no artifact path".to_string())?;
    let download_path = PathBuf::from(download_path);
    let models_root = PathBuf::from(&paths.models);
    let version = spec.version.clone();
    let connection = Connection::open(&paths.database).map_err(|error| error.to_string())?;
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Installing,
        100,
        spec.total_size(),
        None,
        None,
        None,
    )?;
    drop(connection);

    let worker_spec = spec.clone();
    let worker_models_root = models_root.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        models::install_model(&download_path, &worker_models_root, &worker_spec)
    })
    .await
    .map_err(|error| format!("Model installation task failed: {error}"))?;

    let connection = Connection::open(&paths.database).map_err(|error| error.to_string())?;
    match result {
        Ok(info) => {
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Installed,
                100,
                spec.total_size(),
                Some(&version),
                Some(&info.path),
                None,
            )?;
            if previous_install_path.as_deref() != Some(info.path.as_str()) {
                let _ = models::remove_model_files(
                    previous_install_path.as_deref(),
                    None,
                    &models_root,
                    Path::new(&paths.cache),
                );
            }
        }
        Err(error) => {
            packages::transition(
                &connection,
                &package_id,
                packages::PackageStatus::Failed,
                100,
                downloaded.bytes_downloaded,
                None,
                None,
                Some(&error),
            )?;
            return Err(error);
        }
    }
    packages::get(&connection, &package_id)?
        .ok_or_else(|| "Installed model could not be loaded".to_string())
}

#[tauri::command]
fn verify_model_package(app: AppHandle, package_id: String) -> Result<models::ModelInfo, String> {
    let (connection, _) = open_database(&app)?;
    let spec = packages::get_spec(&connection, &package_id)?;
    if spec.kind != packages::PackageKind::Model {
        return Err("The selected package is not a model".into());
    }
    let package = packages::get(&connection, &package_id)?
        .ok_or_else(|| format!("Unknown package: {package_id}"))?;
    let path = package
        .install_path
        .as_deref()
        .ok_or_else(|| "Model is not installed".to_string())?;
    match models::verify_model(Path::new(path), &spec) {
        Ok(info) => Ok(info),
        Err(error) => {
            if package.status == packages::PackageStatus::Installed {
                packages::transition(
                    &connection,
                    &package_id,
                    packages::PackageStatus::Failed,
                    package.progress,
                    package.bytes_downloaded,
                    None,
                    None,
                    Some(&error),
                )?;
            }
            Err(error)
        }
    }
}

#[tauri::command]
async fn remove_model_package(
    app: AppHandle,
    package_id: String,
) -> Result<packages::ManagedPackage, String> {
    let (connection, paths) = open_database(&app)?;
    let spec = packages::get_spec(&connection, &package_id)?;
    if spec.kind != packages::PackageKind::Model {
        return Err("The selected package is not a model".into());
    }
    let current = packages::get(&connection, &package_id)?
        .ok_or_else(|| format!("Unknown package: {package_id}"))?;
    if current.status == packages::PackageStatus::NotInstalled {
        return Ok(current);
    }
    packages::transition(
        &connection,
        &package_id,
        packages::PackageStatus::Removing,
        current.progress,
        current.bytes_downloaded,
        None,
        None,
        None,
    )?;
    drop(connection);

    let install_path = current.install_path.clone();
    let artifact_path = current.artifact_path.clone();
    let models_root = PathBuf::from(paths.models);
    let cache_root = PathBuf::from(paths.cache);
    let result = tauri::async_runtime::spawn_blocking(move || {
        models::remove_model_files(
            install_path.as_deref(),
            artifact_path.as_deref(),
            &models_root,
            &cache_root,
        )
    })
    .await
    .map_err(|error| format!("Model removal task failed: {error}"))?;

    let (connection, _) = open_database(&app)?;
    if let Err(error) = result {
        packages::transition(
            &connection,
            &package_id,
            packages::PackageStatus::Failed,
            current.progress,
            current.bytes_downloaded,
            None,
            None,
            Some(&error),
        )?;
        return Err(error);
    }
    packages::finish_removal(&connection, &package_id)?;
    packages::get(&connection, &package_id)?
        .ok_or_else(|| "Removed model could not be loaded".to_string())
}

fn installed_python(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let (connection, _) = open_database(app)?;
    let package = packages::get(&connection, packages::PYTHON_RUNTIME_ID)?;
    Ok(environment::python_executable(
        package
            .as_ref()
            .and_then(|item| item.install_path.as_deref()),
    ))
}

#[tauri::command]
async fn diagnose_ai_environment(
    app: AppHandle,
) -> Result<environment::EnvironmentDiagnostics, String> {
    let python = installed_python(&app)?;
    let paths = build_paths(&app)?;
    let runtime_root = PathBuf::from(paths.runtime);
    let gpu = detect_gpu();
    let gpu_name = gpu.available.then_some(gpu.name);
    let driver_version = gpu.driver_version;
    tauri::async_runtime::spawn_blocking(move || {
        environment::diagnose(
            python.as_deref(),
            Some(&runtime_root),
            gpu_name,
            driver_version,
        )
    })
    .await
    .map_err(|error| format!("Environment diagnostic task failed: {error}"))
}

#[tauri::command]
async fn diagnose_production_worker(app: AppHandle) -> Result<formal_worker::WorkerHealth, String> {
    let python = installed_python(&app)?.ok_or_else(|| {
        "Install the private Python runtime before checking the worker".to_string()
    })?;
    let (connection, paths) = open_database(&app)?;
    let model = packages::get(&connection, packages::STANDARD_MODEL_ID)?
        .ok_or_else(|| "Managed V32 model package is unavailable".to_string())?;
    let model_path = (model.status == packages::PackageStatus::Installed)
        .then(|| model.install_path.as_deref().map(PathBuf::from))
        .flatten();
    let model_version = model.target_version;
    let runtime_root = PathBuf::from(paths.runtime);
    let source_path = mapperatorinator::managed_source_path(&runtime_root)?;
    let source_commit = mapperatorinator_lock::builtin()?.source.commit;
    let log_path = PathBuf::from(paths.logs).join("production-worker.log");
    tauri::async_runtime::spawn_blocking(move || {
        formal_worker::health_check(
            &python,
            &runtime_root,
            &source_path,
            &source_commit,
            model_path.as_deref(),
            &model_version,
            &log_path,
        )
    })
    .await
    .map_err(|error| format!("Production worker health task failed: {error}"))?
}

#[tauri::command]
async fn install_ai_dependencies(
    app: AppHandle,
    manager: tauri::State<'_, environment::EnvironmentManager>,
    backend: environment::ComputeBackend,
) -> Result<environment::EnvironmentDiagnostics, String> {
    manager.begin()?;
    let result = async {
        let python = installed_python(&app)?.ok_or_else(|| {
            "Install the private Python runtime before AI dependencies".to_string()
        })?;
        let paths = build_paths(&app)?;
        let runtime_root = PathBuf::from(paths.runtime);
        let cache_root = PathBuf::from(paths.cache);
        let gpu = detect_gpu();
        let gpu_name = gpu.available.then_some(gpu.name);
        let driver_version = gpu.driver_version;
        let worker_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            environment::install_dependencies(&worker_app, &python, backend)?;
            mapperatorinator::install(
                &python,
                &runtime_root,
                &cache_root,
                |stage, progress, message| {
                    environment::emit_install_event(&worker_app, stage, progress, message)
                },
            )?;
            Ok::<_, String>(environment::diagnose(
                Some(&python),
                Some(&runtime_root),
                gpu_name,
                driver_version,
            ))
        })
        .await
        .map_err(|error| format!("Dependency installation task failed: {error}"))?
    }
    .await;
    manager.finish();
    result
}

#[tauri::command]
fn cancel_job(
    manager: tauri::State<'_, worker::WorkerManager>,
    job_id: String,
) -> Result<(), String> {
    manager.cancel(job_id)
}

#[tauri::command]
fn retry_job(
    app: AppHandle,
    manager: tauri::State<'_, worker::WorkerManager>,
    job_id: String,
) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    let input =
        jobs::get_input(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    let job = jobs::get(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    if !matches!(
        job.status.as_str(),
        "failed" | "validation_failed" | "cancelled" | "interrupted"
    ) {
        return Err("Only a failed, cancelled, or interrupted job can be retried".into());
    }
    worker::cleanup_job_artifacts(&app, &job_id)?;
    jobs::reset_for_retry(&connection, &job_id)?;
    manager.enqueue(job_id, input)
}

#[tauri::command]
fn regenerate_job(
    app: AppHandle,
    manager: tauri::State<'_, worker::WorkerManager>,
    job_id: String,
) -> Result<jobs::JobRecord, String> {
    let (connection, _) = open_database(&app)?;
    let source = jobs::get(&connection, &job_id)?
        .ok_or_else(|| "The source job was not found".to_string())?;
    if matches!(source.status.as_str(), "running" | "queued" | "validating") {
        return Err("An active job cannot be regenerated".into());
    }
    let input = jobs::get_input(&connection, &job_id)?
        .ok_or_else(|| "The source job request is unavailable".to_string())?;
    validate_job_input(&input).map_err(|error| {
        format!("The source job cannot be regenerated with its saved inputs: {error}")
    })?;

    let new_id = next_job_id();
    let record = jobs::insert(&connection, &new_id, &input)?;
    if let Err(error) = manager.enqueue(new_id.clone(), input) {
        let _ = jobs::rollback_queued_insert(&connection, &new_id);
        return Err(format!("Could not queue the regenerated job: {error}"));
    }
    Ok(record)
}

#[tauri::command]
fn delete_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    let job = jobs::get(&connection, &job_id)?.ok_or_else(|| "Job was not found".to_string())?;
    if matches!(job.status.as_str(), "running" | "queued" | "validating") {
        return Err("An active job cannot be deleted".into());
    }
    // Remove only this job's direct managed directory before deleting its
    // database record. If filesystem cleanup fails, the history record stays
    // available so the mismatch remains visible and can be retried safely.
    worker::delete_job_artifacts(&app, &job_id)?;
    jobs::delete_terminal(&connection, &job_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            mapperatorinator_lock::validate_builtin().map_err(std::io::Error::other)?;
            let (connection, paths) = open_database(app.handle()).map_err(std::io::Error::other)?;
            if let Err(error) =
                osu_integration::cleanup_stale_import_handoffs(Path::new(&paths.cache))
            {
                log::warn!("Could not clean stale osu! import handoffs: {error}");
            }
            let abandoned_jobs = jobs::active_ids(&connection).map_err(std::io::Error::other)?;
            jobs::mark_interrupted(&connection).map_err(std::io::Error::other)?;
            for id in abandoned_jobs {
                let _ = worker::cleanup_job_artifacts(app.handle(), &id);
            }
            packages::mark_interrupted(&connection).map_err(std::io::Error::other)?;
            app.manage(worker::WorkerManager::new(app.handle().clone()));
            app.manage(environment::EnvironmentManager::new());
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_paths,
            get_system_info,
            detect_osu_installations,
            load_settings,
            save_settings,
            export_settings,
            preview_settings_import,
            reset_settings,
            get_storage_usage,
            preview_storage_cleanup,
            cleanup_storage,
            create_job,
            list_jobs,
            list_managed_packages,
            download_managed_package,
            install_python_runtime,
            verify_python_runtime,
            remove_python_runtime,
            diagnose_ai_environment,
            diagnose_production_worker,
            install_ai_dependencies,
            install_model_package,
            verify_model_package,
            remove_model_package,
            cancel_job,
            retry_job,
            regenerate_job,
            delete_job,
            get_job_preview,
            authorize_background_preview,
            reveal_job_output,
            import_job_output,
            import_failed_job_output,
            import_unfiltered_set_output
        ])
        .build(tauri::generate_context!())
        .expect("error while building Beatforge");
    application.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            app_handle.state::<worker::WorkerManager>().shutdown();
        }
    });
}
