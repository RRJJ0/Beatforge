use super::model::{DifficultySection, ParsedBeatmap, RawHitObject, RawTimingPoint};
use crate::validation::{IssueSeverity, ValidationCode, ValidationIssue};
use serde::{Deserialize, Serialize};

const PLAYFIELD_MAX_X: i32 = 512;
const PLAYFIELD_MAX_Y: i32 = 384;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SemanticValidationContext {
    pub audio_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TimingPointKind {
    Uninherited { bpm: f64 },
    Inherited { slider_velocity_multiplier: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedTimingPoint {
    pub line: u32,
    pub time_ms: f64,
    pub meter: u32,
    pub sample_set: u8,
    pub sample_index: i32,
    pub volume: u8,
    pub effects: u32,
    pub point: TimingPointKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliderControlPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SemanticHitObject {
    Circle {
        line: u32,
        x: i32,
        y: i32,
        time_ms: u64,
    },
    Slider {
        line: u32,
        x: i32,
        y: i32,
        time_ms: u64,
        curve_type: String,
        control_points: Vec<SliderControlPoint>,
        repeats: u32,
        pixel_length: f64,
    },
    Spinner {
        line: u32,
        x: i32,
        y: i32,
        time_ms: u64,
        end_time_ms: u64,
    },
}

impl SemanticHitObject {
    pub fn line(&self) -> u32 {
        match self {
            Self::Circle { line, .. } | Self::Slider { line, .. } | Self::Spinner { line, .. } => {
                *line
            }
        }
    }

    pub fn start_time_ms(&self) -> u64 {
        match self {
            Self::Circle { time_ms, .. }
            | Self::Slider { time_ms, .. }
            | Self::Spinner { time_ms, .. } => *time_ms,
        }
    }

    pub fn end_time_ms(&self) -> u64 {
        match self {
            Self::Spinner { end_time_ms, .. } => *end_time_ms,
            _ => self.start_time_ms(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticBeatmap {
    pub timing_points: Vec<ValidatedTimingPoint>,
    pub hit_objects: Vec<SemanticHitObject>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticResult {
    pub beatmap: SemanticBeatmap,
    pub issues: Vec<ValidationIssue>,
}

impl SemanticResult {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }
}

fn issue(
    code: ValidationCode,
    severity: IssueSeverity,
    section: &str,
    line: Option<u32>,
    message: impl Into<String>,
) -> ValidationIssue {
    ValidationIssue {
        code,
        severity,
        section: Some(section.into()),
        line,
        message: message.into(),
        repairable: false,
    }
}

fn parse_value<T: std::str::FromStr>(
    value: Option<&str>,
    label: &str,
    code: ValidationCode,
    section: &str,
    line: u32,
    issues: &mut Vec<ValidationIssue>,
) -> Option<T> {
    match value.and_then(|value| value.parse::<T>().ok()) {
        Some(value) => Some(value),
        None => {
            issues.push(issue(
                code,
                IssueSeverity::Error,
                section,
                Some(line),
                format!("{label} has an invalid numeric value"),
            ));
            None
        }
    }
}

fn validate_difficulty(difficulty: Option<&DifficultySection>, issues: &mut Vec<ValidationIssue>) {
    let Some(difficulty) = difficulty else {
        return;
    };
    for (name, value) in [
        ("HPDrainRate", difficulty.hp_drain_rate.as_ref()),
        ("CircleSize", difficulty.circle_size.as_ref()),
        ("OverallDifficulty", difficulty.overall_difficulty.as_ref()),
        ("ApproachRate", difficulty.approach_rate.as_ref()),
    ] {
        if let Some(value) = value
            && !(0.0..=10.0).contains(&value.value)
        {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                "Difficulty",
                Some(value.line),
                format!("{name} must be between 0 and 10"),
            ));
        }
    }
    for (name, value) in [
        ("SliderMultiplier", difficulty.slider_multiplier.as_ref()),
        ("SliderTickRate", difficulty.slider_tick_rate.as_ref()),
    ] {
        if let Some(value) = value
            && value.value <= 0.0
        {
            issues.push(issue(
                ValidationCode::FieldInvalid,
                IssueSeverity::Error,
                "Difficulty",
                Some(value.line),
                format!("{name} must be greater than zero"),
            ));
        }
    }
}

fn parse_timing_point(
    raw: &RawTimingPoint,
    issues: &mut Vec<ValidationIssue>,
) -> Option<ValidatedTimingPoint> {
    let code = ValidationCode::TimingInvalid;
    let section = "TimingPoints";
    let time_ms = parse_value::<f64>(
        raw.fields.first().map(String::as_str),
        "Time",
        code,
        section,
        raw.line,
        issues,
    )?;
    let beat_length = parse_value::<f64>(
        raw.fields.get(1).map(String::as_str),
        "Beat length",
        code,
        section,
        raw.line,
        issues,
    )?;
    let meter = parse_value::<u32>(
        raw.fields.get(2).map(String::as_str),
        "Meter",
        code,
        section,
        raw.line,
        issues,
    )?;
    let sample_set = parse_value::<u8>(
        raw.fields.get(3).map(String::as_str),
        "Sample set",
        code,
        section,
        raw.line,
        issues,
    )?;
    let sample_index = parse_value::<i32>(
        raw.fields.get(4).map(String::as_str),
        "Sample index",
        code,
        section,
        raw.line,
        issues,
    )?;
    let volume = parse_value::<u8>(
        raw.fields.get(5).map(String::as_str),
        "Volume",
        code,
        section,
        raw.line,
        issues,
    )?;
    let uninherited = parse_value::<u8>(
        raw.fields.get(6).map(String::as_str),
        "Uninherited flag",
        code,
        section,
        raw.line,
        issues,
    )?;
    let effects = parse_value::<u32>(
        raw.fields.get(7).map(String::as_str),
        "Effects",
        code,
        section,
        raw.line,
        issues,
    )?;

    if !time_ms.is_finite() || !beat_length.is_finite() {
        issues.push(issue(
            code,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            "Timing values must be finite",
        ));
        return None;
    }
    if meter == 0 || meter > 32 {
        issues.push(issue(
            code,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            "Meter must be between 1 and 32",
        ));
        return None;
    }
    if sample_set > 3 {
        issues.push(issue(
            code,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            "Sample set must be between 0 and 3",
        ));
        return None;
    }
    if volume > 100 {
        issues.push(issue(
            code,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            "Volume must be between 0 and 100",
        ));
        return None;
    }
    let point = match uninherited {
        1 if beat_length > 0.0 => {
            let bpm = 60_000.0 / beat_length;
            if !bpm.is_finite() {
                issues.push(issue(
                    code,
                    IssueSeverity::Error,
                    section,
                    Some(raw.line),
                    "Beat length produces a non-finite BPM",
                ));
                return None;
            }
            TimingPointKind::Uninherited { bpm }
        }
        0 if beat_length < 0.0 => {
            let slider_velocity_multiplier = -100.0 / beat_length;
            if !slider_velocity_multiplier.is_finite() {
                issues.push(issue(
                    code,
                    IssueSeverity::Error,
                    section,
                    Some(raw.line),
                    "Inherited beat length produces a non-finite slider velocity",
                ));
                return None;
            }
            TimingPointKind::Inherited {
                slider_velocity_multiplier,
            }
        }
        0 | 1 => {
            issues.push(issue(
                code,
                IssueSeverity::Error,
                section,
                Some(raw.line),
                "Uninherited points require a positive beat length and inherited points require a negative beat length",
            ));
            return None;
        }
        _ => {
            issues.push(issue(
                code,
                IssueSeverity::Error,
                section,
                Some(raw.line),
                "Uninherited flag must be 0 or 1",
            ));
            return None;
        }
    };
    Some(ValidatedTimingPoint {
        line: raw.line,
        time_ms,
        meter,
        sample_set,
        sample_index,
        volume,
        effects,
        point,
    })
}

fn parse_origin(
    raw: &RawHitObject,
    issues: &mut Vec<ValidationIssue>,
) -> Option<(i32, i32, u64, u32)> {
    let code = ValidationCode::HitObjectInvalid;
    let section = "HitObjects";
    let x = parse_value::<i32>(
        raw.fields.first().map(String::as_str),
        "X coordinate",
        code,
        section,
        raw.line,
        issues,
    )?;
    let y = parse_value::<i32>(
        raw.fields.get(1).map(String::as_str),
        "Y coordinate",
        code,
        section,
        raw.line,
        issues,
    )?;
    let time_ms = parse_value::<u64>(
        raw.fields.get(2).map(String::as_str),
        "Object time",
        code,
        section,
        raw.line,
        issues,
    )?;
    let object_type = parse_value::<u32>(
        raw.fields.get(3).map(String::as_str),
        "Object type",
        code,
        section,
        raw.line,
        issues,
    )?;
    let hit_sound = parse_value::<u32>(
        raw.fields.get(4).map(String::as_str),
        "Hit sound",
        code,
        section,
        raw.line,
        issues,
    )?;

    if hit_sound & !15 != 0 {
        issues.push(issue(
            code,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            format!("Hit sound contains unsupported flags: {hit_sound}"),
        ));
        return None;
    }
    if !(0..=PLAYFIELD_MAX_X).contains(&x) || !(0..=PLAYFIELD_MAX_Y).contains(&y) {
        issues.push(issue(
            ValidationCode::ObjectOutOfBounds,
            IssueSeverity::Error,
            section,
            Some(raw.line),
            format!(
                "Object origin ({x}, {y}) is outside the osu!standard playfield 0..={PLAYFIELD_MAX_X} x 0..={PLAYFIELD_MAX_Y}"
            ),
        ));
        return None;
    }
    Some((x, y, time_ms, object_type))
}

fn parse_control_point(
    value: &str,
    line: u32,
    issues: &mut Vec<ValidationIssue>,
) -> Option<SliderControlPoint> {
    let Some((x, y)) = value.split_once(':') else {
        issues.push(issue(
            ValidationCode::HitObjectInvalid,
            IssueSeverity::Error,
            "HitObjects",
            Some(line),
            "Slider control point must use x:y syntax",
        ));
        return None;
    };
    let x = parse_value::<i32>(
        Some(x),
        "Slider control point X",
        ValidationCode::HitObjectInvalid,
        "HitObjects",
        line,
        issues,
    )?;
    let y = parse_value::<i32>(
        Some(y),
        "Slider control point Y",
        ValidationCode::HitObjectInvalid,
        "HitObjects",
        line,
        issues,
    )?;
    if !(0..=PLAYFIELD_MAX_X).contains(&x) || !(0..=PLAYFIELD_MAX_Y).contains(&y) {
        issues.push(issue(
            ValidationCode::ObjectOutOfBounds,
            IssueSeverity::Warning,
            "HitObjects",
            Some(line),
            format!("Slider control point ({x}, {y}) is outside the visible playfield"),
        ));
    }
    Some(SliderControlPoint { x, y })
}

fn parse_hit_object(
    raw: &RawHitObject,
    issues: &mut Vec<ValidationIssue>,
) -> Option<SemanticHitObject> {
    let (x, y, time_ms, object_type) = parse_origin(raw, issues)?;
    const BASE_MASK: u32 = 1 | 2 | 8 | 128;
    const ALLOWED_MASK: u32 = 1 | 2 | 4 | 8 | 16 | 32 | 64;
    let base = object_type & BASE_MASK;
    if base.count_ones() != 1 || base == 128 || object_type & !ALLOWED_MASK != 0 {
        issues.push(issue(
            ValidationCode::HitObjectInvalid,
            IssueSeverity::Error,
            "HitObjects",
            Some(raw.line),
            format!("Unsupported or ambiguous osu!standard object type: {object_type}"),
        ));
        return None;
    }
    match base {
        1 => Some(SemanticHitObject::Circle {
            line: raw.line,
            x,
            y,
            time_ms,
        }),
        2 => {
            if raw.fields.len() < 8 {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(raw.line),
                    "Slider requires curve, repeat count, and pixel length fields",
                ));
                return None;
            }
            let curve = &raw.fields[5];
            let mut parts = curve.split('|');
            let curve_type = parts.next().unwrap_or_default();
            if !matches!(curve_type, "B" | "C" | "L" | "P") {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(raw.line),
                    format!("Unsupported slider curve type: {curve_type}"),
                ));
                return None;
            }
            let mut control_points = Vec::new();
            for point in parts {
                if let Some(point) = parse_control_point(point, raw.line, issues) {
                    control_points.push(point);
                }
            }
            if control_points.is_empty() {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(raw.line),
                    "Slider requires at least one control point",
                ));
                return None;
            }
            let repeats = parse_value::<u32>(
                raw.fields.get(6).map(String::as_str),
                "Slider repeat count",
                ValidationCode::HitObjectInvalid,
                "HitObjects",
                raw.line,
                issues,
            )?;
            let pixel_length = parse_value::<f64>(
                raw.fields.get(7).map(String::as_str),
                "Slider pixel length",
                ValidationCode::HitObjectInvalid,
                "HitObjects",
                raw.line,
                issues,
            )?;
            if repeats == 0 || !pixel_length.is_finite() || pixel_length <= 0.0 {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(raw.line),
                    "Slider repeat count and finite pixel length must be greater than zero",
                ));
                return None;
            }
            Some(SemanticHitObject::Slider {
                line: raw.line,
                x,
                y,
                time_ms,
                curve_type: curve_type.into(),
                control_points,
                repeats,
                pixel_length,
            })
        }
        8 => {
            let end_time_ms = parse_value::<u64>(
                raw.fields.get(5).map(String::as_str),
                "Spinner end time",
                ValidationCode::HitObjectInvalid,
                "HitObjects",
                raw.line,
                issues,
            )?;
            if end_time_ms <= time_ms {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(raw.line),
                    "Spinner end time must be after its start time",
                ));
                return None;
            }
            Some(SemanticHitObject::Spinner {
                line: raw.line,
                x,
                y,
                time_ms,
                end_time_ms,
            })
        }
        _ => None,
    }
}

