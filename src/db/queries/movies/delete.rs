//! Movie delete operation
//!
//! This module provides the delete functionality for movies.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Delete a movie by ID
pub fn delete(conn: &Connection, id: i32) -> Result<bool> {
    debug!("Deleting movie with id: {}", id);

    let rows_affected = conn.execute("DELETE FROM movies WHERE id = ?", [id])?;

    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Movie;

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
    fn test_delete_movie() {
        let conn = setup_test_db();
        let movie = Movie::new("Test Movie".to_string(), "2024-01-15".to_string());

        let mut stmt = conn
            .prepare(
                "INSERT INTO movies (title, release_year, director, rating, watched_date, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
                &movie.title,
                &movie.release_year,
                &movie.director,
                &movie.rating,
                &movie.watched_date,
                &movie.notes
            ])
            .unwrap();

        let deleted = delete(&conn, id as i32).unwrap();
        assert!(deleted);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM movies WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
