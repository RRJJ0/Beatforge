use crate::validation::{
    IssueSeverity, ValidationCode, ValidationIssue, ValidationLimits,
    osu::{SemanticValidationContext, parse_osu_file, validate_semantics},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntryInfo {
    pub name: String,
    pub relative_path: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub is_directory: bool,
    pub is_beatmap: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveInspection {
    pub entries: Vec<ArchiveEntryInfo>,
    pub beatmap_paths: Vec<String>,
    pub total_uncompressed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedArchive {
    pub root: PathBuf,
    pub inspection: ArchiveInspection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveResult<T> {
    pub value: Option<T>,
    pub issues: Vec<ValidationIssue>,
}

impl<T> ArchiveResult<T> {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }
}

fn issue(code: ValidationCode, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        code,
        severity: IssueSeverity::Error,
        section: Some("Archive".into()),
        line: None,
        message: message.into(),
        repairable: false,
    }
}

fn failure<T>(code: ValidationCode, message: impl Into<String>) -> ArchiveResult<T> {
    ArchiveResult {
        value: None,
        issues: vec![issue(code, message)],
    }
}

fn windows_reserved(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'))
}

fn normalize_archive_name(name: &str) -> Result<(String, PathBuf), ValidationCode> {
    if name.is_empty() || name.contains('\0') {
        return Err(ValidationCode::ArchiveInvalid);
    }
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/')
        || normalized.starts_with("//")
        || normalized
            .as_bytes()
            .get(1)
            .is_some_and(|value| *value == b':')
    {
        return Err(ValidationCode::ArchiveAbsolutePath);
    }
    let mut output = PathBuf::new();
    let mut display = Vec::new();
    for component in normalized.trim_end_matches('/').split('/') {
        if component == ".." {
            return Err(ValidationCode::ArchivePathTraversal);
        }
        if component.is_empty()
            || component == "."
            || component.contains(':')
            || component.ends_with([' ', '.'])
            || windows_reserved(component)
        {
            return Err(ValidationCode::ArchiveInvalid);
        }
        output.push(component);
        display.push(component);
    }
    if display.is_empty() {
        return Err(ValidationCode::ArchiveInvalid);
    }
    Ok((display.join("/"), output))
}

fn is_supported_compression(method: CompressionMethod) -> bool {
    matches!(
        method,
        CompressionMethod::Stored | CompressionMethod::Deflated
    )
}

pub fn inspect_osz(path: &Path, limits: ValidationLimits) -> ArchiveResult<ArchiveInspection> {
    if !path.is_file() {
        return failure(
            ValidationCode::OutputNotFound,
            "The .osz archive does not exist or is not a file",
        );
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("osz"))
    {
        return failure(
            ValidationCode::UnsupportedOutputType,
            "Archive output must use the .osz extension",
        );
    }
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return failure(
                ValidationCode::ArchiveInvalid,
                "The .osz archive could not be opened",
            );
        }
    };
    let mut archive = match ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(_) => {
            return failure(
                ValidationCode::ArchiveInvalid,
                "The .osz file is not a valid ZIP archive",
            );
        }
    };
    if archive.len() > limits.max_archive_entries {
        return failure(
            ValidationCode::ArchiveLimitExceeded,
            format!(
                "Archive has {} entries; the limit is {}",
                archive.len(),
                limits.max_archive_entries
            ),
        );
    }

    let mut entries = Vec::with_capacity(archive.len());
    let mut beatmap_paths = Vec::new();
    let mut total_uncompressed_bytes = 0_u64;
    let mut seen = BTreeSet::new();
    let mut path_types = BTreeMap::new();
    for index in 0..archive.len() {
        let entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(_) => {
                return failure(
                    ValidationCode::ArchiveInvalid,
                    "An archive entry could not be read",
                );
            }
        };
        let (relative_path, _) = match normalize_archive_name(entry.name()) {
            Ok(value) => value,
            Err(code) => {
                return failure(
                    code,
                    format!("Archive contains an unsafe entry name: {}", entry.name()),
                );
            }
        };
        let collision_key = relative_path.to_lowercase();
        if !seen.insert(collision_key.clone()) {
            return failure(
                ValidationCode::ArchiveCaseCollision,
                format!("Archive contains a duplicate or case-colliding path: {relative_path}"),
            );
        }
        path_types.insert(collision_key, entry.is_dir());
        if entry.is_symlink()
            || entry.unix_mode().is_some_and(|mode| {
                let file_type = mode & 0o170000;
                file_type != 0 && file_type != 0o100000 && file_type != 0o040000
            })
        {
            return failure(
                ValidationCode::ArchiveLinkEntry,
                format!("Archive contains a link-like entry: {relative_path}"),
            );
        }
        if entry.encrypted() {
            return failure(
                ValidationCode::ArchiveEncrypted,
                format!("Archive contains an encrypted entry: {relative_path}"),
            );
        }
        if !is_supported_compression(entry.compression()) {
            return failure(
                ValidationCode::ArchiveUnsupportedCompression,
                format!("Archive entry uses unsupported compression: {relative_path}"),
            );
        }
        if entry.size() > limits.max_single_entry_bytes {
            return failure(
                ValidationCode::ArchiveLimitExceeded,
                format!("Archive entry exceeds the single-file limit: {relative_path}"),
            );
        }
        total_uncompressed_bytes = match total_uncompressed_bytes.checked_add(entry.size()) {
            Some(total) if total <= limits.max_archive_uncompressed_bytes => total,
            _ => {
                return failure(
                    ValidationCode::ArchiveLimitExceeded,
                    "Archive exceeds the total uncompressed-size limit",
                );
            }
        };
        let is_beatmap = !entry.is_dir()
            && Path::new(&relative_path)
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("osu"));
        if is_beatmap {
            if entry.size() > limits.max_beatmap_bytes {
                return failure(
                    ValidationCode::ArchiveLimitExceeded,
                    format!("Beatmap exceeds the configured limit: {relative_path}"),
                );
            }
            beatmap_paths.push(relative_path.clone());
            if beatmap_paths.len() > limits.max_beatmap_entries {
                return failure(
                    ValidationCode::ArchiveLimitExceeded,
                    "Archive contains too many beatmaps",
                );
            }
        }
        entries.push(ArchiveEntryInfo {
            name: entry.name().to_owned(),
            relative_path,
            compressed_size: entry.compressed_size(),
            uncompressed_size: entry.size(),
            is_directory: entry.is_dir(),
            is_beatmap,
        });
    }
    for path in path_types.keys() {
        let parts = path.split('/').collect::<Vec<_>>();
        for length in 1..parts.len() {
            let parent = parts[..length].join("/");
            if path_types.get(&parent) == Some(&false) {
                return failure(
                    ValidationCode::ArchiveCaseCollision,
                    format!("Archive file is also used as a parent directory: {parent}"),
                );
            }
        }
    }
    if beatmap_paths.is_empty() {
        return failure(
            ValidationCode::BeatmapNotFound,
            "Archive does not contain an .osu beatmap",
        );
    }
    ArchiveResult {
        value: Some(ArchiveInspection {
            entries,
            beatmap_paths,
            total_uncompressed_bytes,
        }),
        issues: Vec::new(),
    }
}

