# SPEC_TESTING.md
Testing Specification — Termion

## Overview

Testing strategy based on decisions D6.1-D6.4:
- Unit tests: Core logic only
- Integration tests: Mock gRPC server
- TUI visual tests: Manual
- CI: Unit + mock integration on every PR

---

## Test Categories

### 1. Unit Tests

**Scope:** Core logic — parsing, state machines, data transforms.

**Location:** Inline in source files or `src/*/tests.rs`

**Examples:**

```rust
// Config parsing
#[test]
fn test_config_precedence() {
    let cli = CliArgs { host: Some("cli-host".into()), ..default() };
    let env = EnvConfig { host: Some("env-host".into()), ..default() };
    let file = FileConfig { host: Some("file-host".into()), ..default() };
    
    let merged = Config::merge(cli, env, file);
    assert_eq!(merged.host, "cli-host");  // CLI wins
}

// State machine
#[test]
fn test_run_state_transitions() {
    let mut state = RunState::Idle;
    state.transition(RunEvent::Start);
    assert_eq!(state, RunState::Starting);
    
    state.transition(RunEvent::Started);
    assert_eq!(state, RunState::Running);
}

// Data transforms
#[test]
fn test_throughput_calculation() {
    let stats = StatsSnapshot {
        bases_called: 4_200_000_000,
        elapsed_seconds: 3600,
        ..default()
    };
    
    let throughput = stats.throughput_gb_per_hour();
    assert!((throughput - 4.2).abs() < 0.01);
}
```

**What NOT to unit test:**
- UI rendering (manual testing)
- gRPC wire format (integration tests)
- Third-party library behavior

---

### 2. Integration Tests

**Scope:** gRPC client ↔ mock server interactions.

**Location:** `tests/` directory

**Mock Server:**

```rust
// tests/common/mock_server.rs

pub struct MockMinKnowServer {
    devices: Vec<Device>,
    positions: HashMap<String, Vec<Position>>,
    run_states: HashMap<String, RunState>,
    stats: HashMap<String, StatsSnapshot>,
}

impl MockMinKnowServer {
    pub fn builder() -> MockServerBuilder { ... }
    
    pub async fn start(&self) -> RunningMockServer {
        // Bind to random port, return handle
    }
}

pub struct RunningMockServer {
    pub addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
}

impl RunningMockServer {
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}
```

**Test Examples:**

```rust
// tests/integration/discovery.rs

#[tokio::test]
async fn test_list_devices() {
    let server = MockMinKnowServer::builder()
        .with_device("MN12345", "MinION Mk1C")
        .with_device("GR54321", "GridION X5")
        .build()
        .start()
        .await;
    
    let client = Client::connect(&server.url()).await.unwrap();
    let devices = client.list_devices().await.unwrap();
    
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].name, "MinION Mk1C");
}

#[tokio::test]
async fn test_connection_timeout() {
    // No server running
    let result = Client::connect_with_timeout(
        "http://localhost:19999",
        Duration::from_millis(100),
    ).await;
    
    assert!(matches!(result, Err(ClientError::Timeout { .. })));
}

#[tokio::test]
async fn test_reconnect_on_disconnect() {
    let server = MockMinKnowServer::builder()
        .with_device("MN12345", "MinION")
        .build()
        .start()
        .await;
    
    let client = Client::connect(&server.url()).await.unwrap();
    
    // Simulate disconnect
    server.disconnect_all();
    
    // Client should reconnect
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let devices = client.list_devices().await.unwrap();
    assert_eq!(devices.len(), 1);
}
```

---

### 3. CLI Output Tests

**Scope:** Command output format validation.

**Approach:** Snapshot testing with insta or manual golden files.

```rust
// tests/cli/output.rs

#[test]
fn test_list_human_output() {
    let devices = vec![mock_device("MinION Mk1C", vec![
        mock_position("1", RunState::Running),
        mock_position("2", RunState::Idle),
    ])];
    
    let output = format_list_human(&devices);
    
    insta::assert_snapshot!(output);
}

#[test]
fn test_list_json_output() {
    let devices = vec![mock_device("MinION Mk1C", vec![
        mock_position("1", RunState::Running),
    ])];
    
    let output = format_list_json(&devices).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    
    assert_eq!(parsed["devices"][0]["name"], "MinION Mk1C");
}
```

---

### 4. TUI Visual Testing

**Scope:** Manual testing only (per D6.3).

**Checklist for Manual QA:**

```markdown
## TUI Visual QA Checklist

### Overview Screen
- [ ] Device list renders correctly
- [ ] Position table aligns columns
- [ ] Status colors are correct (green/yellow/gray/red)
- [ ] Selection highlight is visible
- [ ] Footer keybindings are readable

### Position Detail Screen
- [ ] Charts render without artifacts
- [ ] Time series updates smoothly
- [ ] Metrics are formatted correctly
- [ ] Transitions are smooth

### Disconnection
- [ ] Banner appears on disconnect
- [ ] Last data stays visible (dimmed)
- [ ] Reconnection counter updates
- [ ] Clean recovery on reconnect

### Edge Cases
- [ ] Empty device list
- [ ] Single device, single position
- [ ] Many devices (10+)
- [ ] Long device/run names (truncation)
- [ ] Terminal resize handling
```

---

## CI Pipeline

Per decision D6.4: Unit + mock integration on every PR.

```yaml
# .github/workflows/ci.yml

name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      
      - name: Check formatting
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy -- -D warnings
      
      - name: Unit tests
        run: cargo test --lib
      
      - name: Integration tests
        run: cargo test --test '*'

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - run: cargo test
```

---

## Test Organization

```
termion/
├── src/
│   ├── client/
│   │   ├── mod.rs
│   │   └── tests.rs          # Unit tests for client
│   ├── tui/
│   │   ├── mod.rs
│   │   └── state/
│   │       └── tests.rs      # Unit tests for state machine
│   └── cli/
│       └── tests.rs          # Unit tests for CLI formatting
├── tests/
│   ├── common/
│   │   └── mock_server.rs    # Shared mock infrastructure
│   ├── integration/
│   │   ├── discovery.rs
│   │   ├── streaming.rs
│   │   └── reconnect.rs
│   └── cli/
│       └── output.rs         # CLI snapshot tests
└── Cargo.toml
```

---

## Coverage Goals

| Category | Target |
|----------|--------|
| Core logic (parsing, state) | 80%+ |
| Client wrapper methods | 60%+ |
| CLI formatting | 70%+ |
| TUI rendering | N/A (manual) |

Use `cargo-tarpaulin` or `cargo-llvm-cov` for coverage reporting.
