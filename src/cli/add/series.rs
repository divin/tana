//! TV series add command implementation
//!
//! This module handles adding new TV series to the database.

use clap::Args;
use tracing::info;

use crate::db::Database;
use crate::db::models::TVSeries;
use crate::db::queries;
use crate::error::Result;

/// Arguments for adding a TV series
#[derive(Args, Debug)]
pub struct SeriesArgs {
    /// Title of the TV series
    #[arg(short, long)]
    pub title: String,

    /// Year the series started airing
    #[arg(short, long)]
    pub year: Option<i32>,

    /// Status: ongoing, completed, or dropped
    #[arg(short, long)]
    pub status: String,

    /// Total number of seasons
    #[arg(long)]
    pub seasons: Option<i32>,

    /// Current season you're watching
    #[arg(long)]
    pub current_season: Option<i32>,

    /// Current episode you're watching
    #[arg(long)]
    pub current_episode: Option<i32>,

    /// Your rating (1-10)
    #[arg(short, long)]
    pub rating: Option<f64>,

    /// Date you started watching (YYYY-MM-DD)
    #[arg(long)]
    pub date: String,

    /// Date you completed it (YYYY-MM-DD)
    #[arg(long)]
    pub completed_date: Option<String>,

    /// Notes about the series
    #[arg(short, long)]
    pub notes: Option<String>,
}

/// Add a TV series to the database
pub fn execute(db: &Database, args: SeriesArgs) -> Result<()> {
    // Validate rating
    if let Some(rating) = args.rating {
        if rating < 0.0 || rating > 10.0 {
            return Err(crate::TanaError::InvalidRating(rating));
        }
    }

    // Create series entry
    let mut series = TVSeries::new(args.title.clone(), args.date, args.status);
    if let Some(year) = args.year {
        series = series.with_year(year);
    }
    if let Some(seasons) = args.seasons {
        series = series.with_total_seasons(seasons);
    }
    if let Some(current_season) = args.current_season {
        if let Some(current_episode) = args.current_episode {
            series = series.with_progress(current_season, current_episode);
        }
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
    let conn = db.connection();
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
        };

        assert_eq!(args.title, "Breaking Bad");
        assert_eq!(args.status, "completed");
    }
}
