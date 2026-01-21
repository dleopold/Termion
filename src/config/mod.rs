//! Configuration loading and management.
//!
//! Configuration is loaded with the following precedence (highest first):
//! 1. CLI flags
//! 2. Environment variables
//! 3. Config file
//! 4. Defaults

mod types;

pub use types::*;

use crate::cli::Cli;
use std::path::PathBuf;
use std::time::Duration;

impl Config {
    /// Load configuration from all sources with proper precedence.
    pub fn load(cli: &Cli) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Load config file if it exists
        if let Some(file_config) = Self::load_file(cli)? {
            config.merge(file_config);
        }

        // Apply environment variables
        config.apply_env();

        // Apply CLI flags (highest precedence)
        config.apply_cli(cli);

        // Validate final config
        config.validate()?;

        Ok(config)
    }

    fn load_file(cli: &Cli) -> Result<Option<FileConfig>, ConfigError> {
        let path = cli
            .config
            .clone()
            .or_else(|| std::env::var("TERMION_CONFIG").ok().map(PathBuf::from))
            .or_else(|| dirs::config_dir().map(|d| d.join("termion/config.toml")));

        match path {
            Some(p) if p.exists() => {
                tracing::debug!(path = %p.display(), "Loading config file");
                let content = std::fs::read_to_string(&p).map_err(|e| ConfigError::Read {
                    path: p.clone(),
                    source: e,
                })?;
                let file_config: FileConfig = toml::from_str(&content)
                    .map_err(|e| ConfigError::Parse { path: p, source: e })?;
                Ok(Some(file_config))
            }
            _ => Ok(None),
        }
    }

    fn merge(&mut self, file: FileConfig) {
        if let Some(conn) = file.connection {
            if let Some(host) = conn.host {
                self.connection.host = host;
            }
            if let Some(port) = conn.port {
                self.connection.port = port;
            }
            if let Some(timeout) = conn.connect_timeout {
                self.connection.connect_timeout = Duration::from_secs(timeout);
            }
            if let Some(timeout) = conn.request_timeout {
                self.connection.request_timeout = Duration::from_secs(timeout);
            }
        }

        if let Some(tui) = file.tui {
            if let Some(interval) = tui.refresh_interval {
                self.tui.refresh_interval = Duration::from_millis(interval);
            }
            if let Some(history) = tui.chart_history {
                self.tui.chart_history = Duration::from_secs(history);
            }
            if let Some(theme) = tui.theme {
                self.tui.theme = theme;
            }
        }

        if let Some(reconnect) = file.reconnect {
            if let Some(delay) = reconnect.initial_delay {
                self.reconnect.initial_delay = Duration::from_millis(delay);
            }
            if let Some(delay) = reconnect.max_delay {
                self.reconnect.max_delay = Duration::from_millis(delay);
            }
            if let Some(mult) = reconnect.multiplier {
                self.reconnect.multiplier = mult;
            }
        }

        if let Some(logging) = file.logging {
            if let Some(level) = logging.level {
                self.logging.level = level.parse().unwrap_or_default();
            }
            if let Some(file) = logging.file {
                self.logging.file = expand_tilde(&file);
            }
        }
    }

    fn apply_env(&mut self) {
        if let Ok(host) = std::env::var("TERMION_HOST") {
            self.connection.host = host;
        }
        if let Ok(port) = std::env::var("TERMION_PORT") {
            if let Ok(p) = port.parse() {
                self.connection.port = p;
            }
        }
        if let Ok(level) = std::env::var("TERMION_LOG_LEVEL") {
            self.logging.level = level.parse().unwrap_or_default();
        }
        if let Ok(file) = std::env::var("TERMION_LOG_FILE") {
            self.logging.file = PathBuf::from(file);
        }
    }

    fn apply_cli(&mut self, cli: &Cli) {
        if let Some(ref host) = cli.host {
            self.connection.host = host.clone();
        }
        if let Some(port) = cli.port {
            self.connection.port = port;
        }
        if cli.verbose > 0 {
            self.logging.level = match cli.verbose {
                1 => LogLevel::Info,
                2 => LogLevel::Debug,
                _ => LogLevel::Trace,
            };
        }
        if let Some(ref log) = cli.log {
            self.logging.file = log.clone();
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.connection.port == 0 {
            return Err(ConfigError::InvalidPort);
        }

        if self.connection.connect_timeout.is_zero() {
            return Err(ConfigError::InvalidTimeout("connect_timeout"));
        }

        let refresh = self.tui.refresh_interval.as_millis();
        if !(100..=60_000).contains(&refresh) {
            return Err(ConfigError::InvalidRefreshInterval);
        }

        if self.reconnect.multiplier <= 1.0 {
            return Err(ConfigError::InvalidMultiplier);
        }

        Ok(())
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validates() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port_zero() {
        let mut config = Config::default();
        config.connection.port = 0;
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidPort)));
    }

    #[test]
    fn test_invalid_connect_timeout_zero() {
        let mut config = Config::default();
        config.connection.connect_timeout = Duration::ZERO;
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidTimeout(_))));
    }

    #[test]
    fn test_invalid_refresh_interval_too_low() {
        let mut config = Config::default();
        config.tui.refresh_interval = Duration::from_millis(50);
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidRefreshInterval)));
    }

    #[test]
    fn test_invalid_refresh_interval_too_high() {
        let mut config = Config::default();
        config.tui.refresh_interval = Duration::from_secs(120);
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidRefreshInterval)));
    }

    #[test]
    fn test_invalid_multiplier_too_low() {
        let mut config = Config::default();
        config.reconnect.multiplier = 0.5;
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidMultiplier)));
    }

    #[test]
    fn test_invalid_multiplier_exactly_one() {
        let mut config = Config::default();
        config.reconnect.multiplier = 1.0;
        let result = config.validate();
        assert!(matches!(result, Err(ConfigError::InvalidMultiplier)));
    }

    #[test]
    fn test_merge_connection_config() {
        let mut config = Config::default();
        let file = FileConfig {
            connection: Some(FileConnectionConfig {
                host: Some("remote-host".into()),
                port: Some(9999),
                connect_timeout: Some(10),
                request_timeout: None,
            }),
            tui: None,
            reconnect: None,
            logging: None,
        };
        config.merge(file);
        assert_eq!(config.connection.host, "remote-host");
        assert_eq!(config.connection.port, 9999);
        assert_eq!(config.connection.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.connection.request_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_merge_tui_config() {
        let mut config = Config::default();
        let file = FileConfig {
            connection: None,
            tui: Some(FileTuiConfig {
                refresh_interval: Some(500),
                chart_history: Some(3600),
                theme: None,
            }),
            reconnect: None,
            logging: None,
        };
        config.merge(file);
        assert_eq!(config.tui.refresh_interval, Duration::from_millis(500));
        assert_eq!(config.tui.chart_history, Duration::from_secs(3600));
    }

    #[test]
    fn test_merge_reconnect_config() {
        let mut config = Config::default();
        let file = FileConfig {
            connection: None,
            tui: None,
            reconnect: Some(FileReconnectConfig {
                initial_delay: Some(2000),
                max_delay: Some(60000),
                multiplier: Some(3.0),
            }),
            logging: None,
        };
        config.merge(file);
        assert_eq!(config.reconnect.initial_delay, Duration::from_millis(2000));
        assert_eq!(config.reconnect.max_delay, Duration::from_millis(60000));
        assert_eq!(config.reconnect.multiplier, 3.0);
    }

    #[test]
    fn test_expand_tilde() {
        let path = expand_tilde("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));

        let path = expand_tilde("relative/path");
        assert_eq!(path, PathBuf::from("relative/path"));

        if let Some(home) = dirs::home_dir() {
            let path = expand_tilde("~/logs/test.log");
            assert_eq!(path, home.join("logs/test.log"));
        }
    }
}
