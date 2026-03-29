//! Book count operation
//!
//! This module provides the count functionality for books.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Get count of all books
pub fn count(conn: &Connection) -> Result<i64> {
    debug!("Counting books");

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Book;
    use crate::db::queries::books::insert;

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
    fn test_count_books() {
        let conn = setup_test_db();
        let book1 = Book::new(
            "Book 1".to_string(),
            "Author 1".to_string(),
            "2024-01-15".to_string(),
        );
        let book2 = Book::new(
            "Book 2".to_string(),
            "Author 2".to_string(),
            "2024-01-20".to_string(),
        );

        insert::insert(&conn, &book1).unwrap();
        insert::insert(&conn, &book2).unwrap();

        let count = count(&conn).unwrap();
        assert_eq!(count, 2);
    }
}
