//! Logging initialization.
//!
//! Logging is configured to write to a file, never to stderr,
//! because the TUI owns the terminal screen.

use crate::config::{LogLevel, LoggingConfig};
use std::fs;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging based on configuration.
///
/// Returns a guard that must be kept alive for the duration of the program
/// to ensure all logs are flushed.
pub fn init(config: &LoggingConfig) -> anyhow::Result<Option<WorkerGuard>> {
    // If logging is off, don't set up anything
    if config.level == LogLevel::Off {
        return Ok(None);
    }

    // Ensure log directory exists
    if let Some(parent) = config.file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create log file
    let file = fs::File::create(&config.file)?;

    // Set up non-blocking writer
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    // Build filter
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level: tracing::Level = config.level.into();
        EnvFilter::new(level.to_string())
    });

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    tracing::info!(
        level = ?config.level,
        file = %config.file.display(),
        "Logging initialized"
    );

    Ok(Some(guard))
}
