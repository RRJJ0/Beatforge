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
mod protocol;
mod request_adapter;
mod runtime;
#[cfg(test)]
mod stage4_acceptance;
pub mod validation;
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
    executable: Option<String>,
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
            precision: "bf16".into(),
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
    let mut settings = match value {
        Some(json) => serde_json::from_str(&json).map_err(|error| error.to_string())?,
        None => AppSettings::default_for(&paths),
    };
    // V32 uses bf16 on CUDA. Migrate the old fp16 default, which can produce
    // invalid sampling probabilities during timing generation.
    if settings.precision == "fp16" {
        settings.precision = "bf16".into();
    }
    Ok(settings)
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
fn delete_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let (connection, _) = open_database(&app)?;
    jobs::delete(&connection, &job_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            delete_job,
            reveal_job_output,
            import_job_output
        ])
        .build(tauri::generate_context!())
        .expect("error while building Beatforge");
    application.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            app_handle.state::<worker::WorkerManager>().shutdown();
        }
    });
}
