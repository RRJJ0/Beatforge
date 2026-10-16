//! Stable contracts for phase 6 output validation.
//!
//! Parsing and validation execution are introduced in later phase 6 steps. This
//! module deliberately fixes the serialized report, issue, repair, statistics,
//! artifact, and resource-limit shapes first.

pub mod archive;
pub mod limits;
pub mod osu;
pub mod pipeline;
pub mod repair;
pub mod statistics;
pub mod types;

pub use archive::{
    ArchiveEntryInfo, ArchiveInspection, ArchiveResult, ExtractedArchive, extract_osz, inspect_osz,
    package_osz_from_directory,
};
pub use limits::ValidationLimits;
pub use pipeline::{
    filter_invalid_set_difficulties, validate_candidate, validate_candidate_with_delivery_name,
};
pub use repair::{OsuRepairResult, normalize_osu_bytes, repair_extracted_archive};
pub use statistics::{
    aggregate_statistics, calculate_archive_statistics, calculate_beatmap_statistics,
    calculate_file_statistics,
};
pub use types::{
    BeatmapStatistics, IssueSeverity, RepairManifest, RepairManifestStatus, RepairRecord,
    ValidationCode, ValidationIssue, ValidationReport, ValidationStatus,
};

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    fn collect_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(directory).expect("fixture directory is readable") {
            let path = entry.expect("fixture entry is readable").path();
            if path.is_dir() {
                collect_files(&path, files);
            } else {
                files.push(path);
            }
        }
    }

    #[test]
    fn validation_fixtures_are_small_reviewable_text_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("validation");
        let mut files = Vec::new();
        collect_files(&root, &mut files);
        assert!(files.len() >= 7);

        for path in files {
            let metadata = fs::metadata(&path).expect("fixture metadata is readable");
            assert!(
                metadata.len() < 64 * 1024,
                "fixture must remain below 64 KiB: {}",
                path.display()
            );
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            assert!(
                matches!(extension, "osu" | "json" | "md"),
                "fixtures must remain reviewable text: {}",
                path.display()
            );
        }

        let hostile: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("hostile/archive-cases.json")).expect("hostile recipe exists"),
        )
        .expect("hostile recipe is valid JSON");
        assert_eq!(hostile["schemaVersion"], 1);
        assert!(
            hostile["cases"]
                .as_array()
                .is_some_and(|cases| !cases.is_empty())
        );
    }
}
