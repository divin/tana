//! Movie delete command implementation
//!
//! This module handles deleting movies from the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for deleting a movie
#[derive(Args, Debug)]
pub struct MovieDeleteArgs {
    /// ID of the movie to delete
    pub id: i32,
}

/// Delete a movie from the database
pub fn execute(ctx: &AppContext, args: MovieDeleteArgs) -> Result<()> {
    let conn = ctx.db().connection();

    // Fetch the movie to get its title for the confirmation message
    let movie = queries::movies::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("Movie with ID {}", args.id)))?;

    // Delete from database
    queries::movies::delete(conn, args.id)?;

    info!("✓ Deleted movie '{}' (ID {})", movie.title, args.id);
    println!("✓ Deleted movie '{}' (ID {})", movie.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_delete_args() {
        let args = MovieDeleteArgs { id: 1 };
        assert_eq!(args.id, 1);
    }
}