fn remove_created_directory(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn csv_fields(value: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                current.push('"');
                characters.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(character),
        }
    }
    fields.push(current.trim().to_owned());
    fields
}

fn safe_reference_key(map_path: &str, reference: &str) -> Option<String> {
    let reference = reference.trim().trim_matches('"');
    let (normalized, _) = normalize_archive_name(reference).ok()?;
    let parent = Path::new(map_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let joined = parent.join(normalized);
    let mut components = Vec::new();
    for component in joined.components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
            _ => return None,
        }
    }
    Some(components.join("/").to_lowercase())
}

fn referenced_assets(
    beatmap: &crate::validation::osu::ParsedBeatmap,
) -> Vec<(String, bool, &'static str, Option<u32>)> {
    let mut references = Vec::new();
    if let Some(audio) = beatmap
        .general
        .as_ref()
        .and_then(|general| general.audio_filename.as_ref())
    {
        references.push((audio.value.clone(), true, "General", Some(audio.line)));
    }
    for event in &beatmap.events {
        let fields = csv_fields(&event.value);
        let kind = fields.first().map(String::as_str).unwrap_or_default();
        let asset = match kind.to_ascii_lowercase().as_str() {
            "0" | "video" | "1" => fields.get(2),
            "sample" | "5" => fields.get(3),
            _ => None,
        };
        if let Some(asset) = asset
            && !asset.is_empty()
        {
            references.push((asset.clone(), false, "Events", Some(event.line)));
        }
    }
    for object in &beatmap.hit_objects {
        if let Some(last) = object.fields.last() {
            let values = last.split(':').collect::<Vec<_>>();
            if values.len() >= 5 && !values[4].trim().is_empty() {
                references.push((
                    values[4].trim().into(),
                    false,
                    "HitObjects",
                    Some(object.line),
                ));
            }
        }
    }
    references
}

