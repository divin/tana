//! Movie add command implementation
//!
//! This module handles adding new movies to the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::models::Movie;
use crate::db::queries;
use crate::error::Result;

/// Arguments for adding a movie
#[derive(Args, Debug)]
pub struct MovieArgs {
    /// Title of the movie
    #[arg(short, long)]
    pub title: String,

    /// Year the movie was released
    #[arg(short, long)]
    pub year: Option<i32>,

    /// Director of the movie
    #[arg(long)]
    pub director: Option<String>,

    /// Your rating (1-10)
    #[arg(short, long)]
    pub rating: Option<f64>,

    /// Date you watched it (YYYY-MM-DD)
    #[arg(long)]
    pub date: String,

    /// Notes about the movie
    #[arg(short, long)]
    pub notes: Option<String>,

    /// Path to movie poster
    #[arg(long)]
    pub poster: Option<String>,
}

/// Add a movie to the database
pub fn execute(ctx: &AppContext, args: MovieArgs) -> Result<()> {
    // Validate rating
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Create movie entry
    let mut movie = Movie::new(args.title.clone(), args.date);

    // Validate and set poster path if provided
    if let Some(poster) = args.poster {
        let poster_path = crate::image::validate_image_path(&poster)?;
        movie = movie.with_poster_path(poster_path);
    }
    if let Some(year) = args.year {
        movie = movie.with_year(year);
    }
    if let Some(director) = args.director {
        movie = movie.with_director(director);
    }
    if let Some(rating) = args.rating {
        movie = movie.with_rating(rating);
    }
    if let Some(notes) = args.notes {
        movie = movie.with_notes(notes);
    }

    // Insert into database
    let conn = ctx.db().connection();
    let id = queries::movies::insert(conn, &movie)?;

    info!("✓ Added movie '{}' with ID {}", args.title, id);
    println!("✓ Added movie '{}' with ID {}", args.title, id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_args_creation() {
        let args = MovieArgs {
            title: "Inception".to_string(),
            year: Some(2010),
            director: Some("Christopher Nolan".to_string()),
            rating: Some(9.0),
            date: "2024-01-15".to_string(),
            notes: None,
            poster: None,
        };

        assert_eq!(args.title, "Inception");
        assert_eq!(args.year, Some(2010));
        assert_eq!(args.rating, Some(9.0));
    }

    #[test]
    fn test_movie_add_with_poster() {
        let mut args = MovieArgs {
            title: "Inception".to_string(),
            year: Some(2010),
            director: Some("Christopher Nolan".to_string()),
            rating: Some(9.0),
            date: "2024-01-15".to_string(),
            notes: None,
            poster: None,
        };

        // Create a test image file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_poster.png");
        std::fs::File::create(&test_file).expect("Failed to create test file");

        args.poster = Some(test_file.to_string_lossy().to_string());

        // Verify poster path is set
        assert!(args.poster.is_some());

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}
