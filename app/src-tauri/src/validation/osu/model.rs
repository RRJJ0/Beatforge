use crate::validation::{IssueSeverity, ValidationIssue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Located<T> {
    pub value: T,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueField {
    pub key: String,
    pub value: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawLine {
    pub value: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSection {
    pub header_line: u32,
    pub audio_filename: Option<Located<String>>,
    pub mode: Option<Located<u8>>,
    pub fields: Vec<KeyValueField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSection {
    pub header_line: u32,
    pub fields: Vec<KeyValueField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSection {
    pub header_line: u32,
    pub title: Option<Located<String>>,
    pub title_unicode: Option<Located<String>>,
    pub artist: Option<Located<String>>,
    pub artist_unicode: Option<Located<String>>,
    pub creator: Option<Located<String>>,
    pub version: Option<Located<String>>,
    pub fields: Vec<KeyValueField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySection {
    pub header_line: u32,
    pub hp_drain_rate: Option<Located<f64>>,
    pub circle_size: Option<Located<f64>>,
    pub overall_difficulty: Option<Located<f64>>,
    pub approach_rate: Option<Located<f64>>,
    pub slider_multiplier: Option<Located<f64>>,
    pub slider_tick_rate: Option<Located<f64>>,
    pub fields: Vec<KeyValueField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTimingPoint {
    pub line: u32,
    pub raw: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawHitObject {
    pub line: u32,
    pub raw: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownSection {
    pub name: String,
    pub header_line: u32,
    pub lines: Vec<RawLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedBeatmap {
    pub format_version: u32,
    pub general: Option<GeneralSection>,
    pub editor: Option<EditorSection>,
    pub metadata: Option<MetadataSection>,
    pub difficulty: Option<DifficultySection>,
    pub events: Vec<RawLine>,
    pub timing_points: Vec<RawTimingPoint>,
    pub colours: Vec<RawLine>,
    pub hit_objects: Vec<RawHitObject>,
    pub unknown_sections: Vec<UnknownSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub beatmap: Option<ParsedBeatmap>,
    pub issues: Vec<ValidationIssue>,
}

impl ParseResult {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }
}
