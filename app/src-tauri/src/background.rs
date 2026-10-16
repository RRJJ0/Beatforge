use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const MAX_BACKGROUND_BYTES: u64 = 20 * 1024 * 1024;
pub const MIN_BACKGROUND_WIDTH: u32 = 320;
pub const MIN_BACKGROUND_HEIGHT: u32 = 180;
pub const MAX_BACKGROUND_DIMENSION: u32 = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundImageInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub format: String,
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 33
        || &bytes[..8] != SIGNATURE
        || &bytes[12..16] != b"IHDR"
        || !bytes.windows(4).any(|window| window == b"IEND")
    {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4
        || bytes[..2] != [0xff, 0xd8]
        || !bytes.windows(2).any(|window| window == [0xff, 0xd9])
    {
        return None;
    }
    let mut index = 2_usize;
    while index + 3 < bytes.len() {
        if bytes[index] != 0xff {
            index += 1;
            continue;
        }
        while index < bytes.len() && bytes[index] == 0xff {
            index += 1;
        }
        let marker = *bytes.get(index)?;
        index += 1;
        if marker == 0xd9 || marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes([*bytes.get(index)?, *bytes.get(index + 1)?]) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if length < 7 {
                return None;
            }
            let height = u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as u32;
            return Some((width, height));
        }
        index += length;
    }
    None
}

pub fn inspect_background_image(path: &Path) -> Result<BackgroundImageInfo, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not inspect background image: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The selected background must be a regular file".into());
    }
    if metadata.len() == 0 || metadata.len() > MAX_BACKGROUND_BYTES {
        return Err(format!(
            "Background image must be between 1 byte and {} MB",
            MAX_BACKGROUND_BYTES / (1024 * 1024)
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
        return Err("Background image must be PNG or JPEG".into());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("Could not read background image: {error}"))?;
    let (format, dimensions) = match extension.as_str() {
        "png" => ("png", png_dimensions(&bytes)),
        "jpg" | "jpeg" => ("jpeg", jpeg_dimensions(&bytes)),
        _ => unreachable!(),
    };
    let (width, height) = dimensions.ok_or_else(|| {
        format!("Background contents do not match the selected {format} file type")
    })?;
    if width < MIN_BACKGROUND_WIDTH
        || height < MIN_BACKGROUND_HEIGHT
        || width > MAX_BACKGROUND_DIMENSION
        || height > MAX_BACKGROUND_DIMENSION
    {
        return Err(format!(
            "Background dimensions must be at least {MIN_BACKGROUND_WIDTH}×{MIN_BACKGROUND_HEIGHT} and no larger than {MAX_BACKGROUND_DIMENSION}×{MAX_BACKGROUND_DIMENSION}"
        ));
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve background image: {error}"))?;
    Ok(BackgroundImageInfo {
        path: path.to_string_lossy().into_owned(),
        width,
        height,
        size_bytes: metadata.len(),
        format: format.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_file(extension: &str, bytes: &[u8]) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("beatforge-background-{nonce}.{extension}"));
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn accepts_a_bounded_png_header() {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&1920_u32.to_be_bytes());
        bytes.extend_from_slice(&1080_u32.to_be_bytes());
        bytes.extend_from_slice(b"\x08\x02\0\0\0crc\0\0\0\0IENDcrc");
        let path = temporary_file("png", &bytes);
        let info = inspect_background_image(&path).unwrap();
        assert_eq!((info.width, info.height), (1920, 1080));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_extension_content_mismatch() {
        let path = temporary_file("jpg", b"not a jpeg image");
        assert!(inspect_background_image(&path).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn accepts_a_bounded_jpeg_header() {
        let mut bytes = vec![
            0xff, 0xd8, // SOI
            0xff, 0xc0, // baseline SOF
            0x00, 0x11, // segment length
            0x08, // sample precision
            0x04, 0x38, // height: 1080
            0x07, 0x80, // width: 1920
            0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
        ];
        bytes.extend_from_slice(&[0xff, 0xd9]);
        let path = temporary_file("jpeg", &bytes);
        let info = inspect_background_image(&path).unwrap();
        assert_eq!((info.width, info.height), (1920, 1080));
        assert_eq!(info.format, "jpeg");
        fs::remove_file(path).unwrap();
    }
}
