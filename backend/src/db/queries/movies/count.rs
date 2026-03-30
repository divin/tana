//! Movie count operation
//!
//! This module provides the count functionality for movies.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Get count of all movies
pub fn count(conn: &Connection) -> Result<i64> {
    debug!("Counting movies");

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM movies", [], |row| row.get(0))?;

    Ok(count)
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
    fn test_count_movies() {
        let conn = setup_test_db();
        let movie1 = Movie::new("Movie 1".to_string(), "2024-01-15".to_string());
        let movie2 = Movie::new("Movie 2".to_string(), "2024-01-20".to_string());

        let mut stmt = conn
            .prepare(
                "INSERT INTO movies (title, release_year, director, rating, watched_date, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &movie1.title,
            &movie1.release_year,
            &movie1.director,
            &movie1.rating,
            &movie1.watched_date,
            &movie1.notes
        ])
        .unwrap();

        let mut stmt = conn
            .prepare(
                "INSERT INTO movies (title, release_year, director, rating, watched_date, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &movie2.title,
            &movie2.release_year,
            &movie2.director,
            &movie2.rating,
            &movie2.watched_date,
            &movie2.notes
        ])
        .unwrap();

        let cnt = count(&conn).unwrap();
        assert_eq!(cnt, 2);
    }
}
