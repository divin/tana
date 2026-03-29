//! TV Series type definition and operations
//!
//! This module defines the TVSeries struct and implements the Media trait for TV series.

use super::media::Media;
use serde::Serialize;

/// A TV series entry in the database
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TVSeries {
    pub id: Option<i32>,
    pub title: String,
    pub release_year: Option<i32>,
    pub status: String, // "ongoing", "completed", "dropped"
    pub total_seasons: Option<i32>,
    pub current_season: Option<i32>,
    pub current_episode: Option<i32>,
    pub rating: Option<f64>,
    pub started_date: String,
    pub completed_date: Option<String>,
    pub notes: Option<String>,
}

impl TVSeries {
    /// Create a new TV series entry
    pub fn new(title: String, started_date: String, status: String) -> Self {
        TVSeries {
            id: None,
            title,
            release_year: None,
            status,
            total_seasons: None,
            current_season: None,
            current_episode: None,
            rating: None,
            started_date,
            completed_date: None,
            notes: None,
        }
    }

    /// Add a release year
    pub fn with_year(mut self, year: i32) -> Self {
        self.release_year = Some(year);
        self
    }

    /// Set the total number of seasons
    pub fn with_total_seasons(mut self, seasons: i32) -> Self {
        self.total_seasons = Some(seasons);
        self
    }

    /// Set current progress
    pub fn with_progress(mut self, season: i32, episode: i32) -> Self {
        self.current_season = Some(season);
        self.current_episode = Some(episode);
        self
    }

    /// Add a rating
    pub fn with_rating(mut self, rating: f64) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Set the completed date
    pub fn with_completed_date(mut self, date: String) -> Self {
        self.completed_date = Some(date);
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
}

impl Media for TVSeries {
    fn table_name() -> &'static str {
        "tv_series"
    }

    fn media_type_name() -> &'static str {
        "series"
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn rating(&self) -> Option<f64> {
        self.rating
    }

    fn date_added(&self) -> &str {
        &self.started_date
    }

    fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tvseries_creation() {
        let series = TVSeries::new(
            "Breaking Bad".to_string(),
            "2024-01-10".to_string(),
            "completed".to_string(),
        )
        .with_total_seasons(5)
        .with_rating(9.5);

        assert_eq!(series.title, "Breaking Bad");
        assert_eq!(series.total_seasons, Some(5));
        assert_eq!(series.rating, Some(9.5));
    }
}
