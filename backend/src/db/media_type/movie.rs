//! Movie type definition and operations
//!
//! This module defines the Movie struct and implements the Media trait for movies.

use super::media::Media;
use serde::Serialize;

/// A movie entry in the database
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Movie {
    pub id: Option<i32>,
    pub title: String,
    pub release_year: Option<i32>,
    pub director: Option<String>,
    pub rating: Option<f64>,
    pub watched_date: String,
    pub notes: Option<String>,
    pub poster_path: Option<String>,
}

impl Movie {
    /// Create a new movie entry
    pub fn new(title: String, watched_date: String) -> Self {
        Movie {
            id: None,
            title,
            release_year: None,
            director: None,
            rating: None,
            watched_date,
            notes: None,
            poster_path: None,
        }
    }

    /// Add a release year
    pub fn with_year(mut self, year: i32) -> Self {
        self.release_year = Some(year);
        self
    }

    /// Add a director
    pub fn with_director(mut self, director: String) -> Self {
        self.director = Some(director);
        self
    }

    /// Add a rating
    pub fn with_rating(mut self, rating: f64) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }

    /// Add a poster path
    pub fn with_poster_path(mut self, poster_path: String) -> Self {
        self.poster_path = Some(poster_path);
        self
    }
}

impl Media for Movie {
    fn table_name() -> &'static str {
        "movies"
    }

    fn media_type_name() -> &'static str {
        "movie"
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn rating(&self) -> Option<f64> {
        self.rating
    }

    fn date_added(&self) -> &str {
        &self.watched_date
    }

    fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_creation() {
        let movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string())
            .with_rating(9.0);

        assert_eq!(movie.title, "Inception");
        assert_eq!(movie.release_year, Some(2010));
        assert_eq!(movie.director, Some("Christopher Nolan".to_string()));
        assert_eq!(movie.rating, Some(9.0));
    }

    #[test]
    fn test_movie_trait_impl() {
        let movie = Movie::new("Dune".to_string(), "2024-01-20".to_string());

        assert_eq!(Movie::table_name(), "movies");
        assert_eq!(Movie::media_type_name(), "movie");
        assert_eq!(movie.title(), "Dune");
        assert_eq!(movie.date_added(), "2024-01-20");
    }

    #[test]
    fn test_movie_with_poster_path() {
        let movie = Movie::new("Inception".to_string(), "2024-01-15".to_string())
            .with_year(2010)
            .with_director("Christopher Nolan".to_string())
            .with_poster_path("/images/posters/inception.jpg".to_string());

        assert_eq!(movie.title, "Inception");
        assert_eq!(
            movie.poster_path,
            Some("/images/posters/inception.jpg".to_string())
        );
    }

    #[test]
    fn test_movie_without_poster_path() {
        let movie = Movie::new("Dune".to_string(), "2024-01-20".to_string());

        assert_eq!(movie.poster_path, None);
    }
}
