//! TV Series show command implementation
//!
//! This module handles displaying TV series with filtering, sorting, and formatting options.
//!
//! Following RFC 1733, this module is organized as a hub with submodules:
//! - `display`: Functions for formatting output (plain, JSON, CSV)
//! - `sort`: Functions for sorting and filtering series

use clap::Args;
use serde::Serialize;

use crate::cli::context::AppContext;
use crate::db::models::TVSeries;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

pub mod display;
pub mod sort;

/// Arguments for showing TV series
#[derive(Args, Debug)]
pub struct SeriesShowArgs {
    /// Filter by status (ongoing, completed, dropped)
    #[arg(long)]
    pub status: Option<String>,

    /// Filter by year
    #[arg(long)]
    pub year: Option<i32>,

    /// Minimum rating threshold (1-10)
    #[arg(long)]
    pub min_rating: Option<f64>,

    /// Sort by field
    #[arg(long)]
    pub sort: Option<String>,

    /// Sort order
    #[arg(long)]
    pub order: Option<String>,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<i32>,

    /// Output format (plain, json, csv)
    #[arg(long, default_value = "plain")]
    pub format: String,
}

/// TV Series entry for serialization
#[derive(Serialize, Debug, Clone)]
pub struct SeriesEntry {
    pub id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub status: String,
    pub total_seasons: Option<i32>,
    pub current_season: Option<i32>,
    pub current_episode: Option<i32>,
    pub rating: Option<f64>,
    pub started_date: String,
    pub completed_date: Option<String>,
    pub notes: Option<String>,
}

impl From<TVSeries> for SeriesEntry {
    fn from(series: TVSeries) -> Self {
        SeriesEntry {
            id: series.id.unwrap_or(0),
            title: series.title,
            year: series.release_year,
            status: series.status,
            total_seasons: series.total_seasons,
            current_season: series.current_season,
            current_episode: series.current_episode,
            rating: series.rating,
            started_date: series.started_date,
            completed_date: series.completed_date,
            notes: series.notes,
        }
    }
}

/// Execute the show TV series command
pub fn execute(ctx: &AppContext, args: SeriesShowArgs) -> Result<()> {
    let conn = ctx.db().connection();
    let mut series = queries::tv_series::get_all(conn, None)?;

    // Apply filters
    if let Some(status) = &args.status {
        let status_lower = status.to_lowercase();
        series.retain(|s| s.status.to_lowercase() == status_lower);
    }

    if let Some(year) = args.year {
        series.retain(|s| s.release_year == Some(year));
    }

    if let Some(min_rating) = args.min_rating {
        series.retain(|s| s.rating.is_some_and(|r| r >= min_rating));
    }

    // Apply sorting
    if let Some(sort_by) = args.sort {
        let order = args.order.as_deref().unwrap_or("asc");
        sort::sort_series(&mut series, &sort_by, order);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        series.truncate(limit as usize);
    }

    // Format output
    let format_str = args.format.to_lowercase();
    let format = format_str.parse::<Format>()?;
    let truncate_length = ctx.truncate_length();

    match format {
        Format::Plain => display::display_plain(&series, truncate_length),
        Format::Json => display::display_json(&series)?,
        Format::Csv => display::display_csv(&series, truncate_length),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_show_args() {
        let args = SeriesShowArgs {
            status: Some("completed".to_string()),
            year: Some(2020),
            min_rating: Some(8.0),
            sort: Some("rating".to_string()),
            order: Some("desc".to_string()),
            limit: Some(10),
            format: "json".to_string(),
        };

        assert_eq!(args.status, Some("completed".to_string()));
        assert_eq!(args.year, Some(2020));
        assert_eq!(args.limit, Some(10));
        assert_eq!(args.format, "json");
    }

    #[test]
    fn test_series_show_args_defaults() {
        let args = SeriesShowArgs {
            status: None,
            year: None,
            min_rating: None,
            sort: None,
            order: None,
            limit: None,
            format: "plain".to_string(),
        };

        assert_eq!(args.status, None);
        assert_eq!(args.year, None);
        assert_eq!(args.format, "plain");
    }

    #[test]
    fn test_series_entry_from_tvseries() {
        let series = TVSeries {
            id: Some(1),
            title: "Breaking Bad".to_string(),
            release_year: Some(2008),
            status: "completed".to_string(),
            total_seasons: Some(5),
            current_season: Some(5),
            current_episode: Some(16),
            rating: Some(9.5),
            started_date: "2024-01-01".to_string(),
            completed_date: Some("2024-12-31".to_string()),
            notes: Some("Great show!".to_string()),
            poster_path: None,
        };

        let entry: SeriesEntry = series.into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.title, "Breaking Bad");
        assert_eq!(entry.status, "completed");
        assert_eq!(entry.rating, Some(9.5));
    }
}
