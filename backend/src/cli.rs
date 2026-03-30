//! CLI command module
//!
//! This module handles all command-line interface functionality for tana.
//! It defines the command structure, argument parsing, and command dispatching.

pub mod add;
pub mod context;
pub mod delete;
pub mod edit;
pub mod search;
pub mod serve;
pub mod show;
pub mod stats;

use clap::{Parser, Subcommand};
use tracing_subscriber;

use crate::cli::context::AppContext;
use crate::config::Config;
use crate::db::Database;
use crate::error::Result;

/// Tana - Track your consumed media
#[derive(Parser, Debug)]
#[command(name = "tana")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A CLI tool for tracking consumed media", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable debug logging
    #[arg(global = true, short, long)]
    pub debug: bool,

    /// Path to database file (defaults to ~/.local/share/tana/tana.db)
    #[arg(global = true, long)]
    pub db: Option<String>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new media entry
    #[command(about = "Add a new movie, TV series, or book")]
    Add(add::AddCommand),

    /// Edit an existing media entry
    #[command(about = "Edit an existing movie, TV series, or book")]
    Edit(edit::EditCommand),

    /// Delete a media entry
    #[command(about = "Delete an existing movie, TV series, or book")]
    Delete(delete::DeleteCommand),

    /// Search for media entries
    #[command(about = "Search across all media types")]
    Search(search::SearchCommand),

    /// Serve the REST API server
    #[command(about = "Start the REST API server")]
    Serve(serve::ServeCommand),

    /// Show media entries
    #[command(about = "Display media entries with optional filtering")]
    Show(show::ShowCommand),

    /// Show statistics
    #[command(about = "Display statistics about your media")]
    Stats(stats::StatsCommand),
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        // Load configuration from file or use defaults
        let config = Config::load()?;

        // CLI flags override config settings
        let debug_mode = self.debug || config.debug();

        // Initialize logging
        if debug_mode {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .init();
        }

        // Get database path (CLI flag overrides config)
        let db_path = if let Some(path) = self.db {
            path
        } else {
            config.database_path().to_string_lossy().to_string()
        };

        // Open database
        let db = Database::open(&db_path)?;

        // Create application context with database and config
        let ctx = AppContext::new(db, config);

        // Execute the command
        match self.command {
            Commands::Add(cmd) => cmd.execute(&ctx)?,
            Commands::Edit(cmd) => cmd.execute(&ctx)?,
            Commands::Delete(cmd) => cmd.execute(&ctx)?,
            Commands::Search(cmd) => cmd.execute(&ctx)?,
            Commands::Serve(cmd) => cmd.execute(&ctx)?,
            Commands::Show(cmd) => cmd.execute(&ctx)?,
            Commands::Stats(cmd) => cmd.execute(&ctx)?,
        }

        Ok(())
    }
}
