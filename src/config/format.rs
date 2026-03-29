//! Output format configuration
//!
//! Defines the supported display formats (plain text, CSV, JSON) and provides
//! parsing and serialization support for format specifications.

use serde::{Deserialize, Serialize};

/// Display format options for command output
///
/// Specifies how command results should be formatted when displayed to the user.
/// Supported formats are plain text tables, CSV, and JSON.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// Plain text output with formatted tables
    #[default]
    Plain,
    /// CSV (Comma-Separated Values) format
    Csv,
    /// JSON format
    Json,
}

impl std::str::FromStr for Format {
    type Err = String;

    /// Parse a format string into a Format enum variant
    ///
    /// # Arguments
    /// * `s` - Format string (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(Format)` - Successfully parsed format
    /// * `Err(String)` - Invalid format string with helpful error message
    ///
    /// # Supported Values
    /// * "plain" - Plain text output
    /// * "csv" - CSV format
    /// * "json" - JSON format
    ///
    /// # Examples
    /// ```ignore
    /// use std::str::FromStr;
    ///
    /// let format = Format::from_str("json").unwrap();
    /// assert_eq!(format, Format::Json);
    /// ```
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(Format::Plain),
            "csv" => Ok(Format::Csv),
            "json" => Ok(Format::Json),
            _ => Err(format!(
                "Invalid format: {}. Valid options: plain, csv, json",
                s
            )),
        }
    }
}

impl std::fmt::Display for Format {
    /// Display the format as a lowercase string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Plain => write!(f, "plain"),
            Format::Csv => write!(f, "csv"),
            Format::Json => write!(f, "json"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str() {
        assert_eq!("plain".parse::<Format>().unwrap(), Format::Plain);
        assert_eq!("csv".parse::<Format>().unwrap(), Format::Csv);
        assert_eq!("json".parse::<Format>().unwrap(), Format::Json);
        assert_eq!("PLAIN".parse::<Format>().unwrap(), Format::Plain);
    }

    #[test]
    fn test_format_display() {
        assert_eq!(Format::Plain.to_string(), "plain");
        assert_eq!(Format::Csv.to_string(), "csv");
        assert_eq!(Format::Json.to_string(), "json");
    }

    #[test]
    fn test_format_from_str_invalid() {
        let result = "invalid".parse::<Format>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid format"));
    }

    #[test]
    fn test_format_default() {
        assert_eq!(Format::default(), Format::Plain);
    }
}
