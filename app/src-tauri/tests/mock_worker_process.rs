use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
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

#[test]
fn worker_completes_over_json_lines() {
    let output_path = output_directory("success");
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
        start_command("integration-success", &output_path, "success")
    )
    .expect("start command is sent");

    let result = child.wait_with_output().expect("worker exits");
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let events: Vec<serde_json::Value> = String::from_utf8(result.stdout)
        .expect("UTF-8 stdout")
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid JSONL event"))
        .collect();
    assert!(events.iter().any(|event| event["type"] == "progress"));
    assert!(events.iter().any(|event| event["type"] == "completed"));
    assert!(output_path.join("mock-result.json").is_file());
    let _ = fs::remove_dir_all(output_path);
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
    thread::sleep(Duration::from_secs(2));
    child.kill().expect("worker is terminated");
    let status = child.wait().expect("worker is reaped");
    assert!(!status.success());
    let _ = fs::remove_dir_all(output_path);
}
