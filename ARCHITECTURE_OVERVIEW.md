# ARCHITECTURE_OVERVIEW.md
Termion — Architecture Overview

## Purpose

This document describes the high-level architecture of Termion, a showcase-quality TUI for monitoring MinKNOW sequencing runs.

---

## What We're Building

**Termion** is a single Rust binary that provides:

1. **TUI (primary)** — Real-time dashboard for monitoring sequencing runs
2. **CLI (secondary)** — Minimal commands for scripting (`list`, `status`)

### Key Characteristics

- **TUI-first**: `termion` launches the TUI by default
- **Showcase quality**: Polished visual design inspired by scope-tui, bottom, trippy
- **Real-time**: Streaming data visualization with charts and metrics
- **Localhost MVP**: Connects to local MinKNOW instance (remote support deferred)

---

## Module Structure

```
termion/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, arg parsing, mode dispatch
│   ├── lib.rs               # Library root (for testing)
│   │
│   ├── client/              # gRPC client layer
│   │   ├── mod.rs
│   │   ├── connection.rs    # Connection management, reconnect
│   │   ├── manager.rs       # Manager service wrapper
│   │   ├── position.rs      # Position client (acquisition, device, data services)
│   │   ├── error.rs         # Unified error type
│   │   └── types.rs         # Domain types (Device, Position, ChannelLayout, etc.)
│   │
│   ├── tui/                 # Terminal UI
│   │   ├── mod.rs
│   │   ├── app.rs           # Application state and event loop
│   │   ├── screens/
│   │   │   ├── mod.rs
│   │   │   ├── overview.rs  # Device/position list
│   │   │   └── detail.rs    # Position detail with charts
│   │   ├── widgets/
│   │   │   ├── mod.rs
│   │   │   ├── chart.rs     # Time series chart
│   │   │   ├── histogram.rs # Distribution chart
│   │   │   └── status.rs    # Status indicators
│   │   ├── input.rs         # Keybinding handling
│   │   └── theme.rs         # Colors and styling
│   │
│   ├── cli/                 # CLI commands
│   │   ├── mod.rs
│   │   ├── list.rs          # `termion list`
│   │   ├── status.rs        # `termion status`
│   │   └── output.rs        # Formatting (human, JSON)
│   │
│   └── config/              # Configuration
│       ├── mod.rs
│       ├── loader.rs        # Multi-source loading
│       └── types.rs         # Config structs
│
├── proto/                   # Vendored MinKNOW protobufs
│   └── minknow_api/
│
├── build.rs                 # Protobuf codegen
│
├── specs/                   # Detailed specifications
│   ├── SPEC_CLIENT.md
│   ├── SPEC_TUI.md
│   ├── SPEC_CLI.md
│   ├── SPEC_CONFIG.md
│   └── SPEC_TESTING.md
│
└── tests/                   # Integration tests
    ├── common/
    │   └── mock_server.rs
    └── integration/
```

---

## Runtime Architecture

### Startup Flow

```
main()
  │
  ├─► Parse CLI args (clap)
  │
  ├─► Load config (CLI → env → file → defaults)
  │
  ├─► Dispatch based on command:
  │     │
  │     ├─► (no subcommand) → Launch TUI
  │     ├─► `list` → Run list command, exit
  │     └─► `status` → Run status command, exit
  │
  └─► (TUI mode) Initialize:
        ├─► Setup terminal (crossterm)
        ├─► Create client connection
        ├─► Start event loop
        └─► Render initial screen
```

### TUI Event Loop

