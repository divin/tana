//! Health models for the REST API
//!
//! This module defines the response type for the health check endpoint.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// Health check response showing service status and version
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Status of the service
    pub status: String,
    /// Version of the service
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
        };

        assert_eq!(health.status, "healthy");
        assert_eq!(health.version, "0.1.0");
    }

    #[test]
    fn test_health_response_serialization() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_value(&health).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["version"], "0.1.0");
    }
}
