# WORK_PLAN.md
Termion — Rust TUI for MinKNOW API

## Goals

- Build a **showcase-quality TUI** for monitoring MinKNOW sequencing runs
- Real-time streaming data visualization (charts, metrics, status)
- Secondary CLI for scripting/automation
- Support Linux and macOS

## Non-goals (MVP)

- Full MinKNOW feature parity
- Windows support
- Remote/LAN connections (localhost only for MVP)
- TLS/mTLS authentication
- Plugin ecosystem

## Guiding Principles

- **TUI-first**: The terminal interface is the primary product
- **Showcase quality**: Polished, impressive visual design (references: scope-tui, bottom, trippy)
- **Streaming-first**: Subscribe to data streams, avoid polling where possible
- **Observable**: Structured logging (off by default, to file when enabled)
- **Testable**: Mock gRPC server for CI, core logic unit tested
- **Learning-oriented**: Each phase produces educational documentation (see `learning/`)

---

## Current Status

**Phase 3 complete. Ready for Phase 4 — Testing & Polish.**

Development environment ready:
- MinKNOW running with simulated device (MS00001)
- Bulk file playback configured for realistic testing
- `make sim-status` to verify
- Client library working: `termion list`, `termion status`
- TUI implemented with charts, status indicators, and auto-reconnect
- CLI commands with proper exit codes (0, 1, 2, 3, 4)

Next action: Phase 4 — Testing & Polish (unit tests, integration tests, CI).

---

## Phase 0 — Discovery & Foundations ✅

**Status:** Complete

