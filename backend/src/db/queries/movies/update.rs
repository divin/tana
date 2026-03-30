//! Movie update operation
//!
//! This module provides the update functionality for movies.

use crate::db::models::Movie;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Update an existing movie in the database
pub fn update(conn: &Connection, id: i32, movie: &Movie) -> Result<()> {
    debug!("Updating movie with id: {}", id);

    let mut stmt = conn.prepare(
        "UPDATE movies SET title = ?, release_year = ?, director = ?, rating = ?, watched_date = ?, notes = ?, poster_path = ? WHERE id = ?",
    )?;

    stmt.execute(params![
        &movie.title,
        &movie.release_year,
        &movie.director,
        &movie.rating,
        &movie.watched_date,
        &movie.notes,
        &movie.poster_path,
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
            "CREATE TABLE movies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                release_year INTEGER,
                director TEXT,
                rating REAL,
                watched_date DATE NOT NULL,
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
    fn test_update_movie() {
        let conn = setup_test_db();
        let mut movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string())
            .with_rating(9.0);

        // Insert
        let mut stmt = conn
            .prepare(
                "INSERT INTO movies (title, release_year, director, rating, watched_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
                &movie.title,
                &movie.release_year,
                &movie.director,
                &movie.rating,
                &movie.watched_date,
                &movie.notes,
                &movie.poster_path,
            ])
            .unwrap();

        // Update
        movie.rating = Some(9.5);
        movie.notes = Some("Updated rating".to_string());
        update(&conn, id as i32, &movie).unwrap();

        // Verify
        let mut stmt = conn
            .prepare("SELECT rating, notes FROM movies WHERE id = ?")
            .unwrap();
        let (updated_rating, updated_notes): (f64, Option<String>) = stmt
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        assert_eq!(updated_rating, 9.5);
        assert_eq!(updated_notes, Some("Updated rating".to_string()));
    }

    #[test]
    fn test_update_movie_poster_path() {
        let conn = setup_test_db();
        let mut movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string())
            .with_rating(9.0);

        // Insert
        let mut stmt = conn
            .prepare(
                "INSERT INTO movies (title, release_year, director, rating, watched_date, notes, poster_path)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
                &movie.title,
                &movie.release_year,
                &movie.director,
                &movie.rating,
                &movie.watched_date,
                &movie.notes,
                &movie.poster_path,
            ])
            .unwrap();

        // Update with poster path
        movie.poster_path = Some("/images/posters/inception.jpg".to_string());
        update(&conn, id as i32, &movie).unwrap();

        // Verify
        let mut stmt = conn
            .prepare("SELECT poster_path FROM movies WHERE id = ?")
            .unwrap();
        let updated_poster_path: Option<String> = stmt.query_row([id], |row| row.get(0)).unwrap();

        assert_eq!(
            updated_poster_path,
            Some("/images/posters/inception.jpg".to_string())
        );
    }
}
