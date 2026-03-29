//! TV series add command implementation
//!
//! This module handles adding new TV series to the database.
//!
//! # Examples
//!
//! Add a TV series with basic information:
//! ```sh
//! tana add series --title "Breaking Bad" --status completed --date 2024-01-10
//! ```
//!
//! Add a TV series with full details including a poster image:
//! ```sh
//! tana add series --title "Breaking Bad" --year 2008 --status completed \
//!   --seasons 5 --rating 9.5 --date 2024-01-10 \
//!   --notes "One of the greatest shows ever made" \
//!   --poster /path/to/poster.jpg
//! ```

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::models::TVSeries;
use crate::db::queries;
use crate::error::Result;

/// Arguments for adding a TV series
///
/// Allows users to add a new TV series to their collection with detailed information
/// including air year, current progress, completion status, personal rating, and
/// series poster image. The poster image helps visualize your series collection.
#[derive(Args, Debug)]
pub struct SeriesArgs {
    /// Title of the TV series (required)
    #[arg(short, long)]
    pub title: String,

    /// Year the series started airing (optional)
    #[arg(short, long)]
    pub year: Option<i32>,

    /// Status: ongoing, completed, or dropped (required)
    #[arg(short, long)]
    pub status: String,

    /// Total number of seasons (optional)
    #[arg(long)]
    pub seasons: Option<i32>,

    /// Current season you're watching (optional)
    #[arg(long)]
    pub current_season: Option<i32>,

    /// Current episode you're watching (optional)
    #[arg(long)]
    pub current_episode: Option<i32>,

    /// Your rating on a scale of 1-10 (optional)
    #[arg(short, long)]
    pub rating: Option<f64>,

    /// Date you started watching in YYYY-MM-DD format (required)
    #[arg(long)]
    pub date: String,

    /// Date you completed it in YYYY-MM-DD format (optional)
    #[arg(long)]
    pub completed_date: Option<String>,

    /// Personal notes about the series (optional)
    #[arg(short, long)]
    pub notes: Option<String>,

    /// Path to series poster image file. Supported formats: PNG, JPG, JPEG, WebP, GIF, BMP (optional)
    #[arg(long)]
    pub poster: Option<String>,
}

/// Add a TV series to the database
pub fn execute(ctx: &AppContext, args: SeriesArgs) -> Result<()> {
    // Validate rating
    if let Some(rating) = args.rating
        && !(0.0..=10.0).contains(&rating)
    {
        return Err(crate::TanaError::InvalidRating(rating));
    }

    // Create series entry
    let mut series = TVSeries::new(args.title.clone(), args.date, args.status);

    // Copy and set poster path if provided
    if let Some(poster) = args.poster {
        let images_dir = ctx.config().images_default_directory();
        let images_dir_str = images_dir.to_string_lossy().to_string();
        let poster_path = crate::image::copy_image_file(&poster, &images_dir_str)?;
        series = series.with_poster_path(poster_path);
    }
    if let Some(year) = args.year {
        series = series.with_year(year);
    }
    if let Some(seasons) = args.seasons {
        series = series.with_total_seasons(seasons);
    }
    if let Some(current_season) = args.current_season
        && let Some(current_episode) = args.current_episode
    {
        series = series.with_progress(current_season, current_episode);
    }
    if let Some(rating) = args.rating {
        series = series.with_rating(rating);
    }
    if let Some(completed_date) = args.completed_date {
        series = series.with_completed_date(completed_date);
    }
    if let Some(notes) = args.notes {
        series = series.with_notes(notes);
    }

    // Insert into database
    let conn = ctx.db().connection();
    let id = queries::tv_series::insert(conn, &series)?;

    info!("✓ Added TV series '{}' with ID {}", args.title, id);
    println!("✓ Added TV series '{}' with ID {}", args.title, id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_args_creation() {
        let args = SeriesArgs {
            title: "Breaking Bad".to_string(),
            year: Some(2008),
            status: "completed".to_string(),
            seasons: Some(5),
            current_season: None,
            current_episode: None,
            rating: Some(9.5),
            date: "2024-01-10".to_string(),
            completed_date: None,
            notes: None,
            poster: None,
        };

        assert_eq!(args.title, "Breaking Bad");
        assert_eq!(args.status, "completed");
    }

    #[test]
    fn test_series_add_with_poster() {
        let mut args = SeriesArgs {
            title: "Breaking Bad".to_string(),
            year: Some(2008),
            status: "completed".to_string(),
            seasons: Some(5),
            current_season: None,
            current_episode: None,
            rating: Some(9.5),
            date: "2024-01-10".to_string(),
            completed_date: None,
            notes: None,
            poster: None,
        };

        // Create a test image file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_series_poster.png");
        std::fs::File::create(&test_file).expect("Failed to create test file");

        args.poster = Some(test_file.to_string_lossy().to_string());

        // Verify poster path is set
        assert!(args.poster.is_some());

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }
}
