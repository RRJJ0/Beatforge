use crate::validation::{
    BeatmapTimeAnalysis, IssueSeverity, TimeRange, ValidationCode, ValidationIssue,
    ValidationLimits,
    osu::{
        ParsedBeatmap, SemanticBeatmap, SemanticHitObject, SemanticValidationContext,
        parse_osu_file, validate_semantics,
    },
    statistics::slider_end_time,
};
use std::path::Path;

/// An interior interval this long deserves review, but is never a validation error.
pub const LONG_GAP_MS: u64 = 15_000;
pub const SIMULTANEOUS_OBJECTS: u32 = 3;
const MIN_BIN_MS: u64 = 10_000;
const MAX_BINS: u64 = 720;
const MAX_REPORTED_RANGES: usize = 50;
const MAX_ISSUES_PER_KIND: usize = 10;

pub fn analyze_beatmap(
    beatmap: &str,
    stage: &str,
    parsed: &ParsedBeatmap,
    semantic: &SemanticBeatmap,
) -> BeatmapTimeAnalysis {
    let slider_multiplier = parsed
        .difficulty
        .as_ref()
        .and_then(|difficulty| difficulty.slider_multiplier.as_ref())
        .map(|value| value.value);
    let mut objects = semantic
        .hit_objects
        .iter()
        .map(|object| {
            let start = object.start_time_ms();
            let end = match object {
                SemanticHitObject::Slider {
                    repeats,
                    pixel_length,
                    ..
                } => slider_end_time(
                    start,
                    *repeats,
                    *pixel_length,
                    slider_multiplier,
                    &semantic.timing_points,
                ),
                _ => object.end_time_ms(),
            };
            (start, end)
        })
        .collect::<Vec<_>>();
    objects.sort_unstable();
    let first = objects.first().map(|object| object.0).unwrap_or(0);
    let last = objects.last().map(|object| object.0).unwrap_or(first);
    let duration = last.saturating_sub(first);
    let bin_width_ms = MIN_BIN_MS.max(duration / MAX_BINS + 1);
    let bin_count = duration / bin_width_ms + 1;
    let mut counts = vec![0_u32; bin_count as usize];
    let mut long_gaps = Vec::new();
    let mut simultaneous_clusters = Vec::new();
    let mut previous_end = None::<u64>;
    let mut index = 0;
    while index < objects.len() {
        let start = objects[index].0;
        if let Some(end) = previous_end {
            if start.saturating_sub(end) >= LONG_GAP_MS {
                long_gaps.push(TimeRange {
                    start_ms: end,
                    end_ms: start,
                    object_count: 0,
                });
            }
        }
        let mut group_count = 0_u32;
        let mut group_end = start;
        while index < objects.len() && objects[index].0 == start {
            group_count = group_count.saturating_add(1);
            group_end = group_end.max(objects[index].1);
            index += 1;
        }
        if group_count >= SIMULTANEOUS_OBJECTS {
            simultaneous_clusters.push(TimeRange {
                start_ms: start,
                end_ms: start,
                object_count: group_count,
            });
        }
        counts[(start - first) as usize / bin_width_ms as usize] =
            counts[(start - first) as usize / bin_width_ms as usize].saturating_add(group_count);
        previous_end = Some(previous_end.map_or(group_end, |end| end.max(group_end)));
    }
    let long_gap_count = long_gaps.len().min(u32::MAX as usize) as u32;
    let simultaneous_cluster_count = simultaneous_clusters.len().min(u32::MAX as usize) as u32;
    long_gaps.sort_unstable_by_key(|gap| std::cmp::Reverse(gap.end_ms - gap.start_ms));
    long_gaps.truncate(MAX_REPORTED_RANGES);
    long_gaps.sort_unstable_by_key(|gap| gap.start_ms);
    simultaneous_clusters.sort_unstable_by_key(|cluster| std::cmp::Reverse(cluster.object_count));
    simultaneous_clusters.truncate(MAX_REPORTED_RANGES);
    simultaneous_clusters.sort_unstable_by_key(|cluster| cluster.start_ms);
    let distribution = counts
        .into_iter()
        .enumerate()
        .map(|(index, object_count)| {
            let start_ms = first.saturating_add((index as u64).saturating_mul(bin_width_ms));
            TimeRange {
                start_ms,
                end_ms: start_ms.saturating_add(bin_width_ms),
                object_count,
            }
        })
        .collect();
    BeatmapTimeAnalysis {
        beatmap: beatmap.into(),
        stage: stage.into(),
        bin_width_ms,
        distribution,
        long_gaps,
        simultaneous_clusters,
        long_gap_count,
        simultaneous_cluster_count,
    }
}

