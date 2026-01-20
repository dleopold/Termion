# Phase 4 — Testing & Polish

## Overview

Phase 4 focused on ensuring code quality and reliability through:
- Unit tests for core logic (65 tests total)
- CI pipeline with GitHub Actions
- TUI visual QA checklist for manual testing

Integration tests with a mock gRPC server were deferred due to complexity (TLS certificate handling, proto service implementation).

---

## Rust Testing Fundamentals

### Test Organization

Rust supports three types of tests:

```
src/
├── lib.rs           # Unit tests inline with #[cfg(test)]
├── module/
│   └── mod.rs       # Unit tests at module level
tests/
└── integration.rs   # Integration tests (separate compilation)
```

**Unit tests** live alongside the code they test:

```rust
// src/client/types.rs

pub fn pass_rate(&self) -> f64 {
    let total = self.reads_passed + self.reads_failed;
    if total == 0 { 0.0 } else { (self.reads_passed as f64 / total as f64) * 100.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_rate_zero_reads() {
        let stats = StatsSnapshot::default();
        assert_eq!(stats.pass_rate(), 0.0);
    }

    #[test]
    fn test_pass_rate_mixed() {
        let stats = StatsSnapshot {
            reads_passed: 75,
            reads_failed: 25,
            ..Default::default()
        };
        assert_eq!(stats.pass_rate(), 75.0);
    }
}
```

**Integration tests** in `tests/` directory:

```rust
// tests/cli_output.rs
use termion::cli::format_number;

#[test]
fn test_format_millions() {
    assert_eq!(format_number(1_500_000), "1.50M");
}
```

### Running Tests

```bash
cargo test              # All tests
cargo test --lib        # Unit tests only
cargo test --test '*'   # Integration tests only
cargo test test_name    # Tests matching name
cargo test -- --nocapture  # Show println! output
```

---

## Test Patterns

### Testing Enums

```rust
#[test]
fn test_run_state_is_active() {
    assert!(!RunState::Idle.is_active());
    assert!(RunState::Starting.is_active());
    assert!(RunState::Running.is_active());
    assert!(RunState::Paused.is_active());
    assert!(RunState::Finishing.is_active());
    assert!(!RunState::Stopped.is_active());
    assert!(!RunState::Error("test".into()).is_active());
}
```

### Testing State Machines

```rust
#[test]
fn test_select_next_wraps() {
    let mut app = App::new(test_config());
    app.positions = vec![pos("A"), pos("B"), pos("C")];
    app.selected_position = 2;
    app.select_next();
    assert_eq!(app.selected_position, 0);  // Wrapped
}

#[test]
fn test_back_closes_overlay() {
    let mut app = App::new(test_config());
    app.overlay = Overlay::Help;
    app.back();
    assert_eq!(app.overlay, Overlay::None);
}
```

### Testing with Defaults

Use `..Default::default()` for partial construction:

```rust
#[test]
fn test_stats_pass_rate_mixed() {
    let stats = StatsSnapshot {
        reads_passed: 75,
        reads_failed: 25,
        ..Default::default()  // All other fields use defaults
    };
    assert_eq!(stats.pass_rate(), 75.0);
}
```

### Testing Random/Jittered Values

For functions with randomness, test the bounds:

```rust
#[test]
fn test_reconnect_policy_jitter_range() {
    let policy = ReconnectPolicy {
        initial_delay: Duration::from_secs(10),
        jitter_fraction: 0.1,
        ..Default::default()
    };

    let mut min_seen = Duration::from_secs(100);
    let mut max_seen = Duration::from_secs(0);

    for _ in 0..100 {
        let d = policy.delay_for_attempt(0);
        min_seen = min_seen.min(d);
        max_seen = max_seen.max(d);
    }

    assert!(min_seen >= Duration::from_secs(9));
    assert!(max_seen <= Duration::from_secs(11));
    assert!(min_seen != max_seen);  // Verify randomness
}
```

### Test Helpers

Create helper functions to reduce boilerplate:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config::default()
    }

    fn test_position(name: &str) -> Position {
        Position {
            id: name.to_string(),
            name: name.to_string(),
            device_id: "DEV001".to_string(),
            state: PositionState::Idle,
            grpc_port: 8000,
            is_simulated: false,
        }
    }

    #[test]
    fn test_with_helpers() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A"), test_position("B")];
        // ...
    }
}
```

---

## CI with GitHub Actions

### Basic Workflow Structure

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings

  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --lib
```

### Key Actions

| Action | Purpose |
|--------|---------|
| `actions/checkout@v4` | Clone repository |
| `dtolnay/rust-toolchain@stable` | Install Rust |
| `Swatinem/rust-cache@v2` | Cache cargo dependencies |

### Matrix Builds

Test across multiple platforms:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest]
    rust: [stable, beta]
```

### Caching

The `rust-cache` action caches:
- `~/.cargo/registry` — Downloaded crate sources
- `~/.cargo/git` — Git dependencies
- `target/` — Build artifacts

This dramatically speeds up subsequent builds.

### Environment Variables

```yaml
env:
  CARGO_TERM_COLOR: always  # Colored output in logs
  RUST_BACKTRACE: 1         # Show backtraces on panic
```

---

## What to Test (Guidelines)

### DO Test

| Category | Examples |
|----------|----------|
| Pure functions | Calculations, transforms, formatting |
| State machines | Transitions, edge cases |
| Parsing/validation | Config loading, input handling |
| Error mapping | Error types, exit codes |
| Business logic | Domain rules, invariants |

### DON'T Test (or test manually)

| Category | Why |
|----------|-----|
| UI rendering | Visual inspection needed |
| Network I/O | Requires mock server |
| Third-party code | Trust the library |
| Trivial getters | No logic to test |

---

## TUI Testing Strategy

TUI rendering is inherently visual. We use manual testing with a checklist.

### Manual QA Checklist Categories

1. **Layout** — Alignment, spacing, no artifacts
2. **Colors** — Status indicators, theming
3. **Navigation** — Keyboard handling, screen transitions
4. **Edge cases** — Empty states, long content, resize
5. **Connection states** — Disconnected, reconnecting

### When to Run Manual QA

- Before releases
- After significant UI changes
- After dependency updates (ratatui, crossterm)

---

## Coverage Tools (Optional)

### cargo-tarpaulin

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
# Open tarpaulin-report.html
```

### cargo-llvm-cov

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

Coverage targets for this project:
- Core logic: 80%+
- Client wrappers: 60%+
- TUI: N/A (manual testing)

---

## Key Takeaways

1. **Unit tests live with the code** — Use `#[cfg(test)] mod tests` inline.

2. **Test behavior, not implementation** — Focus on inputs and outputs.

3. **Use `..Default::default()`** — Simplifies test setup.

4. **CI catches regressions** — Format, lint, test on every PR.

5. **Cache aggressively** — `rust-cache` saves minutes per build.

6. **Manual QA for visuals** — Checklists ensure consistent quality.

7. **Don't over-test** — Skip trivial code, trust libraries.

---

## Resources

- [Rust Book: Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [GitHub Actions for Rust](https://github.com/actions-rs)
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain)
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
