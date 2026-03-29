//! Data models for media entries
//!
//! This module re-exports the media type definitions from the media_type module
//! for convenience and organization.

pub use super::media_type::{Book, Media, Movie, TVSeries};

/// Enum representing all supported media types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaEnum {
    /// Movie media type
    Movie,
    /// TV Series media type
    Series,
    /// Book media type
    Book,
}

impl MediaEnum {
    /// Get the database table name for this media type
    pub fn table_name(&self) -> &'static str {
        match self {
            MediaEnum::Movie => "movies",
            MediaEnum::Series => "tv_series",
            MediaEnum::Book => "books",
        }
    }

    /// Get the human-readable name for this media type
    pub fn display_name(&self) -> &'static str {
        match self {
            MediaEnum::Movie => "movie",
            MediaEnum::Series => "series",
            MediaEnum::Book => "book",
        }
    }

    /// Get all available media types
    pub fn all() -> &'static [MediaEnum] {
        &[MediaEnum::Movie, MediaEnum::Series, MediaEnum::Book]
    }

    /// Parse a string into a MediaTypeEnum
    pub fn parse_from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "movie" | "movies" => Some(MediaEnum::Movie),
            "series" | "tv" | "tv_series" => Some(MediaEnum::Series),
            "book" | "books" => Some(MediaEnum::Book),
            _ => None,
        }
    }
}

impl std::fmt::Display for MediaEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_enum_table_names() {
        assert_eq!(MediaEnum::Movie.table_name(), "movies");
        assert_eq!(MediaEnum::Series.table_name(), "tv_series");
        assert_eq!(MediaEnum::Book.table_name(), "books");
    }

    #[test]
    fn test_media_type_enum_from_str() {
        assert_eq!(MediaEnum::parse_from_str("movie"), Some(MediaEnum::Movie));
        assert_eq!(MediaEnum::parse_from_str("series"), Some(MediaEnum::Series));
        assert_eq!(MediaEnum::parse_from_str("book"), Some(MediaEnum::Book));
        assert_eq!(MediaEnum::parse_from_str("invalid"), None);
    }

    #[test]
    fn test_media_type_enum_all() {
        let all = MediaEnum::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&MediaEnum::Movie));
        assert!(all.contains(&MediaEnum::Series));
        assert!(all.contains(&MediaEnum::Book));
    }
}
