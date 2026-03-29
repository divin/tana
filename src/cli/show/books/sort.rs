//! Book sorting and utility functions
//!
//! This module provides functions for sorting books by various criteria and
//! utility functions for text manipulation.

use crate::db::models::Book;

/// Sort books by specified field and order
///
/// Sorts the books slice in-place according to the specified field and order.
/// If an unrecognized sort field is provided, the list remains unsorted.
///
/// Supported sort fields:
/// - "title": Sort alphabetically by book title
/// - "author": Sort alphabetically by author name
/// - "genre": Sort alphabetically by genre (unrated books sort first)
/// - "pages": Sort numerically by page count (books without page count sort first)
/// - "rating": Sort numerically by rating (books without rating are treated as 0)
/// - "date": Sort by completion date
///
/// # Arguments
///
/// * `books` - Mutable slice of books to sort
/// * `sort_by` - Field name to sort by (case-insensitive)
/// * `order` - Sort order: "asc" for ascending, "desc" for descending
///
/// # Examples
///
/// ```ignore
/// let mut books = vec![...];
/// sort_books(&mut books, "rating", "desc");
/// ```
pub fn sort_books(books: &mut [Book], sort_by: &str, order: &str) {
    match sort_by.to_lowercase().as_str() {
        "title" => books.sort_by(|a, b| a.title.cmp(&b.title)),
        "author" => books.sort_by(|a, b| a.author.cmp(&b.author)),
        "genre" => books.sort_by(|a, b| {
            let a_genre = a.genre.as_deref().unwrap_or("");
            let b_genre = b.genre.as_deref().unwrap_or("");
            a_genre.cmp(b_genre)
        }),
        "pages" => books.sort_by(|a, b| {
            let a_pages = a.pages.unwrap_or(0);
            let b_pages = b.pages.unwrap_or(0);
            a_pages.cmp(&b_pages)
        }),
        "rating" => books.sort_by(|a, b| {
            let a_rating = a.rating.unwrap_or(0.0);
            let b_rating = b.rating.unwrap_or(0.0);
            a_rating
                .partial_cmp(&b_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "date" => books.sort_by(|a, b| a.completed_date.cmp(&b.completed_date)),
        _ => {} // No sorting for unrecognized fields
    }

    if order.to_lowercase() == "desc" {
        books.reverse();
    }
}

/// Truncate string to specified length with ellipsis
///
/// If the string exceeds the maximum length, it will be truncated to
/// (max_len - 1) characters and an ellipsis character ("…") will be appended.
/// If the string is within the limit, it is returned unchanged.
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - The maximum total length including the ellipsis
///
/// # Examples
///
/// ```ignore
/// assert_eq!(truncate("Hello World", 5), "Hell…");
/// assert_eq!(truncate("Hi", 5), "Hi");
/// assert_eq!(truncate("Hello", 5), "Hello");
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

    // Tests for truncate function
    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("Hello World", 5), "Hell…");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("Hello", 5), "Hello");
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("Hi", 5), "Hi");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_truncate_one_character() {
        assert_eq!(truncate("A", 1), "A");
    }
}
