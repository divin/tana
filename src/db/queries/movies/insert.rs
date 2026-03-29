//! Movie insert operation
//!
//! This module provides the insert functionality for movies.

use crate::db::models::Movie;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Insert a new movie into the database
pub fn insert(conn: &Connection, movie: &Movie) -> Result<i32> {
    debug!("Inserting movie: {}", movie.title);

    let mut stmt = conn.prepare(
        "INSERT INTO movies (title, release_year, director, rating, watched_date, notes)
         VALUES (?, ?, ?, ?, ?, ?)",
    )?;

    let id = stmt.insert(params![
        &movie.title,
        &movie.release_year,
        &movie.director,
        &movie.rating,
        &movie.watched_date,
        &movie.notes,
    ])?;

    Ok(id as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "CREATE TABLE movies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                release_year INTEGER,
                director TEXT,
                rating REAL,
                watched_date DATE NOT NULL,
                notes TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_insert_movie() {
        let conn = setup_test_db();
        let movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string());

        let id = insert(&conn, &movie).unwrap();
        assert!(id > 0);
    }
}
