use crate::AppPaths;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const OLD_LOG_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageCategory {
    Runtime,
    Models,
    Cache,
    Logs,
    JobsRaw,
    JobsWorking,
    JobsValidated,
}

impl StorageCategory {
    fn cleanable(self) -> bool {
        !matches!(self, Self::Runtime | Self::Models)
    }

    fn protected(self) -> bool {
        matches!(self, Self::JobsRaw | Self::JobsValidated)
    }

    fn default_selected(self) -> bool {
        matches!(self, Self::Cache | Self::Logs | Self::JobsWorking)
    }

    fn job_directory(self) -> Option<&'static str> {
        match self {
            Self::JobsRaw => Some("raw"),
            Self::JobsWorking => Some("working"),
            Self::JobsValidated => Some("validated"),
            _ => None,
        }
    }
}

const ALL_CATEGORIES: [StorageCategory; 7] = [
    StorageCategory::Runtime,
    StorageCategory::Models,
    StorageCategory::Cache,
    StorageCategory::Logs,
    StorageCategory::JobsRaw,
    StorageCategory::JobsWorking,
    StorageCategory::JobsValidated,
];

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanSummary {
    bytes: u64,
    files: u64,
    directories: u64,
    latest_modified_ms: Option<u64>,
    rejected_items: u64,
}

impl ScanSummary {
    fn merge(&mut self, other: &Self) {
        self.bytes = self.bytes.saturating_add(other.bytes);
        self.files = self.files.saturating_add(other.files);
        self.directories = self.directories.saturating_add(other.directories);
        self.latest_modified_ms = self.latest_modified_ms.max(other.latest_modified_ms);
        self.rejected_items = self.rejected_items.saturating_add(other.rejected_items);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageCategory {
    category: StorageCategory,
    bytes: u64,
    files: u64,
    directories: u64,
    latest_modified_ms: Option<u64>,
    cleanable: bool,
    protected: bool,
    default_selected: bool,
    eligible_bytes: u64,
    eligible_files: u64,
    eligible_directories: u64,
    rejected_items: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageReport {
    scanned_at_ms: u64,
    total_bytes: u64,
    active_jobs: Vec<String>,
    categories: Vec<StorageUsageCategory>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupPreviewItem {
    category: StorageCategory,
    bytes: u64,
    files: u64,
    directories: u64,
    targets: u64,
    protected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupPreview {
    categories: Vec<StorageCleanupPreviewItem>,
    total_bytes: u64,
    total_files: u64,
    total_directories: u64,
    protected_selected: bool,
    active_jobs: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupFailure {
    category: StorageCategory,
    target: String,
    error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanupResult {
    freed_bytes: u64,
    removed_files: u64,
    removed_directories: u64,
    failures: Vec<StorageCleanupFailure>,
    usage: StorageUsageReport,
}

#[derive(Debug, Clone)]
struct CleanupCandidate {
    category: StorageCategory,
    managed_root: PathBuf,
    target: PathBuf,
    summary: ScanSummary,
}

fn modified_ms(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn scan_path(path: &Path) -> Result<ScanSummary, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ScanSummary::default());
        }
        Err(error) => return Err(format!("Could not inspect {}: {error}", path.display())),
    };
    if is_link_or_reparse(&metadata) {
        return Ok(ScanSummary {
            rejected_items: 1,
            ..ScanSummary::default()
        });
    }
    if metadata.is_file() {
        return Ok(ScanSummary {
            bytes: metadata.len(),
            files: 1,
            latest_modified_ms: modified_ms(&metadata),
            ..ScanSummary::default()
        });
    }
    if !metadata.is_dir() {
        return Ok(ScanSummary {
            rejected_items: 1,
            ..ScanSummary::default()
        });
    }

    let mut summary = ScanSummary {
        directories: 1,
        latest_modified_ms: modified_ms(&metadata),
        ..ScanSummary::default()
    };
    let mut pending = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("Could not read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "Could not inspect an entry in {}: {error}",
                    directory.display()
                )
            })?;
            let metadata = entry.metadata().map_err(|error| {
                format!("Could not inspect {}: {error}", entry.path().display())
            })?;
            let file_type = entry.file_type().map_err(|error| {
                format!("Could not inspect {}: {error}", entry.path().display())
            })?;
            if file_type.is_symlink() || is_link_or_reparse(&metadata) {
                summary.rejected_items += 1;
            } else if file_type.is_file() {
                summary.bytes = summary.bytes.saturating_add(metadata.len());
                summary.files += 1;
                summary.latest_modified_ms = summary.latest_modified_ms.max(modified_ms(&metadata));
            } else if file_type.is_dir() {
                summary.directories += 1;
                summary.latest_modified_ms = summary.latest_modified_ms.max(modified_ms(&metadata));
                pending.push(entry.path());
            } else {
                summary.rejected_items += 1;
            }
        }
    }
    Ok(summary)
}

