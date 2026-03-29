//! TV series delete command implementation
//!
//! This module handles deleting TV series from the database.

use clap::Args;
use tracing::info;

use crate::cli::context::AppContext;
use crate::db::queries;
use crate::error::Result;

/// Arguments for deleting a TV series
#[derive(Args, Debug)]
pub struct SeriesDeleteArgs {
    /// ID of the series to delete
    pub id: i32,
}

/// Delete a TV series from the database
pub fn execute(ctx: &AppContext, args: SeriesDeleteArgs) -> Result<()> {
    let conn = ctx.db().connection();

    // Fetch the series to get its title for the confirmation message
    let series = queries::tv_series::get_by_id(conn, args.id)?
        .ok_or_else(|| crate::TanaError::MediaNotFound(format!("TV Series with ID {}", args.id)))?;

    // Delete from database
    queries::tv_series::delete(conn, args.id)?;

    info!("✓ Deleted TV series '{}' (ID {})", series.title, args.id);
    println!("✓ Deleted TV series '{}' (ID {})", series.title, args.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_delete_args() {
        let args = SeriesDeleteArgs { id: 2 };
        assert_eq!(args.id, 2);
    }
}
