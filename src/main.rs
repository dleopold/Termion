//! Termion — A showcase-quality TUI for monitoring MinKNOW sequencing runs
//!
//! This is the binary entry point. It parses CLI arguments and dispatches
//! to either the TUI or CLI commands.

use std::process::ExitCode;

use clap::Parser;
use termion::cli::{exit_code_for_error, Cli, Commands, Exit};
use termion::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            e.print().ok();
            return match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    Exit::Ok.into()
                }
                _ => Exit::Args.into(),
            };
        }
    };

    match run(cli).await {
        Ok(()) => Exit::Ok.into(),
        Err(e) => {
            let exit = exit_code_for_error(&e);
            eprintln!("Error: {e}");
            exit.into()
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
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
