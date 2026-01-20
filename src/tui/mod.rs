//! Terminal User Interface.
//!
//! This module implements the TUI using ratatui and crossterm.
//! It provides real-time visualization of sequencing data.

use crate::config::Config;

/// Run the TUI application.
pub async fn run(_config: Config) -> anyhow::Result<()> {
    // TODO: Implement TUI in Phase 2
    println!("TUI not yet implemented. Use 'termion list' or 'termion status' for now.");
    Ok(())
}
