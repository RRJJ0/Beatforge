use crate::validation::{ValidationLimits, osu::parse_osu};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};
use zip::ZipArchive;

const MAX_DENSITY_POINTS: usize = 240;
const MAX_WAVEFORM_SOURCE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyPreview {
    pub id: String,
    pub name: String,
    pub object_count: u32,
    pub first_object_ms: Option<u64>,
    pub last_object_ms: Option<u64>,
    pub bin_width_ms: u64,
    pub density: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobPreview {
    pub job_id: String,
    pub audio_path: Option<String>,
    pub audio_unavailable_reason: Option<String>,
    pub waveform_allowed: bool,
    pub max_waveform_points: usize,
    pub difficulties: Vec<DifficultyPreview>,
}

fn difficulty_from_bytes(id: String, bytes: &[u8]) -> Result<DifficultyPreview, String> {
    let parsed = parse_osu(bytes, ValidationLimits::default());
    if parsed.has_errors() {
        return Err(format!(
            "Validated beatmap could not be parsed for preview: {id}"
        ));
    }
    let beatmap = parsed
        .beatmap
        .ok_or_else(|| format!("Validated beatmap has no parsed data: {id}"))?;
    let name = beatmap
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.version.as_ref())
        .map(|version| version.value.trim())
        .filter(|version| !version.is_empty())
        .unwrap_or(&id)
        .to_owned();
    let mut times = beatmap
        .hit_objects
        .iter()
        .filter_map(|object| object.fields.get(2))
        .filter_map(|value| value.trim().parse::<u64>().ok())
        .collect::<Vec<_>>();
    times.sort_unstable();
    let first_object_ms = times.first().copied();
    let last_object_ms = times.last().copied();
    let timeline_ms = last_object_ms.unwrap_or(0).saturating_add(1).max(1);
    let bin_count = if times.is_empty() {
        1
    } else {
        MAX_DENSITY_POINTS.min(timeline_ms as usize).max(1)
    };
    let bin_width_ms = timeline_ms.div_ceil(bin_count as u64).max(1);
    let mut density = vec![0_u32; bin_count];
    for time in &times {
        let index = (*time / bin_width_ms).min((bin_count - 1) as u64) as usize;
        density[index] = density[index].saturating_add(1);
    }
    Ok(DifficultyPreview {
        id,
        name,
        object_count: times.len().min(u32::MAX as usize) as u32,
        first_object_ms,
        last_object_ms,
        bin_width_ms,
        density,
    })
}

fn preview_osu(path: &Path, limits: ValidationLimits) -> Result<Vec<DifficultyPreview>, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > limits.max_beatmap_bytes {
        return Err("Validated beatmap exceeds the preview size limit".into());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let id = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("beatmap.osu")
        .to_owned();
    Ok(vec![difficulty_from_bytes(id, &bytes)?])
}

fn preview_osz(path: &Path, limits: ValidationLimits) -> Result<Vec<DifficultyPreview>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|_| "Validated .osz could not be opened")?;
    if archive.len() > limits.max_archive_entries {
        return Err("Validated archive exceeds the preview entry limit".into());
    }
    let mut difficulties = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| "A validated archive entry could not be opened")?;
        if entry.is_dir()
            || !Path::new(entry.name())
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("osu"))
        {
            continue;
        }
        if difficulties.len() >= limits.max_beatmap_entries
            || entry.size() > limits.max_beatmap_bytes
        {
            return Err("Validated archive exceeds the preview beatmap limits".into());
        }
        let id = entry.name().replace('\\', "/");
        let mut bytes = Vec::with_capacity(entry.size().min(1024 * 1024) as usize);
        entry
            .by_ref()
            .take(limits.max_beatmap_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| format!("Validated beatmap could not be read: {id}"))?;
        if bytes.len() as u64 > limits.max_beatmap_bytes {
            return Err(format!(
                "Validated beatmap exceeds the preview size limit: {id}"
            ));
        }
        difficulties.push(difficulty_from_bytes(id, &bytes)?);
    }
    if difficulties.is_empty() {
        return Err("Validated artifact contains no beatmaps to preview".into());
    }
    difficulties.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(difficulties)
}

pub fn build_difficulties(artifact: &Path) -> Result<Vec<DifficultyPreview>, String> {
    let limits = ValidationLimits::default();
    match artifact
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("osu") => preview_osu(artifact, limits),
        Some("osz") => preview_osz(artifact, limits),
        _ => Err("Validated artifact type is not supported by preview".into()),
    }
}

pub fn audio_details(path: &Path) -> (Option<String>, Option<String>, bool) {
    let supported = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "ogg" | "m4a" | "flac"
            )
        });
    if !supported {
        return (
            None,
            Some("The source audio format is not supported by the preview".into()),
            false,
        );
    }
    let Ok(metadata) = fs::metadata(path) else {
        return (None, Some("The source audio file is missing".into()), false);
    };
    if !metadata.is_file() {
        return (
            None,
            Some("The source audio path is not a file".into()),
            false,
        );
    }
    (
        Some(path.to_string_lossy().into_owned()),
        None,
        metadata.len() <= MAX_WAVEFORM_SOURCE_BYTES,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beatmap(version: &str, times: &[u64]) -> Vec<u8> {
        format!(
            "osu file format v14\n[General]\nAudioFilename:a.mp3\nMode:0\n[Metadata]\nTitle:T\nArtist:A\nCreator:C\nVersion:{version}\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n[TimingPoints]\n0,500,4,2,1,70,1,0\n[HitObjects]\n{}\n",
            times
                .iter()
                .map(|time| format!("256,192,{time},1,0,0:0:0:0:"))
                .collect::<Vec<_>>()
                .join("\n")
        )
        .into_bytes()
    }

    #[test]
    fn density_is_bounded_and_keeps_the_absolute_timeline() {
        let times = (0..500)
            .map(|index| 12_000 + index * 20)
            .collect::<Vec<_>>();
        let preview = difficulty_from_bytes("map.osu".into(), &beatmap("Hard", &times)).unwrap();
        assert_eq!(preview.name, "Hard");
        assert_eq!(preview.object_count, 500);
        assert_eq!(preview.first_object_ms, Some(12_000));
        assert_eq!(preview.last_object_ms, Some(21_980));
        assert_eq!(preview.density.len(), MAX_DENSITY_POINTS);
        assert_eq!(preview.density.iter().sum::<u32>(), 500);
        assert!(preview.density[..100].iter().all(|count| *count == 0));
    }

    #[test]
    fn object_at_zero_has_a_stable_single_bin() {
        let preview = difficulty_from_bytes("map.osu".into(), &beatmap("Zero", &[0])).unwrap();
        assert_eq!(preview.density, vec![1]);
        assert_eq!(preview.bin_width_ms, 1);
        assert_eq!(preview.first_object_ms, Some(0));
    }
}
