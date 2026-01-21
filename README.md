# Termion

```
  ████████╗███████╗██████╗ ███╗   ███╗██╗ ██████╗ ███╗   ██╗
  ╚══██╔══╝██╔════╝██╔══██╗████╗ ████║██║██╔═══██╗████╗  ██║
     ██║   █████╗  ██████╔╝██╔████╔██║██║██║   ██║██╔██╗ ██║
     ██║   ██╔══╝  ██╔══██╗██║╚██╔╝██║██║██║   ██║██║╚██╗██║
     ██║   ███████╗██║  ██║██║ ╚═╝ ██║██║╚██████╔╝██║ ╚████║
     ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝
                 Real-time nanopore sequencing monitor
```

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/dleopold/Termion/actions/workflows/ci.yml/badge.svg)](https://github.com/dleopold/Termion/actions/workflows/ci.yml)

A showcase-quality TUI for monitoring Oxford Nanopore MinKNOW sequencing runs. Built with Rust for performance and reliability.

---

## Features

**Real-time Dashboard**
- Live throughput charts with time-series visualization
- Yield tracking with automatic unit scaling (bp, Kb, Mb, Gb)
- Read length distribution histograms with range selection
- Channel activity heatmap showing pore states

**Run Monitoring**
- Device and position discovery
- Run state indicators (running, paused, idle, finishing)
- Duty time tracking per channel state
- Mux scan detection and status

**Run Control**
- Pause/resume sequencing
- Stop acquisition
- Keyboard-driven interface

**CLI for Scripting**
- `termion list` — List devices and positions
- `termion status` — Get run metrics
- JSON output for automation
- Proper exit codes for scripting

---

## Screenshots

### Yield Chart
![Yield Chart](assets/details-yield.png)

### Read Length Distribution
![Read Length Distribution](assets/details-lengths.png)

### Channel Activity
![Channel Activity](assets/details-channels.png)

---

## Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/dleopold/Termion/releases/latest):

```bash
# Linux (x86_64)
curl -LO https://github.com/dleopold/Termion/releases/latest/download/termion-x86_64-unknown-linux-gnu.tar.gz
tar xzf termion-x86_64-unknown-linux-gnu.tar.gz
sudo mv termion /usr/local/bin/

# Linux (aarch64/ARM64)
curl -LO https://github.com/dleopold/Termion/releases/latest/download/termion-aarch64-unknown-linux-gnu.tar.gz
tar xzf termion-aarch64-unknown-linux-gnu.tar.gz
sudo mv termion /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/dleopold/Termion/releases/latest/download/termion-aarch64-apple-darwin.tar.gz
tar xzf termion-aarch64-apple-darwin.tar.gz
sudo mv termion /usr/local/bin/

# macOS (Intel)
curl -LO https://github.com/dleopold/Termion/releases/latest/download/termion-x86_64-apple-darwin.tar.gz
tar xzf termion-x86_64-apple-darwin.tar.gz
sudo mv termion /usr/local/bin/
```

### From Source

Requires Rust 1.70+ and protoc:

```bash
# Clone the repository
git clone https://github.com/dleopold/Termion.git
cd Termion

# Build and install
cargo install --path .
```

### Requirements

- **Rust 1.70+** — Install via [rustup](https://rustup.rs/)
- **MinKNOW 6.x+** — Running locally or accessible via network
- **Linux or macOS** — Windows is not currently supported

---

## Quick Start

### Launch the TUI

```bash
# Connect to local MinKNOW (default: localhost:9501)
termion

# Connect to a specific host
termion --host 192.168.1.100

# With verbose logging (for debugging)
termion -vv --log /tmp/termion.log
```

### Navigation

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate selection |
| `Enter` | Open position detail |
| `Esc` | Go back / close overlay |
| `1` `2` `3` | Switch detail panels |
| `p` | Pause acquisition |
| `r` | Resume acquisition |
| `s` | Stop acquisition |
| `?` | Show help |
| `q` | Quit |

### CLI Commands

```bash
# List all devices and positions
termion list
termion list --json

# Show run status and metrics
termion status
termion status --json

# Filter to specific position
termion status --position 1
```

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Connection failed |
| 3 | Invalid arguments |
| 4 | Resource not found |

---

## Configuration

Termion uses a layered configuration system:

```
CLI flags → Environment variables → Config file → Defaults
```

### Config File

Location: `~/.config/termion/config.toml`

```toml
[connection]
host = "localhost"
port = 9501
connect_timeout = 5
request_timeout = 30

[tui]
refresh_interval = 1000  # milliseconds

[reconnect]
initial_delay = 1000     # milliseconds
max_delay = 30000
multiplier = 2.0

[logging]
level = "off"  # off, error, warn, info, debug, trace
file = "~/.local/state/termion/termion.log"
```

### Environment Variables

```bash
export TERMION_HOST=192.168.1.100
export TERMION_PORT=9501
export TERMION_LOG_LEVEL=debug
```

---

## Architecture

Termion is built with a clean separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                         termion                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │     TUI     │  │     CLI     │  │      Config         │  │
│  │  (ratatui)  │  │   (clap)    │  │  (TOML + env)       │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                   ┌──────▼──────┐                           │
│                   │   Client    │                           │
│                   │   (tonic)   │                           │
│                   └──────┬──────┘                           │
│                          │                                   │
└──────────────────────────┼───────────────────────────────────┘
                           │ gRPC
                    ┌──────▼──────┐
                    │   MinKNOW   │
                    │   Manager   │
                    └─────────────┘
```

### Key Components

| Module | Purpose |
|--------|---------|
| `client` | gRPC client with connection management, reconnection, and streaming |
| `tui` | Terminal UI with charts, widgets, and event handling |
| `cli` | Non-interactive commands for scripting |
| `config` | Multi-source configuration loading |

### Technology Stack

- **Async Runtime**: [Tokio](https://tokio.rs/)
- **gRPC**: [tonic](https://github.com/hyperium/tonic) + [prost](https://github.com/tokio-rs/prost)
- **TUI**: [ratatui](https://github.com/ratatui-org/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm)
- **CLI**: [clap](https://github.com/clap-rs/clap)
- **Logging**: [tracing](https://github.com/tokio-rs/tracing)

---

## Development

### Prerequisites

1. **Rust toolchain** — Install via [rustup](https://rustup.rs/)
2. **MinKNOW** — For testing against real/simulated devices
3. **Python 3.10+** — For simulation scripts (optional)

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with formatting and linting checks
cargo fmt --check
cargo clippy -- -D warnings
```

### Simulated Testing

Termion includes tools for testing without real sequencing hardware:

```bash
# Set up Python environment for simulation scripts
cd dev
python3 -m venv .venv
source .venv/bin/activate
pip install minknow_api

# Create a simulated MinION
python3 scripts/sim_manager.py create

# Check status
python3 scripts/sim_manager.py status
```

See [dev/README.md](dev/README.md) for detailed setup instructions.

### Project Structure

```
termion/
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library root
│   ├── client/          # gRPC client layer
│   ├── tui/             # Terminal UI
│   ├── cli/             # CLI commands
│   └── config/          # Configuration
├── proto/               # Vendored MinKNOW protobufs
├── specs/               # Design specifications
├── learning/            # Educational documentation
├── dev/                 # Development tools
└── docs/                # Additional documentation
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md) | System architecture and design |
| [specs/](specs/) | Detailed component specifications |
| [learning/](learning/) | Educational material from each development phase |
| [dev/README.md](dev/README.md) | Development environment setup |

---

## Design Philosophy

Termion was built with these principles:

1. **TUI-first** — The terminal interface is the primary product, not an afterthought
2. **Showcase quality** — Polished visual design for real-time data visualization
3. **Real-time** — Streaming data visualization, not polling where possible
4. **Observable** — Structured logging for debugging without corrupting the display
5. **Conventional** — Arrow keys and Enter, not vim-only keybindings

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- **Oxford Nanopore Technologies** — MinKNOW API
- **ratatui** — Terminal UI framework
- **tonic** — gRPC implementation
