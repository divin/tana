//! Media type trait definition
//!
//! This module defines the common interface that all media types must implement.
//! The trait-based design allows for easy addition of new media types in the future
//! without needing to modify core logic.

use std::fmt;

/// Trait that all media types must implement
///
/// This trait defines the common interface for all media types,
/// allowing polymorphic operations like filtering, statistics, and display.
pub trait Media: fmt::Debug {
    /// The database table name for this media type
    fn table_name() -> &'static str
    where
        Self: Sized;

    /// The human-readable media type name (e.g., "movie", "book")
    fn media_type_name() -> &'static str
    where
        Self: Sized;

    /// Get the title of this media
    fn title(&self) -> &str;

    /// Get the rating (1-10) if available
    fn rating(&self) -> Option<f64>;

    /// Get the date this media was added/watched/read (YYYY-MM-DD format)
    fn date_added(&self) -> &str;

    /// Get optional notes about this media
    fn notes(&self) -> Option<&str> {
        None
    }
}
