//! Search models for the REST API
//!
//! This module defines the response type for search queries.
//! Search results are tagged enums that can contain any media type.

use super::book::BookResponse;
use super::movie::MovieResponse;
use super::series::TVSeriesResponse;
use serde::{Deserialize, Serialize};

/// Search result containing a media item (one of the three types)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type")]
pub enum SearchResponse {
    #[serde(rename = "movie")]
    Movie(MovieResponse),
    #[serde(rename = "series")]
    Series(TVSeriesResponse),
    #[serde(rename = "book")]
    Book(BookResponse),
}