### Deliverables
- [x] Project scaffolding decisions
- [x] Architecture overview
- [x] Decisions document (D0-D7 accepted)
- [x] Detailed specifications (specs/*.md)
- [x] Development environment setup (dev/, Makefile)
- [x] Simulated MinKNOW device for testing

### Key Decisions Made
- Single binary: `termion`
- TUI launches by default
- Localhost only for MVP
- MIT license, public OSS
- Linux + macOS targets

---

## Phase 1 — Core Client Library ✅

**Status:** Complete

**Purpose:** Implement gRPC client with connection management and streaming.

### Deliverables
- `termion` crate with client module:
  - Connection to MinKNOW manager (localhost:9501)
  - Device and position discovery
  - Acquisition service wrapper (state, stop/pause/resume)
  - Statistics service wrapper (streaming metrics)
- Reconnection with exponential backoff (1s → 30s)
- Unified error type
- Config loading (CLI → env → file → defaults)

### Exit Criteria
- [x] Connect to MinKNOW, list devices/positions
- [x] Subscribe to stats stream
- [x] Unit tests for config, error mapping (27 tests)
- [x] Learning document: `learning/PHASE_1_CLIENT.md`
- [ ] Integration tests with mock gRPC server → Deferred to Phase 4

### Spec References
- [SPEC_CLIENT.md](specs/SPEC_CLIENT.md)
- [SPEC_CONFIG.md](specs/SPEC_CONFIG.md)

### Learning Topics
- gRPC fundamentals and protocol buffers
- tonic and prost in Rust
- Async Rust patterns (tokio, streams, channels)
- Connection management and reconnection strategies
- Error handling patterns in Rust

---

## Phase 2 — TUI MVP ✅

**Status:** Complete

**Purpose:** Build the primary interface — showcase-quality terminal UI.

### Deliverables
- Overview screen:
  - Device list with selection
  - Position table with status indicators
  - Mini throughput sparklines
- Position detail screen:
  - Run info header
  - Real-time throughput chart (scope-tui/trippy style)
  - ~~Distribution charts (read length, quality)~~ → Deferred (no histogram data from API)
  - Key metrics display
- Navigation:
  - Conventional keybindings (arrows, Enter, Esc)
  - Help overlay (?)
- Disconnection UX:
  - Banner with reconnect status
  - Last-known data preserved (dimmed)
  - Auto-reconnect with backoff

### Exit Criteria
- [x] Smooth rendering under stream load
- [x] Charts update in real-time
- [x] Graceful disconnect/reconnect
- [ ] Manual visual QA pass → Requires interactive terminal
- [x] Learning document: `learning/PHASE_2_TUI.md`

### Spec References
- [SPEC_TUI.md](specs/SPEC_TUI.md)

### Design Process
1. Text mockups for screen layouts
2. Implement with ratatui
3. Visual iteration against reference TUIs
4. Polish pass (colors, spacing, animations)

### Learning Topics
- TUI architecture and immediate-mode rendering
- ratatui widget system and layout
- Event loop design patterns
- Stateful vs stateless widgets
- Terminal graphics and Unicode

---

## Phase 3 — CLI Commands ✅

**Status:** Complete

**Purpose:** Add minimal non-interactive commands for scripting.

### Deliverables
- `termion list` — list devices and positions ✅
- `termion status` — show run states and metrics ✅
- Output formats:
  - Human-readable (default) ✅
  - JSON (`--json` flag) ✅
- Exit codes: 0=ok, 1=error, 2=connection, 3=args, 4=not found ✅

### Exit Criteria
- [x] Commands work without TUI
- [x] JSON output is stable and documented
- [x] Exit codes are correct
- [x] Learning document: `learning/PHASE_3_CLI.md`

### Spec References
- [SPEC_CLI.md](specs/SPEC_CLI.md)

### Learning Topics
- CLI design principles
- clap derive macros and builder patterns
- Output formatting (human vs machine)
- Configuration layering (flags → env → file → defaults)

---

## Phase 4 — Testing & Polish

**Purpose:** Ensure reliability and code quality.

### Deliverables
- Unit tests for core logic (80%+ coverage)
- Integration tests with mock gRPC server
- CI pipeline (format, lint, test on every PR)
- Manual TUI visual QA checklist

### Exit Criteria
- [ ] CI green on all PRs
- [ ] No critical bugs in TUI
- [ ] Documentation complete
- [ ] Learning document: `learning/PHASE_4_TESTING.md`

### Spec References
- [SPEC_TESTING.md](specs/SPEC_TESTING.md)

### Learning Topics
- Rust testing patterns (unit, integration, doc tests)
- Mocking and test doubles in Rust
- Property-based testing
- CI/CD for Rust projects

---

## Phase 5 — Release

**Purpose:** Package and ship.

### Deliverables
- `cargo install termion` works
- README with:
  - Installation instructions
  - Quick start guide
  - Configuration reference
  - Screenshots
- Changelog
- Version: 0.1.0 (SemVer)

### Exit Criteria
- [ ] Published to crates.io (or ready to publish)
- [ ] README is complete
- [ ] Manual smoke test passes
- [ ] Learning document: `learning/PHASE_5_RELEASE.md`

### Learning Topics
- Rust packaging and crates.io publishing
- Semantic versioning in practice
- Release automation
- Documentation best practices (rustdoc, README)

---

## Phase 6 — Post-MVP (Future)

**Potential features (prioritize based on feedback):**
- Remote/LAN connections with TLS
- Additional MinKNOW services (protocol control, etc.)
- Homebrew formula
- More detailed charts and analytics
- Export capabilities
- Multi-position dashboard view

---

## Spec Documents

| Document | Description |
|----------|-------------|
| [DECISIONS.md](DECISIONS.md) | All accepted design decisions |
| [ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md) | System architecture |
| [specs/SPEC_CLIENT.md](specs/SPEC_CLIENT.md) | gRPC client library |
| [specs/SPEC_TUI.md](specs/SPEC_TUI.md) | TUI screens and interaction |
| [specs/SPEC_CLI.md](specs/SPEC_CLI.md) | CLI commands and output |
| [specs/SPEC_CONFIG.md](specs/SPEC_CONFIG.md) | Configuration system |
| [specs/SPEC_TESTING.md](specs/SPEC_TESTING.md) | Testing strategy |
