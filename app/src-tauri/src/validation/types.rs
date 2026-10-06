use serde::{Deserialize, Serialize};

pub const VALIDATION_SCHEMA_VERSION: u16 = 1;
pub const VALIDATOR_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    ValidWithWarnings,
    Repaired,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationCode {
    OutputNotFound,
    OutputOutsideJobRoot,
    UnsupportedOutputType,
    ArchiveInvalid,
    ArchivePathTraversal,
    ArchiveAbsolutePath,
    ArchiveCaseCollision,
    ArchiveLinkEntry,
    ArchiveLimitExceeded,
    BeatmapNotFound,
    BeatmapLimitExceeded,
    OsuInvalidEncoding,
    OsuFormatUnsupported,
    SectionMissing,
    FieldInvalid,
    AudioMissing,
    TimingInvalid,
    HitObjectInvalid,
    ObjectOutOfBounds,
    ObjectAfterAudioEnd,
    MetadataIncomplete,
    AutoRepairApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub severity: IssueSeverity,
    pub section: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub repairable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatmapStatistics {
    pub beatmap_count: u32,
    pub circle_count: u32,
    pub slider_count: u32,
    pub spinner_count: u32,
    pub max_combo: Option<u32>,
    pub star_rating: Option<f64>,
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairRecord {
    pub code: String,
    pub section: Option<String>,
    pub field: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub schema_version: u16,
    pub validator_version: String,
    pub job_id: String,
    pub status: ValidationStatus,
    pub source_path: String,
    pub validated_path: Option<String>,
    pub issues: Vec<ValidationIssue>,
    pub statistics: Option<BeatmapStatistics>,
    pub repairs: Vec<RepairRecord>,
    pub started_at: String,
    pub completed_at: String,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    pub fn can_complete(&self) -> bool {
        !self.has_errors()
            && matches!(
                self.status,
                ValidationStatus::Valid
                    | ValidationStatus::ValidWithWarnings
                    | ValidationStatus::Repaired
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(status: ValidationStatus, severity: IssueSeverity) -> ValidationReport {
        ValidationReport {
            schema_version: VALIDATION_SCHEMA_VERSION,
            validator_version: VALIDATOR_VERSION.into(),
            job_id: "BF-VALIDATION".into(),
            status,
            source_path: "jobs/BF-VALIDATION/raw/result.osz".into(),
            validated_path: None,
            issues: vec![ValidationIssue {
                code: ValidationCode::MetadataIncomplete,
                severity,
                section: Some("Metadata".into()),
                line: Some(12),
                message: "Creator is empty".into(),
                repairable: true,
            }],
            statistics: None,
            repairs: vec![],
            started_at: "2026-10-06T00:00:00Z".into(),
            completed_at: "2026-10-06T00:00:01Z".into(),
        }
    }

    #[test]
    fn report_uses_the_stable_camel_case_wire_shape() {
        let value = serde_json::to_value(report(
            ValidationStatus::ValidWithWarnings,
            IssueSeverity::Warning,
        ))
        .unwrap();
        assert_eq!(value["schemaVersion"], VALIDATION_SCHEMA_VERSION);
        assert_eq!(value["validatorVersion"], VALIDATOR_VERSION);
        assert_eq!(value["status"], "valid_with_warnings");
        assert_eq!(value["issues"][0]["code"], "METADATA_INCOMPLETE");
        assert_eq!(value["issues"][0]["severity"], "warning");
        assert!(value.get("source_path").is_none());
    }

    #[test]
    fn errors_can_never_complete() {
        let invalid = report(ValidationStatus::Invalid, IssueSeverity::Error);
        assert!(invalid.has_errors());
        assert!(!invalid.can_complete());
    }

    #[test]
    fn warnings_can_complete() {
        let warning = report(ValidationStatus::ValidWithWarnings, IssueSeverity::Warning);
        assert!(!warning.has_errors());
        assert!(warning.can_complete());
    }
}
