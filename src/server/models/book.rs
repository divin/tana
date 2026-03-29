//! Book models for the REST API
//!
//! This module defines the request and response types for book endpoints.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// Book response for GET requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookResponse {
    pub id: i32,
    pub title: String,
    pub author: String,
    pub isbn: Option<String>,
    pub genre: Option<String>,
    pub pages: Option<i32>,
    pub rating: Option<f64>,
    pub started_date: Option<String>,
    pub completed_date: String,
    pub notes: Option<String>,
    pub cover_path: Option<String>,
}

/// Book request for POST/PUT operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRequest {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub isbn: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub pages: Option<i32>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub started_date: Option<String>,
    pub completed_date: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub cover_path: Option<String>,
}

impl From<crate::db::models::Book> for BookResponse {
    fn from(book: crate::db::models::Book) -> Self {
        BookResponse {
            id: book.id.unwrap_or(0),
            title: book.title,
            author: book.author,
            isbn: book.isbn,
            genre: book.genre,
            pages: book.pages,
            rating: book.rating,
            started_date: book.started_date,
            completed_date: book.completed_date,
            notes: book.notes,
            cover_path: book.cover_path,
        }
    }
}

impl From<BookRequest> for crate::db::models::Book {
    fn from(req: BookRequest) -> Self {
        crate::db::models::Book {
            id: None,
            title: req.title,
            author: req.author,
            isbn: req.isbn,
            genre: req.genre,
            pages: req.pages,
            rating: req.rating,
            started_date: req.started_date,
            completed_date: req.completed_date,
            notes: req.notes,
            cover_path: req.cover_path,
        }
    }
}
