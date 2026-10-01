use crate::packages::{PackageArtifact, PackageStatus};
use reqwest::{StatusCode, blocking::Client, header};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use sysinfo::Disks;

const BUFFER_SIZE: usize = 256 * 1024;
const FREE_SPACE_RESERVE: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageProgressEvent {
    pub package_id: String,
    pub status: PackageStatus,
    pub progress: u8,
    pub bytes_downloaded: u64,
    pub size_bytes: u64,
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "Failed to open {} for verification: {error}",
            path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; BUFFER_SIZE];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to verify {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn verify_artifact(path: &Path, artifact: &PackageArtifact) -> Result<(), String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?
        .len();
    if size != artifact.size_bytes {
        return Err(format!(
            "Size mismatch for {}: expected {}, received {size}",
            path.display(),
            artifact.size_bytes
        ));
    }
    let actual = sha256_file(path)?;
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(format!(
            "SHA-256 mismatch for {}: expected {}, received {actual}",
            path.display(),
            artifact.sha256
        ));
    }
    Ok(())
}

fn available_space(path: &Path) -> Result<u64, String> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(|disk| disk.available_space())
        .ok_or_else(|| {
            format!(
                "Could not determine available disk space for {}",
                path.display()
            )
        })
}

fn check_disk_space(destination: &Path, remaining_bytes: u64) -> Result<(), String> {
    let available = available_space(destination)?;
    ensure_disk_space(available, remaining_bytes)
}

fn ensure_disk_space(available: u64, remaining_bytes: u64) -> Result<(), String> {
    let required = remaining_bytes.saturating_add(FREE_SPACE_RESERVE);
    if available < required {
        return Err(format!(
            "Not enough disk space: {} MB required, {} MB available",
            required.div_ceil(1024 * 1024),
            available / 1024 / 1024
        ));
    }
    Ok(())
}

fn progress(downloaded: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((downloaded.saturating_mul(100) / total).min(100)) as u8
}

