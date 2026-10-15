use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn output_directory(test_name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("beatforge-{test_name}-{unique}"))
}

fn start_command(job_id: &str, output_path: &std::path::Path, scenario: &str) -> String {
    serde_json::json!({
        "protocolVersion": 1,
        "type": "start_job",
        "jobId": job_id,
        "audioPath": "C:\\Music\\test.mp3",
        "outputPath": output_path,
        "title": "Integration Test",
        "artist": "Beatforge",
        "model": "Mock",
        "difficulty": 4.5,
        "scenario": scenario
    })
    .to_string()
}

fn run_worker(job_id: &str, output_path: &std::path::Path, scenario: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_app"))
        .arg("--mock-worker")
        .env("BEATFORGE_MOCK_STAGE_DELAY_MS", "5")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("worker starts");
    writeln!(
        child.stdin.take().expect("worker stdin"),
        "{}",
        start_command(job_id, output_path, scenario)
    )
    .expect("start command is sent");
    child.wait_with_output().expect("worker exits")
}

fn json_events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8(output.stdout.clone())
        .expect("UTF-8 stdout")
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid JSONL event"))
        .collect()
}

#[test]
fn worker_completes_over_json_lines() {
    let output_path = output_directory("success");
    let result = run_worker("integration-success", &output_path, "success");
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let events = json_events(&result);
    assert!(events.iter().any(|event| event["type"] == "progress"));
    assert!(events.iter().any(|event| event["type"] == "completed"));
    assert!(output_path.join("mock-result.osu").is_file());
    let _ = fs::remove_dir_all(output_path);
}

#[test]
fn worker_faults_have_stable_machine_readable_outcomes() {
    for (scenario, code) in [
        ("failure", "MOCK_GENERATION_FAILED"),
        ("oom", "CUDA_OUT_OF_MEMORY"),
    ] {
        let output_path = output_directory(scenario);
        let result = run_worker(&format!("integration-{scenario}"), &output_path, scenario);
        assert_eq!(result.status.code(), Some(4));
        let events = json_events(&result);
        let failure = events.last().expect("terminal failure event");
        assert_eq!(failure["type"], "failed");
        assert_eq!(failure["code"], code);
        assert_eq!(failure["recoverable"], true);
        assert!(!output_path.join("mock-result.osu").exists());
        let _ = fs::remove_dir_all(output_path);
    }

    let crash_root = output_directory("crash");
    let crash = run_worker("integration-crash", &crash_root, "crash");
    assert_eq!(crash.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&crash.stderr).contains("unexpected worker exit"));
    let _ = fs::remove_dir_all(crash_root);

    let malformed_root = output_directory("malformed");
    let malformed = run_worker("integration-malformed", &malformed_root, "malformed");
    assert!(malformed.status.success());
    assert!(serde_json::from_slice::<serde_json::Value>(&malformed.stdout).is_err());
    let _ = fs::remove_dir_all(malformed_root);
}

#[test]
fn sequential_worker_soak_keeps_jobs_isolated() {
    let iterations = std::env::var("BEATFORGE_SOAK_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8)
        .clamp(1, 1_000);
    let root = output_directory("soak");
    fs::create_dir_all(&root).expect("soak root");

    for index in 0..iterations {
        let job_id = format!("integration-soak-{index}");
        let job_root = root.join(&job_id);
        let result = run_worker(&job_id, &job_root, "success");
        assert!(result.status.success(), "iteration {index} failed");
        let events = json_events(&result);
        assert_eq!(events.last().expect("terminal event")["job_id"], job_id);
        assert_eq!(events.last().expect("terminal event")["type"], "completed");
        assert!(job_root.join("mock-result.osu").is_file());
    }

    assert_eq!(
        fs::read_dir(&root).expect("soak directory").count(),
        iterations
    );
    fs::remove_dir_all(&root).expect("soak artifacts are reclaimed");
    assert!(!root.exists());
}

#[test]
fn stalled_worker_can_be_terminated_and_reaped() {
    let output_path = output_directory("hang");
    let mut child = Command::new(env!("CARGO_BIN_EXE_app"))
        .arg("--mock-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("worker starts");
    writeln!(
        child.stdin.take().expect("worker stdin"),
        "{}",
        start_command("integration-hang", &output_path, "hang")
    )
    .expect("start command is sent");
    thread::sleep(Duration::from_millis(250));
    child.kill().expect("worker is terminated");
    let status = child.wait().expect("worker is reaped");
    assert!(!status.success());
    let _ = fs::remove_dir_all(output_path);
}
