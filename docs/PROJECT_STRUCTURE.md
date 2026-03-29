# 🏗️ Tana Project Structure

## Overview

Tana is organized as a Rust project with a modular architecture that separates concerns into CLI handling, database operations, and configuration management. The codebase follows RFC 1733 conventions (no `mod.rs` files except at crate root).

## Directory Structure

```
tana/
├── src/                          # Source code
│   ├── main.rs                  # Entry point
│   ├── lib.rs                   # Library root, exports public modules
│   ├── cli.rs                   # CLI command definitions and execution
│   ├── cli/                     # CLI subcommands
│   │   ├── add.rs              # Hub module for add command
│   │   ├── add/                # Add command implementations
│   │   │   ├── movie.rs
│   │   │   ├── book.rs
│   │   │   └── series.rs
│   │   ├── show.rs             # Hub module for show command
│   │   ├── show/               # Show command implementations
│   │   │   ├── format.rs       # Output formatting (table, json, csv)
│   │   │   ├── movie.rs
│   │   │   ├── book.rs
│   │   │   └── series.rs
│   │   ├── edit.rs             # Hub module for edit command
│   │   ├── delete.rs           # Delete command
│   │   ├── search.rs           # Search command
│   │   ├── stats.rs            # Statistics command
│   │   └── context.rs          # AppContext with database and config
│   ├── db.rs                    # Database module root
│   ├── db/                      # Database layer
│   │   ├── queries/            # Query modules organized by media type
│   │   │   ├── movies.rs
│   │   │   ├── books.rs
│   │   │   └── tv_series.rs
│   │   └── models/             # Data models
│   │       ├── movie.rs
│   │       ├── book.rs
│   │       └── series.rs
│   ├── config.rs               # Configuration management
│   ├── error.rs                # Error types and handling
│   └── image.rs                # Image support and utilities
│
├── migrations/                 # Database schema migrations
│   ├── 001_initial_schema.sql  # Create tables
│   └── 002_add_poster_paths.sql # Add image columns
│
├── docs/                       # Documentation
│   ├── SCHEMA.md              # Database schema reference
│   ├── CLI.md                 # CLI command documentation
│   └── PROJECT_STRUCTURE.md   # This file
│
├── Cargo.toml                 # Project manifest and dependencies
├── Cargo.lock                 # Locked dependencies
├── README.md                  # Main readme
├── LICENSE-MIT
├── LICENSE-APACHE
└── target/                    # Build output (ignored)
    ├── debug/
    └── release/

```