fn category_root<'a>(paths: &'a AppPaths, category: StorageCategory) -> &'a Path {
    match category {
        StorageCategory::Runtime => Path::new(&paths.runtime),
        StorageCategory::Models => Path::new(&paths.models),
        StorageCategory::Cache => Path::new(&paths.cache),
        StorageCategory::Logs => Path::new(&paths.logs),
        StorageCategory::JobsRaw
        | StorageCategory::JobsWorking
        | StorageCategory::JobsValidated => Path::new(&paths.jobs),
    }
}

fn safe_direct_children(root: &Path, warnings: &mut Vec<String>) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("Could not read {}: {error}", root.display()));
            return targets;
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => match (entry.file_type(), entry.metadata()) {
                (Ok(kind), Ok(metadata))
                    if !kind.is_symlink()
                        && !is_link_or_reparse(&metadata)
                        && (kind.is_file() || kind.is_dir()) =>
                {
                    targets.push(entry.path())
                }
                (Ok(_), Ok(_)) => warnings.push(format!(
                    "Ignored non-regular or linked item {}",
                    entry.path().display()
                )),
                (Err(error), _) | (_, Err(error)) => warnings.push(format!(
                    "Could not inspect {}: {error}",
                    entry.path().display()
                )),
            },
            Err(error) => warnings.push(format!("Could not inspect an entry: {error}")),
        }
    }
    targets
}

fn job_category_targets(
    paths: &AppPaths,
    category: StorageCategory,
    active_jobs: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Vec<PathBuf> {
    let jobs_root = Path::new(&paths.jobs);
    let Some(directory_name) = category.job_directory() else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    for job_root in safe_direct_children(jobs_root, warnings) {
        let metadata = match fs::symlink_metadata(&job_root) {
            Ok(metadata) if metadata.is_dir() && !is_link_or_reparse(&metadata) => metadata,
            Ok(_) => continue,
            Err(error) => {
                warnings.push(format!("Could not inspect {}: {error}", job_root.display()));
                continue;
            }
        };
        let _ = metadata;
        let Some(job_id) = job_root.file_name().and_then(|name| name.to_str()) else {
            warnings.push(format!(
                "Ignored job folder with an invalid name: {}",
                job_root.display()
            ));
            continue;
        };
        if active_jobs.contains(job_id) {
            continue;
        }
        let target = job_root.join(directory_name);
        if target.exists() {
            targets.push(target);
        }
    }
    targets
}

fn candidate_targets(
    paths: &AppPaths,
    category: StorageCategory,
    active_jobs: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Vec<PathBuf> {
    match category {
        StorageCategory::Cache => safe_direct_children(Path::new(&paths.cache), warnings),
        StorageCategory::Logs => {
            let cutoff = SystemTime::now().checked_sub(OLD_LOG_AGE);
            safe_direct_children(Path::new(&paths.logs), warnings)
                .into_iter()
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("log"))
                        && path.is_file()
                        && cutoff.is_some_and(|cutoff| {
                            fs::metadata(path)
                                .and_then(|metadata| metadata.modified())
                                .is_ok_and(|modified| modified <= cutoff)
                        })
                })
                .collect()
        }
        StorageCategory::JobsRaw
        | StorageCategory::JobsWorking
        | StorageCategory::JobsValidated => {
            job_category_targets(paths, category, active_jobs, warnings)
        }
        StorageCategory::Runtime | StorageCategory::Models => Vec::new(),
    }
}

