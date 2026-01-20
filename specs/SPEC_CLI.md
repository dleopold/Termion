# SPEC_CLI.md
CLI Specification — Termion

## Overview

The CLI is secondary to the TUI (per D3.1). It exists to:
1. Launch the TUI (default behavior)
2. Provide minimal non-interactive commands for scripting/automation
3. Configure connection and behavior via flags

---

## Command Structure

### Default: Launch TUI

```bash
termion                        # Launch TUI, connect to localhost
termion --host 192.168.1.100   # Launch TUI, connect to specific host
termion --config ~/.config/termion/prod.toml
```

### Non-Interactive Commands

Per decision D3.2: Minimal surface — `status` and `list` only.

```bash
termion list                   # List devices and positions, exit
termion status                 # Show current run states, exit
termion status --position 1    # Show specific position status
```

---

## Command Reference

### `termion` (default)

Launch the TUI.

```
USAGE:
    termion [OPTIONS]

OPTIONS:
    -h, --host <HOST>        MinKNOW host [default: localhost]
    -p, --port <PORT>        MinKNOW manager port [default: 9501]
    -c, --config <PATH>      Config file path
    -v, --verbose            Enable logging (use -vv, -vvv for more)
        --log <PATH>         Custom log file path
        --help               Print help
        --version            Print version
```

### `termion list`

List devices and positions (non-interactive).

```
USAGE:
    termion list [OPTIONS]

OPTIONS:
        --json               Output as JSON
    -h, --host <HOST>        MinKNOW host [default: localhost]
    -p, --port <PORT>        MinKNOW manager port [default: 9501]
        --help               Print help
```

**Human output:**

```
DEVICE              POSITIONS  STATE
MinION Mk1C         2          Connected
  Position 1                   Running (02:34:12)
  Position 2                   Idle
GridION X5          5          Connected
  Position 1                   Running (01:12:45)
  Position 2                   Running (01:12:45)
  Position 3                   Paused
  Position 4                   Idle
  Position 5                   Idle
```

**JSON output (`--json`):**

```json
{
  "devices": [
    {
      "id": "MN12345",
      "name": "MinION Mk1C",
      "state": "connected",
      "positions": [
        {
          "id": "1",
          "name": "Position 1",
          "state": "running",
          "run_time_seconds": 9252
        },
        {
          "id": "2",
          "name": "Position 2",
          "state": "idle",
          "run_time_seconds": null
        }
      ]
    }
  ]
}
```

### `termion status`

Show run status and key metrics (non-interactive).

```
USAGE:
    termion status [OPTIONS]

OPTIONS:
        --position <ID>      Show specific position only
        --json               Output as JSON
    -h, --host <HOST>        MinKNOW host [default: localhost]
    -p, --port <PORT>        MinKNOW manager port [default: 9501]
        --help               Print help
```

**Human output:**

```
MinION Mk1C / Position 1
  State:       Running
  Run time:    02:34:12
  Protocol:    LSK114
  Reads:       1,234,567
  Bases:       4.2 Gb
  Throughput:  2.3 Gb/h
  Mean Q:      Q18.4

MinION Mk1C / Position 2
  State:       Idle
```

**JSON output (`--json`):**

```json
{
  "positions": [
    {
      "device": "MinION Mk1C",
      "position": "1",
      "state": "running",
      "run_time_seconds": 9252,
      "protocol": "LSK114",
      "reads": 1234567,
      "bases_gb": 4.2,
      "throughput_gb_per_hour": 2.3,
      "mean_quality": 18.4
    }
  ]
}
```

---

## Output Formats

Per decision D3.3: Human-readable default, `--json` for scripting.

### Human Format

- Aligned columns
- Human-readable numbers (1,234,567 not 1234567)
- Units included (Gb, Gb/h, etc.)
- Color when terminal supports it (status indicators)

### JSON Format

- Single JSON object (not line-delimited)
- Stable schema (breaking changes = major version bump)
- Machine-readable values (no formatting)
- Null for missing/unavailable data

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Connection failed |
| 3 | Invalid arguments |
| 4 | Resource not found (device/position) |

---

## Configuration

Per decisions D3.4 and D7.3.

### Precedence

CLI flags → Environment variables → Config file → Defaults

### Environment Variables

```bash
TERMION_HOST=192.168.1.100
TERMION_PORT=9501
TERMION_CONFIG=/path/to/config.toml
TERMION_LOG_LEVEL=debug
```

### Config File

Location: `~/.config/termion/config.toml` (XDG_CONFIG_HOME)

```toml
# Termion configuration

[connection]
host = "localhost"
port = 9501
timeout_seconds = 5

[tui]
refresh_interval_ms = 1000

[logging]
# off, error, warn, info, debug, trace
level = "off"
file = "~/.local/state/termion/termion.log"
```

---

## Logging

Per decision D5.4: Off by default, to file when enabled.

```bash
termion              # No logging
termion -v           # Info level to file
termion -vv          # Debug level to file
termion -vvv         # Trace level to file
termion --log /tmp/termion.log -vv  # Custom log path
```

Default log location: `~/.local/state/termion/termion.log` (XDG_STATE_HOME)

---

## Dependencies

- `clap` — Argument parsing
- `serde` / `serde_json` — JSON output
- `figment` or `config` — Config file loading
- `dirs` — XDG directory resolution
