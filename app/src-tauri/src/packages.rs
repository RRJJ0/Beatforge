use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const PYTHON_RUNTIME_ID: &str = "python-3.10-windows-x64";
pub const STANDARD_MODEL_ID: &str = "mapperatorinator-v32-standard";

pub fn builtin_manifest() -> PackageManifest {
    PackageManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        packages: vec![
            PackageSpec {
                id: PYTHON_RUNTIME_ID.into(),
                kind: PackageKind::Runtime,
                name: "Python Runtime".into(),
                description: "Private Python 3.10 runtime for Mapperatorinator".into(),
                version: "3.10.21".into(),
                artifact: PackageArtifact {
                    url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260929/cpython-3.10.21%2B20260929-x86_64-pc-windows-msvc-install_only.tar.gz".into(),
                    size_bytes: 39_567_674,
                    sha256: "806de086b36a83ad2863a9d6c2945de105f707fded114e248e8974a2dc64b7eb".into(),
                    file_name: "cpython-3.10.21+20260929-x86_64-pc-windows-msvc-install_only.tar.gz".into(),
                },
                additional_artifacts: vec![],
                requirements: vec![],
                recommended: true,
            },
            PackageSpec {
                id: STANDARD_MODEL_ID.into(),
                kind: PackageKind::Model,
                name: "Mapperatorinator V32 — osu!standard".into(),
                description: "Official V32 checkpoint for generating osu!standard beatmaps".into(),
                version: "32.74f22583".into(),
                artifact: PackageArtifact {
                    url: "https://huggingface.co/OliBomby/Mapperatorinator-v32/resolve/74f22583400d259bf424819e11027c17933efe54/gamemode%3D0/model.safetensors".into(),
                    size_bytes: 865_544_236,
                    sha256: "dc60cf15609a1e93bf2280b360f00ba30b833c8e95d088db21babe44e1c4d6e9".into(),
                    file_name: "model.safetensors".into(),
                },
                additional_artifacts: vec![
                    PackageArtifact {
                        url: "https://huggingface.co/OliBomby/Mapperatorinator-v32/resolve/74f22583400d259bf424819e11027c17933efe54/gamemode%3D0/config.json".into(),
                        size_bytes: 2_972,
                        sha256: "ff96b8c059179978c93b6f938e39fac945d5682a3495789c00cc159d571a1a22".into(),
                        file_name: "config.json".into(),
                    },
                    PackageArtifact {
                        url: "https://huggingface.co/OliBomby/Mapperatorinator-v32/resolve/74f22583400d259bf424819e11027c17933efe54/gamemode%3D0/generation_config.json".into(),
                        size_bytes: 259,
                        sha256: "10b901beeb3c982c16c53cd5b37a4f9b3d4674fc70cbc3e64c7b20fdb0433f51".into(),
                        file_name: "generation_config.json".into(),
                    },
                    PackageArtifact {
                        url: "https://huggingface.co/OliBomby/Mapperatorinator-v32/resolve/74f22583400d259bf424819e11027c17933efe54/gamemode%3D0/tokenizer.json".into(),
                        size_bytes: 4_974_127,
                        sha256: "6b98be0fc04a95a9e9d4feb8e8b67cc48728a6667e3091dcd5cc528baeca18bd".into(),
                        file_name: "tokenizer.json".into(),
                    },
                ],
                requirements: vec![PackageRequirement {
                    package_id: PYTHON_RUNTIME_ID.into(),
                    version: "3.10.21".into(),
                }],
                recommended: true,
            },
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    Runtime,
    Model,
}

impl PackageKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Model => "model",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatus {
    NotInstalled,
    Queued,
    Downloading,
    Verifying,
    Downloaded,
    Installing,
    Installed,
    UpdateAvailable,
    Removing,
    Failed,
}

impl PackageStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotInstalled => "not_installed",
            Self::Queued => "queued",
            Self::Downloading => "downloading",
            Self::Verifying => "verifying",
            Self::Downloaded => "downloaded",
            Self::Installing => "installing",
            Self::Installed => "installed",
            Self::UpdateAvailable => "update_available",
            Self::Removing => "removing",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "not_installed" => Ok(Self::NotInstalled),
            "queued" => Ok(Self::Queued),
            "downloading" => Ok(Self::Downloading),
            "verifying" => Ok(Self::Verifying),
            "downloaded" => Ok(Self::Downloaded),
            "installing" => Ok(Self::Installing),
            "installed" => Ok(Self::Installed),
            "update_available" => Ok(Self::UpdateAvailable),
            "removing" => Ok(Self::Removing),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Unknown package status: {value}")),
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use PackageStatus::*;
        matches!(
            (self, next),
            (NotInstalled | Failed | UpdateAvailable, Queued)
                | (Queued, Downloading | Failed | NotInstalled)
                | (Downloading, Verifying | Failed | NotInstalled)
                | (Verifying, Downloaded | Failed | NotInstalled)
                | (Downloaded, Installing | Queued | Removing | Failed)
                | (Installing, Installed | Failed | NotInstalled)
                | (Installed, UpdateAvailable | Removing | Failed)
                | (UpdateAvailable, Removing | Installed)
                | (Removing, NotInstalled | Failed)
                | (Failed, NotInstalled | Removing)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageArtifact {
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageRequirement {
    pub package_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageSpec {
    pub id: String,
    pub kind: PackageKind,
    pub name: String,
    pub description: String,
    pub version: String,
    pub artifact: PackageArtifact,
    #[serde(default)]
    pub additional_artifacts: Vec<PackageArtifact>,
    #[serde(default)]
    pub requirements: Vec<PackageRequirement>,
    #[serde(default)]
    pub recommended: bool,
}

impl PackageSpec {
    pub fn artifacts(&self) -> impl Iterator<Item = &PackageArtifact> {
        std::iter::once(&self.artifact).chain(self.additional_artifacts.iter())
    }

    pub fn total_size(&self) -> u64 {
        self.artifacts().fold(0_u64, |total, artifact| {
            total.saturating_add(artifact.size_bytes)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub packages: Vec<PackageSpec>,
}

impl PackageManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported manifest schema version: {}",
                self.schema_version
            ));
        }
        if self.packages.is_empty() {
            return Err("Manifest must contain at least one package".into());
        }

        let mut ids = HashSet::new();
        for package in &self.packages {
            validate_package(package)?;
            if !ids.insert(package.id.as_str()) {
                return Err(format!("Duplicate package id: {}", package.id));
            }
        }
        for package in &self.packages {
            for requirement in &package.requirements {
                if !ids.contains(requirement.package_id.as_str()) {
                    return Err(format!(
                        "Package {} requires unknown package {}",
                        package.id, requirement.package_id
                    ));
                }
                if requirement.package_id == package.id {
                    return Err(format!("Package {} cannot require itself", package.id));
                }
            }
        }
        Ok(())
    }
}

fn validate_package(package: &PackageSpec) -> Result<(), String> {
    let id_valid = !package.id.is_empty()
        && package.id.len() <= 80
        && package.id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
        });
    if !id_valid {
        return Err(format!("Invalid package id: {}", package.id));
    }
    if package.name.trim().is_empty() || package.name.len() > 120 {
        return Err(format!("Invalid name for package {}", package.id));
    }
    let version_valid = !package.version.is_empty()
        && package.version.len() <= 64
        && package.version.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || byte == b'-'
                || byte == b'_'
                || byte == b'.'
                || byte == b'+'
        });
    if !version_valid {
        return Err(format!("Invalid version for package {}", package.id));
    }
    let mut file_names = HashSet::new();
    for artifact in package.artifacts() {
        if !artifact.url.starts_with("https://") {
            return Err(format!("Package {} must use an HTTPS URL", package.id));
        }
        if artifact.size_bytes == 0 {
            return Err(format!("Package {} has an empty artifact", package.id));
        }
        if artifact.file_name.is_empty()
            || artifact.file_name.contains('/')
            || artifact.file_name.contains('\\')
            || matches!(artifact.file_name.as_str(), "." | "..")
        {
            return Err(format!("Package {} has an unsafe file name", package.id));
        }
        if !file_names.insert(artifact.file_name.as_str()) {
            return Err(format!(
                "Package {} has duplicate artifact names",
                package.id
            ));
        }
        if artifact.sha256.len() != 64
            || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!("Package {} has an invalid SHA-256", package.id));
        }
    }
    for requirement in &package.requirements {
        if requirement.version.trim().is_empty() || requirement.version.len() > 64 {
            return Err(format!(
                "Package {} has an invalid dependency version",
                package.id
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPackage {
    pub id: String,
    pub kind: PackageKind,
    pub name: String,
    pub description: String,
    pub target_version: String,
    pub installed_version: Option<String>,
    pub status: PackageStatus,
    pub progress: u8,
    pub bytes_downloaded: u64,
    pub size_bytes: u64,
    pub artifact_path: Option<String>,
    pub install_path: Option<String>,
    pub error_message: Option<String>,
    pub recommended: bool,
    pub updated_at: String,
}

pub fn migrate(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS package_installations (
               package_id TEXT PRIMARY KEY,
               kind TEXT NOT NULL CHECK (kind IN ('runtime', 'model')),
               name TEXT NOT NULL,
               description TEXT NOT NULL,
               target_version TEXT NOT NULL,
               installed_version TEXT,
               status TEXT NOT NULL DEFAULT 'not_installed',
               progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
               bytes_downloaded INTEGER NOT NULL DEFAULT 0 CHECK (bytes_downloaded >= 0),
               size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
               artifact_path TEXT,
               install_path TEXT,
               error_message TEXT,
               recommended INTEGER NOT NULL DEFAULT 0,
               manifest_json TEXT NOT NULL,
               updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             INSERT OR IGNORE INTO schema_migrations(version) VALUES (3);",
        )
        .map_err(|error| error.to_string())?;
    let has_artifact_path = {
        let mut statement = connection
            .prepare("PRAGMA table_info(package_installations)")
            .map_err(|error| error.to_string())?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| error.to_string())?;
        columns
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
            .iter()
            .any(|column| column == "artifact_path")
    };
    if !has_artifact_path {
        connection
            .execute(
                "ALTER TABLE package_installations ADD COLUMN artifact_path TEXT",
                [],
            )
            .map_err(|error| error.to_string())?;
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (4)",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn sync_manifest(connection: &Connection, manifest: &PackageManifest) -> Result<(), String> {
    manifest.validate()?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    for package in &manifest.packages {
        let manifest_json = serde_json::to_string(package).map_err(|error| error.to_string())?;
        let size_bytes = i64::try_from(package.total_size())
            .map_err(|_| format!("Package {} is too large", package.id))?;
        transaction
            .execute(
                "INSERT INTO package_installations(
                   package_id, kind, name, description, target_version, size_bytes, recommended, manifest_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(package_id) DO UPDATE SET
                   kind = excluded.kind,
                   name = excluded.name,
                   description = excluded.description,
                   target_version = excluded.target_version,
                   size_bytes = excluded.size_bytes,
                   recommended = excluded.recommended,
                   manifest_json = excluded.manifest_json,
                   status = CASE
                     WHEN package_installations.status = 'installed'
                       AND package_installations.installed_version IS NOT excluded.target_version
                     THEN 'update_available'
                     WHEN package_installations.target_version <> excluded.target_version
                       AND package_installations.status <> 'installed'
                     THEN 'not_installed'
                     ELSE package_installations.status
                   END,
                   progress = CASE WHEN package_installations.target_version <> excluded.target_version
                     AND package_installations.status <> 'installed' THEN 0 ELSE package_installations.progress END,
                   bytes_downloaded = CASE WHEN package_installations.target_version <> excluded.target_version
                     AND package_installations.status <> 'installed' THEN 0 ELSE package_installations.bytes_downloaded END,
                   artifact_path = CASE WHEN package_installations.target_version <> excluded.target_version
                     THEN NULL ELSE package_installations.artifact_path END,
                   error_message = CASE WHEN package_installations.target_version <> excluded.target_version
                     THEN NULL ELSE package_installations.error_message END,
                   updated_at = CURRENT_TIMESTAMP",
                params![
                    package.id,
                    package.kind.as_str(),
                    package.name,
                    package.description,
                    package.version,
                    size_bytes,
                    package.recommended,
                    manifest_json
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ManagedPackage> {
    let kind: String = row.get(1)?;
    let status: String = row.get(6)?;
    Ok(ManagedPackage {
        id: row.get(0)?,
        kind: match kind.as_str() {
            "runtime" => PackageKind::Runtime,
            "model" => PackageKind::Model,
            _ => return Err(rusqlite::Error::InvalidQuery),
        },
        name: row.get(2)?,
        description: row.get(3)?,
        target_version: row.get(4)?,
        installed_version: row.get(5)?,
        status: PackageStatus::parse(&status).map_err(|_| rusqlite::Error::InvalidQuery)?,
        progress: row.get(7)?,
        bytes_downloaded: row.get(8)?,
        size_bytes: row.get(9)?,
        artifact_path: row.get(10)?,
        install_path: row.get(11)?,
        error_message: row.get(12)?,
        recommended: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

const SELECT_PACKAGE: &str = "SELECT package_id, kind, name, description, target_version,
  installed_version, status, progress, bytes_downloaded, size_bytes, artifact_path, install_path,
  error_message, recommended, updated_at FROM package_installations";

pub fn list(connection: &Connection) -> Result<Vec<ManagedPackage>, String> {
    let mut statement = connection
        .prepare(&format!(
            "{SELECT_PACKAGE} ORDER BY kind DESC, recommended DESC, name"
        ))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get(connection: &Connection, package_id: &str) -> Result<Option<ManagedPackage>, String> {
    connection
        .query_row(
            &format!("{SELECT_PACKAGE} WHERE package_id = ?1"),
            params![package_id],
            from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_spec(connection: &Connection, package_id: &str) -> Result<PackageSpec, String> {
    let json: Option<String> = connection
        .query_row(
            "SELECT manifest_json FROM package_installations WHERE package_id = ?1",
            params![package_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let json = json.ok_or_else(|| format!("Unknown package: {package_id}"))?;
    let package: PackageSpec = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_package(&package)?;
    Ok(package)
}

pub fn set_artifact_path(
    connection: &Connection,
    package_id: &str,
    artifact_path: &str,
) -> Result<(), String> {
    connection
        .execute(
            "UPDATE package_installations SET artifact_path = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE package_id = ?1",
            params![package_id, artifact_path],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn finish_removal(connection: &Connection, package_id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE package_installations SET status = 'not_installed', progress = 0,
             bytes_downloaded = 0, installed_version = NULL, artifact_path = NULL,
             install_path = NULL, error_message = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE package_id = ?1",
            params![package_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn mark_interrupted(connection: &Connection) -> Result<(), String> {
    connection
        .execute(
            "UPDATE package_installations SET status = 'failed',
             error_message = 'The previous package operation was interrupted',
             updated_at = CURRENT_TIMESTAMP
             WHERE status IN ('queued', 'downloading', 'verifying', 'installing', 'removing')",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn transition(
    connection: &Connection,
    package_id: &str,
    next: PackageStatus,
    progress: u8,
    bytes_downloaded: u64,
    installed_version: Option<&str>,
    install_path: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    if progress > 100 {
        return Err("Package progress cannot exceed 100".into());
    }
    let current: Option<String> = connection
        .query_row(
            "SELECT status FROM package_installations WHERE package_id = ?1",
            params![package_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let current = current.ok_or_else(|| format!("Unknown package: {package_id}"))?;
    let current = PackageStatus::parse(&current)?;
    if current != next && !current.can_transition_to(next) {
        return Err(format!(
            "Invalid package state transition: {} -> {}",
            current.as_str(),
            next.as_str()
        ));
    }
    let bytes_downloaded = i64::try_from(bytes_downloaded)
        .map_err(|_| "Downloaded byte count is too large".to_string())?;
    connection
        .execute(
            "UPDATE package_installations SET status = ?2, progress = ?3, bytes_downloaded = ?4,
             installed_version = COALESCE(?5, installed_version), install_path = COALESCE(?6, install_path),
             error_message = ?7, updated_at = CURRENT_TIMESTAMP WHERE package_id = ?1",
            params![package_id, next.as_str(), progress, bytes_downloaded, installed_version, install_path, error_message],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY);")
            .unwrap();
        migrate(&connection).unwrap();
        connection
    }

    fn package(version: &str) -> PackageSpec {
        PackageSpec {
            id: "mapperatorinator-standard".into(),
            kind: PackageKind::Model,
            name: "Mapperatorinator Standard".into(),
            description: "Default osu!standard model".into(),
            version: version.into(),
            artifact: PackageArtifact {
                url: "https://downloads.example.test/model.zip".into(),
                size_bytes: 1024,
                sha256: "a".repeat(64),
                file_name: "model.zip".into(),
            },
            additional_artifacts: vec![],
            requirements: vec![],
            recommended: true,
        }
    }

    fn manifest(version: &str) -> PackageManifest {
        PackageManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            packages: vec![package(version)],
        }
    }

    #[test]
    fn builtin_manifest_contains_pinned_standard_model() {
        let manifest = builtin_manifest();
        manifest.validate().unwrap();
        let model = manifest
            .packages
            .iter()
            .find(|package| package.id == STANDARD_MODEL_ID)
            .unwrap();
        assert_eq!(model.version, "32.74f22583");
        assert_eq!(model.artifacts().count(), 4);
        assert_eq!(model.total_size(), 870_521_594);
        assert!(model.artifacts().all(|artifact| {
            artifact
                .url
                .contains("74f22583400d259bf424819e11027c17933efe54")
        }));
    }

    #[test]
    fn rejects_unsafe_manifest_values() {
        let mut invalid = manifest("1.0.0");
        invalid.packages[0].artifact.file_name = "../model.zip".into();
        assert!(invalid.validate().is_err());
        invalid.packages[0].artifact.file_name = "model.zip".into();
        invalid.packages[0].artifact.url = "http://example.test/model.zip".into();
        assert!(invalid.validate().is_err());
        invalid.packages[0].artifact.url = "https://example.test/model.zip".into();
        invalid.packages[0].version = "../outside".into();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn syncs_manifest_without_losing_install_state() {
        let connection = database();
        sync_manifest(&connection, &manifest("1.0.0")).unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Queued,
            0,
            0,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Downloading,
            50,
            512,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Verifying,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Downloaded,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Installing,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Installed,
            100,
            1024,
            Some("1.0.0"),
            Some("C:\\models\\standard"),
            None,
        )
        .unwrap();

        sync_manifest(&connection, &manifest("1.1.0")).unwrap();
        let saved = list(&connection).unwrap().pop().unwrap();
        assert_eq!(saved.status, PackageStatus::UpdateAvailable);
        assert_eq!(saved.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(saved.target_version, "1.1.0");
    }

    #[test]
    fn rejects_invalid_state_transitions() {
        let connection = database();
        sync_manifest(&connection, &manifest("1.0.0")).unwrap();
        let error = transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Installed,
            100,
            1024,
            Some("1.0.0"),
            None,
            None,
        )
        .unwrap_err();
        assert!(error.contains("not_installed -> installed"));
    }

    #[test]
    fn a_new_manifest_version_invalidates_an_old_download() {
        let connection = database();
        sync_manifest(&connection, &manifest("1.0.0")).unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Queued,
            0,
            0,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Downloading,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Verifying,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        transition(
            &connection,
            "mapperatorinator-standard",
            PackageStatus::Downloaded,
            100,
            1024,
            None,
            None,
            None,
        )
        .unwrap();
        set_artifact_path(
            &connection,
            "mapperatorinator-standard",
            "C:\\cache\\model.zip",
        )
        .unwrap();

        sync_manifest(&connection, &manifest("1.1.0")).unwrap();
        let saved = get(&connection, "mapperatorinator-standard")
            .unwrap()
            .unwrap();
        assert_eq!(saved.status, PackageStatus::NotInstalled);
        assert_eq!(saved.progress, 0);
        assert_eq!(saved.bytes_downloaded, 0);
        assert!(saved.artifact_path.is_none());
    }
}
