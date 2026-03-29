//! Configuration module for Tana
//!
//! Handles loading and parsing TOML configuration files.
//! Configuration files are expected at `~/.config/tana/config.toml` (XDG standard)
//! or `~/.tanarc` (fallback for backwards compatibility).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{Result, TanaError};

/// Display format options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// Plain text output
    #[default]
    Plain,
    /// CSV format
    Csv,
    /// JSON format
    Json,
}

impl std::str::FromStr for Format {
    type Err = String;

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Plain => write!(f, "plain"),
            Format::Csv => write!(f, "csv"),
            Format::Json => write!(f, "json"),
        }
    }
}

/// Database configuration section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub path: Option<String>,
}

/// Display configuration section
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Maximum length for truncated strings
    pub truncate_length: Option<usize>,
    /// Date format string (chrono format)
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    /// Default rating for new entries (1-10)
    pub default_rating: Option<f64>,
    /// Default year for new entries
    pub default_year: Option<i32>,
}

/// Image configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Default directory to store image files
    pub default_directory: Option<String>,
    /// Whether to auto-copy images to default directory
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

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    /// Display configuration
    #[serde(default)]
    pub display: DisplayConfig,
    /// Formatting configuration
    #[serde(default)]
    pub formatting: FormattingConfig,
    /// Default values for add commands
    #[serde(default)]
    pub defaults: DefaultsConfig,
    /// Image configuration
    #[serde(default)]
    pub images: ImageConfig,
}

impl Config {
    /// Load configuration from file or use defaults
    ///
    /// Attempts to load configuration from:
    /// 1. `~/.config/tana/config.toml` (XDG standard)
    /// 2. `~/.tanarc` (fallback)
    ///
    /// If neither file exists, returns Config with defaults.
    pub fn load() -> Result<Self> {
        let config_paths = vec![get_xdg_config_path(), get_legacy_config_path()];

        for path in config_paths {
            if path.exists() {
                let content = fs::read_to_string(&path)
                    .map_err(|e| TanaError::Other(format!("Failed to read config file: {}", e)))?;

                let mut config: Config = toml::from_str(&content)
                    .map_err(|e| TanaError::Other(format!("Failed to parse config file: {}", e)))?;

                // Apply defaults to missing optional fields
                apply_defaults(&mut config);
                return Ok(config);
            }
        }

        // No config file found, return defaults
        Ok(Config::default())
    }

    /// Get the database path, preferring config value over default
    pub fn database_path(&self) -> PathBuf {
        if let Some(path) = &self.database.path {
            expand_home(path)
        } else {
            get_default_db_path()
        }
    }

    /// Get the output format
    pub fn format(&self) -> Format {
        self.display.format.unwrap_or(Format::Plain)
    }

    /// Check if debug mode is enabled
    pub fn debug(&self) -> bool {
        self.display.debug.unwrap_or(false)
    }

    /// Get results per page setting
    pub fn results_per_page(&self) -> u32 {
        self.display.results_per_page.unwrap_or(20)
    }

    /// Get truncate length setting
    pub fn truncate_length(&self) -> usize {
        self.formatting.truncate_length.unwrap_or(50)
    }

    /// Get date format string
    pub fn date_format(&self) -> &str {
        self.formatting.date_format.as_deref().unwrap_or("%Y-%m-%d")
    }

    /// Get default rating for new entries
    pub fn default_rating(&self) -> Option<f64> {
        self.defaults.default_rating
    }

    /// Get default year for new entries
    pub fn default_year(&self) -> Option<i32> {
        self.defaults.default_year
    }

    /// Get the images default directory, preferring config value over default
    pub fn images_default_directory(&self) -> PathBuf {
        if let Some(path) = &self.images.default_directory {
            expand_home(path)
        } else {
            get_default_images_directory()
        }
    }

    /// Check if auto-copy for images is enabled
    pub fn images_auto_copy(&self) -> bool {
        self.images.auto_copy.unwrap_or(false)
    }
}

/// Expand home directory in path (~/)
fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(path.replacen("~", &home, 1))
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    }
}

/// Get the XDG-compliant config path: ~/.config/tana/config.toml
fn get_xdg_config_path() -> PathBuf {
    let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".config")
    };

    config_dir.join("tana").join("config.toml")
}

/// Get the legacy config path: ~/.tanarc
fn get_legacy_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".tanarc")
    } else {
        PathBuf::from(".tanarc")
    }
}

/// Get the default database path: ~/.local/share/tana/tana.db
fn get_default_db_path() -> PathBuf {
    let data_dir = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share")
    } else {
        PathBuf::from(".local/share")
    };

    data_dir.join("tana").join("tana.db")
}

/// Get the default images directory: ~/.local/share/tana/images
fn get_default_images_directory() -> PathBuf {
    let data_dir = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share")
    } else {
        PathBuf::from(".local/share")
    };

    data_dir.join("tana").join("images")
}