fn validate_extracted_integrity(
    root: &Path,
    inspection: &ArchiveInspection,
    limits: ValidationLimits,
) -> Vec<ValidationIssue> {
    let available = inspection
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.relative_path.to_lowercase())
        .collect::<BTreeSet<_>>();
    let mut issues = Vec::new();
    let mut reported_references = BTreeSet::new();
    for beatmap_path in &inspection.beatmap_paths {
        let parsed = parse_osu_file(&root.join(Path::new(beatmap_path)), limits);
        issues.extend(parsed.issues);
        let Some(beatmap) = parsed.beatmap else {
            continue;
        };
        issues.extend(validate_semantics(&beatmap, SemanticValidationContext::default()).issues);
        for (reference, audio, section, line) in referenced_assets(&beatmap) {
            let Some(key) = safe_reference_key(beatmap_path, &reference) else {
                issues.push(ValidationIssue {
                    code: ValidationCode::FieldInvalid,
                    severity: IssueSeverity::Error,
                    section: Some(section.into()),
                    line,
                    message: format!("Referenced asset uses an unsafe path: {reference}"),
                    repairable: false,
                });
                continue;
            };
            if !available.contains(&key) && reported_references.insert((key, audio)) {
                issues.push(ValidationIssue {
                    code: if audio {
                        ValidationCode::AudioMissing
                    } else {
                        ValidationCode::ReferencedAssetMissing
                    },
                    severity: IssueSeverity::Error,
                    section: Some(section.into()),
                    line,
                    message: format!("Referenced asset is missing from the archive: {reference}"),
                    repairable: false,
                });
            }
        }
    }
    issues
}

