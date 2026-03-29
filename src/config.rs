//! Configuration module for Tana
//!
//! Handles loading and parsing TOML configuration files.
//! Configuration files are expected at `~/.config/tana/config.toml` (XDG standard)
//! or `~/.tanarc` (fallback for backwards compatibility).
//!
//! # Module Structure
//!
//! Following RFC 1733, this module is organized as a hub with submodules:
//! - `format`: Format enum for output formats (plain, csv, json)
//! - `sections`: Configuration section structs (database, display, formatting, defaults, image)
//!
//! # Examples
//!
//! ```ignore
//! use tana::config::Config;
//!
//! let config = Config::load()?;
//! let db_path = config.database_path();
//! let output_format = config.format();
//! ```

pub mod format;
pub mod sections;

use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::error::{Result, TanaError};

// Re-export public API
pub use format::Format;
pub use sections::{DatabaseConfig, DefaultsConfig, DisplayConfig, FormattingConfig, ImageConfig};

/// Main configuration structure
///
/// Combines all configuration sections (database, display, formatting, defaults, images)
/// and provides convenient methods to access configuration values with defaults.
#[derive(Debug, Clone, Deserialize, Default)]
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
    ///
    /// # Returns
    /// * `Ok(Config)` - Successfully loaded or default configuration
    /// * `Err(TanaError)` - File read or parsing error
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
    ///
    /// # Returns
    /// Path to the database file with home directory expanded
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
    ///
    /// # Returns
    /// Path to the images directory with home directory expanded
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
///
/// Replaces the leading `~` with the user's home directory.
/// If HOME environment variable is not set, returns the path as-is.
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
///
/// Uses XDG_CONFIG_HOME environment variable if set, otherwise defaults to ~/.config.
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
///
/// Used for backwards compatibility with older configuration file location.
fn get_legacy_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".tanarc")
    } else {
        PathBuf::from(".tanarc")
    }
}

/// Get the default database path: ~/.local/share/tana/tana.db
///
/// Uses XDG_DATA_HOME environment variable if set, otherwise defaults to ~/.local/share.
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
///
/// Uses XDG_DATA_HOME environment variable if set, otherwise defaults to ~/.local/share.
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
///
/// Fills in default values for any optional config fields that are None.
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
