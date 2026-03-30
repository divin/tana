//! TV Series models for the REST API
//!
//! This module defines the request and response types for TV series endpoints.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// TV Series response for GET requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TVSeriesResponse {
    pub id: i32,
    pub title: String,
    pub release_year: Option<i32>,
    pub status: String,
    pub total_seasons: Option<i32>,
    pub current_season: Option<i32>,
    pub current_episode: Option<i32>,
    pub rating: Option<f64>,
    pub started_date: String,
    pub completed_date: Option<String>,
    pub notes: Option<String>,
    pub poster_path: Option<String>,
}

/// TV Series request for POST/PUT operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TVSeriesRequest {
    pub title: String,
    #[serde(default)]
    pub release_year: Option<i32>,
    pub status: String,
    #[serde(default)]
    pub total_seasons: Option<i32>,
    #[serde(default)]
    pub current_season: Option<i32>,
    #[serde(default)]
    pub current_episode: Option<i32>,
    #[serde(default)]
    pub rating: Option<f64>,
    pub started_date: String,
    #[serde(default)]
    pub completed_date: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub poster_path: Option<String>,
}

impl From<crate::db::models::TVSeries> for TVSeriesResponse {
    fn from(series: crate::db::models::TVSeries) -> Self {
        TVSeriesResponse {
            id: series.id.unwrap_or(0),
            title: series.title,
            release_year: series.release_year,
            status: series.status,
            total_seasons: series.total_seasons,
            current_season: series.current_season,
            current_episode: series.current_episode,
            rating: series.rating,
            started_date: series.started_date,
            completed_date: series.completed_date,
            notes: series.notes,
            poster_path: series.poster_path,
        }
    }
}

impl From<TVSeriesRequest> for crate::db::models::TVSeries {
    fn from(req: TVSeriesRequest) -> Self {
        crate::db::models::TVSeries {
            id: None,
            title: req.title,
            release_year: req.release_year,
            status: req.status,
            total_seasons: req.total_seasons,
            current_season: req.current_season,
            current_episode: req.current_episode,
            rating: req.rating,
            started_date: req.started_date,
            completed_date: req.completed_date,
            notes: req.notes,
            poster_path: req.poster_path,
        }
    }
}
