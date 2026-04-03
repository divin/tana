//! Book handler functions for the REST API
//!
//! This module contains all handler functions for book endpoints.
//! Handlers convert database operations to HTTP responses.

use crate::db::Database;
use crate::db::models::Book;
use crate::db::queries::books;
use crate::server::handlers::error::ApiError;
use crate::server::models::BookRequest;
use crate::server::models::BookResponse;
use crate::server::routes::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{debug, error};

/// List all books
#[utoipa::path(
    get,
    path = "/api/books",
    responses(
        (status = 200, description = "List of all books", body = Vec<BookResponse>),
    ),
    tag = "Books"
)]
pub async fn list_books(
    State(state): State<AppState>,
) -> Result<Json<Vec<BookResponse>>, ApiError> {
    debug!("Listing all books");

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let books_list = books::get_all(db.connection(), None).map_err(|e| {
        error!("Failed to fetch books: {}", e);
        ApiError::internal_server_error(format!("Failed to fetch books: {}", e))
    })?;

    let responses = books_list.into_iter().map(|b| b.into()).collect();
    Ok(Json(responses))
}

/// Get a single book by ID
#[utoipa::path(
    get,
    path = "/api/books/{id}",
    responses(
        (status = 200, description = "Book found", body = BookResponse),
        (status = 404, description = "Book not found"),
    ),
    params(
        ("id" = i32, Path, description = "Book ID")
    ),
    tag = "Books"
)]
pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<BookResponse>, ApiError> {
    debug!("Getting book with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let book = books::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch book {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch book: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("Book not found"))?;

    Ok(Json(book.into()))
}

/// Create a new book
#[utoipa::path(
    post,
    path = "/api/books",
    request_body = BookRequest,
    responses(
        (status = 201, description = "Book created successfully", body = BookResponse),
        (status = 400, description = "Invalid request body"),
    ),
    tag = "Books"
)]
pub async fn create_book(
    State(state): State<AppState>,
    Json(mut req): Json<BookRequest>,
) -> Result<(StatusCode, Json<BookResponse>), ApiError> {
    debug!("Creating book: {}", req.title);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    // Process cover image if provided (supports both URLs and local file paths)
    let mut temp_cover_filename: Option<String> = None;
    if let Some(cover_input) = req.cover_path.take() {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let cover_input_owned = cover_input.clone();
        let images_dir_owned = images_dir_str.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::process_image_input(&cover_input_owned, &images_dir_owned)
        })
        .await;

        let processed_path = match result {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => {
                error!("Failed to process cover image: {}", e);
                return Err(ApiError::bad_request(format!(
                    "Failed to process cover image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                return Err(ApiError::internal_server_error("Failed to process image"));
            }
        };

        temp_cover_filename = Some(processed_path.clone());
        req.cover_path = Some(processed_path);
        debug!("Cover image processed successfully for new book");
    }

    let book: Book = req.into();

    let _id = books::insert(db.connection(), &book).map_err(|e| {
        error!("Failed to create book: {}", e);
        ApiError::bad_request(format!("Failed to create book: {}", e))
    })?;

    let mut created = book;
    created.id = Some(_id);

    // Finalize the image filename with the book ID if an image was provided
    if let Some(temp_filename) = temp_cover_filename {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_filename, "book", _id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                created.cover_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = books::update(db.connection(), _id, &created) {
                    error!("Failed to update book with finalized image: {}", e);
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

    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Update an existing book
#[utoipa::path(
    put,
    path = "/api/books/{id}",
    request_body = BookRequest,
    responses(
        (status = 200, description = "Book updated successfully", body = BookResponse),
        (status = 404, description = "Book not found"),
    ),
    params(
        ("id" = i32, Path, description = "Book ID")
    ),
    tag = "Books"
)]
pub async fn update_book(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(mut req): Json<BookRequest>,
) -> Result<Json<BookResponse>, ApiError> {
    debug!("Updating book with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    // Check if book exists
    let _existing = books::get_by_id(db.connection(), id)
        .map_err(|e| {
            error!("Failed to fetch book {}: {}", id, e);
            ApiError::internal_server_error(format!("Failed to fetch book: {}", e))
        })?
        .ok_or_else(|| ApiError::not_found("Book not found"))?;

    // Process cover image if provided (supports both URLs and local file paths)
    let mut temp_cover_filename: Option<String> = None;
    if let Some(cover_input) = req.cover_path.take() {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let cover_input_owned = cover_input.clone();
        let images_dir_owned = images_dir_str.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::process_image_input(&cover_input_owned, &images_dir_owned)
        })
        .await;

        let processed_path = match result {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => {
                error!("Failed to process cover image: {}", e);
                return Err(ApiError::bad_request(format!(
                    "Failed to process cover image: {}",
                    e
                )));
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                return Err(ApiError::internal_server_error("Failed to process image"));
            }
        };

        temp_cover_filename = Some(processed_path.clone());
        req.cover_path = Some(processed_path);
        debug!("Cover image processed successfully for book update");
    }

    let mut book: Book = req.into();
    book.id = Some(id);

    books::update(db.connection(), id, &book).map_err(|e| {
        error!("Failed to update book {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to update book: {}", e))
    })?;

    // Finalize the image filename with the book ID if an image was provided
    if let Some(temp_filename) = temp_cover_filename {
        let images_dir = state.config.images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::image::finalize_image(&images_dir_str, &temp_filename, "book", id)
        })
        .await;

        match result {
            Ok(Ok(final_filename)) => {
                book.cover_path = Some(final_filename.clone());
                // Update the database with the finalized filename
                if let Err(e) = books::update(db.connection(), id, &book) {
                    error!("Failed to update book with finalized image: {}", e);
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

    Ok(Json(book.into()))
}

/// Delete a book
#[utoipa::path(
    delete,
    path = "/api/books/{id}",
    responses(
        (status = 204, description = "Book deleted successfully"),
        (status = 404, description = "Book not found"),
    ),
    params(
        ("id" = i32, Path, description = "Book ID")
    ),
    tag = "Books"
)]
pub async fn delete_book(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    debug!("Deleting book with id: {}", id);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let deleted = books::delete(db.connection(), id).map_err(|e| {
        error!("Failed to delete book {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to delete book: {}", e))
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Book not found"))
    }
}
