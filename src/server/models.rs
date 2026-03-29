//! Request and response models for the REST API
//!
//! This module defines the JSON request and response types used by the REST API.
//! All types implement Serialize and Deserialize for automatic JSON conversion.
//!
//! The module is split into organized submodules for each resource type and feature.

pub mod book;
pub mod error;
pub mod movie;
pub mod search;
pub mod series;
pub mod stats;

// Re-export public items for convenient access
pub use book::{BookRequest, BookResponse};
pub use error::ErrorResponse;
pub use movie::{MovieRequest, MovieResponse};
pub use search::SearchResponse;
pub use series::{TVSeriesRequest, TVSeriesResponse};
pub use stats::StatsResponse;
