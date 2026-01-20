//! # Termion
//!
//! A showcase-quality TUI for monitoring MinKNOW nanopore sequencing runs.
//!
//! ## Overview
//!
//! Termion provides real-time visualization of nanopore sequencing data via
//! a terminal user interface. It connects to MinKNOW's gRPC API to stream
//! statistics, display run states, and allow basic run control.
//!
//! ## Modules
//!
//! - [`client`] — gRPC client for MinKNOW API
//! - [`tui`] — Terminal user interface
//! - [`cli`] — Command-line interface commands
//! - [`config`] — Configuration loading and validation

pub mod cli;
pub mod client;
pub mod config;
pub mod logging;
mod proto;
pub mod tui;

// Re-export commonly used types
pub use client::Client;
pub use config::Config;
