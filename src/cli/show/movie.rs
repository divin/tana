//! Movie show command implementation
//!
//! This module handles displaying movies with filtering, sorting, and formatting options.

use clap::Args;
use serde::Serialize;

use crate::db::Database;
use crate::db::models::Movie;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

/// Arguments for showing movies
#[derive(Args, Debug)]
pub struct MoviesShowArgs {
    /// Filter by year
    #[arg(long)]
    pub year: Option<i32>,

    /// Filter by director
    #[arg(long)]
    pub director: Option<String>,

    /// Minimum rating threshold (1-10)
    #[arg(long)]
    pub min_rating: Option<f64>,

    /// Sort by field
    #[arg(long)]
    pub sort: Option<String>,

    /// Sort order
    #[arg(long)]
    pub order: Option<String>,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<i32>,

    /// Output format (plain, json, csv)
    #[arg(long, default_value = "plain")]
    pub format: String,
}

/// Movie entry for serialization
#[derive(Serialize, Debug)]
struct MovieEntry {
    id: i32,
    title: String,
    year: Option<i32>,
    director: Option<String>,
    rating: Option<f64>,
    watched_date: String,
    notes: Option<String>,
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
        }
    }
}

/// Execute the show movies command
pub fn execute(db: &Database, args: MoviesShowArgs) -> Result<()> {
    let conn = db.connection();
    let mut movies = queries::movies::get_all(conn, None)?;

    // Apply filters
    if let Some(year) = args.year {
        movies.retain(|m| m.release_year == Some(year));
    }

    if let Some(director) = &args.director {
        let director_lower = director.to_lowercase();
        movies.retain(|m| {
            m.director
                .as_ref()
                .map(|d| d.to_lowercase().contains(&director_lower))
                .unwrap_or(false)
        });
    }

    if let Some(min_rating) = args.min_rating {
        movies.retain(|m| m.rating.is_some_and(|r| r >= min_rating));
    }

    // Apply sorting
    if let Some(sort_by) = args.sort {
        let order = args.order.as_deref().unwrap_or("asc");
        sort_movies(&mut movies, &sort_by, order);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        movies.truncate(limit as usize);
    }

    // Format output
    let format_str = args.format.to_lowercase();
    let format = format_str.parse::<Format>()?;

    match format {
        Format::Plain => display_plain(&movies),
        Format::Json => display_json(&movies)?,
        Format::Csv => display_csv(&movies),
    }

    Ok(())
}

/// Display movies in plain text format
fn display_plain(movies: &[Movie]) {
    if movies.is_empty() {
        println!("No movies found.");
        return;
    }

    println!("\n{:=^100}", " Movies ");
    println!(
        "{:<4} {:<40} {:<8} {:<20} {:<8}",
        "ID", "Title", "Year", "Director", "Rating"
    );
    println!("{}", "=".repeat(100));

    for movie in movies {
        let title = truncate(&movie.title, 38);
        let director = movie
            .director
            .as_ref()
            .map(|d| truncate(d, 18))
            .unwrap_or_else(|| "—".to_string());
        let rating = movie
            .rating
            .map(|r| format!("{}/10", r))
            .unwrap_or_else(|| "—".to_string());
        let year = movie
            .release_year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_string());

        println!(
            "{:<4} {:<40} {:<8} {:<20} {:<8}",
            movie.id.unwrap_or(0),
            title,
            year,
            director,
            rating
        );
    }
    println!();
}

/// Display movies in JSON format
fn display_json(movies: &[Movie]) -> Result<()> {
    let entries: Vec<MovieEntry> = movies.iter().map(|m| m.clone().into()).collect();
    let json = serde_json::to_string_pretty(&entries)?;
    println!("{}", json);
    Ok(())
}

/// Display movies in CSV format
fn display_csv(movies: &[Movie]) {
    use super::format::escape_csv;

    println!("ID,Title,Year,Director,Rating,WatchedDate,Notes");
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

        println!(
            "{},{},{},{},{},{},{}",
            movie.id.unwrap_or(0),
            title,
            year,
            director,
            rating,
            escape_csv(&movie.watched_date),
            notes
        );
    }
}

/// Sort movies by specified field and order
fn sort_movies(movies: &mut [Movie], sort_by: &str, order: &str) {
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
fn truncate(s: &str, max_len: usize) -> String {
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
    fn test_movies_show_args() {
        let args = MoviesShowArgs {
            year: Some(2020),
            director: Some("Nolan".to_string()),
            min_rating: Some(8.0),
            sort: Some("rating".to_string()),
            order: Some("desc".to_string()),
            limit: Some(10),
            format: "json".to_string(),
        };

        assert_eq!(args.year, Some(2020));
        assert_eq!(args.limit, Some(10));
    }
}
