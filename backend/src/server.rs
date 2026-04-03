//! REST API server module for tana
//!
//! This module handles HTTP server setup, routes, handlers, and response types.
//! It provides a complete REST API for managing media entries via HTTP.

pub mod handlers;
pub mod models;
pub mod routes;

use crate::config::Config;
use crate::error::Result;
use std::path::PathBuf;
use tracing::info;

// Re-export handler functions and types
pub use handlers::{
    ApiError, SearchParams, create_book, create_movie, create_series, delete_book, delete_movie,
    delete_series, get_book, get_movie, get_series, health_handler, list_books, list_movies,
    list_series, search_handler, stats_handler, update_book, update_movie, update_series,
};

// Re-export model types
pub use models::{
    BookRequest, BookResponse, ErrorResponse, HealthResponse, MovieRequest, MovieResponse,
    SearchResponse, StatsResponse, TVSeriesRequest, TVSeriesResponse,
};

pub use routes::*;

/// Start the REST API server
///
/// # Arguments
/// * `db_path` - Path to the SQLite database file
/// * `host` - Host address to bind to (e.g., "127.0.0.1")
/// * `port` - Port number to listen on
/// * `cors_origins` - List of allowed CORS origins
/// * `allow_any_origin` - Allow any CORS origin (development mode only)
/// * `config` - Application configuration containing image download settings and other config
///
/// # Returns
/// Result indicating success or error
pub async fn run(
    db_path: PathBuf,
    host: String,
    port: u16,
    cors_origins: Vec<String>,
    allow_any_origin: bool,
    config: Config,
) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let socket_addr: std::net::SocketAddr = addr.parse().map_err(|_| {
        crate::error::TanaError::InvalidInput(format!("Invalid host:port: {}", addr))
    })?;

    // Build the router with database path, config, and CORS origins
    let app = routes::create_router(db_path, config, cors_origins, allow_any_origin);

    // Create a TCP listener
    let listener = tokio::net::TcpListener::bind(&socket_addr)
        .await
        .map_err(crate::error::TanaError::Io)?;

    info!("Server listening on http://{}", socket_addr);

    // Run the server
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::TanaError::Io(std::io::Error::other(e)))?;

    Ok(())
}
