//! Movie show command implementation
//!
//! This module handles displaying movies with filtering, sorting, and formatting options.
//! It serves as a hub that orchestrates display and sorting logic through submodules.
//!
//! # Image Path Display
//!
//! All output formats include the poster path (image file path) in the results:
//! - **Plain format**: Poster path is truncated to fit terminal width for readability
//! - **JSON format**: Includes `poster_path` field with full path
//! - **CSV format**: Includes `PosterPath` column with full path
//!
//! # Examples
//!
//! Show all movies sorted by rating in descending order:
//! ```ignore
//! tana show movies --sort rating --order desc
//! ```
//!
//! Filter movies by director and year, output as JSON:
//! ```ignore
//! tana show movies --director "Christopher Nolan" --year 2020 --format json
//! ```
//!
//! Show movies with minimum rating, limited to 10 results:
//! ```ignore
//! tana show movies --min-rating 8.0 --limit 10
//! ```
//!
//! Export movies by a specific director to CSV:
//! ```ignore
//! tana show movies --director "Quentin Tarantino" --format csv
//! ```

pub mod display;
pub mod sort;

use clap::Args;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

// Re-export MovieEntry for public API
pub use display::MovieEntry;

/// Arguments for showing movies with filtering, sorting, and formatting options
///
/// This struct represents the command-line arguments for the `show movies` command.
/// It supports filtering by year, director, and minimum rating, sorting by various fields,
/// and outputting results in different formats. All output formats include poster paths.
#[derive(Args, Debug)]
pub struct MoviesShowArgs {
    /// Filter by year (optional)
    ///
    /// Filters the movie list to only include movies released in the specified year.
    #[arg(long)]
    pub year: Option<i32>,

    /// Filter by director (optional)
    ///
    /// Filters the movie list to only include movies directed by someone whose name
    /// contains the specified string (case-insensitive).
    #[arg(long)]
    pub director: Option<String>,

    /// Minimum rating threshold on scale of 1-10 (optional)
    ///
    /// Filters the movie list to only include movies with a rating greater than
    /// or equal to the specified value.
    #[arg(long)]
    pub min_rating: Option<f64>,

    /// Sort by field: title, year, director, rating, watched_date (optional, default: title)
    ///
    /// Specifies which field to sort the results by. Valid options are:
    /// - title: Sort alphabetically by movie title
    /// - year: Sort numerically by release year
    /// - director: Sort alphabetically by director name
    /// - rating: Sort numerically by rating
    /// - watched_date: Sort by the date the movie was watched
    ///
    /// If not specified, defaults to sorting by title.
    #[arg(long)]
    pub sort: Option<String>,

    /// Sort order: asc or desc (optional, default: asc)
    ///
    /// Specifies the sort direction. Use "asc" for ascending order (default)
    /// or "desc" for descending order. Only used if `sort` is specified.
    #[arg(long)]
    pub order: Option<String>,

    /// Limit number of results shown (optional)
    ///
    /// If specified, limits the output to the first N results after filtering
    /// and sorting.
    #[arg(long)]
    pub limit: Option<i32>,

    /// Output format: plain, json, or csv (default: plain). All formats include poster path.
    ///
    /// Specifies the output format for displaying movies:
    /// - plain: Human-readable table format with poster paths truncated to fit terminal width (default)
    /// - json: Machine-readable JSON format with complete poster_path field
    /// - csv: Comma-separated values for import into spreadsheets with PosterPath column
    #[arg(long, default_value = "plain")]
    pub format: String,
}

/// Execute the show movies command
///
/// Fetches movies from the database, applies filters, sorting, and formatting based on arguments.
///
/// The execution pipeline works as follows:
/// 1. Fetch all movies from the database
/// 2. Apply year filter (if specified)
/// 3. Apply director filter (if specified)
/// 4. Apply minimum rating filter (if specified)
/// 5. Sort by specified field and order (if specified)
/// 6. Apply result limit (if specified)
/// 7. Format and display output according to the requested format
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
