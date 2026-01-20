//! CLI command definitions and implementations.

use clap::{Parser, Subcommand};

pub mod exit;
pub mod list;
pub mod status;

pub use exit::{exit_code_for_error, Exit};

/// Termion — Monitor MinKNOW sequencing runs
#[derive(Parser, Debug)]
#[command(name = "termion")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// MinKNOW manager host
    #[arg(long, short = 'H', env = "TERMION_HOST")]
    pub host: Option<String>,

    /// MinKNOW manager port
    #[arg(long, short = 'p', env = "TERMION_PORT")]
    pub port: Option<u16>,

    /// Config file path
    #[arg(long, short = 'c', env = "TERMION_CONFIG")]
    pub config: Option<std::path::PathBuf>,

    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log file path
    #[arg(long)]
    pub log: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List devices and positions
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show run status and metrics
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Filter by position ID
        #[arg(long, short = 'P')]
        position: Option<String>,
    },
}
