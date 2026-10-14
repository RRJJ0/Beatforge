use std::path::{Component, Path};

const MAX_STEM_UTF16: usize = 180;
const MAX_ARTIST_UTF16: usize = 72;
const MAX_TITLE_UTF16: usize = 105;

fn is_windows_reserved(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or_default();
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn clean_segment(value: &str, fallback: &str) -> String {
    let replaced = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    let mut cleaned = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned = cleaned.trim_matches([' ', '.']).to_string();
    if cleaned.is_empty() {
        cleaned = fallback.into();
    }
    if is_windows_reserved(&cleaned) {
        cleaned.insert(0, '_');
    }
    cleaned
}

fn truncate_utf16(value: &str, maximum: usize) -> String {
    let mut used = 0;
    value
        .chars()
        .take_while(|character| {
            let next = used + character.len_utf16();
            if next > maximum {
                false
            } else {
                used = next;
                true
            }
        })
        .collect::<String>()
        .trim_end_matches([' ', '.'])
        .to_string()
}

pub fn delivery_osz_name(artist: &str, title: &str) -> String {
    let artist = truncate_utf16(&clean_segment(artist, "Unknown Artist"), MAX_ARTIST_UTF16);
    let title = truncate_utf16(&clean_segment(title, "Untitled"), MAX_TITLE_UTF16);
    let stem = truncate_utf16(&format!("{artist} - {title}"), MAX_STEM_UTF16);
    let stem = if stem.is_empty() { "Beatmap" } else { &stem };
    format!("{stem}.osz")
}

pub fn is_safe_osz_name(value: &str) -> bool {
    Path::new(value).components().collect::<Vec<_>>().as_slice()
        == [Component::Normal(value.as_ref())]
        && value.to_ascii_lowercase().ends_with(".osz")
        && value.len() > 4
        && value.trim_end_matches([' ', '.']) == value
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
        && !is_windows_reserved(
            Path::new(value)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_readable_windows_safe_names() {
        assert_eq!(
            delivery_osz_name("9Lana", "螺旋: RASEN?"),
            "9Lana - 螺旋_ RASEN_.osz"
        );
        assert_eq!(delivery_osz_name("CON", "Song..."), "_CON - Song.osz");
        assert!(is_safe_osz_name(&delivery_osz_name("Artist", "Song")));
    }

    #[test]
    fn bounds_long_unicode_names() {
        let name = delivery_osz_name(&"界".repeat(200), &"曲".repeat(200));
        assert!(name.strip_suffix(".osz").unwrap().encode_utf16().count() <= MAX_STEM_UTF16);
    }

    #[test]
    fn rejects_paths_and_non_osz_names() {
        assert!(!is_safe_osz_name("../song.osz"));
        assert!(!is_safe_osz_name("song.zip"));
        assert!(!is_safe_osz_name("song?.osz"));
        assert!(!is_safe_osz_name("CON.osz"));
    }
}
