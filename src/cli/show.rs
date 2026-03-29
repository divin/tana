//! Show command for displaying media entries
//!
//! This module handles the `tana show` command which displays
//! movies, TV series, and books from the database with optional filtering.

pub mod books;
pub mod format;
pub mod movie;
pub mod series;

use clap::{Args, Subcommand};
use tracing::debug;

use crate::db::Database;
use crate::error::Result;

pub use books::BooksShowArgs;
pub use format::{Format, escape_csv};
pub use movie::MoviesShowArgs;
pub use series::SeriesShowArgs;

/// Arguments for the show command
#[derive(Args, Debug)]
pub struct ShowCommand {
    #[command(subcommand)]
    pub subcommand: Option<ShowSubcommand>,

    /// Show only recent entries (number of days)
    #[arg(long)]
    pub recent: Option<u32>,

    /// Limit number of results
    #[arg(short, long)]
    pub limit: Option<i32>,
}

/// Subcommands for showing specific media types
#[derive(Subcommand, Debug)]
pub enum ShowSubcommand {
    /// Show movies
    Movies(movie::MoviesShowArgs),
    /// Show TV series
    #[command(name = "series")]
    Series(series::SeriesShowArgs),
    /// Show books
    Books(books::BooksShowArgs),
}

impl ShowCommand {
    /// Execute the show command
    pub fn execute(self, db: &Database) -> Result<()> {
        debug!("Executing show command");

        match self.subcommand {
            Some(ShowSubcommand::Movies(args)) => {
                movie::execute(db, args)?;
            }
            Some(ShowSubcommand::Series(args)) => {
                series::execute(db, args)?;
            }
            Some(ShowSubcommand::Books(args)) => {
                books::execute(db, args)?;
            }
            None => {
                // Show all media types
                show_all(db, self.recent, self.limit)?;
            }
        }

        Ok(())
    }
}

/// Show all media types (when no subcommand is specified)
fn show_all(db: &Database, _recent: Option<u32>, limit: Option<i32>) -> Result<()> {
    use crate::db::queries;

    let conn = db.connection();

    // Fetch all media
    let mut movies = queries::movies::get_all(conn, None)?;
    let mut series = queries::tv_series::get_all(conn, None)?;
    let mut books = queries::books::get_all(conn, None)?;

    // Apply limit if specified
    if let Some(limit) = limit {
        let limit = limit as usize;
        movies.truncate(limit);
        series.truncate(limit);
        books.truncate(limit);
    }

    // Display movies
    if !movies.is_empty() {
        println!("\n{:=^100}", " Movies ");
        for movie in &movies {
            println!(
                "[{}] {} ({})",
                movie.id.unwrap_or(0),
                movie.title,
                movie
                    .release_year
                    .map_or("N/A".to_string(), |y| y.to_string())
            );
            if let Some(director) = &movie.director {
                println!("     Director: {}", director);
            }
            if let Some(rating) = movie.rating {
                println!("     Rating: {}/10", rating);
            }
        }
    }

    // Display TV series
    if !series.is_empty() {
        println!("\n{:=^100}", " TV Series ");
        for s in &series {
            println!("[{}] {} ({})", s.id.unwrap_or(0), s.title, s.status);
            if let Some(seasons) = s.total_seasons {
                println!("     Seasons: {}", seasons);
            }
            if let Some(rating) = s.rating {
                println!("     Rating: {}/10", rating);
            }
        }
    }

    // Display books
    if !books.is_empty() {
        println!("\n{:=^100}", " Books ");
        for book in &books {
            println!(
                "[{}] {} by {}",
                book.id.unwrap_or(0),
                book.title,
                book.author
            );
            if let Some(genre) = &book.genre {
                println!("     Genre: {}", genre);
            }
            if let Some(rating) = book.rating {
                println!("     Rating: {}/10", rating);
            }
        }
    }

    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_command_creation() {
        let cmd = ShowCommand {
            subcommand: None,
            recent: None,
            limit: Some(10),
        };

        assert_eq!(cmd.limit, Some(10));
    }
}
