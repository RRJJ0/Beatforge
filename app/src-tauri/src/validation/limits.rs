use serde::{Deserialize, Serialize};

pub const RAW_DIRECTORY: &str = "raw";
pub const WORKING_DIRECTORY: &str = "working";
pub const VALIDATED_DIRECTORY: &str = "validated";
pub const VALIDATION_REPORT_FILE: &str = "validation-report.json";
pub const REPAIR_MANIFEST_FILE: &str = "repair-manifest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationLimits {
    pub max_archive_entries: usize,
    pub max_archive_uncompressed_bytes: u64,
    pub max_single_entry_bytes: u64,
    pub max_beatmap_entries: usize,
    pub max_beatmap_bytes: u64,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_archive_entries: 256,
            max_archive_uncompressed_bytes: 1024 * 1024 * 1024,
            max_single_entry_bytes: 512 * 1024 * 1024,
            max_beatmap_entries: 64,
            max_beatmap_bytes: 16 * 1024 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_match_the_validation_contract() {
        let limits = ValidationLimits::default();
        assert_eq!(limits.max_archive_entries, 256);
        assert_eq!(limits.max_archive_uncompressed_bytes, 1024 * 1024 * 1024);
        assert_eq!(limits.max_single_entry_bytes, 512 * 1024 * 1024);
        assert_eq!(limits.max_beatmap_entries, 64);
        assert_eq!(limits.max_beatmap_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn artifact_names_are_relative_single_components() {
        for name in [
            RAW_DIRECTORY,
            WORKING_DIRECTORY,
            VALIDATED_DIRECTORY,
            VALIDATION_REPORT_FILE,
            REPAIR_MANIFEST_FILE,
        ] {
            assert!(!name.is_empty());
            assert!(!name.contains(['/', '\\']));
            assert_ne!(name, ".");
            assert_ne!(name, "..");
        }
    }
}
