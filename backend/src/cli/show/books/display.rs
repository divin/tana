//! Display formatting for books
//!
//! This module provides functions for displaying books in various formats:
//! - Plain text table format
//! - JSON format
//! - CSV format

use serde::Serialize;

use crate::db::models::Book;
use crate::error::Result;

use super::super::format::escape_csv;
use super::sort::truncate;

/// Book entry for JSON serialization
///
/// This struct is used when serializing books to JSON format.
/// It includes all relevant book information except ISBN, which is
/// available in the JSON output but not in the plain text display.
#[derive(Serialize, Debug)]
pub struct BookEntry {
    pub id: i32,
    pub title: String,
    pub author: String,
    pub genre: Option<String>,
    pub pages: Option<i32>,
    pub rating: Option<f64>,
    pub completed_date: String,
    pub isbn: Option<String>,
    pub notes: Option<String>,
    pub cover_path: Option<String>,
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
            cover_path: book.cover_path.clone(),
        }
    }
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
/// * `truncate_length` - Maximum length for truncating text fields
pub fn display_plain(books: &[Book], truncate_length: usize) {
    if books.is_empty() {
        println!("No books found.");
        return;
    }

    println!("\n{:=^145}", " Books ");
    println!(
        "{:<4} {:<30} {:<20} {:<15} {:<7} {:<8} {:<12} {:<25}",
        "ID", "Title", "Author", "Genre", "Pages", "Rating", "Completed", "Cover Path"
    );
    println!("{}", "=".repeat(145));

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

        let cover_path = book
            .cover_path
            .as_ref()
            .map(|p| truncate(p, truncate_length.min(23)))
            .unwrap_or_else(|| "N/A".to_string());

        println!(
            "{:<4} {:<30} {:<20} {:<15} {:<7} {:<8} {:<12} {:<25}",
            book.id.unwrap_or(0),
            title,
            author,
            genre,
            pages,
            rating,
            book.completed_date,
            cover_path
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
pub fn display_json(books: &[Book]) -> Result<()> {
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
/// * `_truncate_length` - Truncate length (not used for CSV format but maintained for API consistency)
pub fn display_csv(books: &[Book], _truncate_length: usize) {
    println!("ID,Title,Author,Genre,Pages,Rating,CompletedDate,ISBN,Notes,CoverPath");
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
        let cover_path = book
            .cover_path
            .as_ref()
            .map(|p| escape_csv(p))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            book.id.unwrap_or(0),
            title,
            author,
            genre,
            pages,
            rating,
            escape_csv(&book.completed_date),
            isbn,
            notes,
            cover_path
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            cover_path: None,
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
            cover_path: None,
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
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            isbn: None,
            genre: None,
            pages: None,
            rating: None,
            started_date: None,
            completed_date: "2024-05-01".to_string(),
            notes: None,
            cover_path: None,
        };

        let entry: BookEntry = book.into();
        assert_eq!(entry.id, 0);
    }

    #[test]
    fn test_book_entry_with_cover_path() {
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
            cover_path: Some("/covers/the_hobbit.jpg".to_string()),
        };

        let entry: BookEntry = book.into();
        assert_eq!(entry.id, 42);
        assert_eq!(entry.title, "The Hobbit");
        assert_eq!(entry.cover_path, Some("/covers/the_hobbit.jpg".to_string()));
    }
}
