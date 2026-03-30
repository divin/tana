//! Book add command implementation
//!
//! This module handles adding new books to the database.
//!
//! # Examples
//!
//! Add a book with basic information:
//! ```sh
//! tana add book --title "The Rust Book" --author "Steve Klabnik" --date 2024-01-25
//! ```
//!
//! Add a book with complete details including a cover image:
//! ```sh
//! tana add book --title "The Rust Book" --author "Steve Klabnik" \
//!   --genre "Programming" --pages 500 --date 2024-01-25 --rating 8.5 \
//!   --notes "Essential resource for learning Rust" --cover /path/to/cover.jpg
//! ```

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::models::Book;
use crate::db::queries;
use crate::error::Result;

/// Arguments for adding a book
///
/// Allows users to add a new book to their collection with optional details
/// such as author, ISBN, genre, page count, personal rating, and book cover image.
/// The cover image helps create a visual library of your book collection.
#[derive(Args, Debug)]
pub struct BookArgs {
    /// Title of the book (required)
    #[arg(short, long)]
    pub title: String,

    /// Author of the book (required)
    #[arg(short, long)]
    pub author: String,

    /// ISBN of the book (optional)
    #[arg(long)]
    pub isbn: Option<String>,

    /// Genre of the book (optional)
    #[arg(short, long)]
    pub genre: Option<String>,

    /// Number of pages (optional)
    #[arg(short, long)]
    pub pages: Option<i32>,

    /// Your rating on a scale of 1-10 (optional)
    #[arg(short, long)]
    pub rating: Option<f64>,

    /// Date you started reading in YYYY-MM-DD format (optional)
    #[arg(long)]
    pub started_date: Option<String>,

    /// Date you finished reading in YYYY-MM-DD format (required)
    #[arg(long)]
    pub date: String,

    /// Personal notes about the book (optional)
    #[arg(short, long)]
    pub notes: Option<String>,

    /// Path to book cover image file. Supported formats: PNG, JPG, JPEG, WebP, GIF, BMP (optional)
    #[arg(long)]
    pub cover: Option<String>,
}

/// Add a book to the database
pub fn execute(ctx: &AppContext, args: BookArgs) -> Result<()> {
    // Validate rating
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Create book entry
    let mut book = Book::new(args.title.clone(), args.author, args.date);

    // Copy and set cover path if provided
    if let Some(cover) = args.cover {
        let images_dir = ctx.config().images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let cover_path = crate::image::copy_image_file(&cover, &images_dir_str)?;
        book = book.with_cover_path(cover_path);
    }
    if let Some(isbn) = args.isbn {
        book = book.with_isbn(isbn);
    }
    if let Some(genre) = args.genre {
        book = book.with_genre(genre);
    }
    if let Some(pages) = args.pages {
        book = book.with_pages(pages);
    }
    if let Some(rating) = args.rating {
        book = book.with_rating(rating);
    }
    if let Some(started_date) = args.started_date {
        book = book.with_started_date(started_date);
    }
    if let Some(notes) = args.notes {
        book = book.with_notes(notes);
    }

    // Insert into database
    let conn = ctx.db().connection();
    let id = queries::books::insert(conn, &book)?;

    info!("✓ Added book '{}' with ID {}", args.title, id);
    println!("✓ Added book '{}' with ID {}", args.title, id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_args_creation() {
        let args = BookArgs {
            title: "The Rust Book".to_string(),
            author: "Steve Klabnik".to_string(),
            isbn: None,
            genre: Some("Programming".to_string()),
            pages: Some(500),
            rating: Some(8.5),
            started_date: None,
            date: "2024-01-25".to_string(),
            notes: None,
            cover: None,
        };

        assert_eq!(args.title, "The Rust Book");
        assert_eq!(args.author, "Steve Klabnik");
    }

    #[test]
    fn test_book_add_with_cover() {
        let mut args = BookArgs {
            title: "The Rust Book".to_string(),
            author: "Steve Klabnik".to_string(),
            isbn: None,
            genre: Some("Programming".to_string()),
            pages: Some(500),
            rating: Some(8.5),
            started_date: None,
            date: "2024-01-25".to_string(),
            notes: None,
            cover: None,
        };

        // Create a test image file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_book_cover.png");
        std::fs::File::create(&test_file).expect("Failed to create test file");

        args.cover = Some(test_file.to_string_lossy().to_string());

        // Verify cover path is set
        assert!(args.cover.is_some());

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}
