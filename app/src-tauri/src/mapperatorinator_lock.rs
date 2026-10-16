use crate::packages;
use serde::Deserialize;

const BUILTIN_LOCK: &str = include_str!("../resources/manifests/mapperatorinator-v32.lock.json");
const DEPENDENCY_LOCK: &str =
    include_str!("../resources/python/mapperatorinator-windows-py310.requirements.txt");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationLock {
    schema_version: u32,
    pub source: SourceLock,
    pub slider: SliderLock,
    pub dependencies: DependencyLock,
    pub model: ModelLock,
    inspected_files: Vec<InspectedFile>,
    pub integration: IntegrationBoundary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SliderLock {
    pub repository: String,
    pub commit: String,
    committed_at: String,
    version: String,
    pub archive: ArchiveLock,
    license: LicenseLock,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyLock {
    python_version: String,
    platform: String,
    resolver: String,
    pub file_name: String,
    package_count: usize,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLock {
    pub repository: String,
    pub commit: String,
    committed_at: String,
    pub archive: ArchiveLock,
    license: LicenseLock,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveLock {
    pub url: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LicenseLock {
    spdx: String,
    file: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelLock {
    pub repository: String,
    pub revision: String,
    pub package_id: String,
    license: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InspectedFile {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationBoundary {
    pub config_name: String,
    adapter_strategy: String,
    pub supported_gamemodes: Vec<u8>,
    callable_surface: Vec<String>,
    stdout_policy: String,
    excluded_features: Vec<String>,
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl IntegrationLock {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 {
            return Err(format!(
                "Unsupported Mapperatorinator lock schema: {}",
                self.schema_version
            ));
        }
        if self.source.repository != "https://github.com/OliBomby/Mapperatorinator"
            || !valid_commit(&self.source.commit)
            || self.source.committed_at.is_empty()
        {
            return Err("Invalid Mapperatorinator source identity".into());
        }
        let archive = &self.source.archive;
        if !archive
            .url
            .starts_with("https://github.com/OliBomby/Mapperatorinator/archive/")
            || archive.size_bytes == 0
            || archive.file_name.contains(['/', '\\'])
            || !valid_sha(&archive.sha256)
        {
            return Err("Invalid Mapperatorinator source archive".into());
        }
        if self.source.license.spdx != "MIT"
            || self.source.license.file != "LICENSE"
            || !valid_sha(&self.source.license.sha256)
        {
            return Err("Invalid Mapperatorinator source license lock".into());
        }
        if self.slider.repository != "https://github.com/OliBomby/slider"
            || !valid_commit(&self.slider.commit)
            || self.slider.committed_at.is_empty()
            || self.slider.version != "0.8.2"
            || !self
                .slider
                .archive
                .url
                .starts_with("https://github.com/OliBomby/slider/archive/")
            || self.slider.archive.size_bytes == 0
            || !valid_sha(&self.slider.archive.sha256)
            || self.slider.license.spdx != "LGPL-3.0-or-later"
            || self.slider.license.file != "LICENSE"
            || !valid_sha(&self.slider.license.sha256)
        {
            return Err("Invalid slider source lock".into());
        }
        if self.dependencies.python_version != "3.10"
            || self.dependencies.platform != "windows-x86_64"
            || self.dependencies.resolver != "uv 0.12.21"
            || self.dependencies.file_name != "mapperatorinator-windows-py310.requirements.txt"
            || self.dependencies.package_count != 41
            || !valid_sha(&self.dependencies.sha256)
            || sha256(DEPENDENCY_LOCK.as_bytes()) != self.dependencies.sha256
            || !DEPENDENCY_LOCK.contains("--hash=sha256:")
            || DEPENDENCY_LOCK.contains("git+")
        {
            return Err("Invalid Mapperatorinator dependency lock".into());
        }
        if self.model.repository != "https://huggingface.co/OliBomby/Mapperatorinator-v32"
            || !valid_commit(&self.model.revision)
            || self.model.package_id != packages::STANDARD_MODEL_ID
            || self.model.license != "MIT"
        {
            return Err("Invalid Mapperatorinator model identity".into());
        }
        if self.inspected_files.len() < 4
            || self.inspected_files.iter().any(|file| {
                file.path.is_empty() || file.size_bytes == 0 || !valid_sha(&file.sha256)
            })
        {
            return Err("Invalid inspected source file lock".into());
        }
        let integration = &self.integration;
        let required_calls = [
            "compile_args",
            "setup_inference_environment",
            "load_model_with_server",
            "get_config",
            "generate",
        ];
        if integration.config_name != "v32"
            || integration.adapter_strategy != "python_api"
            || integration.supported_gamemodes != [0]
            || integration.stdout_policy != "json_lines_only"
            || required_calls
                .iter()
                .any(|name| !integration.callable_surface.iter().any(|call| call == name))
            || !integration
                .excluded_features
                .iter()
                .any(|feature| feature == "hydra_cli")
        {
            return Err("Invalid Mapperatorinator integration boundary".into());
        }
        Ok(())
    }
}

pub fn builtin() -> Result<IntegrationLock, String> {
    let lock: IntegrationLock =
        serde_json::from_str(BUILTIN_LOCK).map_err(|error| error.to_string())?;
    lock.validate()?;
    Ok(lock)
}

pub fn validate_builtin() -> Result<(), String> {
    builtin().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_integration_lock_is_valid() {
        let lock = builtin().unwrap();
        assert_eq!(
            lock.source.commit,
            "0e2b0e387aab4b35c64b0b11b12d47578dea7587"
        );
        assert_eq!(
            lock.model.revision,
            "74f22583400d259bf424819e11027c17933efe54"
        );
        assert_eq!(
            lock.slider.commit,
            "ea28c484a0a542d6e094e25aa02e1529ff2c8313"
        );
        assert_eq!(lock.dependencies.package_count, 41);
        assert_eq!(lock.integration.supported_gamemodes, [0]);
    }
}
