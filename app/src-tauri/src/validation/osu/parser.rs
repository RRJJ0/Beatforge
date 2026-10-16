use super::model::{
    DifficultySection, EditorSection, GeneralSection, KeyValueField, Located, MetadataSection,
    ParseResult, ParsedBeatmap, RawHitObject, RawLine, RawTimingPoint, UnknownSection,
};
use crate::validation::{IssueSeverity, ValidationCode, ValidationIssue, ValidationLimits};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const SUPPORTED_FORMAT_VERSION: u32 = 14;
const REQUIRED_SECTIONS: [&str; 5] = [
    "General",
    "Metadata",
    "Difficulty",
    "TimingPoints",
    "HitObjects",
];
const KNOWN_SECTIONS: [&str; 8] = [
    "General",
    "Editor",
    "Metadata",
    "Difficulty",
    "Events",
    "TimingPoints",
    "Colours",
    "HitObjects",
];

#[derive(Debug)]
struct SectionBuffer {
    name: String,
    header_line: u32,
    lines: Vec<RawLine>,
}

fn issue(
    code: ValidationCode,
    severity: IssueSeverity,
    section: Option<&str>,
    line: Option<u32>,
    message: impl Into<String>,
    repairable: bool,
) -> ValidationIssue {
    ValidationIssue {
        code,
        severity,
        section: section.map(str::to_owned),
        line,
        message: message.into(),
        repairable,
    }
}

fn fatal(code: ValidationCode, message: impl Into<String>) -> ParseResult {
    ParseResult {
        beatmap: None,
        issues: vec![issue(
            code,
            IssueSeverity::Error,
            None,
            None,
            message,
            false,
        )],
    }
}

fn meaningful(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && !line.starts_with("//")
}

fn parse_header(lines: &[(u32, &str)]) -> Result<(u32, usize), ValidationIssue> {
    let Some((index, (line_number, header))) = lines
        .iter()
        .enumerate()
        .find(|(_, (_, line))| meaningful(line))
    else {
        return Err(issue(
            ValidationCode::OsuFormatUnsupported,
            IssueSeverity::Error,
            None,
            None,
            "The beatmap is empty and has no osu! format header",
            false,
        ));
    };
    let header = header.trim();
    let Some(version) = header.strip_prefix("osu file format v") else {
        return Err(issue(
            ValidationCode::OsuFormatUnsupported,
            IssueSeverity::Error,
            None,
            Some(*line_number),
            "The first content line is not an osu! format header",
            false,
        ));
    };
    let Ok(version) = version.trim().parse::<u32>() else {
        return Err(issue(
            ValidationCode::OsuFormatUnsupported,
            IssueSeverity::Error,
            None,
            Some(*line_number),
            "The osu! format version is not an integer",
            false,
        ));
    };
    if version != SUPPORTED_FORMAT_VERSION {
        return Err(issue(
            ValidationCode::OsuFormatUnsupported,
            IssueSeverity::Error,
            None,
            Some(*line_number),
            format!(
                "osu! file format v{version} is unsupported; expected v{SUPPORTED_FORMAT_VERSION}"
            ),
            false,
        ));
    }
    Ok((version, index))
}

fn split_sections(
    lines: &[(u32, &str)],
    header_index: usize,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<SectionBuffer> {
    let mut sections = Vec::<SectionBuffer>::new();
    let mut current: Option<SectionBuffer> = None;

    for (line_number, raw) in lines.iter().skip(header_index + 1) {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with('[') || trimmed.ends_with(']') {
            if trimmed.starts_with('[')
                && trimmed.ends_with(']')
                && trimmed.len() > 2
                && !trimmed[1..trimmed.len() - 1].contains(['[', ']'])
            {
                if let Some(previous) = current.take() {
                    sections.push(previous);
                }
                current = Some(SectionBuffer {
                    name: trimmed[1..trimmed.len() - 1].to_owned(),
                    header_line: *line_number,
                    lines: Vec::new(),
                });
            } else {
                issues.push(issue(
                    ValidationCode::FieldInvalid,
                    IssueSeverity::Error,
                    None,
                    Some(*line_number),
                    format!("Malformed section header: {trimmed}"),
                    false,
                ));
            }
            continue;
        }
        if let Some(section) = current.as_mut() {
            section.lines.push(RawLine {
                value: trimmed.to_owned(),
                line: *line_number,
            });
        } else {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                None,
                Some(*line_number),
                "Content appears outside a section",
                false,
            ));
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}

fn key_value_fields(
    section: &SectionBuffer,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<KeyValueField> {
    let mut fields = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in &section.lines {
        let Some((key, value)) = raw.value.split_once(':') else {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some(&section.name),
                Some(raw.line),
                "Expected a key-value field separated by ':'",
                false,
            ));
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some(&section.name),
                Some(raw.line),
                "Field key cannot be empty",
                false,
            ));
            continue;
        }
        if !seen.insert(key.to_ascii_lowercase()) {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some(&section.name),
                Some(raw.line),
                format!("Duplicate field: {key}"),
                false,
            ));
            continue;
        }
        fields.push(KeyValueField {
            key: key.to_owned(),
            value: value.to_owned(),
            line: raw.line,
        });
    }
    fields
}