```
┌─────────────────────────────────────────────────────────────┐
│                       Event Loop                             │
│                                                              │
│   ┌──────────┐     ┌──────────┐     ┌──────────┐           │
│   │ Terminal │     │  Client  │     │  Timer   │           │
│   │  Events  │     │ Streams  │     │  Ticks   │           │
│   └────┬─────┘     └────┬─────┘     └────┬─────┘           │
│        │                │                │                  │
│        └───────────────┼────────────────┘                  │
│                        ▼                                    │
│               ┌────────────────┐                           │
│               │  Event Queue   │                           │
│               └───────┬────────┘                           │
│                       ▼                                    │
│               ┌────────────────┐                           │
│               │ Update State   │                           │
│               └───────┬────────┘                           │
│                       ▼                                    │
│               ┌────────────────┐                           │
│               │    Render      │                           │
│               └────────────────┘                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
MinKNOW gRPC Server
        │
        ▼
┌───────────────────┐
│  termion-client   │
│  ─────────────    │
│  • Connection mgmt│
│  • Stream handling│
│  • Type conversion│
│  • Error mapping  │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│   Domain Types    │
│   ────────────    │
│   Device          │
│   Position        │
│   RunState        │
│   StatsSnapshot   │
│   ChannelLayout   │
│   ChannelStates   │
└─────────┬─────────┘
          │
    ┌─────┴─────┐
    ▼           ▼
┌───────┐   ┌───────┐
│  TUI  │   │  CLI  │
└───────┘   └───────┘
```

---

## Key Technical Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Async runtime | Tokio | Industry standard, tonic requires it |
| gRPC | tonic + prost | Best Rust gRPC ecosystem |
| TUI framework | ratatui | Active, well-documented, used by reference TUIs |
| Terminal backend | crossterm | Cross-platform (Linux + macOS) |
| CLI parsing | clap | Standard, derive macros |
| Config | figment | Multi-source with good ergonomics |
| Logging | tracing | Structured, async-friendly |

---

## Connection Management

### Discovery Flow

```
1. Connect to Manager (localhost:9501)
2. list_devices() → Vec<Device>
3. For each device: list_positions() → Vec<Position>
4. User selects position
5. Connect to position-specific services
6. Subscribe to stats stream
```

### Reconnection

Per D5.3: Exponential backoff (1s → 30s cap)

```
Disconnect detected
    │
    ▼
┌─────────────────┐
│ Attempt 1: 1s   │──► Success ──► Resume
└────────┬────────┘
         │ Fail
         ▼
┌─────────────────┐
│ Attempt 2: 2s   │──► Success ──► Resume
└────────┬────────┘
         │ Fail
         ▼
┌─────────────────┐
│ Attempt 3: 4s   │──► Success ──► Resume
└────────┬────────┘
         │ Fail
         ▼
       (continue until 30s cap)
```

---

## Streaming & Backpressure

Per D5.2: Drop stale frames if UI can't keep up.

```rust
// Bounded channel with drop-oldest
let (tx, rx) = bounded_channel::<StatsSnapshot>(16);

// Producer task (gRPC stream)
while let Some(stats) = stream.next().await {
    // If channel full, drop oldest and insert new
    tx.send_overwrite(stats);
}

// Consumer (TUI render loop)
// Gets latest available frame
let latest = rx.latest();
```

---

## Error Strategy

Single error type surfaces to TUI/CLI:

```rust
pub enum ClientError {
    Connection { endpoint, source },
    Grpc { method, status },
    Protocol { message },
    NotFound { resource, id },
    Timeout { operation },
    Disconnected,
}
```

- TUI: Shows error in UI, continues running
- CLI: Prints error, exits with appropriate code

---

## Security Considerations

- **Localhost only** (MVP): No network exposure
- **No secrets in logs**: Redact sensitive data
- **TLS deferred**: Will add for remote support post-MVP

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Startup time | < 500ms |
| Memory usage | < 50MB typical |
| CPU idle | < 1% |
| CPU active | < 5% |
| Render latency | < 33ms (30fps) |

---

## Spec Index

| Document | Purpose |
|----------|---------|
| [DECISIONS.md](DECISIONS.md) | Design decisions log |
| [WORK_PLAN.md](WORK_PLAN.md) | Phase plan and deliverables |
| [specs/SPEC_CLIENT.md](specs/SPEC_CLIENT.md) | gRPC client specification |
| [specs/SPEC_TUI.md](specs/SPEC_TUI.md) | TUI specification |
| [specs/SPEC_CLI.md](specs/SPEC_CLI.md) | CLI specification |
| [specs/SPEC_CONFIG.md](specs/SPEC_CONFIG.md) | Configuration specification |
| [specs/SPEC_TESTING.md](specs/SPEC_TESTING.md) | Testing specification |
