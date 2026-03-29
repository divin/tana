//! Movie query operations
//!
//! This module provides database query functions for movie operations.

pub mod count;
pub mod delete;
pub mod get;
pub mod insert;
pub mod update;

// Re-export public functions for convenience
pub use count::count;
pub use delete::delete;
pub use get::{get_all, get_by_id, get_by_year};
pub use insert::insert;
pub use update::update;