pub fn download_artifact<F>(
    package_id: &str,
    artifact: &PackageArtifact,
    destination: &Path,
    mut on_event: F,
) -> Result<PathBuf, String>
where
    F: FnMut(PackageProgressEvent) -> Result<(), String>,
{
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "Failed to create download directory {}: {error}",
            destination.display()
        )
    })?;
    let final_path = destination.join(&artifact.file_name);
    let part_path = destination.join(format!("{}.part", artifact.file_name));

    if final_path.exists() {
        let metadata = fs::metadata(&final_path).map_err(|error| error.to_string())?;
        if metadata.len() == artifact.size_bytes
            && sha256_file(&final_path)?.eq_ignore_ascii_case(&artifact.sha256)
        {
            on_event(PackageProgressEvent {
                package_id: package_id.into(),
                status: PackageStatus::Verifying,
                progress: 100,
                bytes_downloaded: artifact.size_bytes,
                size_bytes: artifact.size_bytes,
            })?;
            return Ok(final_path);
        }
        fs::remove_file(&final_path).map_err(|error| {
            format!(
                "Failed to remove invalid artifact {}: {error}",
                final_path.display()
            )
        })?;
    }

    let mut downloaded = fs::metadata(&part_path)
        .map(|value| value.len())
        .unwrap_or(0);
    if downloaded > artifact.size_bytes {
        fs::remove_file(&part_path)
            .map_err(|error| format!("Failed to remove oversized partial download: {error}"))?;
        downloaded = 0;
    }
    check_disk_space(destination, artifact.size_bytes.saturating_sub(downloaded))?;

    if downloaded < artifact.size_bytes {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(6 * 60 * 60))
            .build()
            .map_err(|error| format!("Failed to initialize downloader: {error}"))?;
        let mut request = client.get(&artifact.url);
        if downloaded > 0 {
            request = request.header(header::RANGE, format!("bytes={downloaded}-"));
        }
        let mut response = request
            .send()
            .map_err(|error| format!("Download request failed: {error}"))?;
        if !cfg!(test) && response.url().scheme() != "https" {
            return Err("Download was redirected to an insecure URL".into());
        }

        let append = downloaded > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
        if append {
            let expected_prefix = format!("bytes {downloaded}-");
            let content_range = response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if !content_range.starts_with(&expected_prefix) {
                return Err(format!(
                    "Server returned an invalid Content-Range: {content_range}"
                ));
            }
        } else if response.status().is_success() {
            downloaded = 0;
        } else {
            return Err(format!(
                "Download server returned HTTP {}",
                response.status()
            ));
        }

        let mut file = if append {
            OpenOptions::new().append(true).open(&part_path)
        } else {
            File::create(&part_path)
        }
        .map_err(|error| format!("Failed to open partial download: {error}"))?;

        let mut buffer = vec![0; BUFFER_SIZE];
        let mut last_progress = progress(downloaded, artifact.size_bytes);
        on_event(PackageProgressEvent {
            package_id: package_id.into(),
            status: PackageStatus::Downloading,
            progress: last_progress,
            bytes_downloaded: downloaded,
            size_bytes: artifact.size_bytes,
        })?;
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| format!("Download stream failed: {error}"))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .map_err(|error| format!("Failed to write partial download: {error}"))?;
            downloaded = downloaded.saturating_add(read as u64);
            if downloaded > artifact.size_bytes {
                return Err("Download exceeded the size declared by the manifest".into());
            }
            let current_progress = progress(downloaded, artifact.size_bytes);
            if current_progress != last_progress {
                last_progress = current_progress;
                on_event(PackageProgressEvent {
                    package_id: package_id.into(),
                    status: PackageStatus::Downloading,
                    progress: current_progress,
                    bytes_downloaded: downloaded,
                    size_bytes: artifact.size_bytes,
                })?;
            }
        }
        file.sync_all()
            .map_err(|error| format!("Failed to flush partial download: {error}"))?;
    }

    if downloaded != artifact.size_bytes {
        return Err(format!(
            "Incomplete download: expected {} bytes, received {downloaded}",
            artifact.size_bytes
        ));
    }
    on_event(PackageProgressEvent {
        package_id: package_id.into(),
        status: PackageStatus::Verifying,
        progress: 100,
        bytes_downloaded: downloaded,
        size_bytes: artifact.size_bytes,
    })?;
    let actual_hash = sha256_file(&part_path)?;
    if !actual_hash.eq_ignore_ascii_case(&artifact.sha256) {
        fs::remove_file(&part_path).map_err(|error| {
            format!("Checksum failed and the partial file could not be removed: {error}")
        })?;
        return Err(format!(
            "SHA-256 mismatch: expected {}, received {actual_hash}",
            artifact.sha256
        ));
    }
    fs::rename(&part_path, &final_path)
        .map_err(|error| format!("Failed to finalize verified download: {error}"))?;
    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader},
        net::TcpListener,
        sync::mpsc,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_directory(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("beatforge-{name}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn hash(data: &[u8]) -> String {
        Sha256::digest(data)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn server(data: Vec<u8>) -> (String, mpsc::Receiver<Option<u64>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut range_start = None;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(value) = line
                    .strip_prefix("range: bytes=")
                    .or_else(|| line.strip_prefix("Range: bytes="))
                {
                    range_start = value.trim().trim_end_matches('-').parse().ok();
                }
            }
            let _ = sender.send(range_start);
            let start = range_start.unwrap_or(0) as usize;
            let body = &data[start..];
            let header = if range_start.is_some() {
                format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                    body.len(),
                    start,
                    data.len() - 1,
                    data.len()
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
            };
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        });
        (format!("http://{address}/artifact"), receiver, handle)
    }

    #[test]
    fn resumes_and_verifies_a_partial_download() {
        let data = vec![0x5a; 512 * 1024];
        let directory = temp_directory("resume");
        fs::write(directory.join("model.bin.part"), &data[..128 * 1024]).unwrap();
        let (url, received_range, server) = server(data.clone());
        let artifact = PackageArtifact {
            url,
            size_bytes: data.len() as u64,
            sha256: hash(&data),
            file_name: "model.bin".into(),
        };
        let mut events = Vec::new();
        let result = download_artifact("test-model", &artifact, &directory, |event| {
            events.push(event);
            Ok(())
        })
        .unwrap();
        server.join().unwrap();

        assert_eq!(received_range.recv().unwrap(), Some(128 * 1024));
        assert_eq!(fs::read(result).unwrap(), data);
        assert_eq!(events.last().unwrap().status, PackageStatus::Verifying);
        assert!(!directory.join("model.bin.part").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn checksum_failure_does_not_finalize_the_artifact() {
        let data = b"corrupted artifact".to_vec();
        let directory = temp_directory("checksum");
        let (url, _, server) = server(data.clone());
        let artifact = PackageArtifact {
            url,
            size_bytes: data.len() as u64,
            sha256: "0".repeat(64),
            file_name: "model.bin".into(),
        };
        let error = download_artifact("test-model", &artifact, &directory, |_| Ok(())).unwrap_err();
        server.join().unwrap();

        assert!(error.contains("SHA-256 mismatch"));
        assert!(!directory.join("model.bin").exists());
        assert!(!directory.join("model.bin.part").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn disk_check_keeps_a_safety_reserve() {
        let one_megabyte = 1024 * 1024;
        let error =
            ensure_disk_space(FREE_SPACE_RESERVE + one_megabyte - 1, one_megabyte).unwrap_err();
        assert!(error.contains("Not enough disk space"));
        ensure_disk_space(FREE_SPACE_RESERVE + one_megabyte, one_megabyte).unwrap();
    }
}
