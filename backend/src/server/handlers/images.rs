//! Handler for serving image files
//!
//! This module provides an endpoint to serve image files stored in the configured images directory.
//! Supports both modern relative filenames and legacy absolute paths for backward compatibility.

use super::super::routes::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::path::PathBuf;

/// Serve an image file from the configured images directory
///
/// # Arguments
/// * `filename` - The name of the image file to serve (can be a relative filename or legacy absolute path)
/// * `state` - Application state containing configuration
///
/// # Returns
/// The image file contents with appropriate Content-Type header, or an error status
///
/// # Security
/// Prevents directory traversal attacks by validating relative filenames.
/// For legacy absolute paths, extracts the filename component safely.
pub async fn serve_image(
    Path(filename): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let images_dir = state.config.images_default_directory();

    // Handle relative filenames (modern format: "book_123_cover.jpg")
    if !filename.contains("..") && !filename.contains("/") && !filename.contains("\\") {
        let file_path = images_dir.join(&filename);

        // Read the file
        match tokio::fs::read(&file_path).await {
            Ok(contents) => {
                // Determine content type based on file extension
                let content_type = get_content_type(&filename);
                return Ok(
                    ([(axum::http::header::CONTENT_TYPE, content_type)], contents).into_response(),
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Fall through to legacy path handling
            }
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    // Handle legacy absolute paths (e.g., "/home/user/.../image.jpg")
    // Extract just the filename from the absolute path for safety
    let extracted_filename = extract_filename_from_path(&filename);
    let file_path = images_dir.join(&extracted_filename);

    // Read the file using the extracted filename
    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            // Determine content type based on file extension
            let content_type = get_content_type(&extracted_filename);
            Ok(([(axum::http::header::CONTENT_TYPE, content_type)], contents).into_response())
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Extract the filename component from a file path
///
/// # Arguments
/// * `path` - A file path that may be absolute or relative
///
/// # Returns
/// The filename (final path component) with any directory components removed
fn extract_filename_from_path(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Determine the content type based on file extension
fn get_content_type(filename: &str) -> &'static str {
    match filename.split('.').next_back().unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
}
