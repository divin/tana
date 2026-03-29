//! Search handler for the REST API
//!
//! This module contains the handler function for searching across all media types.
//! Search queries are matched against titles, directors, authors, and notes.

use crate::db::Database;
use crate::db::queries::{books, movies, tv_series};
use crate::server::handlers::error::ApiError;
use crate::server::models::SearchResponse;
use crate::server::routes::AppState;
use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use tracing::{debug, error};

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Query string to search for across all media types
    pub q: String,
}

/// Search across all media types
pub async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResponse>>, ApiError> {
    debug!("Searching for: {}", params.q);

    let db = Database::open(state.db_path.as_ref()).map_err(|e| {
        error!("Failed to open database: {}", e);
        ApiError::internal_server_error("Failed to open database")
    })?;

    let query = params.q.to_lowercase();

    let mut results = Vec::new();

    // Search movies
    if let Ok(all_movies) = movies::get_all(db.connection(), None) {
        for movie in all_movies {
            if movie.title.to_lowercase().contains(&query)
                || movie
                    .director
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query))
                    .unwrap_or(false)
                || movie
                    .notes
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&query))
                    .unwrap_or(false)
            {
                results.push(SearchResponse::Movie(movie.into()));
            }
        }
    }

    // Search TV series
    if let Ok(all_series) = tv_series::get_all(db.connection(), None) {
        for series in all_series {
            if series.title.to_lowercase().contains(&query)
                || series
                    .notes
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&query))
                    .unwrap_or(false)
            {
                results.push(SearchResponse::Series(series.into()));
            }
        }
    }

    // Search books
    if let Ok(all_books) = books::get_all(db.connection(), None) {
        for book in all_books {
            if book.title.to_lowercase().contains(&query)
                || book.author.to_lowercase().contains(&query)
                || book
                    .genre
                    .as_ref()
                    .map(|g| g.to_lowercase().contains(&query))
                    .unwrap_or(false)
                || book
                    .notes
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&query))
                    .unwrap_or(false)
            {
                results.push(SearchResponse::Book(book.into()));
            }
        }
    }

    Ok(Json(results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_parsing() {
        let json = r#"{"q": "inception"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.q, "inception");
    }
}
