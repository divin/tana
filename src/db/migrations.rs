//! Database migration system
//!
//! This module implements version-based migrations for the tana database.
//! Each migration is tracked in the schema_migrations table, ensuring
//! that migrations are only applied once and in order.

use crate::error::{Result, TanaError};
use rusqlite::Connection;
use tracing::{debug, info};

/// A single migration with version, name, and SQL
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

impl Migration {
    /// Create a new migration
    pub fn new(version: u32, name: &'static str, sql: &'static str) -> Self {
        Migration { version, name, sql }
    }
}

/// Get all available migrations
///
/// Add new migrations to the list as you add new schema changes.
/// Each migration must have a unique version number.
fn get_migrations() -> Vec<Migration> {
    vec![Migration::new(
        1,
        "initial_schema",
        include_str!("../../migrations/001_initial_schema.sql"),
    )]
}

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> Result<()> {
    debug!("Starting migration process");

    // Create the schema_migrations table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )
    .map_err(|e| TanaError::Migration(format!("Failed to create migrations table: {}", e)))?;

    // Get the current schema version
    let current_version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    debug!("Current schema version: {}", current_version);

    // Run all pending migrations
    for migration in get_migrations() {
        if migration.version > current_version {
            info!(
                "Applying migration {}: {}",
                migration.version, migration.name
            );

            // Execute the migration SQL
            conn.execute_batch(migration.sql).map_err(|e| {
                TanaError::Migration(format!(
                    "Failed to apply migration {}: {}",
                    migration.version, e
                ))
            })?;

            // Record the migration as applied
            conn.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?, ?)",
                rusqlite::params![migration.version, migration.name],
            )
            .map_err(|e| {
                TanaError::Migration(format!(
                    "Failed to record migration {}: {}",
                    migration.version, e
                ))
            })?;

            info!(
                "✓ Applied migration {}: {}",
                migration.version, migration.name
            );
        }
    }

    let final_version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    debug!("Final schema version: {}", final_version);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migrations_table_created() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        assert!(exists, "schema_migrations table should exist");
    }

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations twice
        run_migrations(&conn).expect("First migration run failed");
        run_migrations(&conn).expect("Second migration run failed");

        // Verify the version is correct
        let version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        assert!(version > 0, "Version should be greater than 0");
    }
}
