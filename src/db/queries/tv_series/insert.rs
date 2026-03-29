//! TV Series insert operation
//!
//! This module provides the insert functionality for TV series.

use crate::db::models::TVSeries;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Insert a new TV series into the database
pub fn insert(conn: &Connection, series: &TVSeries) -> Result<i32> {
    debug!("Inserting TV series: {}", series.title);

    let mut stmt = conn.prepare(
        "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
         current_episode, rating, started_date, completed_date, notes, poster_path)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;

    let id = stmt.insert(params![
        &series.title,
        &series.release_year,
        &series.status,
        &series.total_seasons,
        &series.current_season,
        &series.current_episode,
        &series.rating,
        &series.started_date,
        &series.completed_date,
        &series.notes,
        &series.poster_path,
    ])?;

    Ok(id as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "CREATE TABLE tv_series (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                release_year INTEGER,
                status TEXT,
                total_seasons INTEGER,
                current_season INTEGER,
                current_episode INTEGER,
                rating REAL,
                started_date DATE NOT NULL,
                completed_date DATE,
                notes TEXT,
                poster_path TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_insert_series() {
        let conn = setup_test_db();
        let series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "completed".to_string(),
        )
        .with_total_seasons(5);

        let id = insert(&conn, &series).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_insert_series_with_poster_path() {
        let conn = setup_test_db();
        let series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "completed".to_string(),
        )
        .with_total_seasons(5)
        .with_poster_path("/images/posters/breaking_bad.jpg".to_string());

        let id = insert(&conn, &series).unwrap();
        assert!(id > 0);

        // Verify the poster_path was inserted
        let mut stmt = conn
            .prepare("SELECT poster_path FROM tv_series WHERE id = ?")
            .unwrap();
        let poster_path: Option<String> = stmt.query_row([id], |row| row.get(0)).unwrap();
        assert_eq!(
            poster_path,
            Some("/images/posters/breaking_bad.jpg".to_string())
        );
    }
}
