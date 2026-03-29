//! Book show command implementation
//!
//! This module handles displaying books with filtering, sorting, and formatting options.
//! Books can be filtered by author, genre, and minimum rating. They can be sorted by
//! various fields (title, author, genre, pages, rating, date) and output in multiple
//! formats (plain text table, JSON, CSV).
//!
//! # Module Structure
//!
//! This module is organized as a hub with submodules:
//! - `display`: Handles formatting and output of books in various formats
//! - `sort`: Provides sorting and text utility functions
//!
//! # Examples
//!
//! Filter books by author and sort by rating in descending order:
//! ```ignore
//! let args = BooksShowArgs {
//!     author: Some("Tolkien".to_string()),
//!     genre: None,
//!     min_rating: None,
//!     sort: Some("rating".to_string()),
//!     order: Some("desc".to_string()),
//!     limit: None,
//!     format: "plain".to_string(),
//! };
//! execute(&db, args)?;
//! ```

pub mod display;
pub mod sort;

use clap::Args;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

// Re-export BookEntry for convenience
pub use display::BookEntry;

/// Arguments for showing books with filtering, sorting, and formatting options
///
/// This struct represents the command-line arguments for the `show books` command.
/// It supports filtering by author and genre, applying a minimum rating threshold,
/// sorting by various fields, and outputting results in different formats.
#[derive(Args, Debug)]
pub struct BooksShowArgs {
    /// Filter by author name (case-insensitive partial match)
    ///
    /// Filters the book list to only include books by authors whose names contain
    /// the specified string (case-insensitive).
    #[arg(long)]
    pub author: Option<String>,

    /// Filter by genre (case-insensitive partial match)
    ///
    /// Filters the book list to only include books with genres that contain
    /// the specified string (case-insensitive).
    #[arg(long)]
    pub genre: Option<String>,

    /// Minimum rating threshold (1-10)
    ///
    /// Filters the book list to only include books with a rating greater than
    /// or equal to the specified value.
    #[arg(long)]
    pub min_rating: Option<f64>,

    /// Sort by field (title, author, genre, pages, rating, date)
    ///
    /// Specifies which field to sort the results by. Valid options are:
    /// - title: Sort alphabetically by book title
    /// - author: Sort alphabetically by author name
    /// - genre: Sort alphabetically by genre
    /// - pages: Sort numerically by page count
    /// - rating: Sort numerically by rating
    /// - date: Sort by completion date
    #[arg(long)]
    pub sort: Option<String>,

    /// Sort order (asc or desc)
    ///
    /// Specifies the sort direction. Use "asc" for ascending order (default)
    /// or "desc" for descending order. Only used if `sort` is specified.
    #[arg(long)]
    pub order: Option<String>,

    /// Limit number of results
    ///
    /// If specified, limits the output to the first N results after filtering
    /// and sorting.
    #[arg(long)]
    pub limit: Option<i32>,

    /// Output format (plain, json, csv)
    ///
    /// Specifies the output format for displaying books:
    /// - plain: Human-readable table format (default)
    /// - json: Machine-readable JSON format with full details
    /// - csv: Comma-separated values for import into spreadsheets
    #[arg(long, default_value = "plain")]
    pub format: String,
}

