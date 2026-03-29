//! Book show command implementation
//!
//! This module handles displaying books with filtering, sorting, and formatting options.
//! Books can be filtered by author, genre, and minimum rating. They can be sorted by
//! various fields (title, author, genre, pages, rating, date) and output in multiple
//! formats (plain text table, JSON, CSV).
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

use clap::Args;
use serde::Serialize;

use crate::cli::context::AppContext;
use crate::db::models::Book;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

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

/// Book entry for JSON serialization
///
/// This struct is used when serializing books to JSON format.
/// It includes all relevant book information except ISBN, which is
/// available in the JSON output but not in the plain text display.
#[derive(Serialize, Debug)]
struct BookEntry {
    id: i32,
    title: String,
    author: String,
    genre: Option<String>,
    pages: Option<i32>,
    rating: Option<f64>,
    completed_date: String,
    isbn: Option<String>,
    notes: Option<String>,
}

impl From<Book> for BookEntry {
    fn from(book: Book) -> Self {
        BookEntry {
            id: book.id.unwrap_or(0),
            title: book.title,
            author: book.author,
            genre: book.genre,
            pages: book.pages,
            rating: book.rating,
            completed_date: book.completed_date,
            isbn: book.isbn,
            notes: book.notes,
        }
    }
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
/// * `db` - Database connection
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

/// Display books in plain text table format
///
/// Renders a formatted ASCII table showing key book information:
/// - ID: Database identifier
/// - Title: Book title (truncated if necessary)
/// - Author: Author name (truncated if necessary)
/// - Genre: Book genre (truncated if necessary)
/// - Pages: Number of pages
/// - Rating: User rating out of 10
/// - Completed: Date the book was completed
///
/// If no books are found, displays a simple "No books found." message.
///
/// # Arguments
///
/// * `books` - Slice of books to display
fn display_plain(books: &[Book], truncate_length: usize) {
    if books.is_empty() {
        println!("No books found.");
        return;
    }

    println!("\n{:=^120}", " Books ");
    println!(
        "{:<4} {:<30} {:<20} {:<15} {:<7} {:<8} {:<12}",
        "ID", "Title", "Author", "Genre", "Pages", "Rating", "Completed"
    );
    println!("{}", "=".repeat(120));

    for book in books {
        let title = truncate(&book.title, truncate_length.min(28));
        let author = truncate(&book.author, truncate_length.min(18));
        let genre = book
            .genre
            .as_ref()
            .map(|g| truncate(g, truncate_length.min(13)))
            .unwrap_or_else(|| "—".to_string());
        let pages = book
            .pages
            .map(|p| p.to_string())
            .unwrap_or_else(|| "—".to_string());
        let rating = book
            .rating
            .map(|r| format!("{}/10", r))
            .unwrap_or_else(|| "—".to_string());

        println!(
            "{:<4} {:<30} {:<20} {:<15} {:<7} {:<8} {:<12}",
            book.id.unwrap_or(0),
            title,
            author,
            genre,
            pages,
            rating,
            book.completed_date
        );
    }
    println!();
}

/// Display books in JSON format
///
/// Converts books to JSON format and prints them in a pretty-printed,
/// human-readable layout. All book fields are included in the output.
///
/// # Arguments
///
/// * `books` - Slice of books to serialize and display
///
/// # Returns
///
/// Returns `Ok(())` if successful, or a JSON serialization error if it fails.
///
/// # Errors
///
/// Returns an error if JSON serialization fails.
fn display_json(books: &[Book]) -> Result<()> {
    let entries: Vec<BookEntry> = books.iter().map(|b| b.clone().into()).collect();
    let json = serde_json::to_string_pretty(&entries)?;
    println!("{}", json);
    Ok(())
}

/// Display books in CSV format
///
/// Exports books as comma-separated values (CSV) suitable for import into
/// spreadsheet applications. All fields are properly escaped according to CSV standards:
/// fields containing commas, quotes, or newlines are wrapped in double quotes,
/// and internal quotes are escaped by doubling.
///
/// The header row contains: ID, Title, Author, Genre, Pages, Rating, CompletedDate, ISBN, Notes
///
/// # Arguments
///
/// * `books` - Slice of books to export as CSV
fn display_csv(books: &[Book], _truncate_length: usize) {
    use super::format::escape_csv;

    println!("ID,Title,Author,Genre,Pages,Rating,CompletedDate,ISBN,Notes");
    for book in books {
        let title = escape_csv(&book.title);
        let author = escape_csv(&book.author);
        let genre = book
            .genre
            .as_ref()
            .map(|g| escape_csv(g))
            .unwrap_or_default();
        let pages = book.pages.map(|p| p.to_string()).unwrap_or_default();
        let rating = book.rating.map(|r| r.to_string()).unwrap_or_default();
        let isbn = book
            .isbn
            .as_ref()
            .map(|i| escape_csv(i))
            .unwrap_or_default();
        let notes = book
            .notes
            .as_ref()
            .map(|n| escape_csv(n))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{}",
            book.id.unwrap_or(0),
            title,
            author,
            genre,
            pages,
            rating,
            escape_csv(&book.completed_date),
            isbn,
            notes
        );
    }
}

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
fn sort_books(books: &mut [Book], sort_by: &str, order: &str) {
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

    // Tests for BookEntry conversion
    #[test]
    fn test_book_entry_conversion_full() {
        let book = Book {
            id: Some(42),
            title: "The Hobbit".to_string(),
            author: "J.R.R. Tolkien".to_string(),
            isbn: Some("978-0547928227".to_string()),
            genre: Some("Fantasy".to_string()),
            pages: Some(310),
            rating: Some(9.2),
            started_date: Some("2024-01-01".to_string()),
            completed_date: "2024-03-15".to_string(),
            notes: Some("An excellent adventure".to_string()),
        };

        let entry: BookEntry = book.into();
        assert_eq!(entry.id, 42);
        assert_eq!(entry.title, "The Hobbit");
        assert_eq!(entry.author, "J.R.R. Tolkien");
        assert_eq!(entry.genre, Some("Fantasy".to_string()));
        assert_eq!(entry.pages, Some(310));
        assert_eq!(entry.rating, Some(9.2));
        assert_eq!(entry.completed_date, "2024-03-15");
        assert_eq!(entry.isbn, Some("978-0547928227".to_string()));
        assert_eq!(entry.notes, Some("An excellent adventure".to_string()));
    }

    #[test]
    fn test_book_entry_conversion_minimal() {
        let book = Book {
            id: Some(1),
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            isbn: None,
            genre: None,
            pages: None,
            rating: None,
            started_date: None,
            completed_date: "2024-01-01".to_string(),
            notes: None,
        };

        let entry: BookEntry = book.into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.title, "Test Book");
        assert_eq!(entry.author, "Test Author");
        assert!(entry.genre.is_none());
        assert!(entry.pages.is_none());
        assert!(entry.rating.is_none());
        assert!(entry.notes.is_none());
    }

    #[test]
    fn test_book_entry_zero_id() {
        let book = Book {
            id: None,
            title: "No ID Book".to_string(),
            author: "Author".to_string(),
            isbn: None,
            genre: None,
            pages: None,
            rating: None,
            started_date: None,
            completed_date: "2024-01-01".to_string(),
            notes: None,
        };

        let entry: BookEntry = book.into();
        assert_eq!(entry.id, 0);
    }
}
