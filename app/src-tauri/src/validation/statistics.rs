use crate::validation::{
    BeatmapStatistics, ValidationLimits,
    osu::{
        ParsedBeatmap, SemanticBeatmap, SemanticHitObject, SemanticValidationContext,
        TimingPointKind, parse_osu_file, validate_semantics,
    },
};
use std::path::Path;

fn slider_end_time(
    start_time_ms: u64,
    repeats: u32,
    pixel_length: f64,
    slider_multiplier: Option<f64>,
    timing_points: &[crate::validation::osu::ValidatedTimingPoint],
) -> u64 {
    let Some(slider_multiplier) = slider_multiplier.filter(|value| *value > 0.0) else {
        return start_time_ms;
    };
    let mut beat_length = None;
    let mut slider_velocity = 1.0;
    for point in timing_points {
        if point.time_ms > start_time_ms as f64 {
            break;
        }
        match point.point {
            TimingPointKind::Uninherited { bpm } => {
                beat_length = Some(60_000.0 / bpm);
                slider_velocity = 1.0;
            }
            TimingPointKind::Inherited {
                slider_velocity_multiplier,
            } => slider_velocity = slider_velocity_multiplier,
        }
    }
    let Some(beat_length) = beat_length else {
        return start_time_ms;
    };
    let duration = pixel_length * f64::from(repeats) * beat_length
        / (slider_multiplier * 100.0 * slider_velocity);
    if !duration.is_finite() || duration <= 0.0 {
        return start_time_ms;
    }
    start_time_ms.saturating_add(duration.ceil().min(u64::MAX as f64) as u64)
}

pub fn calculate_beatmap_statistics(
    parsed: &ParsedBeatmap,
    semantic: &SemanticBeatmap,
) -> BeatmapStatistics {
    let mut circle_count = 0_u32;
    let mut slider_count = 0_u32;
    let mut spinner_count = 0_u32;
    let mut first_start = None::<u64>;
    let mut last_end = None::<u64>;
    let slider_multiplier = parsed
        .difficulty
        .as_ref()
        .and_then(|difficulty| difficulty.slider_multiplier.as_ref())
        .map(|value| value.value);
    for object in &semantic.hit_objects {
        let start = object.start_time_ms();
        let end = match object {
            SemanticHitObject::Circle { .. } => {
                circle_count = circle_count.saturating_add(1);
                start
            }
            SemanticHitObject::Slider {
                repeats,
                pixel_length,
                ..
            } => {
                slider_count = slider_count.saturating_add(1);
                slider_end_time(
                    start,
                    *repeats,
                    *pixel_length,
                    slider_multiplier,
                    &semantic.timing_points,
                )
            }
            SemanticHitObject::Spinner { end_time_ms, .. } => {
                spinner_count = spinner_count.saturating_add(1);
                *end_time_ms
            }
        };
        first_start = Some(first_start.map_or(start, |current| current.min(start)));
        last_end = Some(last_end.map_or(end, |current| current.max(end)));
    }
    let mut bpm_min = None::<f64>;
    let mut bpm_max = None::<f64>;
    for point in &semantic.timing_points {
        if let TimingPointKind::Uninherited { bpm } = point.point {
            bpm_min = Some(bpm_min.map_or(bpm, |current| current.min(bpm)));
            bpm_max = Some(bpm_max.map_or(bpm, |current| current.max(bpm)));
        }
    }
    BeatmapStatistics {
        beatmap_count: 1,
        circle_count,
        slider_count,
        spinner_count,
        max_combo: None,
        star_rating: None,
        bpm_min,
        bpm_max,
        duration_ms: first_start
            .zip(last_end)
            .map(|(start, end)| end.saturating_sub(start)),
    }
}

pub fn calculate_file_statistics(
    path: &Path,
    limits: ValidationLimits,
) -> Option<BeatmapStatistics> {
    let parsed = parse_osu_file(path, limits);
    if parsed.has_errors() {
        return None;
    }
    let beatmap = parsed.beatmap?;
    let semantic = validate_semantics(&beatmap, SemanticValidationContext::default());
    if semantic.has_errors() {
        return None;
    }
    Some(calculate_beatmap_statistics(&beatmap, &semantic.beatmap))
}