/// Execute the show books command
///
/// Retrieves all books from the database and applies filtering, sorting,
/// and formatting options before displaying them to the user.
///
/// The execution pipeline works as follows:
/// 1. Fetch all books from the database
/// 2. Apply author filter (if specified)
/// 3. Apply genre filter (if specified)
/// 4. Apply minimum rating filter (if specified)
/// 5. Sort by specified field and order (if specified)
/// 6. Apply result limit (if specified)
/// 7. Format and display output according to the requested format
///
/// # Arguments
///
/// * `ctx` - Application context containing database connection
/// * `args` - Command-line arguments specifying filters, sorting, and format
///
/// # Returns
///
/// Returns `Ok(())` if the command executes successfully, or a `Result` error
/// if any step fails (e.g., database error, invalid format).
///
/// # Errors
///
/// Returns an error if:
/// - Database query fails
/// - Invalid format string is provided
/// - JSON serialization fails
pub fn execute(ctx: &AppContext, args: BooksShowArgs) -> Result<()> {
    use self::display::{display_csv, display_json, display_plain};
    use self::sort::sort_books;

    let conn = ctx.db().connection();
    let mut books = queries::books::get_all(conn, None)?;

    // Apply author filter
    if let Some(author) = &args.author {
        let author_lower = author.to_lowercase();
        books.retain(|b| b.author.to_lowercase().contains(&author_lower));
    }

    // Apply genre filter
    if let Some(genre) = &args.genre {
        let genre_lower = genre.to_lowercase();
        books.retain(|b| {
            b.genre
                .as_ref()
                .map(|g| g.to_lowercase().contains(&genre_lower))
                .unwrap_or(false)
        });
    }

    // Apply minimum rating filter
    if let Some(min_rating) = args.min_rating {
        books.retain(|b| b.rating.is_some_and(|r| r >= min_rating));
    }

    // Apply sorting
    if let Some(sort_by) = args.sort {
        let order = args.order.as_deref().unwrap_or("asc");
        sort_books(&mut books, &sort_by, order);
    }

    // Apply result limit
    if let Some(limit) = args.limit {
        books.truncate(limit as usize);
    }

    // Format and display output
    let format_str = args.format.to_lowercase();
    let format = format_str.parse::<Format>()?;
    let truncate_length = ctx.truncate_length();

    match format {
        Format::Plain => display_plain(&books, truncate_length),
        Format::Json => display_json(&books)?,
        Format::Csv => display_csv(&books, truncate_length),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for BooksShowArgs
    #[test]
    fn test_books_show_args_all_filters() {
        let args = BooksShowArgs {
            author: Some("George Orwell".to_string()),
            genre: Some("Fiction".to_string()),
            min_rating: Some(7.5),
            sort: Some("rating".to_string()),
            order: Some("desc".to_string()),
            limit: Some(20),
            format: "json".to_string(),
        };

        assert_eq!(args.author, Some("George Orwell".to_string()));
        assert_eq!(args.genre, Some("Fiction".to_string()));
        assert_eq!(args.min_rating, Some(7.5));
        assert_eq!(args.sort, Some("rating".to_string()));
        assert_eq!(args.order, Some("desc".to_string()));
        assert_eq!(args.limit, Some(20));
        assert_eq!(args.format, "json");
    }

    #[test]
    fn test_books_show_args_defaults() {
        let args = BooksShowArgs {
            author: None,
            genre: None,
            min_rating: None,
            sort: None,
            order: None,
            limit: None,
            format: "plain".to_string(),
        };

        assert!(args.author.is_none());
        assert!(args.genre.is_none());
        assert!(args.min_rating.is_none());
        assert!(args.sort.is_none());
        assert!(args.order.is_none());
        assert!(args.limit.is_none());
        assert_eq!(args.format, "plain");
    }

    #[test]
    fn test_books_show_args_author_only() {
        let args = BooksShowArgs {
            author: Some("Tolkien".to_string()),
            genre: None,
            min_rating: None,
            sort: None,
            order: None,
            limit: None,
            format: "plain".to_string(),
        };

        assert_eq!(args.author, Some("Tolkien".to_string()));
        assert!(args.genre.is_none());
    }

    #[test]
    fn test_books_show_args_rating_filter() {
        let args = BooksShowArgs {
            author: None,
            genre: None,
            min_rating: Some(8.5),
            sort: Some("title".to_string()),
            order: Some("asc".to_string()),
            limit: Some(10),
            format: "csv".to_string(),
        };

        assert_eq!(args.min_rating, Some(8.5));
        assert_eq!(args.format, "csv");
    }
}
