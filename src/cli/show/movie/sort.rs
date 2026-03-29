//! Movie sorting and string truncation utilities
//!
//! Handles sorting movies by various fields and utility functions for display formatting.

use crate::db::models::Movie;

/// Sort movies by specified field and order
///
/// # Arguments
/// * `movies` - Mutable slice of movies to sort
/// * `sort_by` - Field to sort by: "title", "rating", "date", or "year"
/// * `order` - Sort order: "asc" for ascending, "desc" for descending
///
/// # Behavior
/// - Title sorts alphabetically
/// - Rating sorts numerically (missing ratings treated as 0.0)
/// - Date sorts by watched_date string
/// - Year sorts numerically (missing years treated as 0)
/// - Unknown sort fields result in no sorting
/// - Descending order reverses the sorted list
pub fn sort_movies(movies: &mut [Movie], sort_by: &str, order: &str) {
    match sort_by.to_lowercase().as_str() {
        "title" => movies.sort_by(|a, b| a.title.cmp(&b.title)),
        "rating" => movies.sort_by(|a, b| {
            let a_rating = a.rating.unwrap_or(0.0);
            let b_rating = b.rating.unwrap_or(0.0);
            a_rating
                .partial_cmp(&b_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "date" => movies.sort_by(|a, b| a.watched_date.cmp(&b.watched_date)),
        "year" => movies.sort_by(|a, b| {
            let a_year = a.release_year.unwrap_or(0);
            let b_year = b.release_year.unwrap_or(0);
            a_year.cmp(&b_year)
        }),
        _ => {} // No sorting
    }

    if order.to_lowercase() == "desc" {
        movies.reverse();
    }
}

/// Truncate string to specified length with ellipsis
///
/// # Arguments
/// * `s` - String to truncate
/// * `max_len` - Maximum length (including the ellipsis character)
///
/// # Returns
/// Truncated string with "…" suffix if it exceeds max_len, otherwise the original string
///
/// # Examples
/// ```ignore
/// assert_eq!(truncate("Hello World", 5), "Hell…");
/// assert_eq!(truncate("Hi", 5), "Hi");
/// ```
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
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("Hello", 5), "Hello");
    }

    #[test]
    fn test_sort_movies_by_title() {
        let mut movies = vec![
            Movie {
                id: Some(1),
                title: "Zebra".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(8.0),
                watched_date: "2024-01-01".to_string(),
                notes: None,
                poster_path: None,
            },
            Movie {
                id: Some(2),
                title: "Alpha".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(7.0),
                watched_date: "2024-01-02".to_string(),
                notes: None,
                poster_path: None,
            },
        ];

        sort_movies(&mut movies, "title", "asc");
        assert_eq!(movies[0].title, "Alpha");
        assert_eq!(movies[1].title, "Zebra");
    }

    #[test]
    fn test_sort_movies_descending() {
        let mut movies = vec![
            Movie {
                id: Some(1),
                title: "A".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(8.0),
                watched_date: "2024-01-01".to_string(),
                notes: None,
                poster_path: None,
            },
            Movie {
                id: Some(2),
                title: "B".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(7.0),
                watched_date: "2024-01-02".to_string(),
                notes: None,
                poster_path: None,
            },
        ];

        sort_movies(&mut movies, "title", "desc");
        assert_eq!(movies[0].title, "B");
        assert_eq!(movies[1].title, "A");
    }

    #[test]
    fn test_sort_movies_by_rating() {
        let mut movies = vec![
            Movie {
                id: Some(1),
                title: "A".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(5.0),
                watched_date: "2024-01-01".to_string(),
                notes: None,
                poster_path: None,
            },
            Movie {
                id: Some(2),
                title: "B".to_string(),
                release_year: Some(2020),
                director: None,
                rating: Some(9.0),
                watched_date: "2024-01-02".to_string(),
                notes: None,
                poster_path: None,
            },
        ];

        sort_movies(&mut movies, "rating", "asc");
        assert_eq!(movies[0].rating, Some(5.0));
        assert_eq!(movies[1].rating, Some(9.0));
    }
}