pub fn analyze_file(
    path: &Path,
    beatmap: &str,
    stage: &str,
    limits: ValidationLimits,
) -> Option<BeatmapTimeAnalysis> {
    let parsed = parse_osu_file(path, limits);
    if parsed.has_errors() {
        return None;
    }
    let parsed = parsed.beatmap?;
    let semantic = validate_semantics(&parsed, SemanticValidationContext::default());
    if semantic.has_errors() {
        return None;
    }
    Some(analyze_beatmap(beatmap, stage, &parsed, &semantic.beatmap))
}

pub fn quality_issues(analysis: &BeatmapTimeAnalysis) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut gaps = analysis.long_gaps.iter().collect::<Vec<_>>();
    gaps.sort_unstable_by_key(|gap| std::cmp::Reverse(gap.end_ms - gap.start_ms));
    for gap in gaps.into_iter().take(MAX_ISSUES_PER_KIND) {
        issues.push(ValidationIssue {
            code: ValidationCode::LongObjectGap,
            severity: IssueSeverity::Warning,
            section: Some("HitObjects".into()),
            line: None,
            message: format!(
                "{}: review object-free interval {}–{} ms ({} ms); a musical break may be intentional",
                analysis.beatmap,
                gap.start_ms,
                gap.end_ms,
                gap.end_ms - gap.start_ms
            ),
            repairable: false,
        });
    }
    let mut clusters = analysis.simultaneous_clusters.iter().collect::<Vec<_>>();
    clusters.sort_unstable_by_key(|cluster| std::cmp::Reverse(cluster.object_count));
    for cluster in clusters.into_iter().take(MAX_ISSUES_PER_KIND) {
        issues.push(ValidationIssue {
            code: ValidationCode::SimultaneousObjectCluster,
            severity: IssueSeverity::Warning,
            section: Some("HitObjects".into()),
            line: None,
            message: format!(
                "{}: review {} objects at {} ms",
                analysis.beatmap, cluster.object_count, cluster.start_ms
            ),
            repairable: false,
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::osu::parse_osu;

    fn analyzed(times: &[u64]) -> BeatmapTimeAnalysis {
        let source = format!(
            "osu file format v14\n[General]\nAudioFilename:a.wav\nMode:0\n[Metadata]\nTitle:T\nArtist:A\nCreator:C\nVersion:V\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,1,70,1,0\n[HitObjects]\n{}",
            times
                .iter()
                .map(|time| format!("256,192,{time},1,0,0:0:0:0:\n"))
                .collect::<String>()
        );
        let parsed = parse_osu(source.as_bytes(), ValidationLimits::default())
            .beatmap
            .unwrap();
        let semantic = validate_semantics(&parsed, SemanticValidationContext::default());
        assert!(!semantic.has_errors(), "{:?}", semantic.issues);
        analyze_beatmap("Known 6s.osu", "validated", &parsed, &semantic.beatmap)
    }

    #[test]
    fn flags_known_twenty_second_gap_and_cluster_without_errors() {
        let analysis = analyzed(&[70_000, 74_287, 74_287, 74_287, 94_349, 95_000]);
        assert_eq!(analysis.long_gaps.len(), 1);
        assert_eq!(analysis.long_gaps[0].start_ms, 74_287);
        assert_eq!(analysis.long_gaps[0].end_ms, 94_349);
        assert_eq!(analysis.simultaneous_clusters[0].object_count, 3);
        assert_eq!(
            analysis
                .distribution
                .iter()
                .map(|bin| bin.object_count)
                .sum::<u32>(),
            6
        );
        assert!(
            quality_issues(&analysis)
                .iter()
                .all(|issue| issue.severity == IssueSeverity::Warning && !issue.repairable)
        );
    }

    #[test]
    fn ordinary_break_and_pair_do_not_raise_quality_warnings() {
        let analysis = analyzed(&[1_000, 5_000, 5_000, 19_999, 24_000]);
        assert!(quality_issues(&analysis).is_empty());
    }

    #[test]
    fn distribution_is_bounded_for_long_maps() {
        let analysis = analyzed(&[1_000, 7_201_000]);
        assert!(analysis.distribution.len() <= MAX_BINS as usize);
        assert_eq!(
            analysis
                .distribution
                .iter()
                .map(|bin| bin.object_count)
                .sum::<u32>(),
            2
        );
    }
}
