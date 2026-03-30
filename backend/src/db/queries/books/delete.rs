//! Book delete operation
//!
//! This module provides the delete functionality for books.

use crate::error::Result;
use rusqlite::Connection;
use tracing::debug;

/// Delete a book by ID
pub fn delete(conn: &Connection, id: i32) -> Result<bool> {
    debug!("Deleting book with id: {}", id);

    let rows_affected = conn.execute("DELETE FROM books WHERE id = ?", [id])?;

    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Book;
    use crate::db::queries::books::get;
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
                cover_path TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_delete_book() {
        let conn = setup_test_db();
        let book = Book::new(
            "Test Book".to_string(),
            "Test Author".to_string(),
            "2024-01-15".to_string(),
        );
        let id = insert::insert(&conn, &book).unwrap();

        let deleted = delete(&conn, id).unwrap();
        assert!(deleted);

        let retrieved = get::get_by_id(&conn, id).unwrap();
        assert!(retrieved.is_none());
    }
}
