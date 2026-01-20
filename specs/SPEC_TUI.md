# SPEC_TUI.md
TUI Specification — Termion

## Overview

The TUI is the primary interface for Termion. It provides real-time monitoring of MinKNOW sequencing runs with a focus on showcase-quality visual polish and streaming data visualization.

**Design References:** scope-tui, bottom, trippy (per D4.5)

---

## Design Principles

1. **Progressive Disclosure** — Overview first, drill into detail on demand
2. **Real-time Visualization** — Streaming data with charts/sparklines (like bottom, trippy)
3. **Graceful Degradation** — Handle disconnects elegantly, show last-known state
4. **Conventional Navigation** — Arrows, Enter, Esc (not vim-only)
5. **Context-Aware Help** — Always-visible keybinding hints (like lazygit)

---

## Screen Structure

Per decision D4.1: Overview → Position Detail → Overlay

```
┌─────────────────────────────────────────────────────┐
│                    OVERVIEW                          │
│  (Device list, position states, high-level stats)   │
│                                                      │
│  [Enter] to select position                         │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│               POSITION DETAIL                        │
│  (Run state, real-time charts, metrics)             │
│                                                      │
│  [Esc] back to overview                             │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│                   OVERLAYS                           │
│  - Help (?)                                         │
│  - Error details                                    │
│  - Confirmation dialogs                             │
└─────────────────────────────────────────────────────┘
```

---

## Screen Specifications

### 1. Overview Screen

**Purpose:** At-a-glance status of all devices and positions.

```
┌─ Termion ─────────────────────────────────────────────────────┐
│                                                                │
│  Devices                                                       │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ ● MinION Mk1C          2 positions    ▸ Select           │ │
│  │ ○ GridION X5           5 positions                       │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  Positions (MinION Mk1C)                                       │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │ Pos  State      Run Time    Reads     Throughput         │ │
│  │ ─────────────────────────────────────────────────────── │ │
│  │ 1    ● Running  02:34:12    1.2M      █████████▒ 2.3Gb/h │ │
│  │ 2    ○ Idle     --:--:--    --        ░░░░░░░░░░ --      │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  [↑↓] Navigate  [Enter] Details  [q] Quit  [?] Help           │
└───────────────────────────────────────────────────────────────┘
```

**Components:**
- Device list with selection highlight
- Position table for selected device
- Status indicators (colored dots: green=running, yellow=paused, gray=idle, red=error)
- Mini throughput sparklines
- Footer keybinding hints

**Data refresh:** 1s polling (per D5.1)

---

### 2. Position Detail Screen

**Purpose:** Deep dive into a single position's run data with real-time charts.

```
┌─ Position 1 ── MinION Mk1C ── Running ────────────────────────┐
│                                                                │
│  Run: experiment_2026_01_20    Protocol: LSK114               │
│  Started: 10:23:45             Elapsed: 02:34:12              │
│                                                                │
│  ┌─ Throughput ────────────────────────────────────────────┐  │
│  │     2.5 ┤                          ╭─────╮               │  │
│  │ Gb/h    │                    ╭────╯     ╰──╮            │  │
│  │     1.0 ┤          ╭────────╯              ╰────        │  │
│  │         │    ╭────╯                                      │  │
│  │     0   └────┴─────────────────────────────────────────  │  │
│  │          -30m              -15m               now        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  ┌─ Read Length ───────────────┐ ┌─ Quality ───────────────┐  │
│  │ █                           │ │         ████            │  │
│  │ ██                          │ │        ██████           │  │
│  │ ████                        │ │       ████████          │  │
│  │ ██████████                  │ │      ██████████         │  │
│  │ 0   5k   10k   15k   20k    │ │ Q5  Q10  Q15  Q20  Q25  │  │
│  └─────────────────────────────┘ └─────────────────────────┘  │
│                                                                │
│  Stats                                                         │
│  ├─ Reads:     1,234,567       ├─ Bases:    4.2 Gb            │
│  ├─ N50:       12,345 bp       ├─ Mean Q:   Q18.4             │
│  └─ Active:    412 pores       └─ Duty:     78.2%             │
│                                                                │
│  [Esc] Back  [p] Pause  [s] Stop  [r] Resume  [?] Help        │
└───────────────────────────────────────────────────────────────┘
```

**Components:**
- Header: Run info, protocol, timing
- Throughput chart: Time series (scope-tui/trippy style)
- Distribution charts: Read length histogram, quality histogram
- Key metrics: Reads, bases, N50, quality, pore utilization
- Footer keybinding hints (context-aware)

**Data refresh:** Streaming when available, 1s polling fallback

---

### 3. Help Overlay

