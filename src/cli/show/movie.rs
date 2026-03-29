//! Movie show command implementation
//!
//! This module handles displaying movies with filtering, sorting, and formatting options.
//! It serves as a hub that orchestrates display and sorting logic through submodules.

pub mod display;
pub mod sort;

use clap::Args;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

// Re-export MovieEntry for public API
pub use display::MovieEntry;

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

/// Execute the show movies command
///
/// Fetches movies from the database, applies filters, sorting, and formatting based on arguments.
///
/// # Arguments
/// * `ctx` - Application context with database connection
/// * `args` - Command arguments for filtering and formatting
///
/// # Returns
/// `Ok(())` on success, or an error if database or I/O operations fail
pub fn execute(ctx: &AppContext, args: MoviesShowArgs) -> Result<()> {
    let conn = ctx.db().connection();
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
        sort::sort_movies(&mut movies, &sort_by, order);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        movies.truncate(limit as usize);
    }

    // Format output
    let format_str = args.format.to_lowercase();
    let format = format_str.parse::<Format>()?;
    let truncate_length = ctx.truncate_length();

    match format {
        Format::Plain => display::display_plain(&movies, truncate_length),
        Format::Json => display::display_json(&movies)?,
        Format::Csv => display::display_csv(&movies, truncate_length),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
