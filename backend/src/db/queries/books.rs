//! Book query operations
//!
//! This module provides database query functions for book operations.

pub mod count;
pub mod delete;
pub mod get;
pub mod insert;
pub mod update;

// Re-export public functions for convenience
pub use count::count;
pub use delete::delete;
pub use get::{get_all, get_by_author, get_by_genre, get_by_id};
pub use insert::insert;
pub use update::update;