**Purpose:** Context-sensitive keybinding reference.

```
┌─────────────────────────────────────────────────────┐
│                      Help                            │
│                                                      │
│  Navigation                                          │
│  ─────────────────────────────────────────────────  │
│  ↑/↓         Move selection                         │
│  Enter       Select / Drill down                    │
│  Esc         Back / Close overlay                   │
│  Tab         Next panel                             │
│                                                      │
│  Actions                                             │
│  ─────────────────────────────────────────────────  │
│  p           Pause acquisition                      │
│  r           Resume acquisition                     │
│  s           Stop acquisition                       │
│  R           Refresh data                           │
│                                                      │
│  General                                             │
│  ─────────────────────────────────────────────────  │
│  ?           Toggle this help                       │
│  q           Quit                                   │
│                                                      │
│                    [Esc] Close                       │
└─────────────────────────────────────────────────────┘
```

---

## Keybindings

Per decision D4.2: Conventional keybindings.

### Global (all screens)

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `?` | Toggle help overlay |
| `Esc` | Back / Close overlay |
| `Ctrl+C` | Force quit |

### Overview Screen

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate device/position list |
| `Enter` | Select position → detail view |
| `Tab` | Switch between device list and position list |
| `R` | Force refresh |

### Position Detail Screen

| Key | Action |
|-----|--------|
| `Esc` | Return to overview |
| `p` | Pause acquisition |
| `r` | Resume acquisition |
| `s` | Stop acquisition (with confirmation) |
| `←` / `→` | Scroll chart time window |
| `Tab` | Cycle focus between charts |

---

## State Model

```rust
pub enum Screen {
    Overview,
    PositionDetail { position_id: String },
}

pub enum Overlay {
    None,
    Help,
    Error { message: String },
    Confirm { action: PendingAction, message: String },
}

pub struct AppState {
    pub screen: Screen,
    pub overlay: Overlay,
    pub connection: ConnectionState,
    pub devices: Vec<Device>,
    pub positions: HashMap<String, Vec<Position>>,
    pub selected_device: Option<usize>,
    pub selected_position: Option<usize>,
    pub stats_cache: HashMap<String, StatsSnapshot>,
    pub chart_data: HashMap<String, ChartBuffer>,
}

pub enum ConnectionState {
    Connected,
    Connecting,
    Disconnected { since: Instant, reason: String },
    Reconnecting { attempt: u32 },
}
```

---

## Visual Design Guidelines

### Color Palette

| Element | Color | Meaning |
|---------|-------|---------|
| Running | Green | Active, healthy |
| Paused | Yellow | Attention needed |
| Idle | Gray | Inactive |
| Error | Red | Problem |
| Selected | Cyan/highlight | Current focus |
| Chart line | Blue | Primary data |
| Chart fill | Blue (dim) | Area under curve |

### Typography

- Headers: Bold
- Labels: Normal
- Values: Normal or bold for emphasis
- Dimmed: For secondary information

### Spacing

- Consistent 1-char padding inside boxes
- 1 empty line between major sections
- Aligned columns in tables

---

## Disconnection UX

Per decision D4.4: Graceful degradation + auto-reconnect.

### Behavior

1. **On disconnect:**
   - Show banner: "Disconnected — Reconnecting..."
   - Keep last-known data visible (dimmed)
   - Show reconnection attempt counter

2. **During reconnect:**
   - Exponential backoff (1s → 30s)
   - Update banner with attempt count
   - Allow user to force reconnect (`R`)

3. **On reconnect:**
   - Clear banner
   - Refresh all data
   - Brief "Reconnected" toast (2s)

```
┌─ Termion ─────────────────────────────────────────────────────┐
│ ⚠ Disconnected — Reconnecting (attempt 3)...        [R] Retry │
│───────────────────────────────────────────────────────────────│
│                                                                │
│  (Last known data shown dimmed)                               │
│                                                                │
```

---

## Performance

Per decisions D5.1, D5.2:

- **Refresh rate:** 1s data polling
- **Render rate:** Event-driven, max 30fps
- **Backpressure:** Drop stale frames
- **Chart buffer:** Last 30 minutes of data points

---

## Design Iteration Process

Per decision D4.5: Hybrid approach.

1. **Text mockups** — ASCII layouts like above for screen structure
2. **Functional implementation** — Build with ratatui
3. **Visual iteration** — Refine colors, spacing, chart styles
4. **Reference comparison** — Check against scope-tui, bottom, trippy quality bar

---

## Dependencies

- `ratatui` — TUI framework
- `crossterm` — Terminal backend
- `tokio` — Async runtime
- `tracing` — Instrumentation
