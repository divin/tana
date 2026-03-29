//! Configuration sections
//!
//! Defines the individual configuration section structs that make up the main Config.
//! Each section handles a specific aspect of the application configuration:
//! - Database: SQLite database location
//! - Display: Output format, debug mode, pagination
//! - Formatting: Text truncation and date formatting
//! - Defaults: Default values for new entries
//! - Image: Image storage and management settings

use super::format::Format;
use serde::{Deserialize, Serialize};

/// Database configuration section
///
/// Handles database-related settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    ///
    /// If not specified, defaults to `~/.local/share/tana/tana.db`.
    /// Supports home directory expansion with `~`.
    pub path: Option<String>,
}

/// Display configuration section
///
/// Controls how command output is displayed to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Output format (plain, csv, json)
    pub format: Option<Format>,
    /// Enable debug mode for verbose logging
    pub debug: Option<bool>,
    /// Number of results to display per page
    pub results_per_page: Option<u32>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            format: Some(Format::Plain),
            debug: Some(false),
            results_per_page: Some(20),
        }
    }
}

/// Formatting configuration section
///
/// Controls how text and dates are formatted in output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Maximum length for truncated strings
    ///
    /// Longer strings are cut off with an ellipsis (...).
    /// Default: 50 characters.
    pub truncate_length: Option<usize>,
    /// Date format string (chrono format)
    ///
    /// Uses chrono's format string syntax.
    /// Default: "%Y-%m-%d" (YYYY-MM-DD)
    pub date_format: Option<String>,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            truncate_length: Some(50),
            date_format: Some("%Y-%m-%d".to_string()),
        }
    }
}

/// Default values for add commands
///
/// Provides default values that can be used when adding new entries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default rating for new entries (1-10)
    pub default_rating: Option<f64>,
    /// Default year for new entries
    pub default_year: Option<i32>,
}

/// Image configuration section
///
/// Handles image file storage and management settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Default directory to store image files
    ///
    /// If not specified, defaults to `~/.local/share/tana/images`.
    /// Supports home directory expansion with `~`.
    pub default_directory: Option<String>,
    /// Whether to auto-copy images to default directory
    ///
    /// When true, image files are automatically copied to the default directory.
    /// When false, only paths are stored without copying files.
    pub auto_copy: Option<bool>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            default_directory: None,
            auto_copy: Some(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, None);
    }

    #[test]
    fn test_display_config_default() {
        let config = DisplayConfig::default();
        assert_eq!(config.format, Some(Format::Plain));
        assert_eq!(config.debug, Some(false));
        assert_eq!(config.results_per_page, Some(20));
    }

    #[test]
    fn test_formatting_config_default() {
        let config = FormattingConfig::default();
        assert_eq!(config.truncate_length, Some(50));
        assert_eq!(config.date_format, Some("%Y-%m-%d".to_string()));
    }

    #[test]
    fn test_defaults_config_default() {
        let config = DefaultsConfig::default();
        assert_eq!(config.default_rating, None);
        assert_eq!(config.default_year, None);
    }

    #[test]
    fn test_image_config_default() {
        let config = ImageConfig::default();
        assert_eq!(config.default_directory, None);
        assert_eq!(config.auto_copy, Some(false));
    }

    #[test]
    fn test_display_config_from_toml() {
        let toml_content = r#"
format = "json"
debug = true
results_per_page = 50
"#;
        let config: DisplayConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.format, Some(Format::Json));
        assert_eq!(config.debug, Some(true));
        assert_eq!(config.results_per_page, Some(50));
    }

    #[test]
    fn test_formatting_config_from_toml() {
        let toml_content = r#"
truncate_length = 80
date_format = "%d/%m/%Y"
"#;
        let config: FormattingConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.truncate_length, Some(80));
        assert_eq!(config.date_format, Some("%d/%m/%Y".to_string()));
    }

    #[test]
    fn test_database_config_from_toml() {
        let toml_content = r#"
path = "~/.local/share/tana/custom.db"
"#;
        let config: DatabaseConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(
            config.path,
            Some("~/.local/share/tana/custom.db".to_string())
        );
    }

    #[test]
    fn test_defaults_config_from_toml() {
        let toml_content = r#"
default_rating = 7.5
default_year = 2024
"#;
        let config: DefaultsConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.default_rating, Some(7.5));
        assert_eq!(config.default_year, Some(2024));
    }

    #[test]
    fn test_image_config_from_toml() {
        let toml_content = r#"
default_directory = "~/Pictures/tana"
auto_copy = true
"#;
        let config: ImageConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(
            config.default_directory,
            Some("~/Pictures/tana".to_string())
        );
        assert_eq!(config.auto_copy, Some(true));
    }
}
