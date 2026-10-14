use crate::protocol::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const WORKER_SOURCE: &str = include_str!("../resources/python/beatforge_worker.py");
const WORKER_VERSION: &str = "11";
const HEALTH_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_STDERR_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct WorkerHealth {
    pub protocol_version: u16,
    pub worker_version: String,
    pub ready: bool,
    pub python_version: String,
    pub python_executable: String,
    pub pytorch_version: Option<String>,
    pub cuda_runtime: Option<String>,
    pub cuda_available: bool,
    pub gpu_name: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub ffmpeg_executable: Option<String>,
    pub mapperatorinator_commit: Option<String>,
    pub model_version: Option<String>,
    pub model_path: Option<String>,
    pub errors: Vec<String>,
}

fn unique_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{timestamp}", std::process::id())
}

pub fn materialize(runtime_root: &Path) -> Result<PathBuf, String> {
    let worker_root = runtime_root.join("worker").join(WORKER_VERSION);
    fs::create_dir_all(&worker_root)
        .map_err(|error| format!("Failed to create worker runtime directory: {error}"))?;
    let target = worker_root.join("beatforge_worker.py");
    if fs::read_to_string(&target).is_ok_and(|current| current == WORKER_SOURCE) {
        return Ok(target);
    }
    let suffix = unique_suffix();
    let staging = worker_root.join(format!(".beatforge_worker.py.staging-{suffix}"));
    let backup = worker_root.join(format!(".beatforge_worker.py.backup-{suffix}"));
    fs::write(&staging, WORKER_SOURCE)
        .map_err(|error| format!("Failed to stage production worker: {error}"))?;
    if target.exists() {
        fs::rename(&target, &backup)
            .map_err(|error| format!("Failed to preserve previous production worker: {error}"))?;
    }
    if let Err(error) = fs::rename(&staging, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_file(&staging);
        return Err(format!("Failed to activate production worker: {error}"));
    }
    let _ = fs::remove_file(&backup);
    Ok(target)
}

pub fn production_process(python: &Path, runtime_root: &Path) -> Result<Command, String> {
    let script = materialize(runtime_root)?;
    let mut process = Command::new(python);
    process
        .args(["-I", &script.to_string_lossy()])
        .env("PYTHONUTF8", "1")
        .env("PYTHONUNBUFFERED", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x0800_0000);
    }
    Ok(process)
}

fn read_protocol_stdout<R: Read>(reader: R) -> Result<String, String> {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let read = reader
        .by_ref()
        .take((MAX_MESSAGE_BYTES + 2) as u64)
        .read_until(b'\n', &mut line)
        .map_err(|error| format!("Failed to read worker stdout: {error}"))?;
    if read == 0 {
        return Err("Production worker did not return a health message".into());
    }
    if line.len() > MAX_MESSAGE_BYTES + 1 {
        return Err(format!(
            "Worker health message exceeds {MAX_MESSAGE_BYTES} bytes"
        ));
    }
    if !line.ends_with(b"\n") {
        return Err("Worker health message is not newline terminated".into());
    }
    let result = String::from_utf8(line[..line.len() - 1].to_vec())
        .map_err(|_| "Worker health message is not UTF-8".to_string())?;
    let mut extra = Vec::new();
    reader
        .read_to_end(&mut extra)
        .map_err(|error| format!("Failed to finish reading worker stdout: {error}"))?;
    if extra.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err("Production worker wrote non-protocol data to stdout".into());
    }
    Ok(result)
}

fn read_capped_stderr<R: Read>(mut reader: R) -> std::io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = MAX_STDERR_BYTES.saturating_sub(kept.len());
        let copy = remaining.min(read);
        kept.extend_from_slice(&buffer[..copy]);
        truncated |= copy < read;
    }
    if truncated {
        kept.extend_from_slice(b"\n[production worker stderr truncated]\n");
    }
    Ok(kept)
}

pub fn parse_health(line: &str) -> Result<WorkerHealth, String> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(format!(
            "Worker health message exceeds {MAX_MESSAGE_BYTES} bytes"
        ));
    }
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| format!("Invalid worker health JSON: {error}"))?;
    if value.get("type").and_then(|value| value.as_str()) != Some("health") {
        return Err("Production worker returned an unexpected message type".into());
    }
    let health: WorkerHealth = serde_json::from_value(value)
        .map_err(|error| format!("Invalid worker health payload: {error}"))?;
    if health.protocol_version != PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported worker protocol version {}; expected {PROTOCOL_VERSION}",
            health.protocol_version
        ));
    }
    if health.worker_version != WORKER_VERSION {
        return Err(format!(
            "Unsupported production worker version {}; expected {WORKER_VERSION}",
            health.worker_version
        ));
    }
    Ok(health)
}

