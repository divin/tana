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
    Json(mut req): Json<TVSeriesRequest>,
) -> Result<(StatusCode, Json<TVSeriesResponse>), ApiError> {
    debug!("Creating TV series: {}", req.title);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    // Process poster image if provided (supports both URLs and local file paths)
    let temp_poster_path = if let Some(poster_input) = req.poster_path.take() {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let poster_input_owned = poster_input.clone();
        let images_dir_owned = images_dir_str.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::process_image_input(&poster_input_owned, &images_dir_owned)
        })
        .await;

        let processed_path = match result {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => {
                error!("Failed to process poster image: {}", e);
                return Err(ApiError::bad_request(format!(
                    "Failed to process poster image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                return Err(ApiError::internal_server_error("Failed to process image"));
            }
        };

        Some(processed_path)
    } else {
        None
    };

    let mut series: TVSeries = req.into();

    if let Some(temp_path) = &temp_poster_path {
        series.poster_path = Some(temp_path.clone());
    }

    let _id = tv_series::insert(db.connection(), &series).map_err(|e| {
        error!("Failed to create TV series: {}", e);
        ApiError::bad_request(format!("Failed to create TV series: {}", e))
    })?;

    // Finalize the image filename with rule-based naming if an image was provided
    if let Some(temp_path) = temp_poster_path {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_path, "series", _id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                series.poster_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = tv_series::update(db.connection(), _id, &series) {
                    error!("Failed to update series with finalized image: {}", e);
                    return Err(ApiError::internal_server_error(
                        "Failed to finalize image filename",
                    ));
                }
                debug!("Image finalized as: {}", final_filename);
            }
            Ok(Err(e)) => {
                error!("Failed to finalize image: {}", e);
                return Err(ApiError::internal_server_error(format!(
                    "Failed to finalize image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn finalize task: {}", e);
                return Err(ApiError::internal_server_error("Failed to finalize image"));
            }
        }
    }

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
    Json(mut req): Json<TVSeriesRequest>,
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

    // Process poster image if provided (supports both URLs and local file paths)
    let temp_poster_path = if let Some(poster_input) = req.poster_path.take() {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let poster_input_owned = poster_input.clone();
        let images_dir_owned = images_dir_str.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::process_image_input(&poster_input_owned, &images_dir_owned)
        })
        .await;

        let processed_path = match result {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => {
                error!("Failed to process poster image: {}", e);
                return Err(ApiError::bad_request(format!(
                    "Failed to process poster image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                return Err(ApiError::internal_server_error("Failed to process image"));
            }
        };

        Some(processed_path)
    } else {
        None
    };

    let mut series: TVSeries = req.into();
    series.id = Some(id);

    if let Some(temp_path) = &temp_poster_path {
        series.poster_path = Some(temp_path.clone());
    }

    tv_series::update(db.connection(), id, &series).map_err(|e| {
        error!("Failed to update TV series {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to update TV series: {}", e))
    })?;

    // Finalize the image filename with rule-based naming if a new image was provided
    if let Some(temp_path) = temp_poster_path {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_path, "series", id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                series.poster_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = tv_series::update(db.connection(), id, &series) {
                    error!("Failed to update series with finalized image: {}", e);
                    return Err(ApiError::internal_server_error(
                        "Failed to finalize image filename",
                    ));
                }
                debug!("Image finalized as: {}", final_filename);
            }
            Ok(Err(e)) => {
                error!("Failed to finalize image: {}", e);
                return Err(ApiError::internal_server_error(format!(
                    "Failed to finalize image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn finalize task: {}", e);
                return Err(ApiError::internal_server_error("Failed to finalize image"));
            }
        }
    }

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
