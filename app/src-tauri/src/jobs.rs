use crate::{
    protocol::StartJobInput,
    validation::{IssueSeverity, ValidationReport, ValidationStatus},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub audio_file: String,
    pub model: String,
    pub difficulty: f64,
    pub status: String,
    pub progress: u8,
    pub stage: String,
    pub error_message: Option<String>,
    pub output_path: Option<String>,
    pub scenario: String,
    pub created_at: String,
    pub provenance: Option<serde_json::Value>,
    pub validation_report: Option<ValidationReport>,
}

pub fn migrate(connection: &Connection) -> Result<(), String> {
    let applied: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 2",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if applied.is_none() {
        connection
            .execute_batch(
                "ALTER TABLE jobs ADD COLUMN title TEXT NOT NULL DEFAULT '';
                 ALTER TABLE jobs ADD COLUMN artist TEXT NOT NULL DEFAULT '';
                 ALTER TABLE jobs ADD COLUMN audio_file TEXT NOT NULL DEFAULT '';
                 ALTER TABLE jobs ADD COLUMN model TEXT NOT NULL DEFAULT '';
                 ALTER TABLE jobs ADD COLUMN difficulty REAL NOT NULL DEFAULT 0;
                 ALTER TABLE jobs ADD COLUMN progress INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE jobs ADD COLUMN stage TEXT NOT NULL DEFAULT '';
                 ALTER TABLE jobs ADD COLUMN error_message TEXT;
                 ALTER TABLE jobs ADD COLUMN output_path TEXT;
                 ALTER TABLE jobs ADD COLUMN scenario TEXT NOT NULL DEFAULT 'success';
                 ALTER TABLE jobs ADD COLUMN worker_pid INTEGER;
                 INSERT INTO schema_migrations(version) VALUES (2);",
            )
            .map_err(|error| error.to_string())?;
    }
    let validation_applied: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 3",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if validation_applied.is_none() {
        connection
            .execute_batch(
                "ALTER TABLE jobs ADD COLUMN validation_report_json TEXT;
                 INSERT INTO schema_migrations(version) VALUES (3);",
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    let provenance = row
        .get::<_, Option<String>>(13)?
        .and_then(|value| serde_json::from_str(&value).ok());
    let validation_report = row
        .get::<_, Option<String>>(14)?
        .and_then(|value| serde_json::from_str(&value).ok());
    Ok(JobRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        audio_file: row.get(3)?,
        model: row.get(4)?,
        difficulty: row.get(5)?,
        status: row.get(6)?,
        progress: row.get(7)?,
        stage: row.get(8)?,
        error_message: row.get(9)?,
        output_path: row.get(10)?,
        scenario: row.get(11)?,
        created_at: row.get(12)?,
        provenance,
        validation_report,
    })
}

const SELECT_JOB: &str = "SELECT id, title, artist, audio_file, model, difficulty, status, progress, stage, error_message, output_path, scenario, created_at, result_json, validation_report_json FROM jobs";

pub fn insert(
    connection: &Connection,
    id: &str,
    input: &StartJobInput,
) -> Result<JobRecord, String> {
    let request_json = serde_json::to_string(input).map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO jobs(id, status, request_json, title, artist, audio_file, model, difficulty, progress, stage, scenario)
             VALUES (?1, 'queued', ?2, ?3, ?4, ?5, ?6, ?7, 0, 'Waiting for worker', ?8)",
            params![id, request_json, input.title, input.artist, input.audio_file, input.model, input.difficulty, input.scenario],
        )
        .map_err(|error| error.to_string())?;
    get(connection, id)?.ok_or_else(|| "Created job could not be loaded".into())
}

