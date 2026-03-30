//! Error handling for the REST API
//!
//! This module defines the ApiError type which is converted to HTTP responses
//! with appropriate status codes and JSON error bodies.

use crate::server::models::ErrorResponse;
use axum::{Json, http::StatusCode, response::IntoResponse};

/// Custom error type for API responses
///
/// Converts to JSON error responses with appropriate HTTP status codes.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    /// Create a bad request error (400)
    pub fn bad_request<S: Into<String>>(message: S) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    /// Create a not found error (404)
    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    /// Create an internal server error (500)
    pub fn internal_server_error<S: Into<String>>(message: S) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let error_response = ErrorResponse::new(self.message.clone(), self.status.as_u16());
        (self.status, Json(error_response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_bad_request() {
        let error = ApiError::bad_request("Invalid input");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::not_found("Not found");
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_internal_server() {
        let error = ApiError::internal_server_error("Server error");
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
