//! Serve command - for starting the REST API server

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::Database;
use crate::error::Result;
use crate::server;

/// Arguments for the serve command
#[derive(Args, Debug)]
pub struct ServeCommand {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port number to listen on
    #[arg(long, default_value = "8080")]
    pub port: u16,
}

impl ServeCommand {
    /// Execute the serve command
    pub fn execute(&self, ctx: &AppContext) -> Result<()> {
        info!("Starting REST API server on {}:{}", self.host, self.port);

        // Get the database path from config
        let db_path = ctx.config().database_path().to_path_buf();

        // Get CORS origins from config with defaults
        let cors_origins = ctx.config().server.cors_origins.clone().unwrap_or_else(|| {
            vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
            ]
        });

        // Initialize the database once at startup
        // This ensures the database schema is created and logs "Database initialized successfully" once
        let _db = Database::open(&db_path)?;
        drop(_db); // Explicitly drop the database connection to ensure it's closed before the server starts

        // Create a tokio runtime to run the async server
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(crate::error::TanaError::Io)?;

        // Pass the path to the server - handlers will open from this path
        // The database is already initialized, so reopening will be quick
        rt.block_on(server::run(
            db_path,
            self.host.clone(),
            self.port,
            cors_origins,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serve_command_default_values() {
        let cmd = ServeCommand {
            host: "127.0.0.1".to_string(),
            port: 8080,
        };

        assert_eq!(cmd.host, "127.0.0.1");
        assert_eq!(cmd.port, 8080);
    }

    #[test]
    fn test_serve_command_custom_values() {
        let cmd = ServeCommand {
            host: "0.0.0.0".to_string(),
            port: 3000,
        };

        assert_eq!(cmd.host, "0.0.0.0");
        assert_eq!(cmd.port, 3000);
    }
}
