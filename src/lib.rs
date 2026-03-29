//! Tana - A CLI tool for tracking consumed media
//!
//! This library provides the core functionality for the tana media tracking tool.
//! It handles database operations and CLI commands.

pub mod cli;
pub mod config;
pub mod db;
pub mod error;
pub mod image;

pub use cli::context;
pub use error::{Result, TanaError};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
