# Phase 3 — CLI Commands

## Overview

Phase 3 completed the CLI interface for scripting and automation. The CLI was largely implemented during Phase 1 (`list` and `status` commands), but Phase 3 added proper exit codes and polished the error handling.

Key components:
- Argument parsing with clap's derive macros
- Configuration layering (flags → env → file → defaults)
- Dual output formats (human-readable and JSON)
- Semantic exit codes for scripting

---

## clap Derive Macros

clap provides two APIs: builder and derive. We use derive for type-safe, declarative argument parsing.

### Basic Structure

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "termion")]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long, short = 'H', env = "TERMION_HOST")]
    pub host: Option<String>,

    #[arg(long, short = 'p', env = "TERMION_PORT")]
    pub port: Option<u16>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        #[arg(long)]
        json: bool,
    },
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, short = 'P')]
        position: Option<String>,
    },
}
```

### Key Attributes

| Attribute | Purpose |
|-----------|---------|
| `#[command(...)]` | Command-level metadata (name, version, about) |
| `#[arg(...)]` | Argument configuration |
| `#[command(subcommand)]` | Marks field as subcommand container |

### Argument Attributes

```rust
#[arg(
    long,              // --verbose
    short = 'v',       // -v
    env = "VAR_NAME",  // Read from environment
    default_value = "x", // Default if not provided
    action = ArgAction::Count, // -v -v -v → 3
)]
```

### Optional vs Required

```rust
pub host: Option<String>,  // Optional, None if not provided
pub host: String,          // Required, clap errors if missing
```

### Environment Variable Fallback

```rust
#[arg(long, env = "TERMION_HOST")]
pub host: Option<String>,
```

clap automatically reads from the environment if the flag isn't provided. Precedence: CLI flag > environment variable.

---

## Exit Code Handling

POSIX convention: exit 0 = success, non-zero = failure. We use semantic codes:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Connection failed |
| 3 | Invalid arguments |
| 4 | Resource not found |

### Implementation Pattern

```rust
use std::process::ExitCode;

#[repr(u8)]
pub enum Exit {
    Ok = 0,
    Error = 1,
    Connection = 2,
    Args = 3,
    NotFound = 4,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(exit as u8)
    }
}
```

### Mapping Errors to Exit Codes

```rust
pub fn exit_code_for_error(err: &anyhow::Error) -> Exit {
    if let Some(client_err) = err.downcast_ref::<ClientError>() {
        return match client_err {
            ClientError::Connection { .. } => Exit::Connection,
            ClientError::NotFound { .. } => Exit::NotFound,
            ClientError::Timeout { .. } => Exit::Connection,
            _ => Exit::Error,
        };
    }
    Exit::Error
}
```

### Main Function Pattern

```rust
#[tokio::main]
async fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            e.print().ok();
            return match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Exit::Ok.into(),
                _ => Exit::Args.into(),
            };
        }
    };

    match run(cli).await {
        Ok(()) => Exit::Ok.into(),
        Err(e) => {
            eprintln!("Error: {e}");
            exit_code_for_error(&e).into()
        }
    }
}
```

Key points:
- `try_parse()` instead of `parse()` for manual error handling
- Help/version aren't errors — return exit 0
- Separate `run()` function for clean error propagation
- Print errors to stderr, not stdout

---

## Configuration Layering

Configuration sources in precedence order:

1. CLI flags (highest priority)
2. Environment variables
3. Config file
4. Built-in defaults (lowest priority)

### Implementation

```rust
impl Config {
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let mut config = Self::default();
        
        // Layer 1: Load from file if it exists
        if let Some(path) = Self::find_config_file(cli) {
            let file_config: FileConfig = toml::from_str(&std::fs::read_to_string(&path)?)?;
            config.merge_file(file_config);
        }
        
        // Layer 2: Environment variables (handled by clap for flags)
        // Layer 3: CLI flags override everything
        if let Some(host) = &cli.host {
            config.connection.host = host.clone();
        }
        if let Some(port) = cli.port {
            config.connection.port = port;
        }
        
        config.validate()?;
        Ok(config)
    }
}
```

