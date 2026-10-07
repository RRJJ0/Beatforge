use crate::validation::{ArchiveInspection, RepairRecord};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsuRepairResult {
    pub bytes: Vec<u8>,
    pub repairs: Vec<RepairRecord>,
}

fn record(
    code: &str,
    section: Option<&str>,
    field: Option<&str>,
    before: Option<&str>,
    after: Option<&str>,
    reason: &str,
) -> RepairRecord {
    RepairRecord {
        code: code.into(),
        section: section.map(str::to_owned),
        field: field.map(str::to_owned),
        before: before.map(str::to_owned),
        after: after.map(str::to_owned),
        reason: reason.into(),
    }
}

fn metadata_field(key: &str) -> bool {
    matches!(
        key,
        "Title"
            | "TitleUnicode"
            | "Artist"
            | "ArtistUnicode"
            | "Creator"
            | "Version"
            | "Source"
            | "Tags"
            | "BeatmapID"
            | "BeatmapSetID"
    )
}

fn integer_scalar(section: &str, key: &str) -> bool {
    matches!(
        (section, key),
        (
            "General",
            "AudioLeadIn"
                | "PreviewTime"
                | "Countdown"
                | "Mode"
                | "LetterboxInBreaks"
                | "WidescreenStoryboard"
        ) | ("Editor", "BeatDivisor" | "GridSize")
            | ("Metadata", "BeatmapID" | "BeatmapSetID")
    )
}

fn float_scalar(section: &str, key: &str) -> bool {
    matches!(
        (section, key),
        ("General", "StackLeniency")
            | ("Editor", "DistanceSpacing" | "TimelineZoom")
            | (
                "Difficulty",
                "HPDrainRate"
                    | "CircleSize"
                    | "OverallDifficulty"
                    | "ApproachRate"
                    | "SliderMultiplier"
                    | "SliderTickRate"
            )
    )
}

fn canonical_scalar(section: &str, key: &str, value: &str) -> Option<String> {
    if integer_scalar(section, key) {
        return value.parse::<i64>().ok().map(|number| number.to_string());
    }
    if float_scalar(section, key) {
        return value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(|number| number.to_string());
    }
    None
}

pub fn normalize_osu_bytes(bytes: &[u8]) -> Result<OsuRepairResult, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "Automatic repair only accepts valid UTF-8 beatmaps".to_string())?;
    let mut repairs = Vec::new();
    let had_bom = text.starts_with('\u{feff}');
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if had_bom {
        repairs.push(record(
            "OSU_BOM_REMOVED",
            None,
            None,
            Some("UTF-8 with BOM"),
            Some("UTF-8"),
            "Removed the optional byte-order mark without changing beatmap content",
        ));
    }

    let had_crlf = text.contains("\r\n");
    let had_bare_cr = text.replace("\r\n", "").contains('\r');
    let had_final_newline = text.ends_with(['\n', '\r']);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if had_crlf || had_bare_cr {
        repairs.push(record(
            "OSU_LINE_ENDINGS_NORMALIZED",
            None,
            None,
            Some(if had_crlf && had_bare_cr {
                "mixed CRLF/CR"
            } else if had_crlf {
                "CRLF"
            } else {
                "CR"
            }),
            Some("LF"),
            "Normalized text line endings without changing parsed values",
        ));
    }

    let mut section = String::new();
    let mut output = Vec::new();
    for line in normalized.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].to_string();
            output.push(line.to_string());
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once(':') else {
            output.push(line.to_string());
            continue;
        };
        let key = raw_key.trim();
        let value = raw_value.trim();
        if section == "Metadata" && metadata_field(key) {
            let canonical = format!("{key}:{value}");
            if canonical != line {
                repairs.push(record(
                    "METADATA_WHITESPACE_NORMALIZED",
                    Some("Metadata"),
                    Some(key),
                    Some("surrounding whitespace"),
                    Some("trimmed"),
                    "Normalized only whitespace ignored by the osu! key-value parser",
                ));
            }
            output.push(canonical);
            continue;
        }
        if let Some(canonical_value) = canonical_scalar(&section, key, value) {
            if canonical_value != value {
                repairs.push(record(
                    "SCALAR_SPELLING_NORMALIZED",
                    Some(&section),
                    Some(key),
                    Some("equivalent scalar spelling"),
                    Some("canonical scalar spelling"),
                    "Normalized a scalar only after parsing an equivalent finite value",
                ));
                output.push(format!("{key}:{canonical_value}"));
            } else {
                output.push(line.to_string());
            }
        } else {
            output.push(line.to_string());
        }
    }
    let mut output = output.join("\n");
    if !had_final_newline {
        output.push('\n');
        repairs.push(record(
            "OSU_FINAL_NEWLINE_ADDED",
            None,
            None,
            Some("missing"),
            Some("LF"),
            "Added the conventional final newline without changing parsed content",
        ));
    }
    Ok(OsuRepairResult {
        bytes: output.into_bytes(),
        repairs,
    })
}

