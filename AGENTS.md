# AGENTS.md
AI Agent Instructions — Termion

## Project Overview

**Termion** is a showcase-quality TUI for monitoring MinKNOW nanopore sequencing runs. It's written in Rust and designed to be visually impressive while remaining simple to use.

### Key Facts

- **Primary interface**: TUI (terminal user interface)
- **Secondary interface**: CLI for scripting (`termion list`, `termion status`)
- **Language**: Rust
- **Platforms**: Linux, macOS
- **License**: MIT

---

## Before You Start

### Required Reading

Read these documents in order before making changes:

1. **[ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)** — System structure
2. **[specs/](specs/)** — Detailed component specifications

### Spec Documents

Consult the relevant spec before implementing:

| Task | Read First |
|------|------------|
| gRPC client work | [specs/SPEC_CLIENT.md](specs/SPEC_CLIENT.md) |
| TUI screens/widgets | [specs/SPEC_TUI.md](specs/SPEC_TUI.md) |
| CLI commands | [specs/SPEC_CLI.md](specs/SPEC_CLI.md) |
| Configuration | [specs/SPEC_CONFIG.md](specs/SPEC_CONFIG.md) |
| Testing | [specs/SPEC_TESTING.md](specs/SPEC_TESTING.md) |

---

## External Resources

When implementing unfamiliar APIs, debugging issues, or looking for patterns, use these tools:

### Available Tools

| Tool | Purpose | When to Use |
|------|---------|-------------|
| **Context7** | Official library documentation | API signatures, usage patterns, configuration options |
| **grep.app** | Real code from GitHub repos | Production patterns, how others solved similar problems |
| **Exa** | Web search | Articles, tutorials, discussions, troubleshooting |

### Usage Examples

**Implementing ratatui Chart widget:**
```
1. Context7: query "ratatui Chart widget time series Dataset"
2. grep.app: search `Dataset::default()` in Rust files
3. Study reference TUIs: bottom, trippy source code
```

**Debugging tonic gRPC streaming:**
```
1. Context7: query "tonic streaming client reconnect"
2. grep.app: search `Streaming<` with repo:hyperium/tonic
3. Exa: search "tonic gRPC stream disconnect handling"
```

**Understanding MinKNOW API:**
```
1. grep.app: search `minknow` in Python/Rust files for usage examples
2. Exa: search "MinKNOW API documentation nanopore"
```

### When to Search

- **Before implementing** — Find existing patterns, don't reinvent
- **When stuck** — Someone else likely solved this
- **When debugging** — Search for error messages, similar issues
- **For best practices** — Production code > tutorials > docs

---

## Critical Constraints

