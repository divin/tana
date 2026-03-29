//! Add command for creating new media entries
//!
//! This module handles the `tana add` command which allows users to add
//! new movies, TV series, or books to the database.

pub mod book;
pub mod movie;
pub mod series;

pub use book::BookArgs;
pub use movie::MovieArgs;
pub use series::SeriesArgs;

use clap::{Args, Subcommand};

use crate::db::Database;
use crate::error::Result;

/// Arguments for the add command
#[derive(Args, Debug)]
pub struct AddCommand {
    #[command(subcommand)]
    pub media_type: MediaTypeCommand,
}

/// Subcommands for different media types
#[derive(Subcommand, Debug)]
pub enum MediaTypeCommand {
    /// Add a movie
    Movie(MovieArgs),
    /// Add a TV series
    #[command(name = "series")]
    Series(SeriesArgs),
    /// Add a book
    Book(BookArgs),
}

impl AddCommand {
    /// Execute the add command
    pub fn execute(self, db: &Database) -> Result<()> {
        match self.media_type {
            MediaTypeCommand::Movie(args) => movie::execute(db, args),
            MediaTypeCommand::Series(args) => series::execute(db, args),
            MediaTypeCommand::Book(args) => book::execute(db, args),
        }
    }
}
