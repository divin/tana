//! Sorting and truncation utilities for TV series
//!
//! This module provides functions for sorting TV series by various criteria
//! and utility functions for text formatting.

use crate::db::models::TVSeries;

/// Sort TV series by specified field and order
///
/// Supports sorting by the following fields:
/// - "title": Alphabetical sort by series title
/// - "rating": Numeric sort by rating (1-10 scale)
/// - "status": Sort by status (ongoing, completed, dropped, etc.)
/// - "date": Sort by started date
/// - "year": Sort by release year
/// - "progress": Sort by current season and episode progress
///
/// The `order` parameter controls sort direction:
/// - "asc" (default): Ascending order
/// - "desc": Descending order
pub fn sort_series(series_list: &mut [TVSeries], sort_by: &str, order: &str) {
    match sort_by.to_lowercase().as_str() {
        "title" => series_list.sort_by(|a, b| a.title.cmp(&b.title)),
        "rating" => series_list.sort_by(|a, b| {
            let a_rating = a.rating.unwrap_or(0.0);
            let b_rating = b.rating.unwrap_or(0.0);
            a_rating
                .partial_cmp(&b_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "status" => series_list.sort_by(|a, b| a.status.cmp(&b.status)),
        "date" => series_list.sort_by(|a, b| a.started_date.cmp(&b.started_date)),
        "year" => series_list.sort_by(|a, b| {
            let a_year = a.release_year.unwrap_or(0);
            let b_year = b.release_year.unwrap_or(0);
            a_year.cmp(&b_year)
        }),
        "progress" => series_list.sort_by(|a, b| {
            let a_progress = (
                a.current_season.unwrap_or(0),
                a.current_episode.unwrap_or(0),
            );
            let b_progress = (
                b.current_season.unwrap_or(0),
                b.current_episode.unwrap_or(0),
            );
            a_progress.cmp(&b_progress)
        }),
        _ => {} // No sorting
    }

    if order.to_lowercase() == "desc" {
        series_list.reverse();
    }
}

/// Truncate string to specified length with ellipsis
///
/// If the string is longer than `max_len`, it will be truncated and
/// have an ellipsis character appended.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5), "Hell…");
        assert_eq!(truncate("Hi", 5), "Hi");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("Exact", 5), "Exact");
    }

    #[test]
    fn test_truncate_long_string() {
        let long_str = "This is a very long TV series title that should be truncated";
        let truncated = truncate(long_str, 10);
        assert!(truncated.ends_with('…'));
        assert!(truncated.len() < long_str.len());
    }
}
