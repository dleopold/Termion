//! Configuration types.

use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub tui: TuiConfig,
    pub reconnect: ReconnectConfig,
    pub logging: LoggingConfig,
}

/// Connection settings.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// MinKNOW manager host.
    pub host: String,

    /// MinKNOW manager port.
    pub port: u16,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Request timeout.
    pub request_timeout: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 9501,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// TUI settings.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Data refresh interval.
    pub refresh_interval: Duration,

    /// Chart history duration.
    pub chart_history: Duration,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_interval: Duration::from_millis(1000),
            chart_history: Duration::from_secs(1800), // 30 minutes
        }
    }
}

/// Reconnection settings.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial reconnect delay.
    pub initial_delay: Duration,

    /// Maximum reconnect delay.
    pub max_delay: Duration,

    /// Backoff multiplier.
    pub multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_millis(30000),
            multiplier: 2.0,
        }
    }
}

/// Logging settings.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level.
    pub level: LogLevel,

    /// Log file path.
    pub file: PathBuf,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Off,
            file: dirs::state_dir()
                .or_else(dirs::data_local_dir)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("termion/termion.log"),
        }
    }
}

/// Log level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogLevel {
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::str::FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(LogLevel::Off),
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(()),
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Off => tracing::Level::ERROR, // Will be filtered anyway
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

// --- File config (for TOML parsing) ---

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub connection: Option<FileConnectionConfig>,
    pub tui: Option<FileTuiConfig>,
    pub reconnect: Option<FileReconnectConfig>,
    pub logging: Option<FileLoggingConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub connect_timeout: Option<u64>,
    pub request_timeout: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileTuiConfig {
    pub refresh_interval: Option<u64>,
    pub chart_history: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileReconnectConfig {
    pub initial_delay: Option<u64>,
    pub max_delay: Option<u64>,
    pub multiplier: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileLoggingConfig {
    pub level: Option<String>,
    pub file: Option<String>,
}

// --- Errors ---

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file {}: {}", path.display(), source)]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file {}: {}", path.display(), source)]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("Invalid port: must be non-zero")]
    InvalidPort,

    #[error("Invalid timeout: {} must be positive", .0)]
    InvalidTimeout(&'static str),

    #[error("Invalid refresh interval: must be between 100ms and 60s")]
    InvalidRefreshInterval,

    #[error("Invalid multiplier: must be greater than 1.0")]
    InvalidMultiplier,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.connection.host, "localhost");
        assert_eq!(config.connection.port, 9501);
        assert_eq!(config.connection.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.tui.refresh_interval, Duration::from_millis(1000));
        assert_eq!(config.reconnect.multiplier, 2.0);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("off".parse::<LogLevel>().unwrap(), LogLevel::Off);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_level_to_tracing() {
        assert_eq!(tracing::Level::from(LogLevel::Error), tracing::Level::ERROR);
        assert_eq!(tracing::Level::from(LogLevel::Warn), tracing::Level::WARN);
        assert_eq!(tracing::Level::from(LogLevel::Info), tracing::Level::INFO);
        assert_eq!(tracing::Level::from(LogLevel::Debug), tracing::Level::DEBUG);
        assert_eq!(tracing::Level::from(LogLevel::Trace), tracing::Level::TRACE);
    }

    #[test]
    fn test_file_config_deserialization() {
        let toml = r#"
[connection]
host = "192.168.1.100"
port = 9502
connect_timeout = 10

[tui]
refresh_interval = 500

[reconnect]
multiplier = 1.5

[logging]
level = "debug"
"#;
        let file_config: FileConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            file_config.connection.as_ref().unwrap().host,
            Some("192.168.1.100".to_string())
        );
        assert_eq!(file_config.connection.as_ref().unwrap().port, Some(9502));
        assert_eq!(
            file_config.tui.as_ref().unwrap().refresh_interval,
            Some(500)
        );
        assert_eq!(
            file_config.reconnect.as_ref().unwrap().multiplier,
            Some(1.5)
        );
        assert_eq!(
            file_config.logging.as_ref().unwrap().level,
            Some("debug".to_string())
        );
    }

    #[test]
    fn test_file_config_partial() {
        let toml = r#"
[connection]
host = "remote-host"
"#;
        let file_config: FileConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            file_config.connection.as_ref().unwrap().host,
            Some("remote-host".to_string())
        );
        assert!(file_config.connection.as_ref().unwrap().port.is_none());
        assert!(file_config.tui.is_none());
    }
}
