//! Route definitions for the REST API
//!
//! This module defines the routing structure for the REST API server.
//! All routes are mounted under /api and use a shared AppState.

use axum::Router;
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;

use super::handlers::{
    create_book, create_movie, create_series, delete_book, delete_movie, delete_series, get_book,
    get_movie, get_series, list_books, list_movies, list_series, search_handler, stats_handler,
    update_book, update_movie, update_series,
};

/// Application state for the server
///
/// Holds the database path which is used to open connections in handlers.
/// This design ensures thread-safety as paths are immutable and can be shared.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Path to the SQLite database file
    pub db_path: Arc<PathBuf>,
}

impl AppState {
    /// Create a new application state with the given database path
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path: Arc::new(db_path),
        }
    }
}

/// Create the main router with all API routes
///
/// Sets up all routes with shared application state.
/// Routes are mounted under /api path.
pub fn create_router(db_path: PathBuf) -> Router {
    let state = AppState::new(db_path);

    // Build the API routes with shared state
    let api_routes = Router::new()
        // Movie routes: GET/POST /api/movies, GET/PUT/DELETE /api/movies/:id
        .route("/movies", get(list_movies).post(create_movie))
        .route(
            "/movies/{id}",
            get(get_movie).put(update_movie).delete(delete_movie),
        )
        // TV Series routes: GET/POST /api/series, GET/PUT/DELETE /api/series/:id
        .route("/series", get(list_series).post(create_series))
        .route(
            "/series/{id}",
            get(get_series).put(update_series).delete(delete_series),
        )
        // Book routes: GET/POST /api/books, GET/PUT/DELETE /api/books/:id
        .route("/books", get(list_books).post(create_book))
        .route(
            "/books/{id}",
            get(get_book).put(update_book).delete(delete_book),
        )
        // Statistics route: GET /api/stats
        .route("/stats", get(stats_handler))
        // Search route: GET /api/search?q=query
        .route("/search", get(search_handler))
        .with_state(state);

    // Build the main router with the API routes nested under /api
    Router::new().nest("/api", api_routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let path = PathBuf::from("/tmp/test.db");
        let state = AppState::new(path.clone());
        assert_eq!(*state.db_path, path);
    }

    #[test]
    fn test_router_creation() {
        // Test that the router can be created with a valid path
        let path = PathBuf::from("/tmp/test.db");
        let _router = create_router(path);
        // If we get here without panicking, the router was created successfully
    }
}
