//! Book edit command implementation
//!
//! This module handles editing existing books in the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for editing a book
#[derive(Args, Debug)]
pub struct BookEditArgs {
    /// ID of the book to edit
    pub id: i32,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New author
    #[arg(long)]
    pub author: Option<String>,

    /// New ISBN
    #[arg(long)]
    pub isbn: Option<String>,

    /// New genre
    #[arg(long)]
    pub genre: Option<String>,

    /// New page count
    #[arg(long)]
    pub pages: Option<i32>,

    /// New rating (1-10)
    #[arg(long)]
    pub rating: Option<f64>,

    /// New start date (YYYY-MM-DD)
    #[arg(long)]
    pub started_date: Option<String>,

    /// New completion date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<String>,

    /// New notes
    #[arg(long)]
    pub notes: Option<String>,

    /// New cover path
    #[arg(long)]
    pub cover: Option<String>,
}

/// Edit a book in the database
pub fn execute(ctx: &AppContext, args: BookEditArgs) -> Result<()> {
    // Validate rating if provided
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Fetch existing book
    let conn = ctx.db().connection();
    let mut book = queries::books::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("Book with ID {}", args.id)))?;

    // Update fields if provided
    if let Some(title) = args.title {
        book.title = title;
    }
    if let Some(author) = args.author {
        book.author = author;
    }
    if let Some(isbn) = args.isbn {
        book.isbn = Some(isbn);
    }
    if let Some(genre) = args.genre {
        book.genre = Some(genre);
    }
    if let Some(pages) = args.pages {
        book.pages = Some(pages);
    }
    if let Some(rating) = args.rating {
        book.rating = Some(rating);
    }
    if let Some(started_date) = args.started_date {
        book.started_date = Some(started_date);
    }
    if let Some(date) = args.date {
        book.completed_date = date;
    }
    if let Some(notes) = args.notes {
        book.notes = Some(notes);
    }
    if let Some(cover) = args.cover {
        let cover_path = crate::image::validate_image_path(&cover)?;
        book.cover_path = Some(cover_path);
    }

    // Update in database
    queries::books::update(conn, args.id, &book)?;

    info!("✓ Updated book '{}' (ID {})", book.title, args.id);
    println!("✓ Updated book '{}' (ID {})", book.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_edit_args() {
        let args = BookEditArgs {
            id: 3,
            title: None,
            author: None,
            isbn: Some("978-1492052586".to_string()),
            genre: Some("Programming".to_string()),
            pages: Some(500),
            rating: Some(8.5),
            started_date: None,
            date: None,
            notes: None,
            cover: None,
        };

        assert_eq!(args.id, 3);
        assert_eq!(args.isbn, Some("978-1492052586".to_string()));
    }

    #[test]
    fn test_book_edit_with_cover() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join(format!(
            "tana_book_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        let cover_file = temp_dir.join("cover.png");
        let mut f = File::create(&cover_file).expect("Failed to create test cover");
        f.write_all(b"test image data")
            .expect("Failed to write test cover");

        let args = BookEditArgs {
            id: 3,
            title: None,
            author: None,
            isbn: None,
            genre: None,
            pages: None,
            rating: None,
            started_date: None,
            date: None,
            notes: None,
            cover: Some(cover_file.to_string_lossy().to_string()),
        };

        assert_eq!(args.id, 3);
        assert!(args.cover.is_some());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