pub fn extract_osz(
    archive_path: &Path,
    destination: &Path,
    limits: ValidationLimits,
) -> ArchiveResult<ExtractedArchive> {
    let inspected = inspect_osz(archive_path, limits);
    let Some(inspection) = inspected.value else {
        return ArchiveResult {
            value: None,
            issues: inspected.issues,
        };
    };
    if destination.exists() {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Extraction destination must not already exist",
        );
    }
    let Some(destination_parent) = destination.parent() else {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Extraction destination has no parent directory",
        );
    };
    let Ok(destination_parent) = destination_parent.canonicalize() else {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Extraction parent directory is unavailable",
        );
    };
    if fs::create_dir(destination).is_err() {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Extraction destination could not be created",
        );
    }
    if destination
        .canonicalize()
        .ok()
        .and_then(|path| path.parent().map(Path::to_owned))
        .as_ref()
        != Some(&destination_parent)
    {
        remove_created_directory(destination);
        return failure(
            ValidationCode::ArchivePathTraversal,
            "Extraction destination did not resolve under its direct parent",
        );
    }

    let extraction = (|| -> Result<(), ValidationIssue> {
        let file = File::open(archive_path).map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Archive could not be reopened for extraction",
            )
        })?;
        let mut archive = ZipArchive::new(file).map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Archive became unreadable before extraction",
            )
        })?;
        let mut actual_total = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Archive entry could not be opened during extraction",
                )
            })?;
            let expected = inspection.entries.get(index).ok_or_else(|| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Archive entry count changed after inspection",
                )
            })?;
            if entry.name() != expected.name
                || entry.size() != expected.uncompressed_size
                || entry.compressed_size() != expected.compressed_size
                || entry.is_dir() != expected.is_directory
                || entry.encrypted()
                || entry.is_symlink()
                || !is_supported_compression(entry.compression())
            {
                return Err(issue(
                    ValidationCode::ArchiveInvalid,
                    "Archive entry identity changed after inspection",
                ));
            }
            let (_, relative) = normalize_archive_name(entry.name()).map_err(|code| {
                issue(
                    code,
                    "Archive entry path changed between inspection and extraction",
                )
            })?;
            let output_path = destination.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&output_path).map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Archive directory could not be created",
                    )
                })?;
                continue;
            }
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Archive parent directory could not be created",
                    )
                })?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output_path)
                .map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Archive attempted to overwrite an extracted path",
                    )
                })?;
            let mut entry_total = 0_u64;
            loop {
                let read = entry.read(&mut buffer).map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Archive entry failed CRC or stream validation",
                    )
                })?;
                if read == 0 {
                    break;
                }
                entry_total = entry_total.checked_add(read as u64).ok_or_else(|| {
                    issue(
                        ValidationCode::ArchiveLimitExceeded,
                        "Archive entry size overflowed",
                    )
                })?;
                actual_total = actual_total.checked_add(read as u64).ok_or_else(|| {
                    issue(
                        ValidationCode::ArchiveLimitExceeded,
                        "Archive total size overflowed",
                    )
                })?;
                if entry_total > limits.max_single_entry_bytes
                    || actual_total > limits.max_archive_uncompressed_bytes
                {
                    return Err(issue(
                        ValidationCode::ArchiveLimitExceeded,
                        "Archive exceeded a streamed extraction limit",
                    ));
                }
                output.write_all(&buffer[..read]).map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Extracted file could not be written",
                    )
                })?;
            }
            if entry_total != expected.uncompressed_size {
                return Err(issue(
                    ValidationCode::ArchiveInvalid,
                    "Archive entry size did not match its inspected identity",
                ));
            }
            output.sync_all().map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Extracted file could not be finalized",
                )
            })?;
        }
        Ok(())
    })();
    if let Err(error) = extraction {
        remove_created_directory(destination);
        return ArchiveResult {
            value: None,
            issues: vec![error],
        };
    }

    let issues = validate_extracted_integrity(destination, &inspection, limits);
    ArchiveResult {
        value: Some(ExtractedArchive {
            root: destination.to_owned(),
            inspection,
        }),
        issues,
    }
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf, u64)>,
) -> Result<(), ValidationIssue> {
    let mut entries = fs::read_dir(directory)
        .map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package directory could not be read",
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package entry could not be read",
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package entry metadata could not be read",
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(issue(
                ValidationCode::ArchiveLinkEntry,
                "Package source contains a symbolic link",
            ));
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path.strip_prefix(root).map_err(|_| {
                issue(
                    ValidationCode::ArchivePathTraversal,
                    "Package source escaped its root",
                )
            })?;
            let name = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            normalize_archive_name(&name).map_err(|code| {
                issue(code, format!("Package source has an unsafe path: {name}"))
            })?;
            files.push((name, path, metadata.len()));
        }
    }
    Ok(())
}

fn verify_archive_streams(path: &Path, limits: ValidationLimits) -> Result<(), ValidationIssue> {
    let file = File::open(path).map_err(|_| {
        issue(
            ValidationCode::ArchiveInvalid,
            "Packaged archive could not be reopened",
        )
    })?;
    let mut archive = ZipArchive::new(file).map_err(|_| {
        issue(
            ValidationCode::ArchiveInvalid,
            "Packaged output is not a valid ZIP archive",
        )
    })?;
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Packaged entry could not be reopened",
            )
        })?;
        let mut entry_total = 0_u64;
        loop {
            let read = entry.read(&mut buffer).map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Packaged entry failed stream or CRC verification",
                )
            })?;
            if read == 0 {
                break;
            }
            entry_total = entry_total.checked_add(read as u64).ok_or_else(|| {
                issue(
                    ValidationCode::ArchiveLimitExceeded,
                    "Packaged entry size overflowed",
                )
            })?;
            total = total.checked_add(read as u64).ok_or_else(|| {
                issue(
                    ValidationCode::ArchiveLimitExceeded,
                    "Packaged total size overflowed",
                )
            })?;
            if entry_total > limits.max_single_entry_bytes
                || total > limits.max_archive_uncompressed_bytes
            {
                return Err(issue(
                    ValidationCode::ArchiveLimitExceeded,
                    "Packaged output exceeded a streamed verification limit",
                ));
            }
        }
        if entry_total != entry.size() {
            return Err(issue(
                ValidationCode::ArchiveInvalid,
                "Packaged entry size did not match its ZIP metadata",
            ));
        }
    }
    Ok(())
}

