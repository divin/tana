//! Book insert operation
//!
//! This module provides the insert functionality for books.

use crate::db::models::Book;
use crate::error::Result;
use rusqlite::{Connection, params};
use tracing::debug;

/// Insert a new book into the database
pub fn insert(conn: &Connection, book: &Book) -> Result<i32> {
    debug!("Inserting book: {}", book.title);

    let mut stmt = conn.prepare(
        "INSERT INTO books (title, author, isbn, genre, pages, rating, started_date,
         completed_date, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;

    let id = stmt.insert(params![
        &book.title,
        &book.author,
        &book.isbn,
        &book.genre,
        &book.pages,
        &book.rating,
        &book.started_date,
        &book.completed_date,
        &book.notes,
    ])?;

    Ok(id as i32)
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
    fn test_insert_book() {
        let conn = setup_test_db();
        let book = Book::new(
            "The Rust Book".to_string(),
            "Steve Klabnik".to_string(),
            "2024-01-25".to_string(),
        )
        .with_pages(500);

        let id = insert(&conn, &book).unwrap();
        assert!(id > 0);
    }
}
