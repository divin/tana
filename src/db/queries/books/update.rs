//! Book update operation
//!
//! This module provides the update functionality for books.

use crate::db::models::Book;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Update an existing book in the database
pub fn update(conn: &Connection, id: i32, book: &Book) -> Result<()> {
    debug!("Updating book with id: {}", id);

    let mut stmt = conn.prepare(
        "UPDATE books SET title = ?, author = ?, isbn = ?, genre = ?, pages = ?, rating = ?, started_date = ?, completed_date = ?, notes = ? WHERE id = ?",
    )?;

    stmt.execute(params![
        &book.title,
        &book.author,
        &book.isbn,
        &book.genre,
        &book.pages,
        &book.rating,
        &book.started_date,
        &book.completed_date,
        &book.notes,
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
            "CREATE TABLE books (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                isbn TEXT,
                genre TEXT,
                pages INTEGER,
                rating REAL,
                started_date DATE,
                completed_date DATE NOT NULL,
                notes TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_update_book() {
        let conn = setup_test_db();
        let mut book = Book::new(
            "The Rust Book".to_string(),
            "Steve Klabnik".to_string(),
            "2024-01-25".to_string(),
        )
        .with_pages(500)
        .with_rating(8.0);

        // Insert
        let mut stmt = conn
            .prepare(
                "INSERT INTO books (title, author, isbn, genre, pages, rating, started_date,
             completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        let id = stmt
            .insert(rusqlite::params![
                &book.title,
                &book.author,
                &book.isbn,
                &book.genre,
                &book.pages,
                &book.rating,
                &book.started_date,
                &book.completed_date,
                &book.notes
            ])
            .unwrap();

        // Update
        book.rating = Some(9.0);
        book.genre = Some("Programming".to_string());
        update(&conn, id as i32, &book).unwrap();

        // Verify
        let mut stmt = conn
            .prepare("SELECT rating, genre FROM books WHERE id = ?")
            .unwrap();
        let (updated_rating, updated_genre): (f64, Option<String>) = stmt
            .query_row([id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        assert_eq!(updated_rating, 9.0);
        assert_eq!(updated_genre, Some("Programming".to_string()));
    }
}
