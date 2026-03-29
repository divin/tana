//! Movie edit command implementation
//!
//! This module handles editing existing movies in the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for editing a movie
#[derive(Args, Debug)]
pub struct MovieEditArgs {
    /// ID of the movie to edit
    pub id: i32,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New release year
    #[arg(long)]
    pub year: Option<i32>,

    /// New director
    #[arg(long)]
    pub director: Option<String>,

    /// New rating (1-10)
    #[arg(long)]
    pub rating: Option<f64>,

    /// New watched date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<String>,

    /// New notes
    #[arg(long)]
    pub notes: Option<String>,
}

/// Edit a movie in the database
pub fn execute(ctx: &AppContext, args: MovieEditArgs) -> Result<()> {
    // Validate rating if provided
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Fetch existing movie
    let conn = ctx.db().connection();
    let mut movie = queries::movies::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("Movie with ID {}", args.id)))?;

    // Update fields if provided
    if let Some(title) = args.title {
        movie.title = title;
    }
    if let Some(year) = args.year {
        movie.release_year = Some(year);
    }
    if let Some(director) = args.director {
        movie.director = Some(director);
    }
    if let Some(rating) = args.rating {
        movie.rating = Some(rating);
    }
    if let Some(date) = args.date {
        movie.watched_date = date;
    }
    if let Some(notes) = args.notes {
        movie.notes = Some(notes);
    }

    // Update in database
    queries::movies::update(conn, args.id, &movie)?;

    info!("✓ Updated movie '{}' (ID {})", movie.title, args.id);
    println!("✓ Updated movie '{}' (ID {})", movie.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_edit_args() {
        let args = MovieEditArgs {
            id: 1,
            title: Some("Inception".to_string()),
            year: Some(2010),
            director: Some("Christopher Nolan".to_string()),
            rating: Some(9.5),
            date: Some("2024-01-15".to_string()),
            notes: None,
        };

        assert_eq!(args.id, 1);
        assert_eq!(args.title, Some("Inception".to_string()));
        assert_eq!(args.rating, Some(9.5));
    }
}
