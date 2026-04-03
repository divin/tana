//! Statistics handler for the REST API
//!
//! This module contains the handler function for the statistics endpoint.
//! It counts and aggregates media items across all types.

use crate::db::Database;
use crate::db::queries::{books, movies, tv_series};
use crate::server::handlers::error::ApiError;
use crate::server::models::StatsResponse;
use crate::server::routes::AppState;
use axum::{Json, extract::State};
use tracing::{debug, error};

/// Get statistics about all media
#[utoipa::path(
    get,
    path = "/api/stats",
    responses(
        (status = 200, description = "Statistics retrieved successfully", body = StatsResponse),
    ),
    tag = "Statistics"
)]
pub async fn stats_handler(State(state): State<AppState>) -> Result<Json<StatsResponse>, ApiError> {
    debug!("Fetching statistics");

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let movie_count = movies::count(db.connection()).map_err(|e| {
        error!("Failed to count movies: {}", e);
        ApiError::internal_server_error("Failed to count movies")
    })?;

    let series_count = tv_series::count(db.connection()).map_err(|e| {
        error!("Failed to count TV series: {}", e);
        ApiError::internal_server_error("Failed to count TV series")
    })?;

    let book_count = books::count(db.connection()).map_err(|e| {
        error!("Failed to count books: {}", e);
        ApiError::internal_server_error("Failed to count books")
    })?;

    let total = movie_count + series_count + book_count;

    Ok(Json(StatsResponse {
        total_movies: movie_count,
        total_series: series_count,
        total_books: book_count,
        total_count: total,
    }))
}
