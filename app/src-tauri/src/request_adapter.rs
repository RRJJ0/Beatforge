use crate::{packages, protocol::PROTOCOL_VERSION, protocol::StartJobInput};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_DESCRIPTOR_COUNT: usize = 16;
const MAX_DESCRIPTOR_LENGTH: usize = 64;
const MAX_SEED: u32 = 65_535;
const REQUIRED_MODEL_FILES: [&str; 4] = [
    "model.safetensors",
    "config.json",
    "generation_config.json",
    "tokenizer.json",
];

#[derive(Debug, Clone)]
pub struct AdapterContext {
    pub runtime_root: PathBuf,
    pub models_root: PathBuf,
    pub jobs_root: PathBuf,
    pub source_path: PathBuf,
    pub source_commit: String,
    pub model_path: PathBuf,
    pub model_version: String,
    pub device: String,
    pub precision: String,
    pub repair_generated_output: bool,
    pub gpu_name: Option<String>,
    pub gpu_memory_total_mb: Option<u64>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionWorkerStartCommand {
    pub protocol_version: u16,
    #[serde(rename = "type")]
    pub message_type: &'static str,
    pub job_id: String,
    pub source_path: String,
    pub source_commit: String,
    pub model_path: String,
    pub model_version: String,
    pub gpu_name: Option<String>,
    pub gpu_memory_total_mb: Option<u64>,
    pub driver_version: Option<String>,
    pub audio_path: String,
    pub reference_beatmap_path: Option<String>,
    pub output_path: String,
    pub config: ProductionInferenceConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductionInferenceConfig {
    pub config_name: &'static str,
    pub gamemode: u8,
    pub difficulty: f64,
    pub seed: u32,
    pub descriptors: Vec<String>,
    pub negative_descriptors: Vec<String>,
    pub title: String,
    pub artist: String,
    pub approach_rate: Option<f64>,
    pub circle_size: Option<f64>,
    pub hitsounded: bool,
    pub device: String,
    pub precision: String,
    pub repair_generated_output: bool,
}

fn validate_text(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().count() > 200 {
        return Err(format!("{label} must contain between 1 and 200 characters"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} cannot contain control characters"));
    }
    Ok(())
}

fn validate_descriptors(values: &[String], label: &str) -> Result<(), String> {
    if values.len() > MAX_DESCRIPTOR_COUNT {
        return Err(format!(
            "{label} accepts at most {MAX_DESCRIPTOR_COUNT} values"
        ));
    }
    let mut normalized = std::collections::HashSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_DESCRIPTOR_LENGTH {
            return Err(format!(
                "Each {label} value must contain between 1 and {MAX_DESCRIPTOR_LENGTH} characters"
            ));
        }
        if trimmed.chars().any(char::is_control) || trimmed.contains([',', '[', ']', '"']) {
            return Err(format!("{label} contains an unsupported character"));
        }
        if !normalized.insert(trimmed.to_lowercase()) {
            return Err(format!("{label} contains a duplicate value"));
        }
    }
    Ok(())
}

fn validate_rating(value: Option<f64>, label: &str) -> Result<(), String> {
    if let Some(value) = value
        && (!value.is_finite() || !(0.0..=10.0).contains(&value))
    {
        return Err(format!("{label} must be between 0 and 10"));
    }
    Ok(())
}

pub fn validate_domain(input: &StartJobInput) -> Result<(), String> {
    validate_text(&input.title, "Title")?;
    validate_text(&input.artist, "Artist")?;
    if !input.difficulty.is_finite() || !(1.0..=8.0).contains(&input.difficulty) {
        return Err("Difficulty must be between 1.0 and 8.0".into());
    }
    if input.gamemode != 0 {
        return Err("The MVP currently supports osu!standard (gamemode 0) only".into());
    }
    if input.seed > MAX_SEED {
        return Err(format!("Seed must be between 0 and {MAX_SEED}"));
    }
    if input.model != packages::STANDARD_MODEL_ID {
        return Err(
            "Only the application-managed Mapperatorinator V32 standard model is supported".into(),
        );
    }
    validate_descriptors(&input.descriptors, "Descriptors")?;
    validate_descriptors(&input.negative_descriptors, "Negative descriptors")?;
    let positives = input
        .descriptors
        .iter()
        .map(|value| value.trim().to_lowercase())
        .collect::<std::collections::HashSet<_>>();
    if input
        .negative_descriptors
        .iter()
        .any(|value| positives.contains(&value.trim().to_lowercase()))
    {
        return Err("A descriptor cannot be both positive and negative".into());
    }
    validate_rating(input.approach_rate, "Approach rate")?;
    validate_rating(input.circle_size, "Circle size")?;
    validate_input_file(
        Path::new(&input.audio_file),
        "audio",
        &["mp3", "wav", "ogg", "m4a", "flac"],
    )?;
    if let Some(path) = input.reference_beatmap.as_deref() {
        validate_input_file(Path::new(path), "reference beatmap", &["osu"])?;
    }
    if !matches!(
        input.scenario.as_str(),
        "success" | "failure" | "oom" | "hang" | "crash" | "malformed"
    ) {
        return Err("Unknown mock worker scenario".into());
    }
    Ok(())
}

fn validate_input_file(path: &Path, label: &str, extensions: &[&str]) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Selected {label} file does not exist"));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !extensions.contains(&extension.as_str()) {
        return Err(format!("Unsupported {label} file type"));
    }
    Ok(())
}

