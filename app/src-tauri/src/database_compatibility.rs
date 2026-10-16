use rusqlite::{Connection, OptionalExtension};

pub const CURRENT_DATABASE_SCHEMA_VERSION: i64 = 4;

pub fn ensure_supported(connection: &Connection) -> Result<(), String> {
    let has_migrations: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("Could not inspect the database schema: {error}"))?;
    if has_migrations.is_none() {
        return Ok(());
    }

    let newest: Option<i64> = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("Could not read the database schema version: {error}"))?;
    if newest.unwrap_or(0) > CURRENT_DATABASE_SCHEMA_VERSION {
        return Err(format!(
            "This Beatforge data was created by a newer app version (database schema {}). This version supports schema {} and stopped before changing your data. Reinstall the same or a newer Beatforge version.",
            newest.unwrap_or(0),
            CURRENT_DATABASE_SCHEMA_VERSION
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_new_and_supported_databases() {
        let empty = Connection::open_in_memory().unwrap();
        ensure_supported(&empty).unwrap();

        empty
            .execute_batch(
                "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY);
                 INSERT INTO schema_migrations(version) VALUES (1), (4);",
            )
            .unwrap();
        ensure_supported(&empty).unwrap();
    }

    #[test]
    fn rejects_a_future_schema_without_writing() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY);
                 CREATE TABLE future_data(value TEXT NOT NULL);
                 INSERT INTO schema_migrations(version) VALUES (99);
                 INSERT INTO future_data(value) VALUES ('preserve-me');",
            )
            .unwrap();

        let error = ensure_supported(&connection).unwrap_err();
        assert!(error.contains("newer app version"));
        assert!(error.contains("stopped before changing your data"));
        let value: String = connection
            .query_row("SELECT value FROM future_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "preserve-me");
    }
}