pub fn package_osz_from_directory(
    source_root: &Path,
    output_path: &Path,
    limits: ValidationLimits,
) -> ArchiveResult<ArchiveInspection> {
    let source_root = match source_root.canonicalize() {
        Ok(path) if path.is_dir() => path,
        _ => {
            return failure(
                ValidationCode::OutputNotFound,
                "Package source directory is unavailable",
            );
        }
    };
    if output_path.exists() {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Package output already exists and will not be overwritten",
        );
    }
    if !output_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("osz"))
    {
        return failure(
            ValidationCode::UnsupportedOutputType,
            "Package output must use the .osz extension",
        );
    }
    let Some(parent) = output_path.parent() else {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Package output has no parent directory",
        );
    };
    let output_parent = match parent.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return failure(
                ValidationCode::ArchiveInvalid,
                "Package output directory is unavailable",
            );
        }
    };
    if output_parent.starts_with(&source_root) {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Package output cannot be written inside its source directory",
        );
    }

    let mut files = Vec::new();
    if let Err(error) = collect_files(&source_root, &source_root, &mut files) {
        return ArchiveResult {
            value: None,
            issues: vec![error],
        };
    }
    if files.len() > limits.max_archive_entries {
        return failure(
            ValidationCode::ArchiveLimitExceeded,
            "Package source contains too many files",
        );
    }
    let mut total = 0_u64;
    let mut beatmaps = 0_usize;
    let mut seen = BTreeMap::new();
    for (name, _, size) in &files {
        if seen.insert(name.to_lowercase(), name).is_some() {
            return failure(
                ValidationCode::ArchiveCaseCollision,
                format!("Package source contains case-colliding paths: {name}"),
            );
        }
        if *size > limits.max_single_entry_bytes {
            return failure(
                ValidationCode::ArchiveLimitExceeded,
                format!("Package source file exceeds the size limit: {name}"),
            );
        }
        total = match total.checked_add(*size) {
            Some(value) if value <= limits.max_archive_uncompressed_bytes => value,
            _ => {
                return failure(
                    ValidationCode::ArchiveLimitExceeded,
                    "Package source exceeds the total-size limit",
                );
            }
        };
        if Path::new(name)
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("osu"))
        {
            beatmaps += 1;
            if *size > limits.max_beatmap_bytes || beatmaps > limits.max_beatmap_entries {
                return failure(
                    ValidationCode::ArchiveLimitExceeded,
                    "Package source exceeds a beatmap limit",
                );
            }
        }
    }
    if beatmaps == 0 {
        return failure(
            ValidationCode::BeatmapNotFound,
            "Package source does not contain an .osu beatmap",
        );
    }

    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("beatmap.osz");
    let partial = output_parent.join(format!(".{file_name}.partial"));
    if partial.exists() {
        return failure(
            ValidationCode::ArchiveInvalid,
            "Package staging output already exists",
        );
    }
    let packaging = (|| -> Result<(), ValidationIssue> {
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Package staging output could not be created",
                )
            })?;
        let mut writer = ZipWriter::new(output);
        for (name, path, size) in &files {
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644)
                .large_file(*size > u32::MAX as u64);
            writer.start_file(name, options).map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Package entry could not be started",
                )
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Package source metadata changed during packaging",
                )
            })?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != *size {
                return Err(issue(
                    ValidationCode::ArchiveInvalid,
                    "Package source identity changed during packaging",
                ));
            }
            let input = File::open(path).map_err(|_| {
                issue(
                    ValidationCode::ArchiveInvalid,
                    "Package source file could not be opened",
                )
            })?;
            let copied = std::io::copy(&mut input.take(size.saturating_add(1)), &mut writer)
                .map_err(|_| {
                    issue(
                        ValidationCode::ArchiveInvalid,
                        "Package source file could not be written",
                    )
                })?;
            if copied != *size {
                return Err(issue(
                    ValidationCode::ArchiveInvalid,
                    "Package source size changed while it was read",
                ));
            }
        }
        let output = writer.finish().map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package ZIP could not be finalized",
            )
        })?;
        output.sync_all().map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package output could not be synchronized",
            )
        })?;
        drop(output);
        verify_archive_streams(&partial, limits)?;
        fs::rename(&partial, output_path).map_err(|_| {
            issue(
                ValidationCode::ArchiveInvalid,
                "Package output could not be atomically activated",
            )
        })?;
        Ok(())
    })();
    if let Err(error) = packaging {
        let _ = fs::remove_file(&partial);
        return ArchiveResult {
            value: None,
            issues: vec![error],
        };
    }
    let inspected = inspect_osz(output_path, limits);
    if inspected.has_errors() {
        let _ = fs::remove_file(output_path);
        return inspected;
    }
    let verification_directory = output_parent.join(format!(".{file_name}.verify"));
    if verification_directory.exists() {
        let _ = fs::remove_file(output_path);
        return failure(
            ValidationCode::ArchiveInvalid,
            "Package verification directory already exists",
        );
    }
    let verified = extract_osz(output_path, &verification_directory, limits);
    if verification_directory.exists() {
        remove_created_directory(&verification_directory);
    }
    if verified.has_errors() {
        let _ = fs::remove_file(output_path);
        return ArchiveResult {
            value: None,
            issues: verified.issues,
        };
    }
    ArchiveResult {
        value: inspected.value,
        issues: verified.issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "beatforge-archive-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_map() -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/validation/valid/minimal-standard.osu"),
        )
        .unwrap()
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
    }

    fn code<T>(result: &ArchiveResult<T>) -> ValidationCode {
        result.issues.first().unwrap().code
    }

    #[test]
    fn inspects_and_extracts_a_valid_osz_with_referenced_audio() {
        let root = TestRoot::new("valid");
        let archive = root.path().join("valid.osz");
        let beatmap = fixture_map();
        write_zip(
            &archive,
            &[("map.osu", &beatmap), ("fixture.wav", b"synthetic fixture")],
        );

        let inspected = inspect_osz(&archive, ValidationLimits::default());
        assert!(!inspected.has_errors(), "{:?}", inspected.issues);
        let inspection = inspected.value.unwrap();
        assert_eq!(inspection.entries.len(), 2);
        assert_eq!(inspection.beatmap_paths, vec!["map.osu"]);

        let extracted = extract_osz(
            &archive,
            &root.path().join("extracted"),
            ValidationLimits::default(),
        );
        assert!(!extracted.has_errors(), "{:?}", extracted.issues);
        let extracted = extracted.value.unwrap();
        assert!(extracted.root.join("map.osu").is_file());
        assert!(extracted.root.join("fixture.wav").is_file());
    }

    #[test]
    fn rejects_traversal_absolute_and_case_colliding_names_before_extraction() {
        let root = TestRoot::new("paths");
        let beatmap = fixture_map();
        for (name, entry, expected) in [
            (
                "traversal.osz",
                "../outside.osu",
                ValidationCode::ArchivePathTraversal,
            ),
            (
                "absolute.osz",
                "C:/outside.osu",
                ValidationCode::ArchiveAbsolutePath,
            ),
            (
                "posix.osz",
                "/outside.osu",
                ValidationCode::ArchiveAbsolutePath,
            ),
        ] {
            let archive = root.path().join(name);
            write_zip(&archive, &[(entry, &beatmap)]);
            let destination = root.path().join(format!("extract-{name}"));
            let result = extract_osz(&archive, &destination, ValidationLimits::default());
            assert_eq!(code(&result), expected);
            assert!(!destination.exists());
        }

        let archive = root.path().join("collision.osz");
        write_zip(&archive, &[("Map.osu", &beatmap), ("map.osu", &beatmap)]);
        assert_eq!(
            code(&inspect_osz(&archive, ValidationLimits::default())),
            ValidationCode::ArchiveCaseCollision
        );

        let archive = root.path().join("prefix-collision.osz");
        write_zip(
            &archive,
            &[("assets", b"file"), ("assets/map.osu", &beatmap)],
        );
        assert_eq!(
            code(&inspect_osz(&archive, ValidationLimits::default())),
            ValidationCode::ArchiveCaseCollision
        );
    }

    #[test]
    fn rejects_link_entries_and_unsupported_archive_shapes() {
        let root = TestRoot::new("shape");
        let archive = root.path().join("link.osz");
        let file = File::create(&archive).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .add_symlink(
                "map.osu",
                "target.osu",
                SimpleFileOptions::default().unix_permissions(0o777),
            )
            .unwrap();
        writer.finish().unwrap();
        assert_eq!(
            code(&inspect_osz(&archive, ValidationLimits::default())),
            ValidationCode::ArchiveLinkEntry
        );

        let malformed = root.path().join("malformed.osz");
        fs::write(&malformed, b"not a zip").unwrap();
        assert_eq!(
            code(&inspect_osz(&malformed, ValidationLimits::default())),
            ValidationCode::ArchiveInvalid
        );

        let no_map = root.path().join("no-map.osz");
        write_zip(&no_map, &[("fixture.wav", b"audio")]);
        assert_eq!(
            code(&inspect_osz(&no_map, ValidationLimits::default())),
            ValidationCode::BeatmapNotFound
        );
    }

    #[test]
    fn enforces_entry_count_single_file_and_total_size_limits() {
        let root = TestRoot::new("limits");
        let beatmap = fixture_map();
        let archive = root.path().join("limits.osz");
        write_zip(&archive, &[("map.osu", &beatmap), ("audio.wav", b"12345")]);

        let entry_limit = ValidationLimits {
            max_archive_entries: 1,
            ..ValidationLimits::default()
        };
        assert_eq!(
            code(&inspect_osz(&archive, entry_limit)),
            ValidationCode::ArchiveLimitExceeded
        );

        let file_limit = ValidationLimits {
            max_single_entry_bytes: 4,
            ..ValidationLimits::default()
        };
        assert_eq!(
            code(&inspect_osz(&archive, file_limit)),
            ValidationCode::ArchiveLimitExceeded
        );

        let total_limit = ValidationLimits {
            max_archive_uncompressed_bytes: beatmap.len() as u64,
            ..ValidationLimits::default()
        };
        assert_eq!(
            code(&inspect_osz(&archive, total_limit)),
            ValidationCode::ArchiveLimitExceeded
        );
    }

    #[test]
    fn reports_missing_audio_and_event_assets_after_safe_extraction() {
        let root = TestRoot::new("references");
        let archive = root.path().join("missing.osz");
        let source = String::from_utf8(fixture_map()).unwrap().replace(
            "//Background and Video events",
            "0,0,\"missing-background.jpg\",0,0",
        );
        write_zip(&archive, &[("map.osu", source.as_bytes())]);
        let result = extract_osz(
            &archive,
            &root.path().join("extracted"),
            ValidationLimits::default(),
        );
        assert!(result.value.is_some());
        let codes = result
            .issues
            .iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>();
        assert!(codes.contains(&ValidationCode::AudioMissing));
        assert!(codes.contains(&ValidationCode::ReferencedAssetMissing));
    }

    #[test]
    fn packages_reopens_and_reextracts_a_valid_directory_without_overwriting() {
        let root = TestRoot::new("package");
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("map.osu"), fixture_map()).unwrap();
        fs::write(source.join("fixture.wav"), b"synthetic fixture").unwrap();
        let output = root.path().join("result.osz");

        let packaged = package_osz_from_directory(&source, &output, ValidationLimits::default());
        assert!(!packaged.has_errors(), "{:?}", packaged.issues);
        assert!(output.is_file());

        let extracted = extract_osz(
            &output,
            &root.path().join("reopened"),
            ValidationLimits::default(),
        );
        assert!(!extracted.has_errors(), "{:?}", extracted.issues);

        let second = package_osz_from_directory(&source, &output, ValidationLimits::default());
        assert_eq!(code(&second), ValidationCode::ArchiveInvalid);
    }

    #[test]
    fn packaging_rejects_missing_references_and_removes_the_candidate_output() {
        let root = TestRoot::new("package-invalid");
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("map.osu"), fixture_map()).unwrap();
        let output = root.path().join("result.osz");

        let packaged = package_osz_from_directory(&source, &output, ValidationLimits::default());
        assert!(packaged.has_errors());
        assert_eq!(code(&packaged), ValidationCode::AudioMissing);
        assert!(!output.exists());
        assert!(!root.path().join(".result.osz.verify").exists());
        assert!(!root.path().join(".result.osz.partial").exists());
    }
}
