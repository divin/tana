//! Book add command implementation
//!
//! This module handles adding new books to the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::models::Book;
use crate::db::queries;
use crate::error::Result;

/// Arguments for adding a book
#[derive(Args, Debug)]
pub struct BookArgs {
    /// Title of the book
    #[arg(short, long)]
    pub title: String,

    /// Author of the book
    #[arg(short, long)]
    pub author: String,

    /// ISBN of the book
    #[arg(long)]
    pub isbn: Option<String>,

    /// Genre of the book
    #[arg(short, long)]
    pub genre: Option<String>,

    /// Number of pages
    #[arg(short, long)]
    pub pages: Option<i32>,

    /// Your rating (1-10)
    #[arg(short, long)]
    pub rating: Option<f64>,

    /// Date you started reading (YYYY-MM-DD)
    #[arg(long)]
    pub started_date: Option<String>,

    /// Date you finished reading (YYYY-MM-DD)
    #[arg(long)]
    pub date: String,

    /// Notes about the book
    #[arg(short, long)]
    pub notes: Option<String>,

    /// Path to book cover
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

    // Validate and set cover path if provided
    if let Some(cover) = args.cover {
        let cover_path = crate::image::validate_image_path(&cover)?;
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
