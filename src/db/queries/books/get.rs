//! Book get operations
//!
//! This module provides the get/retrieve functionality for books.

use crate::db::models::Book;
use crate::error::Result;
use rusqlite::{Connection, OptionalExtension};
use tracing::debug;

/// Get all books, optionally limited
pub fn get_all(conn: &Connection, limit: Option<i32>) -> Result<Vec<Book>> {
    debug!("Fetching all books");

    let query = if let Some(l) = limit {
        format!(
            "SELECT id, title, author, isbn, genre, pages, rating, started_date,
             completed_date, notes FROM books ORDER BY completed_date DESC LIMIT {}",
            l
        )
    } else {
        "SELECT id, title, author, isbn, genre, pages, rating, started_date,
         completed_date, notes FROM books ORDER BY completed_date DESC"
            .to_string()
    };

    let mut stmt = conn.prepare(&query)?;
    let books = stmt
        .query_map([], |row| {
            Ok(Book {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                author: row.get(2)?,
                isbn: row.get(3)?,
                genre: row.get(4)?,
                pages: row.get(5)?,
                rating: row.get(6)?,
                started_date: row.get(7)?,
                completed_date: row.get(8)?,
                notes: row.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(books)
}

/// Get a book by ID
pub fn get_by_id(conn: &Connection, id: i32) -> Result<Option<Book>> {
    debug!("Fetching book with id: {}", id);

    let mut stmt = conn.prepare(
        "SELECT id, title, author, isbn, genre, pages, rating, started_date,
         completed_date, notes FROM books WHERE id = ?",
    )?;

    let book = stmt
        .query_row([id], |row| {
            Ok(Book {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                author: row.get(2)?,
                isbn: row.get(3)?,
                genre: row.get(4)?,
                pages: row.get(5)?,
                rating: row.get(6)?,
                started_date: row.get(7)?,
                completed_date: row.get(8)?,
                notes: row.get(9)?,
            })
        })
        .optional()?;

    Ok(book)
}

/// Get books by author
pub fn get_by_author(conn: &Connection, author: &str) -> Result<Vec<Book>> {
    debug!("Fetching books by author: {}", author);

    let mut stmt = conn.prepare(
        "SELECT id, title, author, isbn, genre, pages, rating, started_date,
         completed_date, notes FROM books WHERE author LIKE ? ORDER BY completed_date DESC",
    )?;

    let books = stmt
        .query_map([format!("%{}%", author)], |row| {
            Ok(Book {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                author: row.get(2)?,
                isbn: row.get(3)?,
                genre: row.get(4)?,
                pages: row.get(5)?,
                rating: row.get(6)?,
                started_date: row.get(7)?,
                completed_date: row.get(8)?,
                notes: row.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(books)
}

/// Get books by genre
pub fn get_by_genre(conn: &Connection, genre: &str) -> Result<Vec<Book>> {
    debug!("Fetching books by genre: {}", genre);

    let mut stmt = conn.prepare(
        "SELECT id, title, author, isbn, genre, pages, rating, started_date,
         completed_date, notes FROM books WHERE genre LIKE ? ORDER BY completed_date DESC",
    )?;

    let books = stmt
        .query_map([format!("%{}%", genre)], |row| {
            Ok(Book {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                author: row.get(2)?,
                isbn: row.get(3)?,
                genre: row.get(4)?,
                pages: row.get(5)?,
                rating: row.get(6)?,
                started_date: row.get(7)?,
                completed_date: row.get(8)?,
                notes: row.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(books)
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
    fn test_get_book_by_id() {
        let conn = setup_test_db();
        let book = Book::new(
            "The Rust Book".to_string(),
            "Steve Klabnik".to_string(),
            "2024-01-25".to_string(),
        )
        .with_pages(500);

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

        let retrieved = get_by_id(&conn, id as i32).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "The Rust Book");
    }

    #[test]
    fn test_get_books_by_author() {
        let conn = setup_test_db();
        let book1 = Book::new(
            "Book 1".to_string(),
            "Isaac Asimov".to_string(),
            "2024-01-15".to_string(),
        );
        let book2 = Book::new(
            "Book 2".to_string(),
            "Arthur C. Clarke".to_string(),
            "2024-01-20".to_string(),
        );

        let mut stmt = conn
            .prepare(
                "INSERT INTO books (title, author, isbn, genre, pages, rating, started_date,
             completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &book1.title,
            &book1.author,
            &book1.isbn,
            &book1.genre,
            &book1.pages,
            &book1.rating,
            &book1.started_date,
            &book1.completed_date,
            &book1.notes
        ])
        .unwrap();

        let mut stmt = conn
            .prepare(
                "INSERT INTO books (title, author, isbn, genre, pages, rating, started_date,
             completed_date, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();
        stmt.insert(rusqlite::params![
            &book2.title,
            &book2.author,
            &book2.isbn,
            &book2.genre,
            &book2.pages,
            &book2.rating,
            &book2.started_date,
            &book2.completed_date,
            &book2.notes
        ])
        .unwrap();

        let books = get_by_author(&conn, "Asimov").unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].author, "Isaac Asimov");
    }
}