fn field_map(fields: &[KeyValueField]) -> BTreeMap<&str, &KeyValueField> {
    fields
        .iter()
        .map(|field| (field.key.as_str(), field))
        .collect()
}

fn required_string(
    map: &BTreeMap<&str, &KeyValueField>,
    key: &str,
    section: &str,
    issues: &mut Vec<ValidationIssue>,
    allow_empty: bool,
) -> Option<Located<String>> {
    let Some(field) = map.get(key) else {
        issues.push(issue(
            ValidationCode::FieldInvalid,
            IssueSeverity::Error,
            Some(section),
            None,
            format!("Required field is missing: {key}"),
            false,
        ));
        return None;
    };
    if field.value.is_empty() && !allow_empty {
        issues.push(issue(
            ValidationCode::FieldInvalid,
            IssueSeverity::Error,
            Some(section),
            Some(field.line),
            format!("Required field is empty: {key}"),
            false,
        ));
        return None;
    }
    Some(Located {
        value: field.value.clone(),
        line: field.line,
    })
}

fn optional_string(map: &BTreeMap<&str, &KeyValueField>, key: &str) -> Option<Located<String>> {
    map.get(key).map(|field| Located {
        value: field.value.clone(),
        line: field.line,
    })
}

fn required_number(
    map: &BTreeMap<&str, &KeyValueField>,
    key: &str,
    section: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<Located<f64>> {
    let Some(field) = map.get(key) else {
        issues.push(issue(
            ValidationCode::FieldInvalid,
            IssueSeverity::Error,
            Some(section),
            None,
            format!("Required field is missing: {key}"),
            false,
        ));
        return None;
    };
    match field.value.parse::<f64>() {
        Ok(value) if value.is_finite() => Some(Located {
            value,
            line: field.line,
        }),
        _ => {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some(section),
                Some(field.line),
                format!("Field must be a finite number: {key}"),
                false,
            ));
            None
        }
    }
}

fn parse_general(section: &SectionBuffer, issues: &mut Vec<ValidationIssue>) -> GeneralSection {
    let fields = key_value_fields(section, issues);
    let map = field_map(&fields);
    let audio_filename = required_string(&map, "AudioFilename", "General", issues, false);
    let mode = match map.get("Mode") {
        None => {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some("General"),
                None,
                "Required field is missing: Mode",
                false,
            ));
            None
        }
        Some(field) => match field.value.parse::<u8>() {
            Ok(0) => Some(Located {
                value: 0,
                line: field.line,
            }),
            Ok(value) => {
                issues.push(issue(
                    ValidationCode::OsuFormatUnsupported,
                    IssueSeverity::Error,
                    Some("General"),
                    Some(field.line),
                    format!("Only osu!standard Mode 0 is supported; found Mode {value}"),
                    false,
                ));
                None
            }
            Err(_) => {
                issues.push(issue(
                    ValidationCode::FieldInvalid,
                    IssueSeverity::Error,
                    Some("General"),
                    Some(field.line),
                    "Mode must be an unsigned integer",
                    false,
                ));
                None
            }
        },
    };
    GeneralSection {
        header_line: section.header_line,
        audio_filename,
        mode,
        fields,
    }
}

