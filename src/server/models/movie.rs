//! Movie models for the REST API
//!
//! This module defines the request and response types for movie endpoints.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// Movie response for GET requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieResponse {
    pub id: i32,
    pub title: String,
    pub release_year: Option<i32>,
    pub director: Option<String>,
    pub rating: Option<f64>,
    pub watched_date: String,
    pub notes: Option<String>,
    pub poster_path: Option<String>,
}

/// Movie request for POST/PUT operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MovieRequest {
    pub title: String,
    #[serde(default)]
    pub release_year: Option<i32>,
    #[serde(default)]
    pub director: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    pub watched_date: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub poster_path: Option<String>,
}

impl From<crate::db::models::Movie> for MovieResponse {
    fn from(movie: crate::db::models::Movie) -> Self {
        MovieResponse {
            id: movie.id.unwrap_or(0),
            title: movie.title,
            release_year: movie.release_year,
            director: movie.director,
            rating: movie.rating,
            watched_date: movie.watched_date,
            notes: movie.notes,
            poster_path: movie.poster_path,
        }
    }
}

impl From<MovieRequest> for crate::db::models::Movie {
    fn from(req: MovieRequest) -> Self {
        crate::db::models::Movie {
            id: None,
            title: req.title,
            release_year: req.release_year,
            director: req.director,
            rating: req.rating,
            watched_date: req.watched_date,
            notes: req.notes,
            poster_path: req.poster_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_response_serialization() {
        let response = MovieResponse {
            id: 1,
            title: "Inception".to_string(),
            release_year: Some(2010),
            director: Some("Christopher Nolan".to_string()),
            rating: Some(9.0),
            watched_date: "2024-01-15".to_string(),
            notes: Some("Great movie".to_string()),
            poster_path: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Inception\""));
        assert!(json.contains("\"id\":1"));
    }
}
