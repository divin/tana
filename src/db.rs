//! Database module for tana
//!
//! This module handles all database operations including:
//! - SQLite connection management
//! - Schema migrations
//! - CRUD operations for media types
//! - Data validation and persistence

pub mod media_type {
    pub mod book;
    pub mod media;
    pub mod movie;
    pub mod tv_series;

    pub use self::book::Book;
    pub use self::media::Media;
    pub use self::movie::Movie;
    pub use self::tv_series::TVSeries;
}
pub mod migrations;
pub mod models;
pub mod queries {
    pub mod books;
    pub mod movies;
    pub mod tv_series;
}
pub mod schema;

use crate::error::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Database connection wrapper
pub struct Database {
    connection: Connection,
}

impl Database {
    /// Open or create a database at the given path
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new Database instance with migrations applied
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Opening database at {:?}", path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open connection
        let connection = Connection::open(path)?;

        // Enable foreign keys
        connection.execute("PRAGMA foreign_keys = ON;", [])?;

        let db = Database { connection };

        // Run migrations
        db.run_migrations()?;

        info!("Database initialized successfully at {:?}", path);
        Ok(db)
    }

    /// Open an in-memory database (useful for testing)
    pub fn open_memory() -> Result<Self> {
        debug!("Opening in-memory database");
        let connection = Connection::open_in_memory()?;

        connection.execute("PRAGMA foreign_keys = ON;", [])?;

        let db = Database { connection };
        db.run_migrations()?;

        Ok(db)
    }

    /// Run all pending migrations
    fn run_migrations(&self) -> Result<()> {
        debug!("Running database migrations");
        migrations::run_migrations(&self.connection)?;
        Ok(())
    }

    /// Get a mutable reference to the connection (for internal use)
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Get the default database path following XDG Base Directory spec
    ///
    /// Returns `~/.local/share/tana/tana.db` on Unix-like systems
    pub fn default_path() -> PathBuf {
        let data_home = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| format!("{}/.local/share", dirs_home()));

        PathBuf::from(data_home).join("tana").join("tana.db")
    }

    /// Get the images directory path
    ///
    /// Returns `~/.local/share/tana/images/`
    pub fn images_dir() -> PathBuf {
        let data_home = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| format!("{}/.local/share", dirs_home()));

        PathBuf::from(data_home).join("tana").join("images")
    }
}

/// Helper to get home directory
fn dirs_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_database() {
        let db = Database::open_memory().expect("Failed to open in-memory database");
        assert!(db.connection.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |_| Ok(()),
        ).is_ok());
    }
}
