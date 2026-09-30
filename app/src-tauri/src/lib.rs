use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use sysinfo::{Disks, System};
use tauri::{AppHandle, Manager};

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

fn build_paths(app: &AppHandle) -> Result<AppPaths, String> {
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

fn open_database(app: &AppHandle) -> Result<(Connection, AppPaths), String> {
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
    Ok((connection, paths))
}

fn detect_gpu() -> GpuInfo {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            open_database(app.handle()).map_err(std::io::Error::other)?;
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
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while building Beatforge");
}
