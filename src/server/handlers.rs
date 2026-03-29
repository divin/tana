//! HTTP request handlers for the REST API
//!
//! This module organizes all handler functions for the REST API endpoints.
//! Handlers convert database operations to HTTP responses with appropriate status codes.
//!
//! The module is split into organized submodules for each resource type and feature.

pub mod books;
pub mod error;
pub mod movies;
pub mod search;
pub mod series;
pub mod stats;

// Re-export public items for convenient access
pub use books::{create_book, delete_book, get_book, list_books, update_book};
pub use error::ApiError;
pub use movies::{create_movie, delete_movie, get_movie, list_movies, update_movie};
pub use search::{SearchParams, search_handler};
pub use series::{create_series, delete_series, get_series, list_series, update_series};
pub use stats::stats_handler;