### Config File Location

Follow XDG Base Directory Specification:

```rust
fn find_config_file(cli: &Cli) -> Option<PathBuf> {
    // 1. Explicit path from CLI/env
    if let Some(path) = &cli.config {
        return Some(path.clone());
    }
    
    // 2. XDG config directory
    if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join("termion").join("config.toml");
        if path.exists() {
            return Some(path);
        }
    }
    
    None
}
```

---

## Output Formatting

### Dual Format Strategy

Every CLI command supports two output modes:
- **Human-readable** (default): Formatted for terminal reading
- **JSON** (`--json`): Machine-parseable for scripting

```rust
pub async fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let data = fetch_data(config).await?;
    
    if json {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print_human_readable(&data);
    }
    
    Ok(())
}
```

### Human-Readable Formatting

```rust
fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bases(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2} Gb", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2} Mb", n as f64 / 1_000_000.0)
    } else {
        format!("{} b", n)
    }
}
```

### JSON Output Guidelines

1. Use serde's derive macros for automatic serialization
2. Skip optional fields when None: `#[serde(skip_serializing_if = "Option::is_none")]`
3. Use snake_case for field names (serde default)
4. Always output valid JSON, even for errors
5. Keep schema stable — breaking changes need version bumps

```rust
#[derive(serde::Serialize)]
struct PositionStatus {
    name: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    reads: u64,
}
```

---

## Error Handling Patterns

### anyhow for Applications

Use `anyhow::Result` in binary code for ergonomic error handling:

```rust
async fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load(&cli)?;  // Any error type works
    let client = Client::connect(&config.host).await?;
    let data = client.fetch().await?;
    Ok(())
}
```

### thiserror for Libraries

Use `thiserror` in library code for typed errors:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Failed to connect to {endpoint}")]
    Connection { endpoint: String, source: Box<dyn Error> },
    
    #[error("{resource} not found: {id}")]
    NotFound { resource: String, id: String },
}
```

### Error Context

Add context when propagating errors:

```rust
let config = Config::load(&cli)
    .context("Failed to load configuration")?;
```

---

## Testing CLI Commands

### Unit Tests for Exit Codes

```rust
#[test]
fn test_connection_error_maps_to_code_2() {
    let err = ClientError::Connection {
        endpoint: "localhost:9501".into(),
        source: "refused".into(),
    };
    let anyhow_err = anyhow::Error::new(err);
    assert_eq!(exit_code_for_error(&anyhow_err), Exit::Connection);
}
```

### Integration Tests

```bash
# Test invalid args
./termion --invalid 2>&1; echo "Exit: $?"
# Expected: error message, Exit: 3

# Test connection failure
./termion -H 127.0.0.1 -p 1 list 2>&1; echo "Exit: $?"
# Expected: connection error, Exit: 2

# Test help (should be exit 0)
./termion --help > /dev/null; echo "Exit: $?"
# Expected: Exit: 0
```

---

## Key Takeaways

1. **clap derive macros** provide type-safe, declarative argument parsing with minimal boilerplate.

2. **Environment variable fallback** is built into clap — just add `env = "VAR_NAME"`.

3. **Semantic exit codes** make CLI tools scriptable — don't just exit 1 for everything.

4. **try_parse() over parse()** gives control over error presentation.

5. **Dual output formats** (human + JSON) serve different use cases without code duplication.

6. **Config layering** follows the principle of least surprise: explicit flags > env > file > defaults.

7. **Error downcasting** with anyhow allows mapping specific error types to exit codes.

---

## Resources

- [clap documentation](https://docs.rs/clap)
- [clap derive tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)
- [serde documentation](https://serde.rs/)
- [anyhow crate](https://docs.rs/anyhow)
- [thiserror crate](https://docs.rs/thiserror)
