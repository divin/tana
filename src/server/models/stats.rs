//! Statistics models for the REST API
//!
//! This module defines the response type for the statistics endpoint.
//! Types implement Serialize and Deserialize for automatic JSON conversion.

use serde::{Deserialize, Serialize};

/// Statistics response showing counts for each media type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_movies: i64,
    pub total_series: i64,
    pub total_books: i64,
    pub total_media: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_response() {
        let stats = StatsResponse {
            total_movies: 10,
            total_series: 5,
            total_books: 20,
            total_media: 35,
        };

        assert_eq!(stats.total_media, 35);
    }
}
