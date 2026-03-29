//! Book delete command implementation
//!
//! This module handles deleting books from the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for deleting a book
#[derive(Args, Debug)]
pub struct BookDeleteArgs {
    /// ID of the book to delete
    pub id: i32,
}

/// Delete a book from the database
pub fn execute(ctx: &AppContext, args: BookDeleteArgs) -> Result<()> {
    let conn = ctx.db().connection();

    // Fetch the book to get its title for the confirmation message
    let book = queries::books::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("Book with ID {}", args.id)))?;

    // Delete from database
    queries::books::delete(conn, args.id)?;

    info!("✓ Deleted book '{}' (ID {})", book.title, args.id);
    println!("✓ Deleted book '{}' (ID {})", book.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_delete_args() {
        let args = BookDeleteArgs { id: 3 };
        assert_eq!(args.id, 3);
    }
}
