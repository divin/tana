//! Schema utilities and information
//!
//! This module provides utilities for working with the database schema,
//! including table information and schema introspection.

use crate::error::Result;
use rusqlite::Connection;

/// Information about a database table
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Name of the table
    pub name: String,
    /// Number of rows in the table
    pub row_count: i64,
}

impl TableInfo {
    /// Create a new TableInfo
    pub fn new(name: String, row_count: i64) -> Self {
        TableInfo { name, row_count }
    }
}

/// Schema introspection utilities
pub struct SchemaInfo;

impl SchemaInfo {
    /// Get information about all media tables
    pub fn get_table_info(conn: &Connection) -> Result<Vec<TableInfo>> {
        let tables = vec!["movies", "tv_series", "books"];
        let mut info = Vec::new();

        for table in tables {
            let count: i64 =
                conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                    row.get(0)
                })?;

            info.push(TableInfo::new(table.to_string(), count));
        }

        Ok(info)
    }

    /// Get the current schema version
    pub fn get_schema_version(conn: &Connection) -> Result<u32> {
        let version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(version)
    }

    /// Check if a table exists
    pub fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?",
            [table_name],
            |row| row.get(0),
        )?;

        Ok(exists)
    }

    /// Get all applied migrations
    pub fn get_applied_migrations(conn: &Connection) -> Result<Vec<(u32, String)>> {
        let mut stmt =
            conn.prepare("SELECT version, name FROM schema_migrations ORDER BY version ASC")?;

        let migrations = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(migrations)
    }

    /// Get total count of all media items
    pub fn get_total_media_count(conn: &Connection) -> Result<i64> {
        let tables = vec!["movies", "tv_series", "books"];
        let mut total = 0i64;

        for table in tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);

            total += count;
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create test tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS movies (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                poster_path TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tv_series (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                poster_path TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                cover_path TEXT
            )",
            [],
        )
        .unwrap();

        // Insert some test data
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (1, 'initial')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO movies (title) VALUES ('Test Movie')", [])
            .unwrap();
        conn.execute("INSERT INTO books (title) VALUES ('Test Book')", [])
            .unwrap();

        conn
    }

    #[test]
    fn test_table_exists() {
        let conn = setup_test_db();
        assert!(SchemaInfo::table_exists(&conn, "movies").unwrap());
        assert!(SchemaInfo::table_exists(&conn, "books").unwrap());
        assert!(!SchemaInfo::table_exists(&conn, "nonexistent").unwrap());
    }

    #[test]
    fn test_get_schema_version() {
        let conn = setup_test_db();
        let version = SchemaInfo::get_schema_version(&conn).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_get_applied_migrations() {
        let conn = setup_test_db();
        let migrations = SchemaInfo::get_applied_migrations(&conn).unwrap();
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].0, 1);
        assert_eq!(migrations[0].1, "initial");
    }

    #[test]
    fn test_get_table_info() {
        let conn = setup_test_db();
        let info = SchemaInfo::get_table_info(&conn).unwrap();
        assert_eq!(info.len(), 3);

        let movies = info.iter().find(|t| t.name == "movies").unwrap();
        assert_eq!(movies.row_count, 1);

        let books = info.iter().find(|t| t.name == "books").unwrap();
        assert_eq!(books.row_count, 1);
    }

    #[test]
    fn test_get_total_media_count() {
        let conn = setup_test_db();
        let total = SchemaInfo::get_total_media_count(&conn).unwrap();
        assert_eq!(total, 2); // 1 movie + 1 book
    }
}
