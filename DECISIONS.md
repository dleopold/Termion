# DECISIONS.md
Decisions Log — Termion (Rust TUI for MinKNOW)

## How to use this doc
- This is the single place where we track **decisions made** and **why**.
- Each decision includes:
  - **Status:** Proposed / Accepted / Revisit
  - **Date**
  - **Rationale**
  - **Impacts** (what specs/code it affects)

---

## 0) Project Framing

### D0.1 — Primary users & workflows
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Remote monitoring via SSH, TUI-primary tool
- **Rationale:** Lab operators need to monitor sequencing runs remotely. TUI provides rich real-time visualization over SSH without requiring desktop access or web infrastructure.
- **Impacts:** TUI is the primary interface; CLI is secondary for scripting/automation.

### D0.2 — Supported platforms
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Linux + macOS only
- **Rationale:** Primary deployment targets. Windows support adds complexity with minimal user benefit for this use case.
- **Impacts:** CI matrix, terminal backend (crossterm handles both), no Windows-specific code paths.

### D0.3 — License & distribution
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** MIT license, public OSS
- **Rationale:** Maximize adoption, allow integration into other tools.
- **Impacts:** Dependency license compatibility, public GitHub repo.

---

## 1) Connectivity & Authentication

### D1.1 — Connection modes (MVP)
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Localhost only for MVP
- **Rationale:** Simplifies initial implementation. Remote/LAN support can be added later.
- **Impacts:** No complex network configuration in MVP, simpler testing.

### D1.2 — TLS requirements
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Plaintext OK for localhost MVP
- **Rationale:** Localhost connections don't require TLS. Can revisit for remote support.
- **Impacts:** Simpler connection setup, no cert management in MVP.

### D1.3 — Client certificate authentication
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Skip mTLS for MVP
- **Rationale:** Not needed for localhost. Defer until remote support is added.
- **Impacts:** No cert loading code in MVP.

### D1.4 — Certificate provisioning UX
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Deferred
- **Rationale:** Not relevant until TLS/mTLS is needed.
- **Impacts:** None for MVP.

### D1.5 — Endpoint discovery strategy
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Manager → devices → positions discovery flow
- **Rationale:** Matches MinKNOW's native discovery model.
- **Impacts:** Connection manager design, position selector UI.

---

## 2) Protobuf Strategy & Compatibility

### D2.1 — Source of protobufs
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Vendor protobufs from `nanoporetech/minknow_api` into `proto/`
- **Rationale:** Reproducible builds, no external fetch at build time.
- **Impacts:** Proto files committed to repo, build.rs generates stubs.

### D2.2 — Supported MinKNOW versions
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Installed MinKNOW version as floor (no backwards compat requirement)
- **Rationale:** Target current MinKNOW deployments. Avoid complexity of supporting old versions.
- **Impacts:** Single proto version, simpler compatibility story.

### D2.3 — API surface scope (MVP)
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Manager + Acquisition (state + stop/pause/restart) + Stats for charts + Error surfacing
- **Rationale:** Covers core monitoring and basic control needs for showcase demo.
- **Impacts:** Generated stubs scope, wrapper API surface.

---

## 3) CLI

### D3.1 — Default behavior
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** `termion` launches TUI by default
- **Rationale:** TUI-primary tool. Users expect the main interface without subcommands.
- **Impacts:** Binary entry point, arg parsing structure.

### D3.2 — Non-interactive commands
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Minimal: `termion status`, `termion list`
- **Rationale:** Covers 90% of scripting needs without building full CLI parity.
- **Impacts:** Limited command surface, simple clap setup.

### D3.3 — Output formats
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Human-readable default, `--json` flag for scripting
- **Rationale:** Good defaults for humans, machine-readable option for automation.
- **Impacts:** Output formatting layer, stable JSON schema for scripting.

### D3.4 — Config precedence
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** CLI flags → env vars → config file → defaults
- **Rationale:** Standard precedence, no surprises.
- **Impacts:** Config loading order, documentation.

---

## 4) TUI

