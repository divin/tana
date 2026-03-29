//! Stats command - for displaying statistics

use clap::{Args, Subcommand};
use tracing::debug;

use crate::cli::context::AppContext;
use crate::db::{queries, schema::SchemaInfo};
use crate::error::Result;

/// Arguments for the stats command
#[derive(Args, Debug)]
pub struct StatsCommand {
    #[command(subcommand)]
    pub subcommand: Option<StatsSubcommand>,
}

/// Subcommands for different statistics
#[derive(Subcommand, Debug)]
pub enum StatsSubcommand {
    /// Statistics for movies
    Movies,
    /// Statistics for TV series
    #[command(name = "series")]
    Series,
    /// Statistics for books
    Books,
}

impl StatsCommand {
    /// Execute the stats command
    pub fn execute(self, ctx: &AppContext) -> Result<()> {
        match self.subcommand {
            Some(StatsSubcommand::Movies) => show_movies_stats(ctx),
            Some(StatsSubcommand::Series) => show_series_stats(ctx),
            Some(StatsSubcommand::Books) => show_books_stats(ctx),
            None => show_overall_stats(ctx),
        }
    }
}

/// Display overall statistics
fn show_overall_stats(ctx: &AppContext) -> Result<()> {
    debug!("Showing overall stats");

    let conn = ctx.db().connection();

    // Get counts
    let movie_count = queries::movies::count(conn)?;
    let series_count = queries::tv_series::count(conn)?;
    let book_count = queries::books::count(conn)?;
    let total = movie_count + series_count + book_count;

    println!("\n{:=^50}", " Tana Statistics ");
    println!();
    println!("Total Media Tracked: {}", total);
    println!("  🎬 Movies: {}", movie_count);
    println!("  📺 TV Series: {}", series_count);
    println!("  📚 Books: {}", book_count);
    println!();

    let version = SchemaInfo::get_schema_version(conn)?;
    println!("Database Schema Version: {}", version);
    println!();

    Ok(())
}

/// Display movie statistics
fn show_movies_stats(ctx: &AppContext) -> Result<()> {
    debug!("Showing movie stats");

    let conn = ctx.db().connection();
    let movies = queries::movies::get_all(conn, None)?;

    if movies.is_empty() {
        println!("No movies found.");
        return Ok(());
    }

    let count = movies.len();
    let avg_rating = movies.iter().filter_map(|m| m.rating).sum::<f64>() / count.max(1) as f64;
    let rated_count = movies.iter().filter(|m| m.rating.is_some()).count();

    println!("\n{:=^50}", " Movie Statistics ");
    println!();
    println!("Total Movies: {}", count);
    println!("Rated Movies: {}", rated_count);
    println!("Average Rating: {:.2}/10", avg_rating);
    println!();

    Ok(())
}

/// Display TV series statistics
fn show_series_stats(ctx: &AppContext) -> Result<()> {
    debug!("Showing series stats");

    let conn = ctx.db().connection();
    let series_list = queries::tv_series::get_all(conn, None)?;

    if series_list.is_empty() {
        println!("No TV series found.");
        return Ok(());
    }

    let count = series_list.len();
    let completed = series_list
        .iter()
        .filter(|s| s.status == "completed")
        .count();
    let ongoing = series_list.iter().filter(|s| s.status == "ongoing").count();

    let avg_rating = series_list.iter().filter_map(|s| s.rating).sum::<f64>() / count.max(1) as f64;

    println!("\n{:=^50}", " TV Series Statistics ");
    println!();
    println!("Total Series: {}", count);
    println!("  ✓ Completed: {}", completed);
    println!("  ► Ongoing: {}", ongoing);
    println!();
    println!("Average Rating: {:.2}/10", avg_rating);
    println!();

    Ok(())
}

/// Display book statistics
fn show_books_stats(ctx: &AppContext) -> Result<()> {
    debug!("Showing book stats");

    let conn = ctx.db().connection();
    let books = queries::books::get_all(conn, None)?;

    if books.is_empty() {
        println!("No books found.");
        return Ok(());
    }

    let count = books.len();
    let avg_rating = books.iter().filter_map(|b| b.rating).sum::<f64>() / count.max(1) as f64;

    let total_pages: i32 = books.iter().filter_map(|b| b.pages).sum();

    println!("\n{:=^50}", " Book Statistics ");
    println!();
    println!("Total Books: {}", count);
    println!("Total Pages Read: {}", total_pages);
    println!("Average Rating: {:.2}/10", avg_rating);
    println!();

    Ok(())
}