### DO:
- Follow existing code patterns in the codebase
- Match the visual style of reference TUIs (scope-tui, bottom, trippy)
- Use conventional keybindings (arrows, Enter, Esc) — NOT vim-style
- Write unit tests for core logic (parsing, state machines, transforms)
- Use the unified `ClientError` type for all gRPC errors
- Keep TUI rendering smooth (drop stale frames, don't block)

### DO NOT:
- Add Windows support (out of scope)
- Add remote/TLS support (localhost only for MVP)
- Use vim-style keybindings as primary (arrows are primary)
- Log to stderr during TUI mode (corrupts display)
- Block the UI thread on network calls
- Add dependencies without justification

---

## Coding Best Practices

### Rust Conventions

```bash
# Before every commit
cargo fmt
cargo clippy -- -D warnings
cargo test
```

- Prefer `?` over `.unwrap()` in library code
- Use `tracing` for instrumentation, never `println!` or `eprintln!`
- Derive standard traits in this order: `Debug, Clone, Copy, PartialEq, Eq, Hash, Default`

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Types | PascalCase | `DeviceState`, `RunMetrics` |
| Functions | snake_case | `get_run_state`, `parse_config` |
| Constants | SCREAMING_SNAKE | `DEFAULT_PORT`, `MAX_RETRIES` |
| Modules | snake_case | `client`, `tui_widgets` |
| Type parameters | Single uppercase or descriptive | `T`, `E`, `Item` |
| Lifetimes | Short lowercase | `'a`, `'ctx` |

**Semantic naming:**
```rust
// Good: Intent is clear
fn fetch_device_list() -> Result<Vec<Device>, ClientError>
fn is_connected(&self) -> bool
fn try_reconnect(&mut self) -> Result<(), ClientError>

// Bad: Vague or misleading
fn get_data() -> Result<Vec<Device>, ClientError>  // What data?
fn check(&self) -> bool                             // Check what?
fn reconnect(&mut self) -> Result<(), ClientError>  // Can it fail? Yes, use try_
```

**Boolean naming:**
```rust
// Prefix with is_, has_, can_, should_, was_
is_running: bool
has_error: bool
can_pause: bool
should_reconnect: bool
```

### Module Organization

```
src/
├── lib.rs              // Public API surface, re-exports
├── main.rs             // Entry point only, minimal logic
│
├── client/
│   ├── mod.rs          // Module root: pub use, module declarations
│   ├── connection.rs   // Connection management
│   ├── error.rs        // ClientError enum
│   ├── types.rs        // Domain types (Device, Position, etc.)
│   └── services/       // Per-service wrappers
│       ├── mod.rs
│       ├── manager.rs
│       └── acquisition.rs
│
├── tui/
│   ├── mod.rs
│   ├── app.rs          // App state, event loop
│   ├── input.rs        // Key handling
│   ├── screens/        // One file per screen
│   └── widgets/        // Reusable widget components
│
└── cli/
    ├── mod.rs
    ├── commands/       // One file per command
    └── output.rs       // Formatting (human, JSON)
```

**Module file rules:**
- `mod.rs` contains only: module declarations, public re-exports, shared types if small
- Logic goes in dedicated files, not `mod.rs`
- One primary type per file (exceptions: closely related types)
- Tests in same file (`#[cfg(test)] mod tests`) or `tests.rs` for larger test suites

### Error Handling

**Use the unified ClientError type:**
```rust
// Good: Unified error, adds context
pub fn list_devices(&self) -> Result<Vec<Device>, ClientError> {
    self.inner
        .list_devices(Request::new(()))
        .await
        .map_err(|status| ClientError::Grpc {
            method: "list_devices".into(),
            status,
        })?
        .into_inner()
        .devices
        .into_iter()
        .map(Device::try_from)
        .collect()
}

// Bad: Leaking implementation details
pub fn list_devices(&self) -> Result<Vec<Device>, tonic::Status>
```

**Error context with `anyhow` or manual context:**
```rust
// Add context when propagating
let config = load_config(&path)
    .map_err(|e| ClientError::Config {
        path: path.clone(),
        source: e,
    })?;
```

**When to handle vs propagate:**
```rust
// Propagate: Caller can do something meaningful
pub fn connect(&self) -> Result<Connection, ClientError> {
    // Let caller decide what to do on failure
}

// Handle: Recovery is possible and makes sense here
pub async fn fetch_with_retry(&self) -> Result<Data, ClientError> {
    for attempt in 0..MAX_RETRIES {
        match self.fetch().await {
            Ok(data) => return Ok(data),
            Err(e) if e.is_retriable() => {
                tracing::warn!(attempt, "Fetch failed, retrying");
                tokio::time::sleep(backoff(attempt)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(ClientError::MaxRetriesExceeded)
}
```

**Never silently swallow errors:**
```rust
// Bad: Silent failure
let _ = sender.send(message);

// Good: Log if we don't propagate
if let Err(e) = sender.send(message) {
    tracing::warn!(error = %e, "Failed to send message, channel closed");
}
```

### Async/Tokio Patterns

**Never block the async runtime:**
```rust
// TERRIBLE: Blocks the entire runtime
std::thread::sleep(Duration::from_secs(1));
let data = std::fs::read_to_string("file.txt")?;

// Good: Async-friendly
tokio::time::sleep(Duration::from_secs(1)).await;
let data = tokio::fs::read_to_string("file.txt").await?;
```

**Always use timeouts for network operations:**
```rust
// Good: Bounded wait time
let result = tokio::time::timeout(
    Duration::from_secs(5),
    client.list_devices()
).await
    .map_err(|_| ClientError::Timeout { operation: "list_devices".into() })??;

// Bad: Could hang forever
let result = client.list_devices().await?;
```

**Spawn carefully:**
```rust
// Good: Named tasks, handle errors
let handle = tokio::spawn(async move {
    if let Err(e) = process_stream(stream).await {
        tracing::error!(error = %e, "Stream processing failed");
    }
});

// Track spawned tasks for cleanup
self.tasks.push(handle);

// Bad: Fire and forget with no error handling
tokio::spawn(async move {
    process_stream(stream).await;  // Errors silently lost
});
```

**Select with cancellation:**
```rust
// Good: Clean cancellation
tokio::select! {
    result = stream.next() => {
        handle_message(result);
    }
    _ = shutdown.recv() => {
        tracing::info!("Shutting down stream handler");
        return;
    }
}
```

**Channel patterns:**
```rust
// Bounded channels prevent memory blowup
let (tx, rx) = tokio::sync::mpsc::channel::<StatsSnapshot>(16);

// For latest-value semantics (TUI updates)
let (tx, rx) = tokio::sync::watch::channel(initial_state);
```

### ratatui/TUI Patterns

**Widget lifecycle:**
```rust
// Widgets are stateless renderers - they borrow data and draw
impl Widget for DeviceList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render logic here
    }
}

// State lives in App, widgets borrow it
struct App {
    devices: Vec<Device>,
    selected: usize,
}

impl App {
    fn render(&self, frame: &mut Frame) {
        let list = DeviceList::new(&self.devices, self.selected);
        frame.render_widget(list, area);
    }
}
```

**Stateful widgets (selection, scroll):**
```rust
// Use ratatui's StatefulWidget for widgets with internal state
impl StatefulWidget for DeviceList<'_> {
    type State = ListState;
    
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // state.selected() gives current selection
    }
}
```

**Layout calculations:**
```rust
// Good: Responsive layout with constraints
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),      // Fixed header
        Constraint::Min(10),        // Flexible content
        Constraint::Length(1),      // Fixed footer
    ])
    .split(frame.area());

// Bad: Hardcoded sizes that break on resize
let header_area = Rect::new(0, 0, 80, 3);
```

**Event handling pattern:**
```rust
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Data(StatsSnapshot),
    Error(ClientError),
}

impl App {
    pub fn handle_event(&mut self, event: AppEvent) -> Option<Action> {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick => self.handle_tick(),
            AppEvent::Data(stats) => self.handle_data(stats),
            AppEvent::Error(e) => self.handle_error(e),
        }
    }
}
```

### Documentation Style

**Public items need docs:**
```rust
/// Connects to the MinKNOW manager service.
///
/// Attempts to establish a gRPC connection to the specified host and port.
/// Uses exponential backoff for retries.
///
/// # Arguments
///
/// * `host` - Manager hostname or IP address
/// * `port` - Manager port (typically 9501)
///
/// # Errors
///
/// Returns `ClientError::Connection` if the connection cannot be established
/// after all retry attempts.
///
/// # Example
///
/// ```no_run
/// let client = Client::connect("localhost", 9501).await?;
/// ```
pub async fn connect(host: &str, port: u16) -> Result<Self, ClientError>
```

**Internal code gets minimal docs:**
```rust
// Private helper - brief comment is enough
// Calculates exponential backoff with jitter
fn backoff_duration(attempt: u32) -> Duration {
    // ...
}
```

**Module-level docs:**
```rust
//! # Client Module
//!
//! This module provides the gRPC client for communicating with MinKNOW.
//!
//! ## Overview
//!
//! The main entry point is [`Client`], which handles connection management,
//! automatic reconnection, and provides typed wrappers for MinKNOW services.
//!
//! ## Example
//!
//! ```no_run
//! use termion::client::Client;
//!
//! let client = Client::connect("localhost", 9501).await?;
//! let devices = client.list_devices().await?;
//! ```
```

### Performance Guidelines

**Hot path awareness:**
```rust
// Identify hot paths: render loop, stream handlers, event processing
// These run frequently - avoid allocations

// Bad: Allocates on every render
fn format_throughput(&self) -> String {
    format!("{:.2} Gb/h", self.throughput)
}

// Good: Pre-allocate or use stack
fn format_throughput(&self, buf: &mut String) {
    use std::fmt::Write;
    buf.clear();
    write!(buf, "{:.2} Gb/h", self.throughput).unwrap();
}

// Or use a formatting crate that avoids allocation
```

**Avoid cloning in hot paths:**
```rust
// Bad: Cloning data for each widget
fn render(&self, frame: &mut Frame) {
    let data = self.chart_data.clone();  // Unnecessary clone
    let chart = Chart::new(data);
}

// Good: Borrow or reference
fn render(&self, frame: &mut Frame) {
    let chart = Chart::new(&self.chart_data);
}
```

**Buffer reuse:**
```rust
// For data that updates frequently, reuse buffers
struct ChartBuffer {
    data: Vec<(f64, f64)>,
}

impl ChartBuffer {
    fn update(&mut self, new_point: (f64, f64)) {
        if self.data.len() >= MAX_POINTS {
            self.data.remove(0);  // Or use VecDeque
        }
        self.data.push(new_point);
    }
}
```

**Measure before optimizing:**
```rust
// Use tracing for performance spans
#[tracing::instrument(skip(self))]
async fn fetch_stats(&self) -> Result<Stats, ClientError> {
    // ...
}

// Or manual timing in dev
let start = std::time::Instant::now();
expensive_operation();
tracing::debug!(elapsed_ms = start.elapsed().as_millis(), "Operation complete");
```

### Dependency Management

**When to add a new dependency:**
1. It solves a non-trivial problem correctly
2. It's well-maintained (recent commits, responsive maintainer)
3. It doesn't pull in excessive transitive deps
4. The license is compatible (MIT/Apache-2.0 preferred)

**Approved dependencies** (no justification needed):
- `tokio`, `tonic`, `prost` — async runtime, gRPC
- `ratatui`, `crossterm` — TUI
- `clap` — CLI
- `serde`, `serde_json`, `toml` — serialization
- `tracing`, `tracing-subscriber` — logging
- `thiserror`, `anyhow` — error handling
- `dirs` — XDG paths
- `chrono` or `time` — datetime

**Requires justification:**
- Any new network-related dependency
- Anything with native/C dependencies
- Unmaintained or low-download crates

### Common Anti-patterns

**❌ Unwrap in library code:**
```rust
// Bad
let value = map.get("key").unwrap();

// Good  
let value = map.get("key").ok_or(ClientError::NotFound { key: "key".into() })?;
```

**❌ Stringly-typed APIs:**
```rust
// Bad
fn set_state(&mut self, state: &str) { ... }

// Good
fn set_state(&mut self, state: RunState) { ... }
```

**❌ Boolean parameters:**
```rust
// Bad: What does `true` mean?
client.connect(true, false)?;

// Good: Use enums or builder
client.connect()
    .with_tls(TlsMode::Disabled)
    .with_retry(RetryPolicy::Exponential)
    .build()?;
```

**❌ Large functions:**
```rust
// Bad: 200-line function doing everything
fn process_everything(&mut self) { ... }

// Good: Decompose into focused functions
fn parse_response(&self, response: Response) -> Result<Data, ClientError> { ... }
fn validate_data(&self, data: &Data) -> Result<(), ValidationError> { ... }
fn update_state(&mut self, data: Data) { ... }
```

**❌ Ignoring must_use:**
```rust
// Bad: Silently ignoring result
connection.close();  // Returns Result, ignored

// Good
connection.close()?;
// Or if you really don't care:
let _ = connection.close();  // Explicit discard
```

**❌ Hardcoded magic numbers:**
```rust
// Bad
if retries > 5 { ... }
tokio::time::sleep(Duration::from_secs(30)).await;

// Good
const MAX_RETRIES: u32 = 5;
const MAX_BACKOFF: Duration = Duration::from_secs(30);

if retries > MAX_RETRIES { ... }
tokio::time::sleep(MAX_BACKOFF).await;
```

**❌ Mixing sync and async:**
```rust
// Bad: Blocking in async context
async fn fetch(&self) -> Data {
    let file = std::fs::read_to_string("cache.json")?;  // BLOCKS!
}

// Good: Use async IO or spawn_blocking
async fn fetch(&self) -> Data {
    let file = tokio::fs::read_to_string("cache.json").await?;
}

// Or for CPU-bound work
let result = tokio::task::spawn_blocking(|| {
    expensive_cpu_work()
}).await?;
```

---

## TUI Development

### Visual Quality Bar

This is a **showcase/demo project**. The TUI must be visually impressive.

**Reference TUIs** (study these):
- [scope-tui](https://github.com/alemidev/scope-tui) — Scientific visualization
- [bottom](https://github.com/ClementTsang/bottom) — System monitor with charts
- [trippy](https://github.com/fujiapple852/trippy) — Network diagnostics

**Design principles**:
1. Progressive disclosure (overview → detail)
2. Real-time streaming charts (not static)
3. Color-coded status indicators
4. Always-visible keybinding hints
5. Graceful degradation on disconnect

### Screen Structure

```
Overview Screen
    │
    ├── Device list (selectable)
    ├── Position table for selected device
    ├── Status indicators + mini sparklines
    └── Footer: keybinding hints
    
    [Enter] → Position Detail
    
Position Detail Screen
    │
    ├── Header: run info, protocol, timing
    ├── Throughput time-series chart
    ├── Distribution charts (read length, quality)
    ├── Key metrics grid
    └── Footer: context-aware keybindings
    
    [Esc] → Back to Overview
```

### Widget Implementation

```rust
// Charts should look like scope-tui/trippy
// - Smooth lines, not blocky
// - Proper axis labels
// - Time window scrolling

// Status indicators
// - Green (●) = Running
// - Yellow (●) = Paused  
// - Gray (○) = Idle
// - Red (●) = Error
```

---

## Testing Requirements

### What to Test

| Component | Test Type | Coverage Target |
|-----------|-----------|-----------------|
| Config loading | Unit | 80% |
| State machines | Unit | 80% |
| Data transforms | Unit | 80% |
| gRPC client | Integration (mock) | 60% |
| CLI output | Snapshot | 70% |
| TUI rendering | Manual | N/A |

### Mock Server

Use the mock gRPC server for integration tests:

```rust
let server = MockMinKnowServer::builder()
    .with_device("MN12345", "MinION")
    .build()
    .start()
    .await;

let client = Client::connect(&server.url()).await?;
```

### CI Must Pass

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

---

## Common Tasks

### Adding a New TUI Screen

1. Read [specs/SPEC_TUI.md](specs/SPEC_TUI.md)
2. Create text mockup first (ASCII art)
3. Add screen enum variant to `AppState`
4. Implement render function in `src/tui/screens/`
5. Add keybinding handling
6. Update help overlay
7. Manual visual QA

### Adding a CLI Command

1. Read [specs/SPEC_CLI.md](specs/SPEC_CLI.md)
2. Add subcommand to clap config
3. Implement in `src/cli/`
4. Add human + JSON output formatters
5. Add snapshot tests
6. Document exit codes

### Adding a gRPC Service Wrapper

1. Read [specs/SPEC_CLIENT.md](specs/SPEC_CLIENT.md)
2. Add wrapper in `src/client/`
3. Define domain types (not proto types)
4. Implement `From<proto::X>` conversions
5. Use `ClientError` for all errors
6. Add unit tests + integration tests

---

## Questions?

If something is unclear or contradicts the specs:
1. Check the relevant SPEC_*.md in specs/
2. If still unclear, ask before implementing
