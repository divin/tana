//! Error types for the tana application
//!
//! This module defines custom error types for various error conditions
//! that can occur during CLI operations and database interactions.

use std::io;
use thiserror::Error;

/// The main error type for tana operations
#[derive(Error, Debug)]
pub enum TanaError {
    /// Database-related errors from rusqlite
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Migration system errors
    #[error("Migration error: {0}")]
    Migration(String),

    /// Invalid date format in user input
    #[error("Invalid date format: '{0}'. Expected YYYY-MM-DD")]
    InvalidDateFormat(String),

    /// Rating value outside acceptable range
    #[error("Invalid rating: {0}. Rating must be between 0 and 10")]
    InvalidRating(f64),

    /// General input validation error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Media entry not found in database
    #[error("Media not found: {0}")]
    MediaNotFound(String),

    /// File/IO operations error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

/// Result type alias using TanaError as the error type
pub type Result<T> = std::result::Result<T, TanaError>;

impl From<String> for TanaError {
    fn from(s: String) -> Self {
        TanaError::InvalidInput(s)
    }
}

impl TanaError {
    /// Create a new migration error
    pub fn migration<S: Into<String>>(msg: S) -> Self {
        TanaError::Migration(msg.into())
    }

    /// Create a new input validation error
    pub fn invalid_input<S: Into<String>>(msg: S) -> Self {
        TanaError::InvalidInput(msg.into())
    }

    /// Create a new generic error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        TanaError::Other(msg.into())
    }
}