fn canonical(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("Could not resolve {label}: {error}"))
}

fn canonical_managed_child(path: &Path, root: &Path, label: &str) -> Result<PathBuf, String> {
    let root = canonical(root, &format!("managed {label} root"))?;
    let path = canonical(path, label)?;
    if !path.starts_with(&root) {
        return Err(format!(
            "{label} is outside the application-managed directory"
        ));
    }
    Ok(path)
}

fn path_string(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(path) = value.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{path}")
    } else if let Some(path) = value.strip_prefix("\\\\?\\") {
        path.into()
    } else {
        value.into_owned()
    }
}

pub fn build(
    job_id: &str,
    input: &StartJobInput,
    context: &AdapterContext,
) -> Result<ProductionWorkerStartCommand, String> {
    validate_domain(input)?;
    if job_id.is_empty()
        || job_id.len() > 128
        || !job_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Job id contains unsupported characters".into());
    }
    if !matches!(context.device.as_str(), "cuda" | "cpu" | "auto") {
        return Err("Unsupported inference device".into());
    }
    if !matches!(context.precision.as_str(), "fp16" | "bf16" | "fp32") {
        return Err("Unsupported inference precision".into());
    }

    let source_path = canonical_managed_child(
        &context.source_path,
        &context.runtime_root,
        "Mapperatorinator source",
    )?;
    if !source_path.join("inference.py").is_file()
        || !source_path.join(".beatforge-install.json").is_file()
    {
        return Err("Managed Mapperatorinator source is incomplete".into());
    }
    let model_path =
        canonical_managed_child(&context.model_path, &context.models_root, "model package")?;
    let missing = REQUIRED_MODEL_FILES
        .iter()
        .filter(|name| !model_path.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("Managed model is missing {}", missing.join(", ")));
    }

    let jobs_root = canonical(&context.jobs_root, "managed jobs root")?;
    let output_path = jobs_root.join(job_id);
    fs::create_dir_all(&output_path)
        .map_err(|error| format!("Could not create managed job output: {error}"))?;
    let output_path = canonical_managed_child(&output_path, &jobs_root, "job output")?;
    let audio_path = canonical(Path::new(&input.audio_file), "audio file")?;
    let reference_beatmap_path = input
        .reference_beatmap
        .as_deref()
        .map(|path| canonical(Path::new(path), "reference beatmap"))
        .transpose()?;

    Ok(ProductionWorkerStartCommand {
        protocol_version: PROTOCOL_VERSION,
        message_type: "start_job",
        job_id: job_id.into(),
        source_path: path_string(&source_path),
        source_commit: context.source_commit.clone(),
        model_path: path_string(&model_path),
        model_version: context.model_version.clone(),
        gpu_name: context.gpu_name.clone(),
        gpu_memory_total_mb: context.gpu_memory_total_mb,
        driver_version: context.driver_version.clone(),
        audio_path: path_string(&audio_path),
        reference_beatmap_path: reference_beatmap_path.as_deref().map(path_string),
        output_path: path_string(&output_path),
        config: ProductionInferenceConfig {
            config_name: "v32",
            gamemode: input.gamemode,
            difficulty: input.difficulty,
            seed: input.seed,
            descriptors: input
                .descriptors
                .iter()
                .map(|value| value.trim().into())
                .collect(),
            negative_descriptors: input
                .negative_descriptors
                .iter()
                .map(|value| value.trim().into())
                .collect(),
            title: input.title.trim().into(),
            artist: input.artist.trim().into(),
            approach_rate: input.approach_rate,
            circle_size: input.circle_size,
            hitsounded: input.hitsounded,
            device: context.device.clone(),
            precision: context.precision.clone(),
            repair_generated_output: context.repair_generated_output,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> (PathBuf, StartJobInput, AdapterContext) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("beatforge-adapter-{suffix}"));
        let runtime_root = root.join("runtime");
        let models_root = root.join("models");
        let jobs_root = root.join("jobs");
        let source_path = runtime_root.join("mapperatorinator").join("commit");
        let model_path = models_root.join("standard");
        fs::create_dir_all(&source_path).unwrap();
        fs::create_dir_all(&model_path).unwrap();
        fs::create_dir_all(&jobs_root).unwrap();
        fs::write(source_path.join("inference.py"), "").unwrap();
        fs::write(source_path.join(".beatforge-install.json"), "{}").unwrap();
        for name in REQUIRED_MODEL_FILES {
            fs::write(model_path.join(name), "fixture").unwrap();
        }
        let audio = root.join("song.mp3");
        fs::write(&audio, "fixture").unwrap();
        let input = StartJobInput {
            title: "Song".into(),
            artist: "Artist".into(),
            audio_file: path_string(&audio),
            model: packages::STANDARD_MODEL_ID.into(),
            difficulty: 5.2,
            gamemode: 0,
            seed: 12345,
            descriptors: vec!["skillset/jumps".into()],
            negative_descriptors: vec![],
            reference_beatmap: None,
            approach_rate: Some(9.0),
            circle_size: Some(4.0),
            hitsounded: true,
            scenario: "success".into(),
        };
        let context = AdapterContext {
            runtime_root,
            models_root,
            jobs_root,
            source_path,
            source_commit: "commit".into(),
            model_path,
            model_version: "32.test".into(),
            device: "cuda".into(),
            precision: "fp16".into(),
            repair_generated_output: true,
            gpu_name: Some("Fixture GPU".into()),
            gpu_memory_total_mb: Some(8192),
            driver_version: Some("999.0".into()),
        };
        (root, input, context)
    }

    #[test]
    fn builds_only_resolved_managed_paths_and_whitelisted_config() {
        let (root, input, context) = fixture();
        let command = build("BF-test_1", &input, &context).unwrap();
        assert_eq!(command.config.gamemode, 0);
        assert_eq!(command.config.seed, 12345);
        assert!(command.config.repair_generated_output);
        assert_eq!(command.gpu_name.as_deref(), Some("Fixture GPU"));
        assert_eq!(command.gpu_memory_total_mb, Some(8192));
        assert_eq!(command.driver_version.as_deref(), Some("999.0"));
        assert!(Path::new(&command.audio_path).is_absolute());
        assert!(
            Path::new(&command.model_path)
                .canonicalize()
                .unwrap()
                .starts_with(context.models_root.canonicalize().unwrap())
        );
        assert!(
            Path::new(&command.output_path)
                .canonicalize()
                .unwrap()
                .starts_with(context.jobs_root.canonicalize().unwrap())
        );
        assert!(!command.source_path.starts_with("\\\\?\\"));
        assert!(!command.model_path.starts_with("\\\\?\\"));
        assert!(!command.audio_path.starts_with("\\\\?\\"));
        assert!(!command.output_path.starts_with("\\\\?\\"));
        let json = serde_json::to_value(command).unwrap();
        assert!(json.get("scenario").is_none());
        assert!(json.get("model").is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn forwards_raw_output_preference_to_the_worker() {
        let (root, input, mut context) = fixture();
        context.repair_generated_output = false;

        let command = build("BF-raw-output", &input, &context).unwrap();

        assert!(!command.config.repair_generated_output);
        assert_eq!(
            serde_json::to_value(command).unwrap()["config"]["repair_generated_output"],
            false
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_non_standard_and_ambiguous_descriptors() {
        let (root, mut input, context) = fixture();
        input.gamemode = 3;
        assert!(
            build("BF-test", &input, &context)
                .unwrap_err()
                .contains("gamemode 0")
        );
        input.gamemode = 0;
        input.negative_descriptors = input.descriptors.clone();
        assert!(
            build("BF-test", &input, &context)
                .unwrap_err()
                .contains("both positive and negative")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_model_outside_managed_root() {
        let (root, input, mut context) = fixture();
        context.model_path = root.join("outside-model");
        fs::create_dir_all(&context.model_path).unwrap();
        for name in REQUIRED_MODEL_FILES {
            fs::write(context.model_path.join(name), "fixture").unwrap();
        }
        assert!(
            build("BF-test", &input, &context)
                .unwrap_err()
                .contains("outside")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn serializes_windows_extended_paths_for_third_party_libraries() {
        assert_eq!(
            path_string(Path::new(r"\\?\C:\Beatforge\runtime")),
            r"C:\Beatforge\runtime"
        );
        assert_eq!(
            path_string(Path::new(r"\\?\UNC\server\share\model")),
            r"\\server\share\model"
        );
    }
}
