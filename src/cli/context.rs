//! Application context
//!
//! Holds shared application state including database connection and configuration.
//! Provides convenience methods for accessing configuration values with CLI flag overrides.

use crate::config::Config;
use crate::db::Database;

/// Application context holding database and configuration
pub struct AppContext {
    /// Database connection
    pub database: Database,
    /// Configuration settings
    pub config: Config,
}

impl AppContext {
    /// Create a new application context
    pub fn new(database: Database, config: Config) -> Self {
        Self { database, config }
    }

    /// Get database reference
    pub fn db(&self) -> &Database {
        &self.database
    }

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the output format from config
    pub fn format(&self) -> crate::config::Format {
        self.config.format()
    }

    /// Get debug mode (from config)
    pub fn debug(&self) -> bool {
        self.config.debug()
    }

    /// Get results per page from config
    pub fn results_per_page(&self) -> u32 {
        self.config.results_per_page()
    }

    /// Get truncate length from config
    pub fn truncate_length(&self) -> usize {
        self.config.truncate_length()
    }

    /// Get date format from config
    pub fn date_format(&self) -> &str {
        self.config.date_format()
    }

    /// Get default rating from config (optional)
    pub fn default_rating(&self) -> Option<f64> {
        self.config.default_rating()
    }

    /// Get default year from config (optional)
    pub fn default_year(&self) -> Option<i32> {
        self.config.default_year()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_context_creation() {
        let db = Database::open_memory().expect("Failed to create memory database");
        let config = Config::default();
        let ctx = AppContext::new(db, config);

        assert_eq!(ctx.format(), crate::config::Format::Plain);
        assert_eq!(ctx.debug(), false);
    }

    #[test]
    fn test_app_context_getters() {
        let db = Database::open_memory().expect("Failed to create memory database");
        let config = Config::default();
        let ctx = AppContext::new(db, config);

        assert_eq!(ctx.truncate_length(), 50);
        assert_eq!(ctx.results_per_page(), 20);
        assert_eq!(ctx.date_format(), "%Y-%m-%d");
    }
}