pub fn validate_semantics(
    beatmap: &ParsedBeatmap,
    context: SemanticValidationContext,
) -> SemanticResult {
    let mut issues = Vec::new();
    validate_difficulty(beatmap.difficulty.as_ref(), &mut issues);

    let mut timing_points = Vec::new();
    let mut previous_timing_time = f64::NEG_INFINITY;
    let mut has_uninherited = false;
    for raw in &beatmap.timing_points {
        if let Some(point) = parse_timing_point(raw, &mut issues) {
            if point.time_ms < previous_timing_time {
                issues.push(issue(
                    ValidationCode::TimingInvalid,
                    IssueSeverity::Error,
                    "TimingPoints",
                    Some(point.line),
                    "Timing points must be ordered by time",
                ));
            }
            previous_timing_time = previous_timing_time.max(point.time_ms);
            has_uninherited |= matches!(point.point, TimingPointKind::Uninherited { .. });
            timing_points.push(point);
        }
    }
    if !has_uninherited {
        issues.push(issue(
            ValidationCode::TimingInvalid,
            IssueSeverity::Error,
            "TimingPoints",
            None,
            "At least one uninherited timing point is required",
        ));
    }

    let mut hit_objects = Vec::new();
    let mut previous_object_time = 0_u64;
    let mut has_object_time = false;
    for raw in &beatmap.hit_objects {
        if let Some(object) = parse_hit_object(raw, &mut issues) {
            let start_time = object.start_time_ms();
            if has_object_time && start_time < previous_object_time {
                issues.push(issue(
                    ValidationCode::HitObjectInvalid,
                    IssueSeverity::Error,
                    "HitObjects",
                    Some(object.line()),
                    "Hit objects must be ordered by start time",
                ));
            }
            previous_object_time = previous_object_time.max(start_time);
            has_object_time = true;
            if context
                .audio_duration_ms
                .is_some_and(|duration| object.end_time_ms() > duration)
            {
                issues.push(issue(
                    ValidationCode::ObjectAfterAudioEnd,
                    IssueSeverity::Warning,
                    "HitObjects",
                    Some(object.line()),
                    "Hit object extends beyond the known audio duration",
                ));
            }
            hit_objects.push(object);
        }
    }

    SemanticResult {
        beatmap: SemanticBeatmap {
            timing_points,
            hit_objects,
        },
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{ValidationLimits, osu::parse_osu};
    use std::{fs, path::PathBuf};

    fn parsed_fixture(path: &str) -> ParsedBeatmap {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/validation")
            .join(path);
        let bytes = fs::read(path).unwrap();
        let result = parse_osu(&bytes, ValidationLimits::default());
        assert!(result.beatmap.is_some());
        result.beatmap.unwrap()
    }

    fn codes(result: &SemanticResult) -> Vec<ValidationCode> {
        result.issues.iter().map(|issue| issue.code).collect()
    }

    #[test]
    fn validates_the_standard_fixture() {
        let result = validate_semantics(
            &parsed_fixture("valid/minimal-standard.osu"),
            SemanticValidationContext::default(),
        );
        assert!(!result.has_errors(), "{:?}", result.issues);
        assert_eq!(result.beatmap.timing_points.len(), 1);
        assert_eq!(result.beatmap.hit_objects.len(), 3);
        assert!(matches!(
            result.beatmap.timing_points[0].point,
            TimingPointKind::Uninherited { bpm } if bpm == 120.0
        ));
    }

    #[test]
    fn rejects_zero_timing_and_requires_an_uninherited_point() {
        let result = validate_semantics(
            &parsed_fixture("invalid/invalid-timing.osu"),
            SemanticValidationContext::default(),
        );
        assert!(result.has_errors());
        assert_eq!(
            codes(&result),
            vec![ValidationCode::TimingInvalid, ValidationCode::TimingInvalid]
        );
    }

    #[test]
    fn rejects_invalid_slider_repeat_and_length() {
        let result = validate_semantics(
            &parsed_fixture("invalid/invalid-slider.osu"),
            SemanticValidationContext::default(),
        );
        assert!(result.has_errors());
        assert!(codes(&result).contains(&ValidationCode::HitObjectInvalid));
        assert!(result.beatmap.hit_objects.is_empty());
    }

    #[test]
    fn rejects_an_out_of_bounds_object_origin() {
        let result = validate_semantics(
            &parsed_fixture("invalid/out-of-bounds.osu"),
            SemanticValidationContext::default(),
        );
        assert!(result.has_errors());
        assert_eq!(codes(&result), vec![ValidationCode::ObjectOutOfBounds]);
    }

    #[test]
    fn validates_inherited_timing_and_warns_after_audio_end() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        beatmap.timing_points.push(RawTimingPoint {
            line: 100,
            raw: "500,-50,4,2,1,70,0,0".into(),
            fields: ["500", "-50", "4", "2", "1", "70", "0", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        });
        let result = validate_semantics(
            &beatmap,
            SemanticValidationContext {
                audio_duration_ms: Some(2_000),
            },
        );
        assert!(!result.has_errors(), "{:?}", result.issues);
        assert!(codes(&result).contains(&ValidationCode::ObjectAfterAudioEnd));
        assert!(matches!(
            result.beatmap.timing_points[1].point,
            TimingPointKind::Inherited {
                slider_velocity_multiplier
            } if slider_velocity_multiplier == 2.0
        ));
    }

    #[test]
    fn accepts_negative_legacy_sample_indices() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        beatmap.timing_points[0] = RawTimingPoint {
            line: 100,
            raw: "0,500,4,2,-1,70,1,0".into(),
            fields: ["0", "500", "4", "2", "-1", "70", "1", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };

        let result = validate_semantics(&beatmap, SemanticValidationContext::default());

        assert!(!result.has_errors(), "{:?}", result.issues);
        assert_eq!(result.beatmap.timing_points[0].sample_index, -1);
    }

    #[test]
    fn rejects_unordered_timing_and_objects() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        beatmap.timing_points.push(RawTimingPoint {
            line: 100,
            raw: "-100,500,4,2,1,70,1,0".into(),
            fields: ["-100", "500", "4", "2", "1", "70", "1", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        });
        beatmap.hit_objects.swap(0, 2);
        let result = validate_semantics(&beatmap, SemanticValidationContext::default());
        assert!(result.has_errors());
        assert!(codes(&result).contains(&ValidationCode::TimingInvalid));
        assert!(codes(&result).contains(&ValidationCode::HitObjectInvalid));
    }

    #[test]
    fn rejects_spinner_end_and_unsupported_object_types() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        beatmap.hit_objects = vec![
            RawHitObject {
                line: 100,
                raw: "256,192,1000,8,0,900".into(),
                fields: ["256", "192", "1000", "8", "0", "900"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
            RawHitObject {
                line: 101,
                raw: "256,192,1100,128,0,1500:0:0:0:0:".into(),
                fields: ["256", "192", "1100", "128", "0", "1500:0:0:0:0:"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
        ];
        let result = validate_semantics(&beatmap, SemanticValidationContext::default());
        assert_eq!(
            codes(&result),
            vec![
                ValidationCode::HitObjectInvalid,
                ValidationCode::HitObjectInvalid
            ]
        );
    }

    #[test]
    fn rejects_difficulty_ranges_and_hit_sound_flags() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        let difficulty = beatmap.difficulty.as_mut().unwrap();
        difficulty.hp_drain_rate.as_mut().unwrap().value = 11.0;
        difficulty.slider_tick_rate.as_mut().unwrap().value = 0.0;
        beatmap.hit_objects = vec![RawHitObject {
            line: 100,
            raw: "256,192,1000,1,16".into(),
            fields: ["256", "192", "1000", "1", "16"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }];
        let result = validate_semantics(&beatmap, SemanticValidationContext::default());
        assert!(result.has_errors());
        assert_eq!(
            codes(&result),
            vec![
                ValidationCode::FieldInvalid,
                ValidationCode::FieldInvalid,
                ValidationCode::HitObjectInvalid,
            ]
        );
    }

    #[test]
    fn slider_control_points_are_parsed_and_outside_points_warn() {
        let mut beatmap = parsed_fixture("valid/minimal-standard.osu");
        beatmap.hit_objects = vec![RawHitObject {
            line: 100,
            raw: "128,192,1500,2,0,B|256:96|700:400,1,200".into(),
            fields: [
                "128",
                "192",
                "1500",
                "2",
                "0",
                "B|256:96|700:400",
                "1",
                "200",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }];
        let result = validate_semantics(&beatmap, SemanticValidationContext::default());
        assert!(!result.has_errors());
        assert_eq!(codes(&result), vec![ValidationCode::ObjectOutOfBounds]);
        assert_eq!(result.beatmap.hit_objects.len(), 1);
    }
}