fn parse_metadata(section: &SectionBuffer, issues: &mut Vec<ValidationIssue>) -> MetadataSection {
    let fields = key_value_fields(section, issues);
    let map = field_map(&fields);
    let title = required_string(&map, "Title", "Metadata", issues, false);
    let artist = required_string(&map, "Artist", "Metadata", issues, false);
    let creator = required_string(&map, "Creator", "Metadata", issues, true);
    let version = required_string(&map, "Version", "Metadata", issues, false);
    if creator.as_ref().is_some_and(|value| value.value.is_empty()) {
        issues.push(issue(
            ValidationCode::MetadataIncomplete,
            IssueSeverity::Warning,
            Some("Metadata"),
            creator.as_ref().map(|value| value.line),
            "Creator is empty",
            false,
        ));
    }
    MetadataSection {
        header_line: section.header_line,
        title,
        title_unicode: optional_string(&map, "TitleUnicode"),
        artist,
        artist_unicode: optional_string(&map, "ArtistUnicode"),
        creator,
        version,
        fields,
    }
}

fn parse_difficulty(
    section: &SectionBuffer,
    issues: &mut Vec<ValidationIssue>,
) -> DifficultySection {
    let fields = key_value_fields(section, issues);
    let map = field_map(&fields);
    DifficultySection {
        header_line: section.header_line,
        hp_drain_rate: required_number(&map, "HPDrainRate", "Difficulty", issues),
        circle_size: required_number(&map, "CircleSize", "Difficulty", issues),
        overall_difficulty: required_number(&map, "OverallDifficulty", "Difficulty", issues),
        approach_rate: required_number(&map, "ApproachRate", "Difficulty", issues),
        slider_multiplier: required_number(&map, "SliderMultiplier", "Difficulty", issues),
        slider_tick_rate: required_number(&map, "SliderTickRate", "Difficulty", issues),
        fields,
    }
}

fn parse_csv_lines<T>(
    section: &SectionBuffer,
    minimum_fields: usize,
    code: ValidationCode,
    constructor: impl Fn(u32, String, Vec<String>) -> T,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<T> {
    let mut parsed = Vec::new();
    for raw in &section.lines {
        let fields = raw
            .value
            .split(',')
            .map(|value| value.trim().to_owned())
            .collect::<Vec<_>>();
        if fields.len() < minimum_fields
            || fields
                .iter()
                .take(minimum_fields)
                .any(|field| field.is_empty())
        {
            issues.push(issue(
                code,
                IssueSeverity::Error,
                Some(&section.name),
                Some(raw.line),
                format!("Expected at least {minimum_fields} non-empty comma-separated fields"),
                false,
            ));
            continue;
        }
        parsed.push(constructor(raw.line, raw.value.clone(), fields));
    }
    if section.lines.is_empty() {
        issues.push(issue(
            ValidationCode::SectionMissing,
            IssueSeverity::Error,
            Some(&section.name),
            Some(section.header_line),
            format!("{} must contain at least one entry", section.name),
            false,
        ));
    }
    parsed
}

pub fn parse_osu(bytes: &[u8], limits: ValidationLimits) -> ParseResult {
    if bytes.len() as u64 > limits.max_beatmap_bytes {
        return fatal(
            ValidationCode::BeatmapLimitExceeded,
            format!(
                "Beatmap is {} bytes; the configured limit is {} bytes",
                bytes.len(),
                limits.max_beatmap_bytes
            ),
        );
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return fatal(
            ValidationCode::OsuInvalidEncoding,
            "Beatmap must be valid UTF-8",
        );
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| ((index + 1) as u32, line.trim_end_matches('\r')))
        .collect::<Vec<_>>();
    let (format_version, header_index) = match parse_header(&lines) {
        Ok(result) => result,
        Err(error) => {
            return ParseResult {
                beatmap: None,
                issues: vec![error],
            };
        }
    };

    let mut issues = Vec::new();
    let sections = split_sections(&lines, header_index, &mut issues);
    let mut seen = BTreeSet::new();
    let mut general = None;
    let mut editor = None;
    let mut metadata = None;
    let mut difficulty = None;
    let mut events = Vec::new();
    let mut timing_points = Vec::new();
    let mut colours = Vec::new();
    let mut hit_objects = Vec::new();
    let mut unknown_sections = Vec::new();

    for section in sections {
        let duplicate = !seen.insert(section.name.clone());
        if duplicate && KNOWN_SECTIONS.contains(&section.name.as_str()) {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                Some(&section.name),
                Some(section.header_line),
                format!("Duplicate section: {}", section.name),
                false,
            ));
            continue;
        }
        match section.name.as_str() {
            "General" => general = Some(parse_general(&section, &mut issues)),
            "Editor" => {
                editor = Some(EditorSection {
                    header_line: section.header_line,
                    fields: key_value_fields(&section, &mut issues),
                });
            }
            "Metadata" => metadata = Some(parse_metadata(&section, &mut issues)),
            "Difficulty" => difficulty = Some(parse_difficulty(&section, &mut issues)),
            "Events" => events = section.lines,
            "TimingPoints" => {
                timing_points = parse_csv_lines(
                    &section,
                    8,
                    ValidationCode::TimingInvalid,
                    |line, raw, fields| RawTimingPoint { line, raw, fields },
                    &mut issues,
                );
            }
            "Colours" => colours = section.lines,
            "HitObjects" => {
                hit_objects = parse_csv_lines(
                    &section,
                    5,
                    ValidationCode::HitObjectInvalid,
                    |line, raw, fields| RawHitObject { line, raw, fields },
                    &mut issues,
                );
            }
            _ => unknown_sections.push(UnknownSection {
                name: section.name,
                header_line: section.header_line,
                lines: section.lines,
            }),
        }
    }

    for section in REQUIRED_SECTIONS {
        if !seen.contains(section) {
            issues.push(issue(
                ValidationCode::SectionMissing,
                IssueSeverity::Error,
                Some(section),
                None,
                format!("Required section is missing: {section}"),
                false,
            ));
        }
    }

    ParseResult {
        beatmap: Some(ParsedBeatmap {
            format_version,
            general,
            editor,
            metadata,
            difficulty,
            events,
            timing_points,
            colours,
            hit_objects,
            unknown_sections,
        }),
        issues,
    }
}

