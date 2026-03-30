//! Tana - A CLI tool for tracking consumed media

use clap::Parser;
use tana::cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = cli.execute() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