#[allow(clippy::too_many_arguments)]
pub fn health_check(
    python: &Path,
    runtime_root: &Path,
    source_path: &Path,
    source_commit: &str,
    model_path: Option<&Path>,
    model_version: &str,
    log_path: &Path,
) -> Result<WorkerHealth, String> {
    let command = serde_json::json!({
        "protocol_version": PROTOCOL_VERSION,
        "type": "health_check",
        "source_path": source_path.to_string_lossy(),
        "source_commit": source_commit,
        "model_path": model_path.map(|path| path.to_string_lossy().into_owned()),
        "model_version": model_version,
    });
    let mut process = production_process(python, runtime_root)?;
    let mut child = process
        .spawn()
        .map_err(|error| format!("Failed to start production worker: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Production worker stdin is unavailable".to_string())?;
    writeln!(stdin, "{command}")
        .map_err(|error| format!("Failed to send health command: {error}"))?;
    drop(stdin);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Production worker stdout is unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Production worker stderr is unavailable".to_string())?;
    let stdout_reader = thread::spawn(move || read_protocol_stdout(stdout));
    let stderr_reader = thread::spawn(move || read_capped_stderr(stderr));

    let deadline = Instant::now() + HEALTH_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Production worker health check timed out after 90 seconds".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Failed to monitor production worker: {error}"));
            }
        }
    };
    let stdout_line = stdout_reader
        .join()
        .map_err(|_| "Production worker stdout reader panicked".to_string())??;
    let stderr_bytes = stderr_reader
        .join()
        .map_err(|_| "Production worker stderr reader panicked".to_string())?
        .map_err(|error| format!("Failed to read production worker log: {error}"))?;
    let stderr_text = String::from_utf8_lossy(&stderr_bytes);
    if !stderr_bytes.is_empty() {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create worker log directory: {error}"))?;
        }
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .map_err(|error| format!("Failed to open production worker log: {error}"))?;
        writeln!(log, "--- health check ---")
            .and_then(|_| log.write_all(&stderr_bytes))
            .map_err(|error| format!("Failed to write production worker log: {error}"))?;
    }
    let health = parse_health(&stdout_line)?;
    if !status.success() && health.ready {
        return Err(format!(
            "Production worker exited with {status}: {}",
            stderr_text.trim()
        ));
    }
    Ok(health)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_directory() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("beatforge-formal-worker-{}", unique_suffix()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn parses_versioned_health_message() {
        let health = parse_health(
            r#"{"type":"health","protocol_version":1,"worker_version":"11","ready":true,"python_version":"3.10.21","python_executable":"python.exe","pytorch_version":"2.10.0","cuda_runtime":"13.0","cuda_available":true,"gpu_name":"GPU","ffmpeg_version":"7.0","ffmpeg_executable":"ffmpeg.exe","mapperatorinator_commit":"abc","model_version":"32.test","model_path":"model","errors":[]}"#,
        )
        .unwrap();
        assert!(health.ready);
        assert_eq!(health.worker_version, "11");
    }

    #[test]
    fn rejects_non_health_and_wrong_versions() {
        assert!(parse_health(r#"{"type":"progress"}"#).is_err());
        assert!(
            parse_health(
                r#"{"type":"health","protocol_version":99,"worker_version":"11","ready":false,"python_version":"3.10","python_executable":"python.exe","cuda_available":false,"errors":[]}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn bootstrap_reserves_stdout_for_json_lines() {
        assert!(WORKER_SOURCE.contains("PROTOCOL_STDOUT = reserve_protocol_stdout()"));
        assert!(WORKER_SOURCE.contains("os.dup2(sys.stderr.fileno(), sys.stdout.fileno())"));
        assert!(WORKER_SOURCE.contains("SetStdHandle"));
        assert!(WORKER_SOURCE.contains("redirect_stdout(sys.stderr)"));
        assert!(WORKER_SOURCE.contains("MAX_MESSAGE_BYTES = 64 * 1024"));
        assert!(WORKER_SOURCE.contains("inference.compile_args(args"));
        assert!(WORKER_SOURCE.contains("inference.load_model_with_server("));
        assert!(WORKER_SOURCE.contains("inference.generate("));
        assert!(WORKER_SOURCE.contains("HF_HUB_OFFLINE"));
        assert!(WORKER_SOURCE.contains("install_local_v32_backbone_config"));
        assert!(WORKER_SOURCE.contains("backbone_config"));
        assert!(
            WORKER_SOURCE
                .contains("args.precision = \"bf16\" if config[\"precision\"] == \"fp16\"")
        );
        assert!(WORKER_SOURCE.contains("args.fast_decoder_loop = False"));
        assert!(WORKER_SOURCE.contains("classify_inference_error"));
        assert!(WORKER_SOURCE.contains("watch_cancellation"));
        assert!(WORKER_SOURCE.contains("loading_upstream"));
        assert!(WORKER_SOURCE.contains("import numpy"));
        assert!(WORKER_SOURCE.contains("CANCEL_REQUESTED.is_set()"));
        assert!(WORKER_SOURCE.contains("JOB_CANCELLED"));
        assert!(WORKER_SOURCE.contains("Beatmap inference is still running"));
        assert!(WORKER_SOURCE.contains("install_generated_group_guard"));
        assert!(WORKER_SOURCE.contains("config[\"repair_generated_output\"]"));
        assert!(WORKER_SOURCE.contains("update_event_times"));
        assert!(WORKER_SOURCE.contains("tracked_timing_point_at"));
        assert!(WORKER_SOURCE.contains("guarded_calc_length"));
        assert!(WORKER_SOURCE.contains("defaulted_non_positive_slider_lengths"));
        assert!(WORKER_SOURCE.contains("defaulted_non_positive_slider_svs"));
        assert!(WORKER_SOURCE.contains("dropped_out_of_order_groups"));
        assert!(WORKER_SOURCE.contains("data_utils.TIMED_EVENTS"));
    }

    #[test]
    fn materialization_repairs_a_modified_bootstrap() {
        let root = temp_directory();
        let path = materialize(&root).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), WORKER_SOURCE);
        fs::write(&path, "modified").unwrap();
        assert_eq!(materialize(&root).unwrap(), path);
        assert_eq!(fs::read_to_string(&path).unwrap(), WORKER_SOURCE);
        fs::remove_dir_all(root).unwrap();
    }
}
