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
pub async fn create_book(
    State(state): State<AppState>,
    Json(req): Json<BookRequest>,
) -> Result<(StatusCode, Json<BookResponse>), ApiError> {
    debug!("Creating book: {}", req.title);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let book: Book = req.into();

    let _id = books::insert(db.connection(), &book).map_err(|e| {
        error!("Failed to create book: {}", e);
        ApiError::bad_request(format!("Failed to create book: {}", e))
    })?;

    let mut created = book;
    created.id = Some(_id);

    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Update an existing book
pub async fn update_book(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<BookRequest>,
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

    let mut book: Book = req.into();
    book.id = Some(id);

    books::update(db.connection(), id, &book).map_err(|e| {
        error!("Failed to update book {}: {}", id, e);
        ApiError::internal_server_error(format!("Failed to update book: {}", e))
    })?;

    Ok(Json(book.into()))
}

/// Delete a book
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
