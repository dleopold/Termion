# SPEC_CONFIG.md
Configuration Specification — Termion

## Overview

Configuration defines connection parameters, TUI behavior, and logging. Based on decisions D3.4, D5.4, and D7.3.

---

## Precedence

Per decision D3.4:

```
CLI flags → Environment variables → Config file → Defaults
```

Higher precedence sources override lower ones.

---

## Configuration Sources

### 1. CLI Flags

```bash
termion --host 192.168.1.100 --port 9501 -vv
```

| Flag | Type | Description |
|------|------|-------------|
| `--host`, `-h` | String | MinKNOW manager host |
| `--port`, `-p` | u16 | MinKNOW manager port |
| `--config`, `-c` | Path | Config file path |
| `--verbose`, `-v` | Count | Log verbosity (repeatable) |
| `--log` | Path | Custom log file path |

### 2. Environment Variables

```bash
export TERMION_HOST=192.168.1.100
export TERMION_PORT=9501
export TERMION_CONFIG=/etc/termion/config.toml
export TERMION_LOG_LEVEL=debug
```

| Variable | Type | Description |
|----------|------|-------------|
| `TERMION_HOST` | String | MinKNOW manager host |
| `TERMION_PORT` | u16 | MinKNOW manager port |
| `TERMION_CONFIG` | Path | Config file path |
| `TERMION_LOG_LEVEL` | String | Log level (off/error/warn/info/debug/trace) |
| `TERMION_LOG_FILE` | Path | Log file path |

### 3. Config File

Per decision D7.3: `~/.config/termion/config.toml` (XDG_CONFIG_HOME)

```toml
# ~/.config/termion/config.toml

[connection]
# MinKNOW manager host
host = "localhost"

# MinKNOW manager port
port = 9501

# Connection timeout in seconds
connect_timeout = 5

# Request timeout in seconds
request_timeout = 30

[tui]
# Data refresh interval in milliseconds
refresh_interval = 1000

# Chart history duration in seconds
chart_history = 1800  # 30 minutes

[reconnect]
# Initial reconnect delay in milliseconds
initial_delay = 1000

# Maximum reconnect delay in milliseconds  
max_delay = 30000

# Backoff multiplier
multiplier = 2.0

[logging]
# Log level: off, error, warn, info, debug, trace
level = "off"

# Log file path (supports ~ expansion)
file = "~/.local/state/termion/termion.log"
```

### 4. Defaults

| Setting | Default |
|---------|---------|
| `connection.host` | `"localhost"` |
| `connection.port` | `9501` |
| `connection.connect_timeout` | `5` (seconds) |
| `connection.request_timeout` | `30` (seconds) |
| `tui.refresh_interval` | `1000` (ms) |
| `tui.chart_history` | `1800` (seconds) |
| `reconnect.initial_delay` | `1000` (ms) |
| `reconnect.max_delay` | `30000` (ms) |
| `reconnect.multiplier` | `2.0` |
| `logging.level` | `"off"` |
| `logging.file` | `~/.local/state/termion/termion.log` |

---

## Config File Locations

### Search Order

1. Path specified via `--config` flag
2. Path specified via `TERMION_CONFIG` env var
3. `$XDG_CONFIG_HOME/termion/config.toml`
4. `~/.config/termion/config.toml`

### XDG Base Directories

| Purpose | XDG Variable | Default |
|---------|--------------|---------|
| Config | `XDG_CONFIG_HOME` | `~/.config` |
| State (logs) | `XDG_STATE_HOME` | `~/.local/state` |

---

## Rust Types

```rust
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub tui: TuiConfig,
    pub reconnect: ReconnectConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub refresh_interval: Duration,
    pub chart_history: Duration,
}

#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum LogLevel {
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
```

---

## Config Loading

```rust
impl Config {
    /// Load config from all sources with proper precedence
    pub fn load(cli: &CliArgs) -> Result<Self, ConfigError> {
        // 1. Start with defaults
        let mut config = Config::default();
        
        // 2. Load config file (if exists)
        if let Some(file_config) = Self::load_file(cli)? {
            config.merge(file_config);
        }
        
        // 3. Apply environment variables
        config.apply_env();
        
        // 4. Apply CLI flags (highest precedence)
        config.apply_cli(cli);
        
        Ok(config)
    }
    
    fn load_file(cli: &CliArgs) -> Result<Option<FileConfig>, ConfigError> {
        let path = cli.config.clone()
            .or_else(|| std::env::var("TERMION_CONFIG").ok().map(PathBuf::from))
            .or_else(|| dirs::config_dir().map(|d| d.join("termion/config.toml")));
        
        match path {
            Some(p) if p.exists() => {
                let content = std::fs::read_to_string(&p)?;
                let config: FileConfig = toml::from_str(&content)?;
                Ok(Some(config))
            }
            _ => Ok(None),
        }
    }
}
```

---

## Validation

```rust
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Port must be valid
        if self.connection.port == 0 {
            return Err(ConfigError::InvalidPort);
        }
        
        // Timeouts must be positive
        if self.connection.connect_timeout.is_zero() {
            return Err(ConfigError::InvalidTimeout("connect_timeout"));
        }
        
        // Refresh interval must be reasonable (100ms - 60s)
        let refresh = self.tui.refresh_interval.as_millis();
        if refresh < 100 || refresh > 60_000 {
            return Err(ConfigError::InvalidRefreshInterval);
        }
        
        // Backoff multiplier must be > 1
        if self.reconnect.multiplier <= 1.0 {
            return Err(ConfigError::InvalidMultiplier);
        }
        
        Ok(())
    }
}
```

---

## Example Configurations

### Minimal (rely on defaults)

```toml
# Empty file - all defaults
```

### Remote Connection

```toml
[connection]
host = "sequencer.lab.local"
port = 9501
connect_timeout = 10
```

### Debug Mode

```toml
[logging]
level = "debug"
file = "/tmp/termion-debug.log"
```

### High-Frequency Monitoring

```toml
[tui]
refresh_interval = 500  # 500ms = 2Hz

[reconnect]
initial_delay = 500     # Faster reconnect
max_delay = 10000
```

---

## Dependencies

- `figment` or `config` — Multi-source config loading
- `toml` — TOML parsing
- `serde` — Deserialization
- `dirs` — XDG directory resolution
