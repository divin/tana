//! Movie display formatting module
//!
//! Handles formatting and outputting movie data in multiple formats:
//! - Plain text table format
//! - JSON format
//! - CSV format

use serde::Serialize;

use crate::db::models::Movie;
use crate::error::Result;

/// Movie entry for serialization
#[derive(Serialize, Debug, Clone)]
pub struct MovieEntry {
    pub id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub director: Option<String>,
    pub rating: Option<f64>,
    pub watched_date: String,
    pub notes: Option<String>,
    pub poster_path: Option<String>,
}

impl From<Movie> for MovieEntry {
    fn from(movie: Movie) -> Self {
        MovieEntry {
            id: movie.id.unwrap_or(0),
            title: movie.title,
            year: movie.release_year,
            director: movie.director,
            rating: movie.rating,
            watched_date: movie.watched_date,
            notes: movie.notes,
            poster_path: movie.poster_path.clone(),
        }
    }
}

/// Display movies in plain text format with a formatted table
///
/// # Arguments
/// * `movies` - Slice of movies to display
/// * `_truncate_length` - Length for truncating long fields (currently unused)
///
/// # Output
/// Prints a formatted table to stdout with columns for ID, Title, Year, Director, and Rating.
pub fn display_plain(movies: &[Movie], _truncate_length: usize) {
    if movies.is_empty() {
        println!("No movies found.");
        return;
    }

    println!("\n{:=^115}", " Movies ");
    println!(
        "{:<4} {:<40} {:<8} {:<20} {:<8} {:<12}",
        "ID", "Title", "Year", "Director", "Rating", "Poster"
    );
    println!("{}", "=".repeat(115));

    for movie in movies {
        let title = super::sort::truncate(&movie.title, 38);
        let director = movie
            .director
            .as_ref()
            .map(|d| super::sort::truncate(d, 18))
            .unwrap_or_else(|| "—".to_string());
        let rating = movie
            .rating
            .map(|r| format!("{}/10", r))
            .unwrap_or_else(|| "—".to_string());
        let year = movie
            .release_year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_string());
        let poster = movie
            .poster_path
            .as_ref()
            .map(|p| super::sort::truncate(p, 10))
            .unwrap_or_else(|| "N/A".to_string());

        println!(
            "{:<4} {:<40} {:<8} {:<20} {:<8} {:<12}",
            movie.id.unwrap_or(0),
            title,
            year,
            director,
            rating,
            poster
        );
    }
    println!();
}

/// Display movies in JSON format
///
/// # Arguments
/// * `movies` - Slice of movies to display
///
/// # Returns
/// `Ok(())` on success, `Err` if serialization fails
///
/// # Output
/// Prints prettified JSON to stdout
pub fn display_json(movies: &[Movie]) -> Result<()> {
    let entries: Vec<MovieEntry> = movies.iter().map(|m| m.clone().into()).collect();
    let json = serde_json::to_string_pretty(&entries)?;
    println!("{}", json);
    Ok(())
}

/// Display movies in CSV format
///
/// # Arguments
/// * `movies` - Slice of movies to display
/// * `_truncate_length` - Length for truncating long fields (currently unused)
///
/// # Output
/// Prints CSV format with headers and movie data to stdout
pub fn display_csv(movies: &[Movie], _truncate_length: usize) {
    use super::super::format::escape_csv;

    println!("ID,Title,Year,Director,Rating,WatchedDate,Notes,PosterPath");
    for movie in movies {
        let title = escape_csv(&movie.title);
        let director = movie
            .director
            .as_ref()
            .map(|d| escape_csv(d))
            .unwrap_or_default();
        let year = movie
            .release_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let rating = movie.rating.map(|r| r.to_string()).unwrap_or_default();
        let notes = movie
            .notes
            .as_ref()
            .map(|n| escape_csv(n))
            .unwrap_or_default();
        let poster_path = movie
            .poster_path
            .as_ref()
            .map(|p| escape_csv(p))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{}",
            movie.id.unwrap_or(0),
            title,
            year,
            director,
            rating,
            escape_csv(&movie.watched_date),
            notes,
            poster_path
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_entry_from_movie() {
        use crate::db::models::Movie;

        let movie = Movie {
            id: Some(1),
            title: "Test Movie".to_string(),
            release_year: Some(2020),
            director: Some("Test Director".to_string()),
            rating: Some(8.5),
            watched_date: "2024-01-01".to_string(),
            notes: Some("Great!".to_string()),
            poster_path: None,
        };

        let entry: MovieEntry = movie.into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.title, "Test Movie");
        assert_eq!(entry.year, Some(2020));
        assert_eq!(entry.director, Some("Test Director".to_string()));
        assert_eq!(entry.rating, Some(8.5));
    }

    #[test]
    fn test_movie_entry_with_poster_path() {
        use crate::db::models::Movie;

        let movie = Movie {
            id: Some(2),
            title: "Poster Movie".to_string(),
            release_year: Some(2023),
            director: Some("Test Director".to_string()),
            rating: Some(7.5),
            watched_date: "2024-01-15".to_string(),
            notes: Some("Great poster!".to_string()),
            poster_path: Some("/images/posters/test.jpg".to_string()),
        };

        let entry: MovieEntry = movie.into();
        assert_eq!(entry.id, 2);
        assert_eq!(entry.title, "Poster Movie");
        assert_eq!(
            entry.poster_path,
            Some("/images/posters/test.jpg".to_string())
        );
    }
}
