//! TV Series update operation
//!
//! This module provides the update functionality for TV series.

use crate::db::models::TVSeries;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Update an existing TV series in the database
pub fn update(conn: &Connection, id: i32, series: &TVSeries) -> Result<()> {
    debug!("Updating TV series with id: {}", id);

    let mut stmt = conn.prepare(
        "UPDATE tv_series SET title = ?, release_year = ?, status = ?, total_seasons = ?, current_season = ?, current_episode = ?, rating = ?, started_date = ?, completed_date = ?, notes = ?, poster_path = ? WHERE id = ?",
    )?;

    stmt.execute(params![
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
        id,
    ])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tv_series (
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
    fn test_update_series() {
        let conn = setup_test_db();
        let mut series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "ongoing".to_string(),
        )
        .with_total_seasons(5)
        .with_rating(8.5);

        // Insert
        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
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
                &series.poster_path
            ])
            .unwrap();

        // Update
        series.status = "completed".to_string();
        series.rating = Some(9.5);
        update(&conn, id as i32, &series).unwrap();

        // Verify
        let mut stmt = conn
            .prepare("SELECT status, rating FROM tv_series WHERE id = ?")
            .unwrap();
        let (updated_status, updated_rating): (String, f64) = stmt
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        assert_eq!(updated_status, "completed");
        assert_eq!(updated_rating, 9.5);
    }

    #[test]
    fn test_update_series_poster_path() {
        let conn = setup_test_db();
        let mut series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "ongoing".to_string(),
        )
        .with_total_seasons(5)
        .with_rating(8.5);

        // Insert
        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
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
            ])
            .unwrap();

        // Update with poster path
        series.poster_path = Some("/images/posters/breaking_bad.jpg".to_string());
        update(&conn, id as i32, &series).unwrap();

        // Verify
        let mut stmt = conn
            .prepare("SELECT poster_path FROM tv_series WHERE id = ?")
            .unwrap();
        let updated_poster_path: Option<String> = stmt.query_row([id], |row| row.get(0)).unwrap();

        assert_eq!(
            updated_poster_path,
            Some("/images/posters/breaking_bad.jpg".to_string())
        );
    }
}
