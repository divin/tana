//! Movie get operations
//!
//! This module provides the get/retrieve functionality for movies.

use crate::db::models::Movie;
use crate::error::Result;
use rusqlite::{Connection, OptionalExtension};
use tracing::debug;

/// Get all movies, optionally limited
pub fn get_all(conn: &Connection, limit: Option<i32>) -> Result<Vec<Movie>> {
    debug!("Fetching all movies");

    let query = if let Some(l) = limit {
        format!(
            "SELECT id, title, release_year, director, rating, watched_date, notes
             FROM movies ORDER BY watched_date DESC LIMIT {}",
            l
        )
    } else {
        "SELECT id, title, release_year, director, rating, watched_date, notes
         FROM movies ORDER BY watched_date DESC"
            .to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let movies = stmt
        .query_map([], |row| {
            Ok(Movie {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                director: row.get(3)?,
                rating: row.get(4)?,
                watched_date: row.get(5)?,
                notes: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(movies)
}

/// Get a movie by ID
pub fn get_by_id(conn: &Connection, id: i32) -> Result<Option<Movie>> {
    debug!("Fetching movie with id: {}", id);

    let mut stmt = conn.prepare(
        "SELECT id, title, release_year, director, rating, watched_date, notes
         FROM movies WHERE id = ?",
    )?;

    let movie = stmt
        .query_row([id], |row| {
            Ok(Movie {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                director: row.get(3)?,
                rating: row.get(4)?,
                watched_date: row.get(5)?,
                notes: row.get(6)?,
            })
        })
        .optional()?;

    Ok(movie)
}

/// Get movies by year
pub fn get_by_year(conn: &Connection, year: i32) -> Result<Vec<Movie>> {
    debug!("Fetching movies from year: {}", year);

    let mut stmt = conn.prepare(
        "SELECT id, title, release_year, director, rating, watched_date, notes
         FROM movies WHERE release_year = ? ORDER BY watched_date DESC",
    )?;

    let movies = stmt
        .query_map([year], |row| {
            Ok(Movie {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                release_year: row.get(2)?,
                director: row.get(3)?,
                rating: row.get(4)?,
                watched_date: row.get(5)?,
                notes: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(movies)
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
    fn test_get_all_movies() {
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

        let movies = get_all(&conn, None).unwrap();
        assert_eq!(movies.len(), 2);
    }

    #[test]
    fn test_get_movie_by_id() {
        let conn = setup_test_db();
        let movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string());

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

        let retrieved = get_by_id(&conn, id as i32).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, "Inception");
    }

    #[test]
    fn test_get_movies_by_year() {
        let conn = setup_test_db();
        let movie1 = Movie::new("Movie 2010".to_string(), "2024-01-15".to_string()).with_year(2010);
        let movie2 = Movie::new("Movie 2020".to_string(), "2024-01-20".to_string()).with_year(2020);

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

        let movies = get_by_year(&conn, 2010).unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "Movie 2010");
    }
}
