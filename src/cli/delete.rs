//! Delete command for removing media entries
//!
//! This module handles the `tana delete` command which allows users to remove
//! existing movies, TV series, or books from the database.

pub mod book;
pub mod movie;
pub mod series;

pub use book::BookDeleteArgs;
pub use movie::MovieDeleteArgs;
pub use series::SeriesDeleteArgs;

use clap::{Args, Subcommand};

use crate::cli::context::AppContext;
use crate::error::Result;

/// Arguments for the delete command
#[derive(Args, Debug)]
pub struct DeleteCommand {
    #[command(subcommand)]
    pub media_type: MediaTypeCommand,
}

/// Subcommands for different media types
#[derive(Subcommand, Debug)]
pub enum MediaTypeCommand {
    /// Delete a movie
    Movie(movie::MovieDeleteArgs),
    /// Delete a TV series
    #[command(name = "series")]
    Series(series::SeriesDeleteArgs),
    /// Delete a book
    Book(book::BookDeleteArgs),
}

impl DeleteCommand {
    /// Execute the delete command
    pub fn execute(self, ctx: &AppContext) -> Result<()> {
        match self.media_type {
            MediaTypeCommand::Movie(args) => movie::execute(ctx, args),
            MediaTypeCommand::Series(args) => series::execute(ctx, args),
            MediaTypeCommand::Book(args) => book::execute(ctx, args),
        }
    }
}
