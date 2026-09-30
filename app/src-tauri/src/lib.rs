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
use tauri::{AppHandle, Manager};

mod jobs;
mod mock_worker;
mod protocol;
mod worker;

static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn run_mock_worker() -> i32 {
    mock_worker::run()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppPaths {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    output_path: String,
    temp_path: String,
    osu_path: String,
    device: String,
    precision: String,
    workers: u8,
    auto_import: bool,
    keep_intermediate: bool,
    check_updates: bool,
}

impl AppSettings {
    fn default_for(paths: &AppPaths) -> Self {
        Self {
            output_path: paths.outputs.clone(),
            temp_path: paths.cache.clone(),
            osu_path: String::new(),
            device: "cuda".into(),
            precision: "fp16".into(),
            workers: 2,
            auto_import: true,
            keep_intermediate: false,
            check_updates: true,
        }
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
        })
        .collect()
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let (connection, paths) = open_database(&app)?;
    let value: Option<String> = connection
        .query_row("SELECT value FROM settings WHERE id = 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| error.to_string())?;
    match value {
        Some(json) => serde_json::from_str(&json).map_err(|error| error.to_string()),
        None => Ok(AppSettings::default_for(&paths)),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    let value = serde_json::to_string(&settings).map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO settings(id, value, updated_at) VALUES (1, ?1, CURRENT_TIMESTAMP)
     ON CONFLICT(id) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![value],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
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
    if input.title.trim().is_empty() || input.title.len() > 200 {
        return Err("Title must contain between 1 and 200 characters".into());
    }
    if input.artist.trim().is_empty() || input.artist.len() > 200 {
        return Err("Artist must contain between 1 and 200 characters".into());
    }
    if !(1.0..=8.0).contains(&input.difficulty) {
        return Err("Difficulty must be between 1.0 and 8.0".into());
    }
    if input.model.trim().is_empty() || input.model.len() > 200 {
        return Err("Model must contain between 1 and 200 characters".into());
    }
    let audio_path = Path::new(&input.audio_file);
    if !audio_path.is_file() {
        return Err("Selected audio file does not exist".into());
    }
    let extension = audio_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "mp3" | "wav" | "ogg" | "flac") {
        return Err("Unsupported audio file type".into());
    }
    if !matches!(
        input.scenario.as_str(),
        "success" | "failure" | "oom" | "hang" | "crash" | "malformed"
    ) {
        return Err("Unknown mock worker scenario".into());
    }
    Ok(())
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
    jobs::reset_for_retry(&connection, &job_id)?;
    manager.enqueue(job_id, input)
}

#[tauri::command]
fn delete_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    jobs::delete(&connection, &job_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let (connection, _) = open_database(app.handle()).map_err(std::io::Error::other)?;
            jobs::mark_interrupted(&connection).map_err(std::io::Error::other)?;
            app.manage(worker::WorkerManager::new(app.handle().clone()));
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
            create_job,
            list_jobs,
            cancel_job,
            retry_job,
            delete_job
        ])
        .build(tauri::generate_context!())
        .expect("error while building Beatforge");
    application.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            app_handle.state::<worker::WorkerManager>().shutdown();
        }
    });
}