/// Apply defaults to optional config fields
fn apply_defaults(config: &mut Config) {
    if config.display.format.is_none() {
        config.display.format = Some(Format::Plain);
    }
    if config.display.debug.is_none() {
        config.display.debug = Some(false);
    }
    if config.display.results_per_page.is_none() {
        config.display.results_per_page = Some(20);
    }
    if config.formatting.truncate_length.is_none() {
        config.formatting.truncate_length = Some(50);
    }
    if config.formatting.date_format.is_none() {
        config.formatting.date_format = Some("%Y-%m-%d".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.format(), Format::Plain);
        assert_eq!(config.debug(), false);
        assert_eq!(config.results_per_page(), 20);
        assert_eq!(config.truncate_length(), 50);
        assert_eq!(config.date_format(), "%Y-%m-%d");
    }

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
    fn test_config_from_toml_string() {
        let toml_content = r#"
[database]
path = "~/.local/share/tana/custom.db"

[display]
format = "json"
debug = true
results_per_page = 50

[formatting]
truncate_length = 80
date_format = "%d/%m/%Y"

[defaults]
default_rating = 7.5
default_year = 2024
"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.format(), Format::Json);
        assert_eq!(config.debug(), true);
        assert_eq!(config.results_per_page(), 50);
        assert_eq!(config.truncate_length(), 80);
        assert_eq!(config.date_format(), "%d/%m/%Y");
        assert_eq!(config.default_rating(), Some(7.5));
        assert_eq!(config.default_year(), Some(2024));
    }

    #[test]
    fn test_config_partial_toml() {
        let toml_content = r#"
[display]
format = "csv"
"#;

        let mut config: Config = toml::from_str(toml_content).unwrap();
        apply_defaults(&mut config);

        assert_eq!(config.format(), Format::Csv);
        assert_eq!(config.debug(), false);
        assert_eq!(config.results_per_page(), 20);
    }

    #[test]
    fn test_config_getter_methods() {
        let config = Config {
            database: DatabaseConfig {
                path: Some("~/.config/tana/db.db".to_string()),
            },
            display: DisplayConfig {
                format: Some(Format::Json),
                debug: Some(true),
                results_per_page: Some(100),
            },
            formatting: FormattingConfig {
                truncate_length: Some(100),
                date_format: Some("%Y/%m/%d".to_string()),
            },
            defaults: DefaultsConfig {
                default_rating: Some(8.0),
                default_year: Some(2025),
            },
            images: ImageConfig {
                default_directory: Some("~/.local/share/tana/images".to_string()),
                auto_copy: Some(false),
            },
        };

        assert_eq!(config.format(), Format::Json);
        assert_eq!(config.debug(), true);
        assert_eq!(config.results_per_page(), 100);
        assert_eq!(config.truncate_length(), 100);
        assert_eq!(config.date_format(), "%Y/%m/%d");
        assert_eq!(config.default_rating(), Some(8.0));
        assert_eq!(config.default_year(), Some(2025));
    }

    #[test]
    fn test_expand_home() {
        let expanded = expand_home("~/test.txt");
        assert!(expanded.to_string_lossy().contains("test.txt"));

        let not_expanded = expand_home("/absolute/path.txt");
        assert_eq!(not_expanded.to_string_lossy(), "/absolute/path.txt");
    }

    #[test]
    fn test_xdg_config_path() {
        let path = get_xdg_config_path();
        assert!(path.to_string_lossy().contains("tana/config.toml"));
    }

    #[test]
    fn test_legacy_config_path() {
        let path = get_legacy_config_path();
        assert!(path.to_string_lossy().contains(".tanarc"));
    }

    #[test]
    fn test_default_db_path() {
        let path = get_default_db_path();
        assert!(path.to_string_lossy().contains("tana/tana.db"));
    }

    #[test]
    fn test_default_images_directory() {
        let path = get_default_images_directory();
        assert!(path.to_string_lossy().contains("tana/images"));
    }

    #[test]
    fn test_image_config_default() {
        let config = Config::default();
        assert_eq!(config.images_auto_copy(), false);
        assert!(
            config
                .images_default_directory()
                .to_string_lossy()
                .contains("tana/images")
        );
    }

    #[test]
    fn test_image_config_from_toml() {
        let toml_content = r#"
[images]
default_directory = "~/Pictures/tana"
auto_copy = true
"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.images_auto_copy(), true);
        assert!(
            config
                .images_default_directory()
                .to_string_lossy()
                .contains("Pictures/tana")
        );
    }

    #[test]
    fn test_image_config_partial() {
        let toml_content = r#"
[images]
auto_copy = true
"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.images_auto_copy(), true);
        assert!(
            config
                .images_default_directory()
                .to_string_lossy()
                .contains("tana/images")
        );
    }

    #[test]
    fn test_config_with_all_sections() {
        let toml_content = r#"
[database]
path = "~/.local/share/tana/custom.db"

[display]
format = "json"
debug = true

[formatting]
truncate_length = 100

[defaults]
default_rating = 8.0

[images]
default_directory = "~/custom_images"
auto_copy = true
"#;

        let config: Config = toml::from_str(toml_content).unwrap();

        assert_eq!(config.format(), Format::Json);
        assert_eq!(config.debug(), true);
        assert_eq!(config.images_auto_copy(), true);
        assert!(
            config
                .images_default_directory()
                .to_string_lossy()
                .contains("custom_images")
        );
    }
}
