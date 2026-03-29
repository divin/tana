//! Route definitions for the REST API
//!
//! This module defines the routing structure for the REST API server.
//! All routes are mounted under /api and use a shared AppState.
//! OpenAPI documentation and Swagger UI are available at /api/docs

use axum::Router;
use axum::http::Method;
use axum::http::header::{ACCEPT, CONTENT_TYPE, HeaderValue};
use axum::routing::get;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::handlers;
use super::models::{
    BookRequest, BookResponse, ErrorResponse, MovieRequest, MovieResponse, SearchResponse,
    StatsResponse, TVSeriesRequest, TVSeriesResponse,
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

/// Create a CORS layer with the specified origins
///
/// Configures CORS to allow specified methods and headers with explicit origin validation.
/// Uses `CorsLayer::new()` as a base and adds each origin explicitly to support credentials.
/// This is the correct approach when `allow_credentials(true)` is needed.
///
/// # Arguments
/// * `origins` - List of allowed origins to explicitly allow
///
/// # Returns
/// A configured CorsLayer with explicit origin allowlist and credentials support
fn create_cors_layer(origins: Vec<String>) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([CONTENT_TYPE, ACCEPT])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600));

    // Add origins explicitly
    for origin in origins {
        if let Ok(parsed) = origin.parse::<HeaderValue>() {
            cors = cors.allow_origin(parsed);
        }
    }

    cors
}

/// OpenAPI documentation for the Tana API
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::movies::list_movies,
        handlers::movies::get_movie,
        handlers::movies::create_movie,
        handlers::movies::update_movie,
        handlers::movies::delete_movie,
        handlers::series::list_series,
        handlers::series::get_series,
        handlers::series::create_series,
        handlers::series::update_series,
        handlers::series::delete_series,
        handlers::books::list_books,
        handlers::books::get_book,
        handlers::books::create_book,
        handlers::books::update_book,
        handlers::books::delete_book,
        handlers::stats::stats_handler,
        handlers::search::search_handler,
    ),
    components(
        schemas(
            MovieRequest,
            MovieResponse,
            TVSeriesRequest,
            TVSeriesResponse,
            BookRequest,
            BookResponse,
            StatsResponse,
            SearchResponse,
            ErrorResponse,
        )
    ),
    info(
        title = "Tana API",
        description = "A REST API for managing movies, TV series, and books with search and statistics capabilities.",
        version = "0.1.0",
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
    ),
    tags(
        (name = "Movies", description = "Operations for managing movies"),
        (name = "TV Series", description = "Operations for managing TV series"),
        (name = "Books", description = "Operations for managing books"),
        (name = "Statistics", description = "API statistics"),
        (name = "Search", description = "Search across all media types"),
    )
)]
pub struct ApiDoc;

/// Create the main router with all API routes
///
/// Sets up all routes with shared application state.
/// Routes are mounted under /api path.
/// Swagger UI is available at /api/docs with OpenAPI JSON at /api/docs/openapi.json
/// CORS is configured with default origins (localhost:3000 and localhost:8080)
pub fn create_router(db_path: PathBuf) -> Router {
    let state = AppState::new(db_path);

    // Build the API routes with shared state
    let api_routes = Router::new()
        // Movie routes: GET/POST /api/movies, GET/PUT/DELETE /api/movies/:id
        .route(
            "/movies",
            get(handlers::list_movies).post(handlers::create_movie),
        )
        .route(
            "/movies/{id}",
            get(handlers::get_movie)
                .put(handlers::update_movie)
                .delete(handlers::delete_movie),
        )
        // TV Series routes: GET/POST /api/series, GET/PUT/DELETE /api/series/:id
        .route(
            "/series",
            get(handlers::list_series).post(handlers::create_series),
        )
        .route(
            "/series/{id}",
            get(handlers::get_series)
                .put(handlers::update_series)
                .delete(handlers::delete_series),
        )
        // Book routes: GET/POST /api/books, GET/PUT/DELETE /api/books/:id
        .route(
            "/books",
            get(handlers::list_books).post(handlers::create_book),
        )
        .route(
            "/books/{id}",
            get(handlers::get_book)
                .put(handlers::update_book)
                .delete(handlers::delete_book),
        )
        // Statistics route: GET /api/stats
        .route("/stats", get(handlers::stats_handler))
        // Search route: GET /api/search?q=query
        .route("/search", get(handlers::search_handler))
        .with_state(state);

    // Create CORS layer with default origins
    let cors_origins = vec![
        "http://localhost:3000".to_string(),
        "http://localhost:8080".to_string(),
    ];
    let cors_layer = create_cors_layer(cors_origins);

    // Build the main router with the API routes nested under /api
    // Apply CORS layer before nesting routes
    // and Swagger UI mounted at /api/docs
    Router::new()
        .nest("/api", api_routes)
        .layer(cors_layer)
        .merge(SwaggerUi::new("/api/docs").url("/api/docs/openapi.json", ApiDoc::openapi()))
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

    #[test]
    fn test_cors_layer_creation_single_origin() {
        let origins = vec!["http://localhost:3000".to_string()];
        let _cors = create_cors_layer(origins);
        // If we get here without panicking, the CORS layer was created successfully
    }

    #[test]
    fn test_cors_layer_creation_multiple_origins() {
        let origins = vec![
            "http://localhost:3000".to_string(),
            "http://localhost:8080".to_string(),
            "https://example.com".to_string(),
        ];
        let _cors = create_cors_layer(origins);
        // If we get here without panicking, the CORS layer was created successfully
    }

    #[test]
    fn test_cors_layer_creation_empty_origins() {
        let origins = vec![];
        let _cors = create_cors_layer(origins);
        // If we get here without panicking, the CORS layer was created with empty origins
    }

    #[test]
    fn test_create_router_includes_cors() {
        // Test that the router can be created and includes CORS configuration
        let path = PathBuf::from("/tmp/test.db");
        let _router = create_router(path);
        // If we get here without panicking, the router with CORS was created successfully
    }
}
