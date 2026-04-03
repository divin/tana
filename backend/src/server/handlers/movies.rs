//! Movie handler functions for the REST API
//!
//! This module contains all handler functions for movie endpoints.
//! Handlers convert database operations to HTTP responses.

use crate::db::Database;
use crate::db::models::Movie;
use crate::db::queries::movies;
use crate::server::handlers::error::ApiError;
use crate::server::models::MovieRequest;
use crate::server::models::MovieResponse;
use crate::server::routes::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{debug, error};

/// List all movies
#[utoipa::path(
    get,
    path = "/api/movies",
    responses(
        (status = 200, description = "List of all movies", body = Vec<MovieResponse>),
    ),
    tag = "Movies"
)]
pub async fn list_movies(
    State(state): State<AppState>,
) -> Result<Json<Vec<MovieResponse>>, ApiError> {
    debug!("Listing all movies");

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let movies = movies::get_all(db.connection(), None).map_err(|e| {
        error!("Failed to fetch movies: {}", e);
        ApiError::internal_server_error(format!("Failed to fetch movies: {}", e))
    })?;

    let responses = movies.into_iter().map(|m| m.into()).collect();
    Ok(Json(responses))
}

/// Get a single movie by ID
#[utoipa::path(
    get,
    path = "/api/movies/{id}",
    responses(
        (status = 200, description = "Movie found", body = MovieResponse),
        (status = 404, description = "Movie not found"),
    ),
    params(
        ("id" = i32, Path, description = "Movie ID")
    ),
    tag = "Movies"
)]
pub async fn get_movie(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<MovieResponse>, ApiError> {
    debug!("Getting movie with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let movie = movies::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch movie {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch movie: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("Movie not found"))?;

    Ok(Json(movie.into()))
}

/// Create a new movie
#[utoipa::path(
    post,
    path = "/api/movies",
    request_body = MovieRequest,
    responses(
        (status = 201, description = "Movie created successfully", body = MovieResponse),
        (status = 400, description = "Invalid request body"),
    ),
    tag = "Movies"
)]
pub async fn create_movie(
    State(state): State<AppState>,
    Json(mut req): Json<MovieRequest>,
) -> Result<(StatusCode, Json<MovieResponse>), ApiError> {
    debug!("Creating movie: {}", req.title);

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

    let movie: Movie = req.into();
    let mut final_movie = movie.clone();

    if let Some(temp_path) = &temp_poster_path {
        final_movie.poster_path = Some(temp_path.clone());
    }

    let _id = movies::insert(db.connection(), &final_movie).map_err(|e| {
        error!("Failed to create movie: {}", e);
        ApiError::bad_request(format!("Failed to create movie: {}", e))
    })?;

    // Finalize the image filename with rule-based naming if an image was provided
    if let Some(temp_path) = temp_poster_path {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_path, "movie", _id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                final_movie.poster_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = movies::update(db.connection(), _id, &final_movie) {
                    error!("Failed to update movie with finalized image: {}", e);
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

    let mut created = final_movie;
    created.id = Some(_id);

    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Update an existing movie
#[utoipa::path(
    put,
    path = "/api/movies/{id}",
    request_body = MovieRequest,
    responses(
        (status = 200, description = "Movie updated successfully", body = MovieResponse),
        (status = 404, description = "Movie not found"),
    ),
    params(
        ("id" = i32, Path, description = "Movie ID")
    ),
    tag = "Movies"
)]
pub async fn update_movie(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(mut req): Json<MovieRequest>,
) -> Result<Json<MovieResponse>, ApiError> {
    debug!("Updating movie with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    // Check if movie exists
    let _existing = movies::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch movie {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch movie: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("Movie not found"))?;

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

    let mut movie: Movie = req.into();
    movie.id = Some(id);

    if let Some(temp_path) = &temp_poster_path {
        movie.poster_path = Some(temp_path.clone());
    }

    movies::update(db.connection(), id, &movie).map_err(|e| {
        error!("Failed to update movie {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to update movie: {}", e))
    })?;

    // Finalize the image filename with rule-based naming if a new image was provided
    if let Some(temp_path) = temp_poster_path {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_path, "movie", id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                movie.poster_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = movies::update(db.connection(), id, &movie) {
                    error!("Failed to update movie with finalized image: {}", e);
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

    Ok(Json(movie.into()))
}

/// Delete a movie
#[utoipa::path(
    delete,
    path = "/api/movies/{id}",
    responses(
        (status = 204, description = "Movie deleted successfully"),
        (status = 404, description = "Movie not found"),
    ),
    params(
        ("id" = i32, Path, description = "Movie ID")
    ),
    tag = "Movies"
)]
pub async fn delete_movie(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    debug!("Deleting movie with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let deleted = movies::delete(db.connection(), id).map_err(|e| {
        error!("Failed to delete movie {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to delete movie: {}", e))
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Movie not found"))
    }
}