### D4.1 — Screen structure
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Overview → Position Detail → Overlay structure
- **Rationale:** Progressive disclosure. Start with high-level, drill into detail.
- **Impacts:** Screen state model, navigation flow.

### D4.2 — Keybindings
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Conventional keybindings (arrows, Enter, Esc)
- **Rationale:** Accessible to all users, not just vim users.
- **Impacts:** Input handling, help overlay content.

### D4.3 — Update cadence
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** 1s polling for overview, streaming for detail views (TBD specifics)
- **Rationale:** Balance responsiveness with resource usage.
- **Impacts:** Data fetch strategy, channel design.

### D4.4 — Offline/reconnect UX
- **Status:** Accepted
- **Date:** 2026-01-19
- **Decision:** Graceful degradation + auto-reconnect with exponential backoff
- **Rationale:** Keep last-known state visible, automatically recover.
- **Impacts:** Connection state machine, UI states for disconnected mode.

### D4.5 — Design iteration process
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Hybrid approach — text mockups first, then iterate on functional UI
- **Design references:** scope-tui, bottom, trippy (streaming data visualization focus)
- **Rationale:** Demo/showcase quality requires intentional design. References provide quality bar for real-time data visualization.
- **Impacts:** Development workflow, design documentation needs.

---

## 5) Performance

### D5.1 — Polling interval
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** 1s default, configurable via flag/config
- **Rationale:** Reasonable default, flexibility for different needs.
- **Impacts:** Config schema, data fetch timing.

### D5.2 — Backpressure policy
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Drop stale frames if UI can't keep up
- **Rationale:** Prefer fresh data over complete history for real-time monitoring.
- **Impacts:** Channel strategy, no unbounded buffering.

### D5.3 — Reconnect policy
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Exponential backoff (1s → 30s cap)
- **Rationale:** Standard approach, avoids thundering herd, user-friendly.
- **Impacts:** Connection manager implementation.

### D5.4 — Logging defaults
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Off by default; `-v`/`-vv`/`-vvv` enables logging to file
- **Log location:** `~/.local/state/termion/termion.log` (XDG_STATE_HOME)
- **Rationale:** TUI owns the screen, logging would corrupt display. File logging for debugging.
- **Impacts:** Tracing setup, no stderr output during TUI.

---

## 6) Testing

### D6.1 — Unit test scope
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Core logic only (parsing, state machines, data transforms)
- **Rationale:** Focus testing effort on critical logic, not UI rendering.
- **Impacts:** Test structure, coverage targets.

### D6.2 — Integration testing
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Mock gRPC server (no real MinKNOW required in CI)
- **Rationale:** Reproducible CI, no hardware dependency.
- **Impacts:** Test harness design, mock server implementation.

### D6.3 — TUI visual testing
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Manual testing only (no golden/snapshot tests for MVP)
- **Rationale:** Visual testing adds complexity, manual review sufficient for MVP.
- **Impacts:** No visual regression CI, relies on manual QA.

### D6.4 — CI strategy
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Unit + mock integration tests on every PR
- **Rationale:** Fast feedback, catch regressions early.
- **Impacts:** CI pipeline configuration.

---

## 7) Packaging

### D7.1 — Binary distribution
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** Single binary: `termion`
- **Rationale:** Simpler distribution, TUI and CLI in one package.
- **Impacts:** Crate structure (single binary crate with library).

### D7.2 — Install targets
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** MVP: `cargo install` only; Homebrew post-launch
- **Rationale:** Minimize packaging work for MVP, add channels based on demand.
- **Impacts:** Release process, documentation.

### D7.3 — Config file location
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** `~/.config/termion/config.toml` (XDG_CONFIG_HOME compliant)
- **Rationale:** Standard location, cross-platform via dirs crate.
- **Impacts:** Config loading, documentation.

### D7.4 — Versioning
- **Status:** Accepted
- **Date:** 2026-01-20
- **Decision:** SemVer
- **Rationale:** Standard for tools, communicates compatibility. Pre-1.0 signals early stage.
- **Impacts:** Release process, changelog format.
