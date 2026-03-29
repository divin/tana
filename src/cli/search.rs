//! Search command for finding media entries
//!
//! This module handles the `tana search` command which allows users to search
//! across all media types (movies, TV series, books) for matching entries.

use clap::Args;
use tracing::debug;

use crate::db::Database;
use crate::db::queries;
use crate::error::Result;

/// Arguments for the search command
#[derive(Args, Debug)]
pub struct SearchCommand {
    /// Search query (searches titles, authors, directors, etc.)
    pub query: String,

    /// Limit number of results per media type
    #[arg(long, default_value = "10")]
    pub limit: i32,
}

impl SearchCommand {
    /// Execute the search command
    pub fn execute(self, db: &Database) -> Result<()> {
        debug!("Searching for: {}", self.query);

        let conn = db.connection();
        let query_lower = self.query.to_lowercase();

        // Search movies
        let all_movies = queries::movies::get_all(conn, None)?;
        let matching_movies: Vec<_> = all_movies
            .iter()
            .filter(|m| {
                m.title.to_lowercase().contains(&query_lower)
                    || m.director
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || m.notes
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .take(self.limit as usize)
            .collect();

        // Search TV series
        let all_series = queries::tv_series::get_all(conn, None)?;
        let matching_series: Vec<_> = all_series
            .iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query_lower)
                    || s.status.to_lowercase().contains(&query_lower)
                    || s.notes
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .take(self.limit as usize)
            .collect();

        // Search books
        let all_books = queries::books::get_all(conn, None)?;
        let matching_books: Vec<_> = all_books
            .iter()
            .filter(|b| {
                b.title.to_lowercase().contains(&query_lower)
                    || b.author.to_lowercase().contains(&query_lower)
                    || b.genre
                        .as_ref()
                        .map(|g| g.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || b.notes
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .take(self.limit as usize)
            .collect();

        // Display results
        let total_results = matching_movies.len() + matching_series.len() + matching_books.len();

        if total_results == 0 {
            println!("No results found for '{}'", self.query);
            return Ok(());
        }

        println!(
            "\n╔══════════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║ Search Results for \"{}\" ({} total)",
            self.query, total_results
        );
        println!(
            "╚══════════════════════════════════════════════════════════════════════════════════╝"
        );

        // Display movies
        if !matching_movies.is_empty() {
            println!("\n──────────── Movies ({})", matching_movies.len());
            for movie in matching_movies {
                println!(
                    "  [{}] {} ({})",
                    movie.id.unwrap_or(0),
                    movie.title,
                    movie
                        .release_year
                        .map_or("N/A".to_string(), |y| y.to_string())
                );
                if let Some(director) = &movie.director {
                    println!("       Director: {}", director);
                }
                if let Some(rating) = movie.rating {
                    println!("       Rating: {}/10", rating);
                }
            }
        }

        // Display TV series
        if !matching_series.is_empty() {
            println!("\n──────────── TV Series ({})", matching_series.len());
            for series in matching_series {
                println!(
                    "  [{}] {} ({})",
                    series.id.unwrap_or(0),
                    series.title,
                    series.status
                );
                if let Some(seasons) = series.total_seasons {
                    println!("       Seasons: {}", seasons);
                }
                if let Some(rating) = series.rating {
                    println!("       Rating: {}/10", rating);
                }
            }
        }

        // Display books
        if !matching_books.is_empty() {
            println!("\n──────────── Books ({})", matching_books.len());
            for book in matching_books {
                println!(
                    "  [{}] {} by {}",
                    book.id.unwrap_or(0),
                    book.title,
                    book.author
                );
                if let Some(genre) = &book.genre {
                    println!("       Genre: {}", genre);
                }
                if let Some(rating) = book.rating {
                    println!("       Rating: {}/10", rating);
                }
            }
        }

        println!();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_command_creation() {
        let cmd = SearchCommand {
            query: "inception".to_string(),
            limit: 10,
        };

        assert_eq!(cmd.query, "inception");
        assert_eq!(cmd.limit, 10);
    }

    #[test]
    fn test_search_command_custom_limit() {
        let cmd = SearchCommand {
            query: "test".to_string(),
            limit: 5,
        };

        assert_eq!(cmd.query, "test");
        assert_eq!(cmd.limit, 5);
    }

    #[test]
    fn test_search_query_case_insensitive() {
        let query = "Inception";
        let query_lower = query.to_lowercase();
        assert_eq!(query_lower, "inception");

        let title = "INCEPTION";
        assert!(title.to_lowercase().contains(&query_lower));
    }
}
