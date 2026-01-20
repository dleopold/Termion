//! Termion — A showcase-quality TUI for monitoring MinKNOW sequencing runs
//!
//! This is the binary entry point. It parses CLI arguments and dispatches
//! to either the TUI or CLI commands.

use clap::Parser;
use termion::cli::{Cli, Commands};
use termion::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration with proper precedence
    let config = Config::load(&cli)?;

    // Initialize logging (to file, not stderr — TUI owns the screen)
    termion::logging::init(&config.logging)?;

    match cli.command {
        Some(Commands::List { json }) => termion::cli::list::run(&config, json).await,
        Some(Commands::Status { json, position }) => {
            termion::cli::status::run(&config, json, position).await
        }
        None => {
            // Default: launch TUI
            termion::tui::run(config).await
        }
    }
}
