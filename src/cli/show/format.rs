//! Output formatting utilities for the show command
//!
//! This module provides format enum and helper functions for
//! serializing media entries in different output formats.

use std::str::FromStr;

/// Output format for show command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Plain text table format (default)
    Plain,
    /// JSON format for machine readability
    Json,
    /// CSV format for spreadsheets
    Csv,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Format::Plain),
            "json" => Ok(Format::Json),
            "csv" => Ok(Format::Csv),
            _ => Err(format!(
                "Invalid format: {}. Valid options: plain, json, csv",
                s
            )),
        }
    }
}

/// Escape CSV field by wrapping in quotes if needed
///
/// Fields containing commas, quotes, or newlines are wrapped in double quotes,
/// and any internal quotes are escaped by doubling them.
pub fn escape_csv(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str_plain() {
        assert_eq!(Format::from_str("plain"), Ok(Format::Plain));
        assert_eq!(Format::from_str("PLAIN"), Ok(Format::Plain));
    }

    #[test]
    fn test_format_from_str_json() {
        assert_eq!(Format::from_str("json"), Ok(Format::Json));
        assert_eq!(Format::from_str("JSON"), Ok(Format::Json));
    }

    #[test]
    fn test_format_from_str_csv() {
        assert_eq!(Format::from_str("csv"), Ok(Format::Csv));
        assert_eq!(Format::from_str("CSV"), Ok(Format::Csv));
    }

    #[test]
    fn test_format_from_str_invalid() {
        assert!(Format::from_str("invalid").is_err());
    }

    #[test]
    fn test_escape_csv_simple() {
        assert_eq!(escape_csv("Hello"), "Hello");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        assert_eq!(escape_csv("Hello, World"), "\"Hello, World\"");
    }

    #[test]
    fn test_escape_csv_with_quote() {
        assert_eq!(escape_csv("Say \"Hi\""), "\"Say \"\"Hi\"\"\"");
    }

    #[test]
    fn test_escape_csv_with_newline() {
        assert_eq!(escape_csv("Line1\nLine2"), "\"Line1\nLine2\"");
    }
}