pub fn repair_extracted_archive(
    root: &Path,
    inspection: &ArchiveInspection,
) -> Result<Vec<RepairRecord>, String> {
    let mut repairs = Vec::new();
    for entry in &inspection.entries {
        if !entry.is_directory {
            let original = entry.name.trim_end_matches('/');
            if original != entry.relative_path {
                repairs.push(record(
                    "ARCHIVE_FILENAME_NORMALIZED",
                    Some("Archive"),
                    None,
                    Some(&entry.name),
                    Some(&entry.relative_path),
                    "Normalized a safe archive-relative filename during isolated extraction",
                ));
            }
        }
    }
    for relative in &inspection.beatmap_paths {
        let path = root.join(relative);
        let bytes = fs::read(&path)
            .map_err(|error| format!("Could not read extracted beatmap for repair: {error}"))?;
        let result = normalize_osu_bytes(&bytes)?;
        if result.repairs.is_empty() {
            continue;
        }
        fs::write(&path, result.bytes)
            .map_err(|error| format!("Could not write repaired working beatmap: {error}"))?;
        repairs.extend(result.repairs);
    }
    Ok(repairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{ArchiveEntryInfo, ArchiveInspection};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalizes_only_allowlisted_text_repairs() {
        let source = b"\xef\xbb\xbfosu file format v14\r\n[Metadata]\r\n Title :  Song  \r\n[Difficulty]\r\nCircleSize:04.000\r\n[TimingPoints]\r\n0,500,4,2,1,70,1,0";
        let repaired = normalize_osu_bytes(source).unwrap();
        let text = String::from_utf8(repaired.bytes).unwrap();
        assert!(!text.starts_with('\u{feff}'));
        assert!(!text.contains('\r'));
        assert!(text.contains("Title:Song\n"));
        assert!(text.contains("CircleSize:4\n"));
        assert!(text.ends_with('\n'));
        assert_eq!(repaired.repairs.len(), 5);
    }

    #[test]
    fn never_rewrites_timing_or_hit_object_lines() {
        let source = b"osu file format v14\n[TimingPoints]\n0,500.000,4,2,1,70,1,0\n[HitObjects]\n256,192,1000,1,0,0:0:0:0:\n";
        let repaired = normalize_osu_bytes(source).unwrap();
        assert_eq!(repaired.bytes, source);
        assert!(repaired.repairs.is_empty());
    }

    #[test]
    fn repairs_extracted_maps_and_records_safe_archive_name_changes() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("beatforge-repair-{nonce}"));
        fs::create_dir_all(root.join("folder")).unwrap();
        fs::write(
            root.join("folder/map.osu"),
            b"osu file format v14\r\n[TimingPoints]\r\n0,500,4,2,1,70,1,0",
        )
        .unwrap();
        let inspection = ArchiveInspection {
            entries: vec![ArchiveEntryInfo {
                name: "folder\\map.osu".into(),
                relative_path: "folder/map.osu".into(),
                compressed_size: 1,
                uncompressed_size: 1,
                is_directory: false,
                is_beatmap: true,
            }],
            beatmap_paths: vec!["folder/map.osu".into()],
            total_uncompressed_bytes: 1,
        };

        let repairs = repair_extracted_archive(&root, &inspection).unwrap();

        assert!(
            repairs
                .iter()
                .any(|repair| repair.code == "ARCHIVE_FILENAME_NORMALIZED")
        );
        assert!(
            repairs
                .iter()
                .any(|repair| repair.code == "OSU_LINE_ENDINGS_NORMALIZED")
        );
        assert!(
            !fs::read_to_string(root.join("folder/map.osu"))
                .unwrap()
                .contains('\r')
        );
        fs::remove_dir_all(root).unwrap();
    }
}