pub fn aggregate_statistics(
    statistics: impl IntoIterator<Item = BeatmapStatistics>,
) -> Option<BeatmapStatistics> {
    let mut values = statistics.into_iter();
    let first = values.next()?;
    Some(values.fold(first, |mut total, item| {
        total.beatmap_count = total.beatmap_count.saturating_add(item.beatmap_count);
        total.circle_count = total.circle_count.saturating_add(item.circle_count);
        total.slider_count = total.slider_count.saturating_add(item.slider_count);
        total.spinner_count = total.spinner_count.saturating_add(item.spinner_count);
        total.bpm_min = match (total.bpm_min, item.bpm_min) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
        total.bpm_max = match (total.bpm_max, item.bpm_max) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (left, right) => left.or(right),
        };
        total.duration_ms = match (total.duration_ms, item.duration_ms) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (left, right) => left.or(right),
        };
        total.max_combo = match (total.max_combo, item.max_combo) {
            (Some(left), Some(right)) => Some(left.max(right)),
            _ => None,
        };
        total.star_rating = match (total.star_rating, item.star_rating) {
            (Some(left), Some(right)) => Some(left.max(right)),
            _ => None,
        };
        total
    }))
}

pub fn calculate_archive_statistics(
    root: &Path,
    beatmap_paths: &[String],
    limits: ValidationLimits,
) -> Option<BeatmapStatistics> {
    let statistics = beatmap_paths
        .iter()
        .map(|relative| calculate_file_statistics(&root.join(relative), limits))
        .collect::<Option<Vec<_>>>()?;
    aggregate_statistics(statistics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::osu::parse_osu;
    use std::fs;

    fn fixture() -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/validation/valid/minimal-standard.osu"),
        )
        .unwrap()
    }

    #[test]
    fn calculates_counts_bpm_and_playable_duration() {
        let parsed = parse_osu(&fixture(), ValidationLimits::default())
            .beatmap
            .unwrap();
        let semantic = validate_semantics(&parsed, SemanticValidationContext::default());
        let stats = calculate_beatmap_statistics(&parsed, &semantic.beatmap);
        assert_eq!(stats.beatmap_count, 1);
        assert_eq!(stats.circle_count, 1);
        assert_eq!(stats.slider_count, 1);
        assert_eq!(stats.spinner_count, 1);
        assert_eq!(stats.bpm_min, Some(120.0));
        assert_eq!(stats.bpm_max, Some(120.0));
        assert_eq!(stats.duration_ms, Some(2000));
        assert!(stats.max_combo.is_none());
        assert!(stats.star_rating.is_none());
    }

    #[test]
    fn aggregate_uses_totals_ranges_and_longest_difficulty() {
        let first = BeatmapStatistics {
            beatmap_count: 1,
            circle_count: 10,
            slider_count: 4,
            spinner_count: 1,
            max_combo: None,
            star_rating: None,
            bpm_min: Some(120.0),
            bpm_max: Some(180.0),
            duration_ms: Some(90_000),
        };
        let mut second = first.clone();
        second.circle_count = 20;
        second.bpm_min = Some(90.0);
        second.bpm_max = Some(160.0);
        second.duration_ms = Some(120_000);
        let total = aggregate_statistics([first, second]).unwrap();
        assert_eq!(total.beatmap_count, 2);
        assert_eq!(total.circle_count, 30);
        assert_eq!(total.slider_count, 8);
        assert_eq!(total.spinner_count, 2);
        assert_eq!(total.bpm_min, Some(90.0));
        assert_eq!(total.bpm_max, Some(180.0));
        assert_eq!(total.duration_ms, Some(120_000));
    }

    #[test]
    fn slider_duration_uses_active_beat_length_sv_and_repeats() {
        let source = String::from_utf8(fixture())
            .unwrap()
            .replace(
                "0,500,4,2,1,70,1,0",
                "0,500,4,2,1,70,1,0\n1000,-50,4,2,1,70,0,0",
            )
            .replace("256,192,2500,8,0,3000\n", "");
        let parsed = parse_osu(source.as_bytes(), ValidationLimits::default())
            .beatmap
            .unwrap();
        let semantic = validate_semantics(&parsed, SemanticValidationContext::default());
        assert!(!semantic.has_errors(), "{:?}", semantic.issues);

        let stats = calculate_beatmap_statistics(&parsed, &semantic.beatmap);

        assert_eq!(stats.duration_ms, Some(858));
    }
}
