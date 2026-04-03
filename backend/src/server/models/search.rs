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

/// Grouped search results organized by media type
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GroupedSearchResults {
    /// Matching movies
    pub movies: Vec<MovieResponse>,
    /// Matching TV series
    pub series: Vec<TVSeriesResponse>,
    /// Matching books
    pub books: Vec<BookResponse>,
}
