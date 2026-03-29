//! TV Series count operation
//!
//! This module provides the count functionality for TV series.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Get count of all TV series
pub fn count(conn: &Connection) -> Result<i64> {
    debug!("Counting TV series");

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM tv_series", [], |row| row.get(0))?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::TVSeries;

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
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_count_series() {
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
             current_episode, rating, started_date, completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
            &series1.notes
        ])
        .unwrap();

        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
            &series2.notes
        ])
        .unwrap();

        let cnt = count(&conn).unwrap();
        assert_eq!(cnt, 2);
    }
}
