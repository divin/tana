//! Display formatting for TV series
//!
//! This module provides functions for formatting and displaying TV series data
//! in various output formats: plain text tables, JSON, and CSV.

use super::SeriesEntry;
use crate::db::models::TVSeries;
use crate::error::Result;

/// Truncate string to specified length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

/// Display TV series in plain text format with formatted table output
///
/// Displays series in a fixed-width table with columns for ID, Title, Year, Status,
/// Progress (Season/Episode), and Rating. Titles longer than 33 characters are
/// truncated with an ellipsis.
pub fn display_plain(series_list: &[TVSeries], _truncate_length: usize) {
    if series_list.is_empty() {
        println!("No series found.");
        return;
    }

    println!("\n{:=^130}", " TV Series ");
    println!(
        "{:<4} {:<30} {:<8} {:<12} {:<12} {:<8} {:<40}",
        "ID", "Title", "Year", "Status", "Progress", "Rating", "Poster Path"
    );
    println!("{}", "=".repeat(130));

    for series in series_list {
        let title = truncate(&series.title, 28);
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
        let poster_path = series
            .poster_path
            .as_ref()
            .map(|p| truncate(p, 38))
            .unwrap_or_else(|| "N/A".to_string());

        println!(
            "{:<4} {:<30} {:<8} {:<12} {:<12} {:<8} {:<40}",
            series.id.unwrap_or(0),
            title,
            year,
            status,
            progress,
            rating,
            poster_path
        );
    }
    println!();
}

/// Display TV series in JSON format
///
/// Serializes the series list to JSON and prints it with pretty formatting.
/// Each series is converted to a SeriesEntry for serialization.
pub fn display_json(series_list: &[TVSeries]) -> Result<()> {
    let entries: Vec<SeriesEntry> = series_list.iter().map(|s| s.clone().into()).collect();
    let json = serde_json::to_string_pretty(&entries)?;
    println!("{}", json);
    Ok(())
}

/// Display TV series in CSV format
///
/// Outputs series data in comma-separated values format with proper escaping.
/// Includes headers for all fields and properly escapes text fields.
pub fn display_csv(series_list: &[TVSeries], _truncate_length: usize) {
    use super::super::format::escape_csv;

    println!(
        "ID,Title,Year,Status,TotalSeasons,CurrentSeason,CurrentEpisode,Rating,StartedDate,CompletedDate,Notes,PosterPath"
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
        let poster_path = series
            .poster_path
            .as_ref()
            .map(|p| escape_csv(p))
            .unwrap_or_default();

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
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
            notes,
            poster_path
        );
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
}