fn cleanup_candidates(
    paths: &AppPaths,
    categories: &[StorageCategory],
    active_jobs: &HashSet<String>,
) -> (Vec<CleanupCandidate>, Vec<String>) {
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    for &category in categories {
        if !category.cleanable() || !seen.insert(category) {
            continue;
        }
        let managed_root = category_root(paths, category).to_path_buf();
        for target in candidate_targets(paths, category, active_jobs, &mut warnings) {
            match scan_path(&target) {
                Ok(summary) if summary.rejected_items == 0 => candidates.push(CleanupCandidate {
                    category,
                    managed_root: managed_root.clone(),
                    target,
                    summary,
                }),
                Ok(_) => warnings.push(format!(
                    "Refused {} because it contains a symlink or non-regular item",
                    target.display()
                )),
                Err(error) => warnings.push(error),
            }
        }
    }
    (candidates, warnings)
}

pub fn usage(paths: &AppPaths, active_job_ids: &[String]) -> Result<StorageUsageReport, String> {
    let active_jobs = active_job_ids.iter().cloned().collect::<HashSet<_>>();
    let mut warnings = Vec::new();
    let mut categories = Vec::new();
    let mut total_bytes = 0_u64;
    for category in ALL_CATEGORIES {
        let summary = if let Some(directory_name) = category.job_directory() {
            let mut total = ScanSummary::default();
            for job_root in safe_direct_children(Path::new(&paths.jobs), &mut warnings) {
                if job_root.is_dir() {
                    total.merge(&scan_path(&job_root.join(directory_name))?);
                }
            }
            total
        } else {
            scan_path(category_root(paths, category))?
        };
        let selected = [category];
        let (candidates, candidate_warnings) = cleanup_candidates(paths, &selected, &active_jobs);
        warnings.extend(candidate_warnings);
        let mut eligible = ScanSummary::default();
        for candidate in candidates {
            eligible.merge(&candidate.summary);
        }
        total_bytes = total_bytes.saturating_add(summary.bytes);
        categories.push(StorageUsageCategory {
            category,
            bytes: summary.bytes,
            files: summary.files,
            directories: summary.directories,
            latest_modified_ms: summary.latest_modified_ms,
            cleanable: category.cleanable(),
            protected: category.protected(),
            default_selected: category.default_selected(),
            eligible_bytes: eligible.bytes,
            eligible_files: eligible.files,
            eligible_directories: eligible.directories,
            rejected_items: summary.rejected_items,
        });
    }
    let mut active_jobs = active_job_ids.to_vec();
    active_jobs.sort();
    warnings.sort();
    warnings.dedup();
    Ok(StorageUsageReport {
        scanned_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        total_bytes,
        active_jobs,
        categories,
        warnings,
    })
}

pub fn preview_cleanup(
    paths: &AppPaths,
    active_job_ids: &[String],
    categories: &[StorageCategory],
) -> Result<StorageCleanupPreview, String> {
    if categories.is_empty() {
        return Err("Select at least one storage category".into());
    }
    if categories.iter().any(|category| !category.cleanable()) {
        return Err("Runtime and model storage must be managed from the Models screen".into());
    }
    let active_set = active_job_ids.iter().cloned().collect::<HashSet<_>>();
    let (candidates, mut warnings) = cleanup_candidates(paths, categories, &active_set);
    if !active_job_ids.is_empty() {
        warnings.push("Active job folders are excluded from cleanup".into());
    }
    let mut items = Vec::new();
    for &category in categories {
        if items
            .iter()
            .any(|item: &StorageCleanupPreviewItem| item.category == category)
        {
            continue;
        }
        let selected = candidates
            .iter()
            .filter(|candidate| candidate.category == category)
            .collect::<Vec<_>>();
        items.push(StorageCleanupPreviewItem {
            category,
            bytes: selected
                .iter()
                .map(|candidate| candidate.summary.bytes)
                .sum(),
            files: selected
                .iter()
                .map(|candidate| candidate.summary.files)
                .sum(),
            directories: selected
                .iter()
                .map(|candidate| candidate.summary.directories)
                .sum(),
            targets: selected.len() as u64,
            protected: category.protected(),
        });
    }
    warnings.sort();
    warnings.dedup();
    let total_bytes = items.iter().map(|item| item.bytes).sum();
    let total_files = items.iter().map(|item| item.files).sum();
    let total_directories = items.iter().map(|item| item.directories).sum();
    let protected_selected = items.iter().any(|item| item.protected);
    let mut active_jobs = active_job_ids.to_vec();
    active_jobs.sort();
    Ok(StorageCleanupPreview {
        categories: items,
        total_bytes,
        total_files,
        total_directories,
        protected_selected,
        active_jobs,
        warnings,
    })
}

