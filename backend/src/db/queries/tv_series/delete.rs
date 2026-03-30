//! TV Series delete operation
//!
//! This module provides the delete functionality for TV series.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Delete a TV series by ID
pub fn delete(conn: &Connection, id: i32) -> Result<bool> {
    debug!("Deleting TV series with id: {}", id);

    let rows_affected = conn.execute("DELETE FROM tv_series WHERE id = ?", [id])?;

    Ok(rows_affected > 0)
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
    fn test_delete_series() {
        let conn = setup_test_db();
        let series = TVSeries::new(
            "Test Series".to_string(),
            "2024-01-10".to_string(),
            "ongoing".to_string(),
        );

        let mut stmt = conn
            .prepare(
                "INSERT INTO tv_series (title, release_year, status, total_seasons, current_season,
             current_episode, rating, started_date, completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
                &series.notes
            ])
            .unwrap();

        let deleted = delete(&conn, id as i32).unwrap();
        assert!(deleted);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tv_series WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
