use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

pub const MAX_VIDEO_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_VIDEO_START_TIME_MS: i64 = 3_600_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub path: String,
    pub size_bytes: u64,
    pub format: String,
}

fn matches_format(extension: &str, header: &[u8]) -> bool {
    match extension {
        "avi" => header.len() >= 12 && &header[..4] == b"RIFF" && &header[8..12] == b"AVI ",
        "flv" => header.len() >= 9 && &header[..3] == b"FLV" && header[3] == 1,
        "mp4" => header
            .windows(4)
            .enumerate()
            .any(|(index, value)| value == b"ftyp" && index >= 4 && index <= 64),
        _ => false,
    }
}

pub fn validate_video_start_time(value: i64) -> Result<(), String> {
    if !(0..=MAX_VIDEO_START_TIME_MS).contains(&value) {
        return Err(format!(
            "Video start time must be between 0 and {MAX_VIDEO_START_TIME_MS} milliseconds"
        ));
    }
    Ok(())
}

pub fn inspect_video(path: &Path) -> Result<VideoInfo, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("Could not inspect video: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The selected video must be a regular file".into());
    }
    if metadata.len() == 0 || metadata.len() > MAX_VIDEO_BYTES {
        return Err(format!(
            "Video must be between 1 byte and {} MB",
            MAX_VIDEO_BYTES / (1024 * 1024)
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "avi" | "flv" | "mp4") {
        return Err("Video must be AVI, FLV, or MP4".into());
    }
    let mut header = [0_u8; 128];
    let mut file = File::open(path).map_err(|error| format!("Could not read video: {error}"))?;
    let read = file
        .read(&mut header)
        .map_err(|error| format!("Could not read video header: {error}"))?;
    if !matches_format(&extension, &header[..read]) {
        return Err(format!(
            "Video contents do not match the selected {extension} file type"
        ));
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve video: {error}"))?;
    Ok(VideoInfo {
        path: path.to_string_lossy().into_owned(),
        size_bytes: metadata.len(),
        format: extension,
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
        let path = std::env::temp_dir().join(format!("beatforge-video-{nonce}.{extension}"));
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn accepts_supported_video_headers() {
        for (extension, bytes) in [
            ("avi", b"RIFF\x20\x00\x00\x00AVI fixture".as_slice()),
            ("flv", b"FLV\x01\x05\x00\x00\x00\x09fixture".as_slice()),
            ("mp4", b"\x00\x00\x00\x18ftypisomfixture".as_slice()),
        ] {
            let path = temporary_file(extension, bytes);
            assert_eq!(inspect_video(&path).unwrap().format, extension);
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn rejects_extension_content_mismatch() {
        let path = temporary_file("mp4", b"not an mp4 video");
        assert!(inspect_video(&path).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounds_video_start_time() {
        assert!(validate_video_start_time(0).is_ok());
        assert!(validate_video_start_time(MAX_VIDEO_START_TIME_MS).is_ok());
        assert!(validate_video_start_time(-1).is_err());
        assert!(validate_video_start_time(MAX_VIDEO_START_TIME_MS + 1).is_err());
    }
}
