use crate::{
    build_paths, jobs, open_database,
    protocol::{
        PROTOCOL_VERSION, StartJobInput, WorkerEvent, WorkerStartCommand, parse_worker_event,
    },
};
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

const WORKER_TIMEOUT: Duration = Duration::from_secs(8);

enum ManagerCommand {
    Enqueue { id: String, input: StartJobInput },
    Cancel { id: String },
    Shutdown { acknowledged: Sender<()> },
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
            .send(ManagerCommand::Enqueue { id, input })
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
            let _ = receiver.recv_timeout(Duration::from_secs(3));
        }
    }
}

struct ActiveWorker {
    id: String,
    child: Child,
    events: Receiver<Result<WorkerEvent, String>>,
    last_event: Instant,
    terminal_event_received: bool,
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
    let output_directory = std::path::PathBuf::from(&paths.jobs).join(&id);
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .arg("--mock-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to start worker: {error}"))?;
    let pid = child.id();
    let start_command = WorkerStartCommand {
        protocol_version: PROTOCOL_VERSION,
        message_type: "start_job".into(),
        job_id: id.clone(),
        audio_path: input.audio_file.clone(),
        output_path: output_directory.to_string_lossy().into_owned(),
        title: input.title,
        artist: input.artist,
        model: input.model,
        difficulty: input.difficulty,
        scenario: input.scenario,
    };
    let start_line = serde_json::to_string(&start_command).map_err(|error| error.to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Worker stdin is unavailable".to_string())?;
    writeln!(stdin, "{start_line}").map_err(|error| error.to_string())?;
    drop(stdin);

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
    thread::spawn(move || {
        if let Ok(mut log) = OpenOptions::new().create(true).append(true).open(log_path) {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = writeln!(log, "{line}");
            }
        }
    });

    update_job(
        app,
        &id,
        "running",
        0,
        "Starting worker",
        None,
        None,
        Some(pid),
    );
    Ok(ActiveWorker {
        id,
        child,
        events,
        last_event: Instant::now(),
        terminal_event_received: false,
    })
}

fn fail_active(app: &AppHandle, active: &mut ActiveWorker, code: &str, message: &str) {
    let _ = active.child.kill();
    let _ = active.child.wait();
    update_job(
        app,
        &active.id,
        "failed",
        0,
        code,
        Some(message),
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
            ..
        } => {
            let stage = if warnings.is_empty() {
                "Completed".into()
            } else {
                format!("Completed with warning: {}", warnings.join("; "))
            };
            update_job(
                app,
                &active.id,
                "completed",
                100,
                &stage,
                None,
                Some(&output_path),
                None,
            );
            active.terminal_event_received = true;
            true
        }
        WorkerEvent::Failed {
            code,
            message,
            recoverable,
            ..
        } => {
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
            true
        }
    }
}

fn coordinator(app: AppHandle, receiver: Receiver<ManagerCommand>) {
    let mut pending: VecDeque<(String, StartJobInput)> = VecDeque::new();
    let mut active: Option<ActiveWorker> = None;

    loop {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(ManagerCommand::Enqueue { id, input }) => pending.push_back((id, input)),
            Ok(ManagerCommand::Cancel { id }) => {
                if let Some(position) = pending.iter().position(|(pending_id, _)| pending_id == &id)
                {
                    pending.remove(position);
                    update_job(
                        &app,
                        &id,
                        "cancelled",
                        0,
                        "Cancelled before start",
                        None,
                        None,
                        None,
                    );
                } else if active.as_ref().is_some_and(|worker| worker.id == id)
                    && let Some(mut worker) = active.take()
                {
                    let _ = worker.child.kill();
                    let _ = worker.child.wait();
                    update_job(
                        &app,
                        &id,
                        "cancelled",
                        0,
                        "Worker cancelled",
                        None,
                        None,
                        None,
                    );
                }
            }
            Ok(ManagerCommand::Shutdown { acknowledged }) => {
                if let Some(mut worker) = active.take() {
                    let _ = worker.child.kill();
                    let _ = worker.child.wait();
                    update_job(
                        &app,
                        &worker.id,
                        "interrupted",
                        0,
                        "Application is shutting down",
                        Some("Worker interrupted by application shutdown"),
                        None,
                        None,
                    );
                }
                for (id, _) in pending.drain(..) {
                    update_job(
                        &app,
                        &id,
                        "interrupted",
                        0,
                        "Application is shutting down",
                        Some("Queued job interrupted by application shutdown"),
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
                Err(error) => update_job(
                    &app,
                    &id,
                    "failed",
                    0,
                    "WORKER_START_FAILED",
                    Some(&error),
                    None,
                    None,
                ),
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
            if !worker.terminal_event_received && worker.last_event.elapsed() > WORKER_TIMEOUT {
                fail_active(
                    &app,
                    worker,
                    "WORKER_TIMEOUT",
                    "Worker stopped reporting progress and was terminated",
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
            active = None;
        }
    }
}
