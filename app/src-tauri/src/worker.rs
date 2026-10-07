use crate::{
    build_paths, formal_worker, jobs, open_database,
    protocol::{
        PROTOCOL_VERSION, StartJobInput, WorkerCancelCommand, WorkerEvent, WorkerStartCommand,
        parse_worker_event,
    },
};
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const MOCK_WORKER_TIMEOUT: Duration = Duration::from_secs(8);
const PRODUCTION_WORKER_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const RUNTIME_LOADING_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const GRACEFUL_STOP_TIMEOUT: Duration = Duration::from_secs(3);

fn is_runtime_loading_stage(stage: &str) -> bool {
    matches!(
        stage,
        "loading_numpy"
            | "loading_pytorch"
            | "loading_audio_runtime"
            | "loading_upstream"
            | "loading_inference_config"
    )
}

enum ManagerCommand {
    Enqueue {
        id: String,
        input: Box<StartJobInput>,
    },
    Cancel {
        id: String,
    },
    Shutdown {
        acknowledged: Sender<()>,
    },
}

pub struct WorkerManager {
    sender: Sender<ManagerCommand>,
}

impl WorkerManager {
    pub fn new(app: AppHandle) -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || coordinator(app, receiver));
        Self { sender }
    }

    pub fn enqueue(&self, id: String, input: StartJobInput) -> Result<(), String> {
        self.sender
            .send(ManagerCommand::Enqueue {
                id,
                input: Box::new(input),
            })
            .map_err(|_| "Worker manager is not available".into())
    }

    pub fn cancel(&self, id: String) -> Result<(), String> {
        self.sender
            .send(ManagerCommand::Cancel { id })
            .map_err(|_| "Worker manager is not available".into())
    }

    pub fn shutdown(&self) {
        let (acknowledged, receiver) = mpsc::channel();
        if self
            .sender
            .send(ManagerCommand::Shutdown { acknowledged })
            .is_ok()
        {
            let _ = receiver.recv_timeout(Duration::from_secs(6));
        }
    }
}

struct ActiveWorker {
    id: String,
    child: Child,
    stdin: Option<ChildStdin>,
    events: Receiver<Result<WorkerEvent, String>>,
    last_event: Instant,
    stage: String,
    stage_started: Instant,
    timeout: Duration,
    terminal_event_received: bool,
    cleanup_on_exit: bool,
    production: bool,
}

fn drain_worker_stderr<R: Read>(mut stderr: R, log_path: &Path) {
    match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(mut log) => {
            let _ = io::copy(&mut stderr, &mut log);
        }
        Err(_) => {
            // Never close the child's stderr pipe just because logging failed.
            // A closed pipe can turn an otherwise successful Python shutdown
            // into exit code 120 while it flushes its standard streams.
            let _ = io::copy(&mut stderr, &mut io::sink());
        }
    }
}

fn remove_managed_job_directory(jobs_root: &Path, id: &str) -> Result<(), String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.contains(['/', '\\'])
        || Path::new(id).components().count() != 1
    {
        return Err("Refusing to clean an invalid job id".into());
    }
    let target = jobs_root.join(id);
    if !target.exists() {
        return Ok(());
    }
    let root = jobs_root
        .canonicalize()
        .map_err(|error| format!("Failed to resolve jobs directory: {error}"))?;
    let resolved = target
        .canonicalize()
        .map_err(|error| format!("Failed to resolve job directory: {error}"))?;
    if resolved.parent() != Some(root.as_path()) {
        return Err("Refusing to clean a job directory outside the managed root".into());
    }
    std::fs::remove_dir_all(&resolved)
        .map_err(|error| format!("Failed to clean job temporary files: {error}"))
}

