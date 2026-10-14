use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyRequest {
    pub id: String,
    pub name: String,
    pub difficulty: f64,
    pub seed: u32,
    #[serde(default)]
    pub descriptors: Vec<String>,
    #[serde(default)]
    pub negative_descriptors: Vec<String>,
    #[serde(default)]
    pub approach_rate: Option<f64>,
    #[serde(default)]
    pub circle_size: Option<f64>,
    #[serde(default = "default_true")]
    pub hitsounded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartJobInput {
    pub title: String,
    pub artist: String,
    pub audio_file: String,
    pub model: String,
    pub difficulty: f64,
    /// Empty for requests created before multi-difficulty beatmap sets.
    #[serde(default)]
    pub difficulties: Vec<DifficultyRequest>,
    #[serde(default)]
    pub gamemode: u8,
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub descriptors: Vec<String>,
    #[serde(default)]
    pub negative_descriptors: Vec<String>,
    #[serde(default)]
    pub reference_beatmap: Option<String>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub video_file: Option<String>,
    #[serde(default)]
    pub video_start_time_ms: i64,
    #[serde(default)]
    pub approach_rate: Option<f64>,
    #[serde(default)]
    pub circle_size: Option<f64>,
    #[serde(default = "default_true")]
    pub hitsounded: bool,
    #[serde(default = "default_scenario")]
    pub scenario: String,
}

fn default_scenario() -> String {
    "success".into()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStartCommand {
    pub protocol_version: u16,
    #[serde(rename = "type")]
    pub message_type: String,
    pub job_id: String,
    pub audio_path: String,
    pub output_path: String,
    pub title: String,
    pub artist: String,
    pub model: String,
    pub difficulty: f64,
    pub scenario: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerCancelCommand {
    pub protocol_version: u16,
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerEvent {
    Progress {
        protocol_version: u16,
        job_id: String,
        stage: String,
        percent: u8,
        message: String,
    },
    Completed {
        protocol_version: u16,
        job_id: String,
        output_path: String,
        warnings: Vec<String>,
        #[serde(default)]
        provenance: Option<serde_json::Value>,
    },
    Failed {
        protocol_version: u16,
        job_id: String,
        code: String,
        message: String,
        recoverable: bool,
    },
}

impl WorkerEvent {
    pub fn protocol_version(&self) -> u16 {
        match self {
            Self::Progress {
                protocol_version, ..
            }
            | Self::Completed {
                protocol_version, ..
            }
            | Self::Failed {
                protocol_version, ..
            } => *protocol_version,
        }
    }

    pub fn job_id(&self) -> &str {
        match self {
            Self::Progress { job_id, .. }
            | Self::Completed { job_id, .. }
            | Self::Failed { job_id, .. } => job_id,
        }
    }
}

pub fn parse_worker_event(line: &str) -> Result<WorkerEvent, String> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(format!("Worker message exceeds {MAX_MESSAGE_BYTES} bytes"));
    }
    let event: WorkerEvent =
        serde_json::from_str(line).map_err(|error| format!("Invalid worker JSON: {error}"))?;
    if event.protocol_version() != PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported protocol version {}; expected {PROTOCOL_VERSION}",
            event.protocol_version()
        ));
    }
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_event() {
        let event = parse_worker_event(
            r#"{"type":"progress","protocol_version":1,"job_id":"job_1","stage":"generating","percent":42,"message":"Working"}"#,
        )
        .expect("valid event");
        assert_eq!(event.job_id(), "job_1");
    }

    #[test]
    fn parses_completed_provenance() {
        let event = parse_worker_event(
            r#"{"type":"completed","protocol_version":1,"job_id":"job_1","output_path":"result.osz","warnings":[],"provenance":{"schema_version":1,"reproduction_key":"abc"}}"#,
        )
        .expect("valid completed event");
        match event {
            WorkerEvent::Completed { provenance, .. } => {
                assert_eq!(provenance.unwrap()["reproduction_key"], "abc");
            }
            _ => panic!("expected completed event"),
        }
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_worker_event("not-json").is_err());
    }

    #[test]
    fn rejects_unknown_protocol_version() {
        let error = parse_worker_event(
            r#"{"type":"progress","protocol_version":99,"job_id":"job_1","stage":"generating","percent":1,"message":"Working"}"#,
        )
        .expect_err("unknown version must fail");
        assert!(error.contains("Unsupported protocol version"));
    }

    #[test]
    fn serializes_versioned_cancel_command() {
        let value = serde_json::to_value(WorkerCancelCommand {
            protocol_version: PROTOCOL_VERSION,
            message_type: "cancel_job",
            job_id: "BF-TEST".into(),
        })
        .unwrap();
        assert_eq!(value["type"], "cancel_job");
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(value["job_id"], "BF-TEST");
    }
}