pub fn parse_osu_file(path: &Path, limits: ValidationLimits) -> ParseResult {
    let Ok(metadata) = std::fs::metadata(path) else {
        return fatal(
            ValidationCode::OutputNotFound,
            "The beatmap file does not exist or cannot be inspected",
        );
    };
    if !metadata.is_file() {
        return fatal(
            ValidationCode::OutputNotFound,
            "The beatmap path does not point to a file",
        );
    }
    if metadata.len() > limits.max_beatmap_bytes {
        return fatal(
            ValidationCode::BeatmapLimitExceeded,
            format!(
                "Beatmap is {} bytes; the configured limit is {} bytes",
                metadata.len(),
                limits.max_beatmap_bytes
            ),
        );
    }
    match std::fs::read(path) {
        Ok(bytes) => parse_osu(&bytes, limits),
        Err(_) => fatal(
            ValidationCode::OutputNotFound,
            "The beatmap file could not be read",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn fixture(path: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/validation")
            .join(path);
        fs::read(path).unwrap()
    }

    fn codes(result: &ParseResult) -> Vec<ValidationCode> {
        result.issues.iter().map(|issue| issue.code).collect()
    }

    #[test]
    fn parses_the_minimal_standard_fixture() {
        let result = parse_osu(
            &fixture("valid/minimal-standard.osu"),
            ValidationLimits::default(),
        );
        assert!(!result.has_errors(), "{:?}", result.issues);
        let beatmap = result.beatmap.unwrap();
        assert_eq!(beatmap.format_version, 14);
        assert_eq!(beatmap.general.unwrap().mode.unwrap().value, 0);
        assert_eq!(
            beatmap.metadata.unwrap().title.unwrap().value,
            "Validation Fixture"
        );
        assert_eq!(beatmap.timing_points.len(), 1);
        assert_eq!(beatmap.hit_objects.len(), 3);
    }

    #[test]
    fn preserves_unknown_sections_and_fields() {
        let source = b"osu file format v14\n[General]\nAudioFilename:a.wav\nMode:0\nFutureKey:value\n[Metadata]\nTitle:T\nArtist:A\nCreator:C\nVersion:V\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,1,70,1,0\n[HitObjects]\n256,192,1000,1,0,\n[FutureSection]\nFutureValue\n[FutureSection]\nFutureValue2\n";
        let result = parse_osu(source, ValidationLimits::default());
        assert!(!result.has_errors(), "{:?}", result.issues);
        let beatmap = result.beatmap.unwrap();
        assert!(
            beatmap
                .general
                .unwrap()
                .fields
                .iter()
                .any(|field| field.key == "FutureKey")
        );
        assert_eq!(beatmap.unknown_sections.len(), 2);
        assert!(
            beatmap
                .unknown_sections
                .iter()
                .all(|section| section.name == "FutureSection")
        );
    }

    #[test]
    fn accepts_bom_and_crlf() {
        let source = fixture("valid/minimal-standard.osu");
        let text = std::str::from_utf8(&source).unwrap().replace('\n', "\r\n");
        let with_bom = format!("\u{feff}{text}");
        let result = parse_osu(with_bom.as_bytes(), ValidationLimits::default());
        assert!(!result.has_errors(), "{:?}", result.issues);
    }

    #[test]
    fn reports_invalid_encoding_and_size_as_fatal() {
        let invalid = parse_osu(&[0xff, 0xfe], ValidationLimits::default());
        assert_eq!(codes(&invalid), vec![ValidationCode::OsuInvalidEncoding]);
        assert!(invalid.beatmap.is_none());

        let limits = ValidationLimits {
            max_beatmap_bytes: 4,
            ..ValidationLimits::default()
        };
        let oversized = parse_osu(b"12345", limits);
        assert_eq!(
            codes(&oversized),
            vec![ValidationCode::BeatmapLimitExceeded]
        );
        assert!(oversized.beatmap.is_none());
    }

    #[test]
    fn file_entrypoint_checks_metadata_before_reading() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/validation/valid/minimal-standard.osu");
        let parsed = parse_osu_file(&path, ValidationLimits::default());
        assert!(!parsed.has_errors());

        let missing = parse_osu_file(
            &path.with_file_name("does-not-exist.osu"),
            ValidationLimits::default(),
        );
        assert_eq!(codes(&missing), vec![ValidationCode::OutputNotFound]);
    }

    #[test]
    fn reports_header_section_and_duplicate_field_errors_with_context() {
        let missing_header = parse_osu(b"[General]\nMode:0", ValidationLimits::default());
        assert_eq!(
            codes(&missing_header),
            vec![ValidationCode::OsuFormatUnsupported]
        );

        let duplicate = b"osu file format v14\n[General]\nAudioFilename:a.wav\nMode:0\nmode:0\n[General]\nMode:0\n";
        let result = parse_osu(duplicate, ValidationLimits::default());
        assert!(result.has_errors());
        assert!(result.issues.iter().any(|issue| {
            issue.code == ValidationCode::FieldInvalid
                && issue.section.as_deref() == Some("General")
                && issue.line.is_some()
        }));
        assert!(codes(&result).contains(&ValidationCode::SectionMissing));
    }

    #[test]
    fn maps_contract_fixtures_to_stable_structural_codes() {
        let missing = parse_osu(
            &fixture("invalid/missing-hitobjects.osu"),
            ValidationLimits::default(),
        );
        assert!(codes(&missing).contains(&ValidationCode::SectionMissing));

        let timing = parse_osu(
            &fixture("invalid/invalid-timing.osu"),
            ValidationLimits::default(),
        );
        assert!(timing.beatmap.is_some());
        assert!(!codes(&timing).contains(&ValidationCode::TimingInvalid));

        let slider = parse_osu(
            &fixture("invalid/invalid-slider.osu"),
            ValidationLimits::default(),
        );
        assert!(slider.beatmap.is_some());
        assert!(!codes(&slider).contains(&ValidationCode::HitObjectInvalid));
    }

    #[test]
    fn incomplete_metadata_is_a_non_fatal_warning() {
        let result = parse_osu(
            &fixture("warnings/incomplete-metadata.osu"),
            ValidationLimits::default(),
        );
        assert!(!result.has_errors());
        assert_eq!(codes(&result), vec![ValidationCode::MetadataIncomplete]);
        assert_eq!(result.issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn malformed_inputs_never_panic_and_are_deterministic() {
        let cases: &[&[u8]] = &[
            b"",
            b"osu file format v14\nnot-in-section",
            b"osu file format v14\n[General",
            b"osu file format v14\n[]",
            b"osu file format v14\n[General]\n:no-key",
            b"osu file format v14\n[General]\nno-colon",
        ];
        for source in cases {
            let first = parse_osu(source, ValidationLimits::default());
            let second = parse_osu(source, ValidationLimits::default());
            assert_eq!(first, second);
        }
    }
}