pub(crate) fn cleanup_job_artifacts(app: &AppHandle, id: &str) -> Result<(), String> {
    let paths = build_paths(app)?;
    remove_managed_job_directory(&PathBuf::from(paths.jobs), id)
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if child.try_wait().is_ok_and(|status| status.is_some()) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

fn force_stop_process_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut taskkill = Command::new("taskkill");
        taskkill
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = taskkill.status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn stop_worker(active: &mut ActiveWorker) {
    if let Some(mut stdin) = active.stdin.take() {
        let cancel = WorkerCancelCommand {
            protocol_version: PROTOCOL_VERSION,
            message_type: "cancel_job",
            job_id: active.id.clone(),
        };
        if let Ok(cancel) = serde_json::to_string(&cancel) {
            let _ = writeln!(stdin, "{cancel}");
        }
        let _ = stdin.flush();
    }
    if !wait_for_exit(&mut active.child, GRACEFUL_STOP_TIMEOUT) {
        force_stop_process_tree(&mut active.child);
    }
}

fn emit_job(app: &AppHandle, id: &str) {
    if let Ok((connection, _)) = open_database(app)
        && let Ok(Some(record)) = jobs::get(&connection, id)
    {
        let _ = app.emit("job-event", record);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_job(
    app: &AppHandle,
    id: &str,
    status: &str,
    progress: u8,
    stage: &str,
    error: Option<&str>,
    output_path: Option<&str>,
    pid: Option<u32>,
) {
    if let Ok((connection, _)) = open_database(app) {
        let _ = jobs::update_state(
            &connection,
            id,
            status,
            progress,
            stage,
            error,
            output_path,
            pid,
        );
    }
    emit_job(app, id);
}

fn spawn_worker(app: &AppHandle, id: String, input: StartJobInput) -> Result<ActiveWorker, String> {
    let paths = build_paths(app)?;
    let production = input.scenario == "success";
    let (mut command, start_line, timeout) = if production {
        let python = crate::installed_python(app)?.ok_or_else(|| {
            "Install the private Python runtime before starting inference".to_string()
        })?;
        let start_command = crate::build_production_request(app, &id, &input)?;
        let runtime_root = std::path::PathBuf::from(&paths.runtime);
        let process = formal_worker::production_process(&python, &runtime_root)?;
        let line = serde_json::to_string(&start_command).map_err(|error| error.to_string())?;
        (process, line, PRODUCTION_WORKER_TIMEOUT)
    } else {
        let output_directory = std::path::PathBuf::from(&paths.jobs).join(&id);
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut process = Command::new(executable);
        process
            .arg("--mock-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x0800_0000);
        }
        let start_command = WorkerStartCommand {
            protocol_version: PROTOCOL_VERSION,
            message_type: "start_job".into(),
            job_id: id.clone(),
            audio_path: input.audio_file.clone(),
            output_path: output_directory.to_string_lossy().into_owned(),
            title: input.title.clone(),
            artist: input.artist.clone(),
            model: input.model.clone(),
            difficulty: input.difficulty,
            scenario: input.scenario.clone(),
        };
        let line = serde_json::to_string(&start_command).map_err(|error| error.to_string())?;
        (process, line, MOCK_WORKER_TIMEOUT)
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to start worker: {error}"))?;
    let pid = child.id();
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Worker stdin is unavailable".to_string())?;
    writeln!(stdin, "{start_line}").map_err(|error| error.to_string())?;
    stdin.flush().map_err(|error| error.to_string())?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Worker stdout is unavailable".to_string())?;
    let (event_sender, events) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let parsed = line
                .map_err(|error| error.to_string())
                .and_then(|line| parse_worker_event(&line));
            if event_sender.send(parsed).is_err() {
                break;
            }
        }
    });

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Worker stderr is unavailable".to_string())?;
    let log_path = std::path::PathBuf::from(paths.logs).join(format!("job-{id}.log"));
    thread::spawn(move || drain_worker_stderr(stderr, &log_path));

    update_job(
        app,
        &id,
        "running",
        0,
        if production {
            "Starting Mapperatorinator"
        } else {
            "Starting mock worker"
        },
        None,
        None,
        Some(pid),
    );
    Ok(ActiveWorker {
        id,
        child,
        stdin: Some(stdin),
        events,
        last_event: Instant::now(),
        stage: "starting_worker".into(),
        stage_started: Instant::now(),
        timeout,
        terminal_event_received: false,
        cleanup_on_exit: false,
        production,
    })
}

fn fail_active(app: &AppHandle, active: &mut ActiveWorker, code: &str, message: &str) {
    stop_worker(active);
    let cleanup_error = cleanup_job_artifacts(app, &active.id).err();
    let message = cleanup_error
        .map(|error| format!("{message} Temporary cleanup also failed: {error}"))
        .unwrap_or_else(|| message.to_string());
    update_job(
        app,
        &active.id,
        "failed",
        0,
        code,
        Some(&message),
        None,
        None,
    );
    active.terminal_event_received = true;
}

fn process_event(app: &AppHandle, active: &mut ActiveWorker, event: WorkerEvent) -> bool {
    if event.job_id() != active.id {
        fail_active(
            app,
            active,
            "PROTOCOL_JOB_MISMATCH",
            "Worker event referenced a different job",
        );
        return true;
    }
    active.last_event = Instant::now();
    match event {
        WorkerEvent::Progress {
            stage,
            percent,
            message,
            ..
        } => {
            if active.stage != stage {
                active.stage.clone_from(&stage);
                active.stage_started = Instant::now();
            }
            update_job(
                app,
                &active.id,
                "running",
                percent.min(99),
                &message,
                None,
                None,
                Some(active.child.id()),
            );
            let _ = stage;
            false
        }
        WorkerEvent::Completed {
            output_path,
            warnings,
            provenance,
            ..
        } => {
            if active.production && provenance.is_none() {
                fail_active(
                    app,
                    active,
                    "PROVENANCE_MISSING",
                    "The production worker completed without a reproducibility record",
                );
                return true;
            }
            if let Some(provenance) = provenance {
                let saved = open_database(app).and_then(|(connection, _)| {
                    jobs::save_provenance(&connection, &active.id, &provenance)
                });
                if let Err(error) = saved {
                    fail_active(
                        app,
                        active,
                        "PROVENANCE_SAVE_FAILED",
                        &format!("Could not save the reproducibility record: {error}"),
                    );
                    return true;
                }
            }
            update_job(
                app,
                &active.id,
                "validating",
                98,
                "Preserving and validating generated beatmap",
                None,
                None,
                Some(active.child.id()),
            );
            let validation = build_paths(app).and_then(|paths| {
                let job_root = PathBuf::from(paths.jobs).join(&active.id);
                crate::validation::validate_candidate(
                    &active.id,
                    &job_root,
                    Path::new(&output_path),
                    &warnings,
                    crate::validation::ValidationLimits::default(),
                )
                .map(|report| (job_root, report))
            });
            let (job_root, report) = match validation {
                Ok(result) => result,
                Err(error) => {
                    update_job(
                        app,
                        &active.id,
                        "validation_failed",
                        100,
                        "VALIDATION_PIPELINE_FAILED",
                        Some(&error),
                        None,
                        None,
                    );
                    active.terminal_event_received = true;
                    active.cleanup_on_exit = false;
                    return true;
                }
            };
            let validated_output = report
                .can_complete()
                .then_some(report.validated_path.as_deref())
                .flatten()
                .map(|relative| job_root.join(relative).to_string_lossy().into_owned());
            let saved = open_database(app).and_then(|(connection, _)| {
                jobs::finish_validation(
                    &connection,
                    &active.id,
                    &report,
                    validated_output.as_deref(),
                )
            });
            if let Err(error) = saved {
                update_job(
                    app,
                    &active.id,
                    "validation_failed",
                    100,
                    "VALIDATION_REPORT_SAVE_FAILED",
                    Some(&format!("Could not save the validation report: {error}")),
                    None,
                    None,
                );
            } else {
                emit_job(app, &active.id);
                if report.can_complete()
                    && let Err(error) = crate::try_auto_import(app, &active.id)
                {
                    log::warn!("Automatic osu! import was not started: {error}");
                }
            }
            active.terminal_event_received = true;
            active.cleanup_on_exit = false;
            true
        }
        WorkerEvent::Failed {
            code,
            message,
            recoverable,
            ..
        } => {
            if code == "JOB_CANCELLED" {
                update_job(
                    app,
                    &active.id,
                    "cancelled",
                    0,
                    "Cancelled",
                    None,
                    None,
                    None,
                );
                active.terminal_event_received = true;
                active.cleanup_on_exit = true;
                return true;
            }
            let stage = if recoverable {
                format!("{code} · Retry available")
            } else {
                code
            };
            update_job(
                app,
                &active.id,
                "failed",
                0,
                &stage,
                Some(&message),
                None,
                None,
            );
            active.terminal_event_received = true;
            active.cleanup_on_exit = true;
            true
        }
    }
}

fn coordinator(app: AppHandle, receiver: Receiver<ManagerCommand>) {
    let mut pending: VecDeque<(String, StartJobInput)> = VecDeque::new();
    let mut active: Option<ActiveWorker> = None;

    loop {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(ManagerCommand::Enqueue { id, input }) => pending.push_back((id, *input)),
            Ok(ManagerCommand::Cancel { id }) => {
                if let Some(position) = pending.iter().position(|(pending_id, _)| pending_id == &id)
                {
                    pending.remove(position);
                    let cleanup_error = cleanup_job_artifacts(&app, &id).err();
                    update_job(
                        &app,
                        &id,
                        "cancelled",
                        0,
                        "Cancelled before start",
                        cleanup_error.as_deref(),
                        None,
                        None,
                    );
                } else if active
                    .as_ref()
                    .is_some_and(|worker| worker.id == id && !worker.terminal_event_received)
                    && let Some(mut worker) = active.take()
                {
                    stop_worker(&mut worker);
                    let cleanup_error = cleanup_job_artifacts(&app, &id).err();
                    update_job(
                        &app,
                        &id,
                        "cancelled",
                        0,
                        "Worker cancelled",
                        cleanup_error.as_deref(),
                        None,
                        None,
                    );
                }
            }
            Ok(ManagerCommand::Shutdown { acknowledged }) => {
                if let Some(mut worker) = active.take() {
                    stop_worker(&mut worker);
                    let cleanup_error = cleanup_job_artifacts(&app, &worker.id).err();
                    update_job(
                        &app,
                        &worker.id,
                        "interrupted",
                        0,
                        "Application is shutting down",
                        cleanup_error
                            .as_deref()
                            .or(Some("Worker interrupted by application shutdown")),
                        None,
                        None,
                    );
                }
                for (id, _) in pending.drain(..) {
                    let cleanup_error = cleanup_job_artifacts(&app, &id).err();
                    update_job(
                        &app,
                        &id,
                        "interrupted",
                        0,
                        "Application is shutting down",
                        cleanup_error
                            .as_deref()
                            .or(Some("Queued job interrupted by application shutdown")),
                        None,
                        None,
                    );
                }
                let _ = acknowledged.send(());
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if active.is_none()
            && let Some((id, input)) = pending.pop_front()
        {
            match spawn_worker(&app, id.clone(), input) {
                Ok(worker) => active = Some(worker),
                Err(error) => {
                    let cleanup_error = cleanup_job_artifacts(&app, &id).err();
                    let message = cleanup_error
                        .map(|cleanup| format!("{error} Temporary cleanup also failed: {cleanup}"))
                        .unwrap_or(error);
                    update_job(
                        &app,
                        &id,
                        "failed",
                        0,
                        "WORKER_START_FAILED",
                        Some(&message),
                        None,
                        None,
                    );
                }
            }
        }

        let mut finished = false;
        if let Some(worker) = active.as_mut() {
            while let Ok(event) = worker.events.try_recv() {
                match event {
                    Ok(event) => {
                        process_event(&app, worker, event);
                    }
                    Err(error) => {
                        fail_active(&app, worker, "PROTOCOL_ERROR", &error);
                    }
                }
            }
            if !worker.terminal_event_received && worker.last_event.elapsed() > worker.timeout {
                fail_active(
                    &app,
                    worker,
                    "WORKER_TIMEOUT",
                    "Worker stopped reporting progress and was terminated",
                );
            }
            if !worker.terminal_event_received
                && is_runtime_loading_stage(&worker.stage)
                && worker.stage_started.elapsed() > RUNTIME_LOADING_TIMEOUT
            {
                fail_active(
                    &app,
                    worker,
                    "RUNTIME_LOADING_TIMEOUT",
                    "The managed Python runtime stopped responding while loading dependencies. Restart Beatforge and retry; repair the AI environment if it happens again.",
                );
            }
            match worker.child.try_wait() {
                Ok(Some(status)) => {
                    if !worker.terminal_event_received {
                        for _ in 0..3 {
                            match worker.events.recv_timeout(Duration::from_millis(30)) {
                                Ok(Ok(event)) => {
                                    process_event(&app, worker, event);
                                }
                                Ok(Err(error)) => {
                                    fail_active(&app, worker, "PROTOCOL_ERROR", &error);
                                    break;
                                }
                                Err(_) => break,
                            }
                            if worker.terminal_event_received {
                                break;
                            }
                        }
                    }
                    if !worker.terminal_event_received {
                        worker.cleanup_on_exit = true;
                        update_job(
                            &app,
                            &worker.id,
                            "failed",
                            0,
                            "WORKER_EXITED",
                            Some(&format!("Worker exited unexpectedly with status {status}")),
                            None,
                            None,
                        );
                    }
                    finished = true;
                }
                Ok(None) => {}
                Err(error) => {
                    fail_active(&app, worker, "WORKER_MONITOR_FAILED", &error.to_string());
                    finished = true;
                }
            }
        }
        if finished {
            if let Some(worker) = active.as_ref()
                && worker.cleanup_on_exit
            {
                let _ = cleanup_job_artifacts(&app, &worker.id);
            }
            active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{drain_worker_stderr, is_runtime_loading_stage, remove_managed_job_directory};
    use std::{fs, time::SystemTime};

    #[test]
    fn stderr_logging_preserves_non_utf8_progress_bytes() {
        let suffix = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("beatforge-stderr-{suffix}.log"));
        let bytes = b"Generating timing\r\x81\x40 progress\r100%\n";
        drain_worker_stderr(&bytes[..], &path);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn only_dependency_imports_use_the_runtime_loading_timeout() {
        assert!(is_runtime_loading_stage("loading_numpy"));
        assert!(is_runtime_loading_stage("loading_upstream"));
        assert!(is_runtime_loading_stage("loading_inference_config"));
        assert!(!is_runtime_loading_stage("loading_model"));
        assert!(!is_runtime_loading_stage("generating"));
    }

    #[test]
    fn cleanup_removes_only_a_direct_managed_job_directory() {
        let suffix = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("beatforge-job-cleanup-{suffix}"));
        let job = root.join("BF-TEST");
        fs::create_dir_all(&job).unwrap();
        fs::write(job.join("partial.tmp"), b"partial").unwrap();
        remove_managed_job_directory(&root, "BF-TEST").unwrap();
        assert!(!job.exists());
        assert!(remove_managed_job_directory(&root, "../outside").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
