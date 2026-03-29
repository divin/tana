//! TV series edit command implementation
//!
//! This module handles editing existing TV series in the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for editing a TV series
#[derive(Args, Debug)]
pub struct SeriesEditArgs {
    /// ID of the series to edit
    pub id: i32,

    /// New title
    #[arg(long)]
    pub title: Option<String>,

    /// New release year
    #[arg(long)]
    pub year: Option<i32>,

    /// New status (ongoing, completed, dropped)
    #[arg(long)]
    pub status: Option<String>,

    /// New total number of seasons
    #[arg(long)]
    pub seasons: Option<i32>,

    /// New current season
    #[arg(long)]
    pub current_season: Option<i32>,

    /// New current episode
    #[arg(long)]
    pub current_episode: Option<i32>,

    /// New rating (1-10)
    #[arg(long)]
    pub rating: Option<f64>,

    /// New start date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<String>,

    /// New completion date (YYYY-MM-DD)
    #[arg(long)]
    pub completed_date: Option<String>,

    /// New notes
    #[arg(long)]
    pub notes: Option<String>,

    /// New poster path
    #[arg(long)]
    pub poster: Option<String>,
}

/// Edit a TV series in the database
pub fn execute(ctx: &AppContext, args: SeriesEditArgs) -> Result<()> {
    // Validate rating if provided
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Fetch existing series
    let conn = ctx.db().connection();
    let mut series = queries::tv_series::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("TV Series with ID {}", args.id)))?;

    // Update fields if provided
    if let Some(title) = args.title {
        series.title = title;
    }
    if let Some(year) = args.year {
        series.release_year = Some(year);
    }
    if let Some(status) = args.status {
        series.status = status;
    }
    if let Some(seasons) = args.seasons {
        series.total_seasons = Some(seasons);
    }
    if let Some(current_season) = args.current_season {
        series.current_season = Some(current_season);
    }
    if let Some(current_episode) = args.current_episode {
        series.current_episode = Some(current_episode);
    }
    if let Some(rating) = args.rating {
        series.rating = Some(rating);
    }
    if let Some(date) = args.date {
        series.started_date = date;
    }
    if let Some(completed_date) = args.completed_date {
        series.completed_date = Some(completed_date);
    }
    if let Some(notes) = args.notes {
        series.notes = Some(notes);
    }
    if let Some(poster) = args.poster {
        let poster_path = crate::image::validate_image_path(&poster)?;
        series.poster_path = Some(poster_path);
    }

    // Update in database
    queries::tv_series::update(conn, args.id, &series)?;

    info!("✓ Updated TV series '{}' (ID {})", series.title, args.id);
    println!("✓ Updated TV series '{}' (ID {})", series.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_edit_args() {
        let args = SeriesEditArgs {
            id: 2,
            title: Some("Breaking Bad".to_string()),
            year: None,
            status: Some("completed".to_string()),
            seasons: Some(5),
            current_season: None,
            current_episode: None,
            rating: Some(9.5),
            date: None,
            completed_date: None,
            notes: None,
            poster: None,
        };

        assert_eq!(args.id, 2);
        assert_eq!(args.status, Some("completed".to_string()));
    }

    #[test]
    fn test_series_edit_with_poster() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join(format!(
            "tana_series_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);

        let poster_file = temp_dir.join("poster.png");
        let mut f = File::create(&poster_file).expect("Failed to create test poster");
        f.write_all(b"test image data")
            .expect("Failed to write test poster");

        let args = SeriesEditArgs {
            id: 2,
            title: None,
            year: None,
            status: None,
            seasons: None,
            current_season: None,
            current_episode: None,
            rating: None,
            date: None,
            completed_date: None,
            notes: None,
            poster: Some(poster_file.to_string_lossy().to_string()),
        };

        assert_eq!(args.id, 2);
        assert!(args.poster.is_some());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
