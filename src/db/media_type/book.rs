//! Book type definition and operations
//!
//! This module defines the Book struct and implements the Media trait for books.

use super::media::Media;
use serde::Serialize;

/// A book entry in the database
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Book {
    pub id: Option<i32>,
    pub title: String,
    pub author: String,
    pub isbn: Option<String>,
    pub genre: Option<String>,
    pub pages: Option<i32>,
    pub rating: Option<f64>,
    pub started_date: Option<String>,
    pub completed_date: String,
    pub notes: Option<String>,
}

impl Book {
    /// Create a new book entry
    pub fn new(title: String, author: String, completed_date: String) -> Self {
        Book {
            id: None,
            title,
            author,
            isbn: None,
            genre: None,
            pages: None,
            rating: None,
            started_date: None,
            completed_date,
            notes: None,
        }
    }

    /// Add an ISBN
    pub fn with_isbn(mut self, isbn: String) -> Self {
        self.isbn = Some(isbn);
        self
    }

    /// Add a genre
    pub fn with_genre(mut self, genre: String) -> Self {
        self.genre = Some(genre);
        self
    }

    /// Add page count
    pub fn with_pages(mut self, pages: i32) -> Self {
        self.pages = Some(pages);
        self
    }

    /// Add a rating
    pub fn with_rating(mut self, rating: f64) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Add a start date
    pub fn with_started_date(mut self, date: String) -> Self {
        self.started_date = Some(date);
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
}

impl Media for Book {
    fn table_name() -> &'static str {
        "books"
    }

    fn media_type_name() -> &'static str {
        "book"
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn rating(&self) -> Option<f64> {
        self.rating
    }

    fn date_added(&self) -> &str {
        &self.completed_date
    }

    fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_creation() {
        let book = Book::new(
            "The Rust Book".to_string(),
            "Steve Klabnik".to_string(),
            "2024-01-26".to_string(),
        )
        .with_pages(500)
        .with_rating(8.5);

        assert_eq!(book.title, "The Rust Book");
        assert_eq!(book.author, "Steve Klabnik");
        assert_eq!(book.pages, Some(500));
        assert_eq!(book.rating, Some(8.5));
    }

    #[test]
    fn test_book_trait_impl() {
        let book = Book::new(
            "1984".to_string(),
            "George Orwell".to_string(),
            "2026-02-10".to_string(),
        );

        assert_eq!(Book::table_name(), "books");
        assert_eq!(Book::media_type_name(), "book");
        assert_eq!(book.title(), "1984");
        assert_eq!(book.date_added(), "2026-02-10");
    }
}
