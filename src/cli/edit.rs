//! Edit command for modifying existing media entries
//!
//! This module handles the `tana edit` command which allows users to modify
//! existing movies, TV series, or books in the database.

pub mod book;
pub mod movie;
pub mod series;

pub use book::BookEditArgs;
pub use movie::MovieEditArgs;
pub use series::SeriesEditArgs;

use clap::{Args, Subcommand};

use crate::db::Database;
use crate::error::Result;

/// Arguments for the edit command
#[derive(Args, Debug)]
pub struct EditCommand {
    #[command(subcommand)]
    pub media_type: MediaTypeCommand,
}

/// Subcommands for different media types
#[derive(Subcommand, Debug)]
pub enum MediaTypeCommand {
    /// Edit a movie
    Movie(movie::MovieEditArgs),
    /// Edit a TV series
    #[command(name = "series")]
    Series(series::SeriesEditArgs),
    /// Edit a book
    Book(book::BookEditArgs),
}

impl EditCommand {
    /// Execute the edit command
    pub fn execute(self, db: &Database) -> Result<()> {
        match self.media_type {
            MediaTypeCommand::Movie(args) => movie::execute(db, args),
            MediaTypeCommand::Series(args) => series::execute(db, args),
            MediaTypeCommand::Book(args) => book::execute(db, args),
        }
    }
}