fn remove_candidate(candidate: &CleanupCandidate) -> Result<(), String> {
    let root = candidate
        .managed_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve managed root: {error}"))?;
    let metadata = fs::symlink_metadata(&candidate.target)
        .map_err(|error| format!("Could not inspect cleanup target: {error}"))?;
    if is_link_or_reparse(&metadata) || (!metadata.is_file() && !metadata.is_dir()) {
        return Err("Refusing to remove a linked or non-regular target".into());
    }
    let resolved = candidate
        .target
        .canonicalize()
        .map_err(|error| format!("Could not resolve cleanup target: {error}"))?;
    if resolved == root || !resolved.starts_with(&root) {
        return Err("Refusing to remove a target outside its managed root".into());
    }
    let checked = scan_path(&resolved)?;
    if checked.rejected_items != 0 {
        return Err("Refusing to remove a target containing a symlink or non-regular item".into());
    }
    if metadata.is_dir() {
        fs::remove_dir_all(&resolved)
            .map_err(|error| format!("Could not remove managed directory: {error}"))
    } else {
        fs::remove_file(&resolved)
            .map_err(|error| format!("Could not remove managed file: {error}"))
    }
}

pub fn cleanup(
    paths: &AppPaths,
    active_job_ids: &[String],
    categories: &[StorageCategory],
    allow_protected: bool,
) -> Result<StorageCleanupResult, String> {
    let preview = preview_cleanup(paths, active_job_ids, categories)?;
    if preview.protected_selected && !allow_protected {
        return Err("Cleaning raw or validated results requires explicit confirmation".into());
    }
    let active_set = active_job_ids.iter().cloned().collect::<HashSet<_>>();
    let (candidates, _) = cleanup_candidates(paths, categories, &active_set);
    let mut freed_bytes = 0_u64;
    let mut removed_files = 0_u64;
    let mut removed_directories = 0_u64;
    let mut failures = Vec::new();
    for candidate in candidates {
        match remove_candidate(&candidate) {
            Ok(()) => {
                freed_bytes = freed_bytes.saturating_add(candidate.summary.bytes);
                removed_files = removed_files.saturating_add(candidate.summary.files);
                removed_directories =
                    removed_directories.saturating_add(candidate.summary.directories);
            }
            Err(error) => failures.push(StorageCleanupFailure {
                category: candidate.category,
                target: candidate.target.to_string_lossy().into_owned(),
                error,
            }),
        }
    }
    let usage = usage(paths, active_job_ids)?;
    Ok(StorageCleanupResult {
        freed_bytes,
        removed_files,
        removed_directories,
        failures,
        usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, FileTimes};
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (PathBuf, AppPaths) {
        let root = std::env::temp_dir().join(format!(
            "beatforge-storage-{}-{}",
            std::process::id(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let paths = AppPaths {
            root: root.to_string_lossy().into_owned(),
            models: root.join("models").to_string_lossy().into_owned(),
            runtime: root.join("runtime").to_string_lossy().into_owned(),
            cache: root.join("cache").to_string_lossy().into_owned(),
            logs: root.join("logs").to_string_lossy().into_owned(),
            jobs: root.join("jobs").to_string_lossy().into_owned(),
            outputs: root.join("outputs").to_string_lossy().into_owned(),
            database: root.join("beatforge.db").to_string_lossy().into_owned(),
        };
        for directory in [
            &paths.models,
            &paths.runtime,
            &paths.cache,
            &paths.logs,
            &paths.jobs,
            &paths.outputs,
        ] {
            fs::create_dir_all(directory).unwrap();
        }
        (root, paths)
    }

    #[test]
    fn preview_matches_cleanup_and_preserves_protected_results() {
        let (root, paths) = fixture();
        fs::write(Path::new(&paths.cache).join("archive.bin"), b"12345").unwrap();
        let job = Path::new(&paths.jobs).join("BF-DONE");
        fs::create_dir_all(job.join("raw")).unwrap();
        fs::create_dir_all(job.join("validated")).unwrap();
        fs::write(job.join("raw/source.osz"), b"raw").unwrap();
        fs::write(job.join("validated/result.osz"), b"result").unwrap();

        let preview = preview_cleanup(&paths, &[], &[StorageCategory::Cache]).unwrap();
        assert_eq!(preview.total_bytes, 5);
        assert_eq!(preview.total_files, 1);
        let result = cleanup(&paths, &[], &[StorageCategory::Cache], false).unwrap();
        assert_eq!(result.freed_bytes, preview.total_bytes);
        assert_eq!(result.removed_files, preview.total_files);
        assert!(job.join("raw/source.osz").exists());
        assert!(job.join("validated/result.osz").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn protected_and_active_job_data_require_safe_handling() {
        let (root, paths) = fixture();
        let job = Path::new(&paths.jobs).join("BF-ACTIVE");
        fs::create_dir_all(job.join("validated")).unwrap();
        fs::write(job.join("validated/result.osz"), b"result").unwrap();
        let active = vec!["BF-ACTIVE".to_string()];

        let preview = preview_cleanup(&paths, &active, &[StorageCategory::JobsValidated]).unwrap();
        assert_eq!(preview.total_bytes, 0);
        assert!(!preview.warnings.is_empty());
        assert!(cleanup(&paths, &[], &[StorageCategory::JobsValidated], false).is_err());
        assert!(job.join("validated/result.osz").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn usage_reports_all_managed_categories() {
        let (root, paths) = fixture();
        fs::write(Path::new(&paths.runtime).join("runtime.bin"), b"runtime").unwrap();
        let report = usage(&paths, &[]).unwrap();
        assert_eq!(report.categories.len(), ALL_CATEGORIES.len());
        assert!(
            report
                .categories
                .iter()
                .any(|item| item.category == StorageCategory::Runtime && item.bytes == 7)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_old_regular_logs_are_cleanup_candidates() {
        let (root, paths) = fixture();
        let fresh = Path::new(&paths.logs).join("fresh.log");
        let old = Path::new(&paths.logs).join("old.log");
        fs::write(&fresh, b"fresh").unwrap();
        fs::write(&old, b"old").unwrap();
        File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_times(
                FileTimes::new()
                    .set_modified(SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60)),
            )
            .unwrap();

        let preview = preview_cleanup(&paths, &[], &[StorageCategory::Logs]).unwrap();
        assert_eq!(preview.total_bytes, 3);
        assert_eq!(preview.total_files, 1);
        cleanup(&paths, &[], &[StorageCategory::Logs], false).unwrap();
        assert!(fresh.exists());
        assert!(!old.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn linked_cache_items_are_never_selected() {
        let (root, paths) = fixture();
        let outside = root.join("outside.bin");
        let link = Path::new(&paths.cache).join("linked.bin");
        fs::write(&outside, b"outside").unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_file(&outside, &link).is_ok();
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&outside, &link).is_ok();
        if !linked {
            fs::remove_dir_all(root).unwrap();
            return;
        }

        let preview = preview_cleanup(&paths, &[], &[StorageCategory::Cache]).unwrap();
        assert_eq!(preview.total_bytes, 0);
        assert!(!preview.warnings.is_empty());
        cleanup(&paths, &[], &[StorageCategory::Cache], false).unwrap();
        assert_eq!(fs::read(&outside).unwrap(), b"outside");
        fs::remove_dir_all(root).unwrap();
    }
}
