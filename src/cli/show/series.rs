//! TV Series show command implementation
//!
//! This module handles displaying TV series with filtering, sorting, and formatting options.

use clap::Args;
use serde::Serialize;

use crate::cli::context::AppContext;
use crate::db::models::TVSeries;
use crate::db::queries;
use crate::error::Result;

use super::format::Format;

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
#[derive(Serialize, Debug)]
struct SeriesEntry {
    id: i32,
    title: String,
    year: Option<i32>,
    status: String,
    total_seasons: Option<i32>,
    current_season: Option<i32>,
    current_episode: Option<i32>,
    rating: Option<f64>,
    started_date: String,
    completed_date: Option<String>,
    notes: Option<String>,
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
        sort_series(&mut series, &sort_by, order);
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
        Format::Plain => display_plain(&series, truncate_length),
        Format::Json => display_json(&series)?,
        Format::Csv => display_csv(&series, truncate_length),
    }

    Ok(())
}

/// Display TV series in plain text format
fn display_plain(series_list: &[TVSeries], _truncate_length: usize) {
    if series_list.is_empty() {
        println!("No series found.");
        return;
    }

    println!("\n{:=^120}", " TV Series ");
    println!(
        "{:<4} {:<35} {:<8} {:<12} {:<12} {:<8}",
        "ID", "Title", "Year", "Status", "Progress", "Rating"
    );
    println!("{}", "=".repeat(120));

    for series in series_list {
        let title = truncate(&series.title, 33);
        let status = truncate(&series.status, 10);
        let progress = if let (Some(season), Some(episode)) =
            (series.current_season, series.current_episode)
        {
            format!("S{}E{}", season, episode)
        } else {
            "—".to_string()
        };
        let rating = series
            .rating
            .map(|r| format!("{}/10", r))
            .unwrap_or_else(|| "—".to_string());
        let year = series
            .release_year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "—".to_string());

        println!(
            "{:<4} {:<35} {:<8} {:<12} {:<12} {:<8}",
            series.id.unwrap_or(0),
            title,
            year,
            status,
            progress,
            rating
        );
    }
    println!();
}

/// Display TV series in JSON format
fn display_json(series_list: &[TVSeries]) -> Result<()> {
    let entries: Vec<SeriesEntry> = series_list.iter().map(|s| s.clone().into()).collect();
    let json = serde_json::to_string_pretty(&entries)?;
    println!("{}", json);
    Ok(())
}

/// Display TV series in CSV format
fn display_csv(series_list: &[TVSeries], _truncate_length: usize) {
    use super::format::escape_csv;

    println!(
        "ID,Title,Year,Status,TotalSeasons,CurrentSeason,CurrentEpisode,Rating,StartedDate,CompletedDate,Notes"
    );
    for series in series_list {
        let title = escape_csv(&series.title);
        let status = escape_csv(&series.status);
        let year = series
            .release_year
            .map(|y| y.to_string())
            .unwrap_or_default();
        let total_seasons = series
            .total_seasons
            .map(|s| s.to_string())
            .unwrap_or_default();
        let current_season = series
            .current_season
            .map(|s| s.to_string())
            .unwrap_or_default();
        let current_episode = series
            .current_episode
            .map(|e| e.to_string())
            .unwrap_or_default();
        let rating = series.rating.map(|r| r.to_string()).unwrap_or_default();
        let completed_date = series
            .completed_date
            .as_ref()
            .map(|d| escape_csv(d))
            .unwrap_or_default();
        let notes = series
            .notes
            .as_ref()
            .map(|n| escape_csv(n))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            series.id.unwrap_or(0),
            title,
            year,
            status,
            total_seasons,
            current_season,
            current_episode,
            rating,
            escape_csv(&series.started_date),
            completed_date,
            notes
        );
    }
}

/// Sort TV series by specified field and order
fn sort_series(series_list: &mut [TVSeries], sort_by: &str, order: &str) {
    match sort_by.to_lowercase().as_str() {
        "title" => series_list.sort_by(|a, b| a.title.cmp(&b.title)),
        "rating" => series_list.sort_by(|a, b| {
            let a_rating = a.rating.unwrap_or(0.0);
            let b_rating = b.rating.unwrap_or(0.0);
            a_rating
                .partial_cmp(&b_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "status" => series_list.sort_by(|a, b| a.status.cmp(&b.status)),
        "date" => series_list.sort_by(|a, b| a.started_date.cmp(&b.started_date)),
        "year" => series_list.sort_by(|a, b| {
            let a_year = a.release_year.unwrap_or(0);
            let b_year = b.release_year.unwrap_or(0);
            a_year.cmp(&b_year)
        }),
        "progress" => series_list.sort_by(|a, b| {
            let a_progress = (
                a.current_season.unwrap_or(0),
                a.current_episode.unwrap_or(0),
            );
            let b_progress = (
                b.current_season.unwrap_or(0),
                b.current_episode.unwrap_or(0),
            );
            a_progress.cmp(&b_progress)
        }),
        _ => {} // No sorting
    }

    if order.to_lowercase() == "desc" {
        series_list.reverse();
    }
}

/// Truncate string to specified length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5), "Hell…");
        assert_eq!(truncate("Hi", 5), "Hi");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("Exact", 5), "Exact");
    }

    #[test]
    fn test_truncate_long_string() {
        let long_str = "This is a very long TV series title that should be truncated";
        let truncated = truncate(long_str, 10);
        assert!(truncated.ends_with('…'));
        assert!(truncated.len() < long_str.len());
    }

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
        };

        let entry: SeriesEntry = series.into();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.title, "Breaking Bad");
        assert_eq!(entry.status, "completed");
        assert_eq!(entry.rating, Some(9.5));
    }
}
