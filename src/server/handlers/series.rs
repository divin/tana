//! TV Series handler functions for the REST API
//!
//! This module contains all handler functions for TV series endpoints.
//! Handlers convert database operations to HTTP responses.

use crate::db::Database;
use crate::db::models::TVSeries;
use crate::db::queries::tv_series;
use crate::server::handlers::error::ApiError;
use crate::server::models::TVSeriesRequest;
use crate::server::models::TVSeriesResponse;
use crate::server::routes::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{debug, error};

/// List all TV series
#[utoipa::path(
    get,
    path = "/api/series",
    responses(
        (status = 200, description = "List of all TV series", body = Vec<TVSeriesResponse>),
    ),
    tag = "TV Series"
)]
pub async fn list_series(
    State(state): State<AppState>,
) -> Result<Json<Vec<TVSeriesResponse>>, ApiError> {
    debug!("Listing all TV series");

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let series = tv_series::get_all(db.connection(), None).map_err(|e| {
        error!("Failed to fetch TV series: {}", e);
        ApiError::internal_server_error(format!("Failed to fetch TV series: {}", e))
    })?;

    let responses = series.into_iter().map(|s| s.into()).collect();
    Ok(Json(responses))
}

/// Get a single TV series by ID
#[utoipa::path(
    get,
    path = "/api/series/{id}",
    responses(
        (status = 200, description = "TV series found", body = TVSeriesResponse),
        (status = 404, description = "TV series not found"),
    ),
    params(
        ("id" = i32, Path, description = "TV Series ID")
    ),
    tag = "TV Series"
)]
pub async fn get_series(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<TVSeriesResponse>, ApiError> {
    debug!("Getting TV series with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let series = tv_series::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch TV series {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch TV series: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("TV series not found"))?;

    Ok(Json(series.into()))
}

/// Create a new TV series
#[utoipa::path(
    post,
    path = "/api/series",
    request_body = TVSeriesRequest,
    responses(
        (status = 201, description = "TV series created successfully", body = TVSeriesResponse),
        (status = 400, description = "Invalid request body"),
    ),
    tag = "TV Series"
)]
pub async fn create_series(
    State(state): State<AppState>,
    Json(req): Json<TVSeriesRequest>,
) -> Result<(StatusCode, Json<TVSeriesResponse>), ApiError> {
    debug!("Creating TV series: {}", req.title);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let series: TVSeries = req.into();

    let _id = tv_series::insert(db.connection(), &series).map_err(|e| {
        error!("Failed to create TV series: {}", e);
        ApiError::bad_request(format!("Failed to create TV series: {}", e))
    })?;

    let mut created = series;
    created.id = Some(_id);

    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Update an existing TV series
#[utoipa::path(
    put,
    path = "/api/series/{id}",
    request_body = TVSeriesRequest,
    responses(
        (status = 200, description = "TV series updated successfully", body = TVSeriesResponse),
        (status = 404, description = "TV series not found"),
    ),
    params(
        ("id" = i32, Path, description = "TV Series ID")
    ),
    tag = "TV Series"
)]
pub async fn update_series(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<TVSeriesRequest>,
) -> Result<Json<TVSeriesResponse>, ApiError> {
    debug!("Updating TV series with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    // Check if series exists
    let _existing = tv_series::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch TV series {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch TV series: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("TV series not found"))?;

    let mut series: TVSeries = req.into();
    series.id = Some(id);

    tv_series::update(db.connection(), id, &series).map_err(|e| {
        error!("Failed to update TV series {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to update TV series: {}", e))
    })?;

    Ok(Json(series.into()))
}

/// Delete a TV series
#[utoipa::path(
    delete,
    path = "/api/series/{id}",
    responses(
        (status = 204, description = "TV series deleted successfully"),
        (status = 404, description = "TV series not found"),
    ),
    params(
        ("id" = i32, Path, description = "TV Series ID")
    ),
    tag = "TV Series"
)]
pub async fn delete_series(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    debug!("Deleting TV series with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let deleted = tv_series::delete(db.connection(), id).map_err(|e| {
        error!("Failed to delete TV series {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to delete TV series: {}", e))
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("TV series not found"))
    }
}
