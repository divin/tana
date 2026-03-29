//! Error models for the REST API
//!
//! This module defines the response type for API errors.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// Error response for API errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: String, status: u16) -> Self {
        Self {
            error,
            status,
            details: None,
        }
    }

    /// Add details to the error response
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response() {
        let error = ErrorResponse::new("Not found".to_string(), 404)
            .with_details("Movie with id 1 not found".to_string());

        assert_eq!(error.status, 404);
        assert_eq!(error.error, "Not found");
        assert!(error.details.is_some());
    }
}
