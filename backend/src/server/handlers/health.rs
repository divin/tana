//! Health check handler for the REST API
//!
//! This module contains the handler function for the health endpoint.
//! It provides a simple health check without database operations.

use crate::server::models::HealthResponse;
use axum::Json;

/// Get health status of the service
///
/// A simple health check endpoint that doesn't require database access.
/// Returns the current service status and version.
/// Useful for monitoring and load balancers to verify the service is running.
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "Health"
)]
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler_returns_healthy() {
        let response = health_handler().await;
        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_health_handler_json_serialization() {
        let response = health_handler().await;
        // Json<T> wraps the inner type, access it via .0 to serialize
        let json = serde_json::to_string(&response.0).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }
}
