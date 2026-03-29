//! TV Series get operations
//!
//! This module provides the get/retrieve functionality for TV series.

use crate::db::models::TVSeries;
use crate::error::Result;
use rusqlite::{Connection, OptionalExtension};
use tracing::debug;

/// Get all TV series, optionally limited
pub fn get_all(conn: &Connection, limit: Option<i32>) -> Result<Vec<TVSeries>> {
    debug!("Fetching all TV series");

    let query = if let Some(l) = limit {
        format!(
            "SELECT id, title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes, poster_path
             FROM tv_series ORDER BY started_date DESC LIMIT {}",
            l
        )
    } else {
        "SELECT id, title, release_year, status, total_seasons, current_season,
         current_episode, rating, started_date, completed_date, notes, poster_path
         FROM tv_series ORDER BY started_date DESC"
            .to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let series = stmt
        .query_map([], |row| {
            Ok(TVSeries {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                status: row.get(3)?,
                total_seasons: row.get(4)?,
                current_season: row.get(5)?,
                current_episode: row.get(6)?,
                rating: row.get(7)?,
                started_date: row.get(8)?,
                completed_date: row.get(9)?,
                notes: row.get(10)?,
                poster_path: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(series)
}

/// Get a TV series by ID
pub fn get_by_id(conn: &Connection, id: i32) -> Result<Option<TVSeries>> {
    debug!("Fetching TV series with id: {}", id);

    let mut stmt = conn.prepare(
        "SELECT id, title, release_year, status, total_seasons, current_season,
         current_episode, rating, started_date, completed_date, notes, poster_path
         FROM tv_series WHERE id = ?",
    )?;

    let series = stmt
        .query_row([id], |row| {
            Ok(TVSeries {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                status: row.get(3)?,
                total_seasons: row.get(4)?,
                current_season: row.get(5)?,
                current_episode: row.get(6)?,
                rating: row.get(7)?,
                started_date: row.get(8)?,
                completed_date: row.get(9)?,
                notes: row.get(10)?,
                poster_path: row.get(11)?,
            })
        })
        .optional()?;

    Ok(series)
}

/// Get TV series by status
pub fn get_by_status(conn: &Connection, status: &str) -> Result<Vec<TVSeries>> {
    debug!("Fetching TV series with status: {}", status);

    let mut stmt = conn.prepare(
        "SELECT id, title, release_year, status, total_seasons, current_season,
         current_episode, rating, started_date, completed_date, notes, poster_path
         FROM tv_series WHERE status = ? ORDER BY started_date DESC",
    )?;

    let series = stmt
        .query_map([status], |row| {
            Ok(TVSeries {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                status: row.get(3)?,
                total_seasons: row.get(4)?,
                current_season: row.get(5)?,
                current_episode: row.get(6)?,
                rating: row.get(7)?,
                started_date: row.get(8)?,
                completed_date: row.get(9)?,
                notes: row.get(10)?,
                poster_path: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(series)
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
    fn test_get_series_by_id() {
        let conn = setup_test_db();
        let series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "completed".to_string(),
        )
        .with_total_seasons(5);

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

        let retrieved = get_by_id(&conn, id as i32).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Breaking Bad");
    }

    #[test]
    fn test_get_series_by_status() {
        let conn = setup_test_db();
        let series1 = TVSeries::new(
            "Series 1".to_string(),
            "2024-01-10".to_string(),
            "ongoing".to_string(),
        );
        let series2 = TVSeries::new(
            "Series 2".to_string(),
            "2024-01-15".to_string(),
            "completed".to_string(),
        );

        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &series1.title,
            &series1.release_year,
            &series1.status,
            &series1.total_seasons,
            &series1.current_season,
            &series1.current_episode,
            &series1.rating,
            &series1.started_date,
            &series1.completed_date,
            &series1.notes,
            &series1.poster_path,
        ])
        .unwrap();

        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &series2.title,
            &series2.release_year,
            &series2.status,
            &series2.total_seasons,
            &series2.current_season,
            &series2.current_episode,
            &series2.rating,
            &series2.started_date,
            &series2.completed_date,
            &series2.notes,
            &series2.poster_path,
        ])
        .unwrap();

        let series = get_by_status(&conn, "completed").unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].title, "Series 2");
    }

    #[test]
    fn test_get_series_with_poster_path() {
        let conn = setup_test_db();
        let series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "completed".to_string(),
        )
        .with_total_seasons(5)
        .with_poster_path("/images/posters/breaking_bad.jpg".to_string());

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

        let retrieved = get_by_id(&conn, id as i32).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, "Breaking Bad");
        assert_eq!(
            retrieved.poster_path,
            Some("/images/posters/breaking_bad.jpg".to_string())
        );
    }
}