pub fn list(connection: &Connection) -> Result<Vec<JobRecord>, String> {
    let mut statement = connection
        .prepare(&format!("{SELECT_JOB} ORDER BY created_at DESC"))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn get(connection: &Connection, id: &str) -> Result<Option<JobRecord>, String> {
    connection
        .query_row(
            &format!("{SELECT_JOB} WHERE id = ?1"),
            params![id],
            from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_input(connection: &Connection, id: &str) -> Result<Option<StartJobInput>, String> {
    let json: Option<String> = connection
        .query_row(
            "SELECT request_json FROM jobs WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    json.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

pub fn active_ids(connection: &Connection) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare("SELECT id FROM jobs WHERE status IN ('running', 'queued')")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn update_state(
    connection: &Connection,
    id: &str,
    status: &str,
    progress: u8,
    stage: &str,
    error_message: Option<&str>,
    output_path: Option<&str>,
    worker_pid: Option<u32>,
) -> Result<(), String> {
    connection
        .execute(
            "UPDATE jobs SET status = ?2, progress = ?3, stage = ?4, error_message = ?5,
             output_path = COALESCE(?6, output_path), worker_pid = ?7, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id, status, progress, stage, error_message, output_path, worker_pid],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn save_provenance(
    connection: &Connection,
    id: &str,
    provenance: &serde_json::Value,
) -> Result<(), String> {
    let json = serde_json::to_string(provenance).map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE jobs SET result_json = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id, json],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn finish_validation(
    connection: &Connection,
    id: &str,
    report: &ValidationReport,
    output_path: Option<&str>,
) -> Result<(), String> {
    let report_json = serde_json::to_string(report).map_err(|error| error.to_string())?;
    let warning_count = report
        .issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Warning)
        .count();
    let error_count = report
        .issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .count();
    let (status, stage, error_message) = if report.can_complete() {
        let stage = if report.status == ValidationStatus::Repaired {
            format!(
                "Completed after {} non-destructive repair(s)",
                report.repairs.len()
            )
        } else if warning_count == 0 {
            "Completed and validated".to_string()
        } else {
            format!("Completed with {warning_count} validation warning(s)")
        };
        ("completed", stage, None)
    } else {
        (
            "validation_failed",
            format!("Validation failed with {error_count} error(s)"),
            Some("The generated beatmap did not pass output validation"),
        )
    };
    connection
        .execute(
            "UPDATE jobs SET status = ?2, progress = 100, stage = ?3, error_message = ?4,
             output_path = ?5, validation_report_json = ?6, worker_pid = NULL,
             updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id, status, stage, error_message, output_path, report_json],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn reset_for_retry(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE jobs SET status = 'queued', progress = 0, stage = 'Waiting for worker', error_message = NULL,
             output_path = NULL, result_json = NULL, validation_report_json = NULL, worker_pid = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn delete(connection: &Connection, id: &str) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM jobs WHERE id = ?1 AND status NOT IN ('running', 'queued', 'validating')",
            params![id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn mark_interrupted(connection: &Connection) -> Result<(), String> {
    connection
        .execute(
            "UPDATE jobs SET status = 'interrupted', stage = 'Application closed before the job finished',
             error_message = 'The previous worker session was interrupted', worker_pid = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE status IN ('running', 'queued', 'validating')",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{
        IssueSeverity, ValidationCode, ValidationIssue,
        types::{VALIDATION_SCHEMA_VERSION, VALIDATOR_VERSION},
    };

    fn version_one_database() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 CREATE TABLE jobs (id TEXT PRIMARY KEY, status TEXT NOT NULL, request_json TEXT NOT NULL, result_json TEXT,
                   created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
                 INSERT INTO schema_migrations(version) VALUES (1);",
            )
            .expect("version one schema");
        connection
    }

    #[test]
    fn migrates_and_persists_a_job() {
        let connection = version_one_database();
        migrate(&connection).expect("migration succeeds");
        migrate(&connection).expect("migration is idempotent");
        let input = StartJobInput {
            title: "Test Track".into(),
            artist: "Test Artist".into(),
            audio_file: "C:\\Music\\test.mp3".into(),
            model: crate::packages::STANDARD_MODEL_ID.into(),
            difficulty: 4.5,
            gamemode: 0,
            seed: 12345,
            descriptors: vec![],
            negative_descriptors: vec![],
            reference_beatmap: None,
            approach_rate: Some(9.0),
            circle_size: Some(4.0),
            hitsounded: true,
            scenario: "success".into(),
        };
        let created = insert(&connection, "BF-TEST", &input).expect("job is inserted");
        assert_eq!(created.status, "queued");
        assert!(created.provenance.is_none());
        assert!(created.validation_report.is_none());
        let provenance = serde_json::json!({
            "schema_version": 1,
            "reproduction_key": "same-input-settings"
        });
        save_provenance(&connection, "BF-TEST", &provenance).unwrap();
        assert_eq!(
            get(&connection, "BF-TEST")
                .unwrap()
                .unwrap()
                .provenance
                .unwrap()["reproduction_key"],
            "same-input-settings"
        );
        assert_eq!(list(&connection).expect("jobs load").len(), 1);
    }

    #[test]
    fn marks_active_jobs_interrupted() {
        let connection = version_one_database();
        migrate(&connection).expect("migration succeeds");
        let input = StartJobInput {
            title: "Test".into(),
            artist: "Artist".into(),
            audio_file: "test.wav".into(),
            model: crate::packages::STANDARD_MODEL_ID.into(),
            difficulty: 3.0,
            gamemode: 0,
            seed: 12345,
            descriptors: vec![],
            negative_descriptors: vec![],
            reference_beatmap: None,
            approach_rate: Some(9.0),
            circle_size: Some(4.0),
            hitsounded: true,
            scenario: "hang".into(),
        };
        insert(&connection, "BF-TEST", &input).expect("job is inserted");
        assert_eq!(active_ids(&connection).unwrap(), vec!["BF-TEST"]);
        mark_interrupted(&connection).expect("recovery succeeds");
        assert!(active_ids(&connection).unwrap().is_empty());
        assert_eq!(
            get(&connection, "BF-TEST").unwrap().unwrap().status,
            "interrupted"
        );
    }

    #[test]
    fn validation_result_is_persisted_with_the_terminal_state() {
        let connection = version_one_database();
        migrate(&connection).expect("migration succeeds");
        let input = StartJobInput {
            title: "Test".into(),
            artist: "Artist".into(),
            audio_file: "test.wav".into(),
            model: crate::packages::STANDARD_MODEL_ID.into(),
            difficulty: 3.0,
            gamemode: 0,
            seed: 12345,
            descriptors: vec![],
            negative_descriptors: vec![],
            reference_beatmap: None,
            approach_rate: Some(9.0),
            circle_size: Some(4.0),
            hitsounded: true,
            scenario: "success".into(),
        };
        insert(&connection, "BF-VALIDATED", &input).unwrap();
        let report = ValidationReport {
            schema_version: VALIDATION_SCHEMA_VERSION,
            validator_version: VALIDATOR_VERSION.into(),
            job_id: "BF-VALIDATED".into(),
            status: ValidationStatus::ValidWithWarnings,
            source_path: "raw/result.osz".into(),
            validated_path: Some("validated/result.osz".into()),
            issues: vec![ValidationIssue {
                code: ValidationCode::MetadataIncomplete,
                severity: IssueSeverity::Warning,
                section: Some("Metadata".into()),
                line: None,
                message: "Metadata is incomplete".into(),
                repairable: true,
            }],
            statistics: None,
            repairs: Vec::new(),
            started_at: "2026-10-07T00:00:00Z".into(),
            completed_at: "2026-10-07T00:00:01Z".into(),
        };

        finish_validation(
            &connection,
            "BF-VALIDATED",
            &report,
            Some("C:\\jobs\\BF-VALIDATED\\validated\\result.osz"),
        )
        .unwrap();

        let persisted = get(&connection, "BF-VALIDATED").unwrap().unwrap();
        assert_eq!(persisted.status, "completed");
        assert_eq!(persisted.validation_report, Some(report));
        assert!(persisted.output_path.unwrap().contains("validated"));
    }
}
