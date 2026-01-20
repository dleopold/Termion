# Phase 2 — TUI MVP

## Overview

Phase 2 built the terminal user interface for monitoring MinKNOW sequencing runs. This involved:
- Setting up the terminal handling with crossterm
- Building an event loop with keyboard input and timed ticks
- Creating screens (Overview, Position Detail) with ratatui
- Implementing real-time charts, sparklines, and status indicators
- Handling connection state and auto-reconnection

The TUI follows the "immediate mode" rendering paradigm where the entire screen is redrawn on each frame based on current application state.

---

## Concepts

### Immediate Mode vs Retained Mode

**Retained Mode** (like HTML/DOM): You build a tree of UI elements, and the framework tracks changes and updates only what's needed.

**Immediate Mode** (like ratatui): You redraw everything every frame. There's no persistent UI tree — you call rendering functions that produce output directly.

```rust
// Immediate mode: every frame, we render the entire UI
loop {
    terminal.draw(|frame| {
        // Render everything based on current state
        render_header(frame, &app);
        render_content(frame, &app);
        render_footer(frame, &app);
    })?;
    
    // Handle events, update state
    handle_events(&mut app);
}
```

**Why immediate mode for TUIs?**
- Simpler mental model — what you render is what you see
- No diffing overhead for text-based UIs
- Natural fit for terminal rendering (write chars to buffer, flush)
- Easy to reason about state → UI mapping

### Terminal Raw Mode

Terminals normally operate in "cooked mode" with line buffering and special character handling. For interactive TUIs, we need "raw mode":

```rust
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

enable_raw_mode()?;  // Disable line buffering, echo, etc.
// ... run TUI ...
disable_raw_mode()?; // Restore normal terminal behavior
```

**Raw mode gives us:**
- Character-by-character input (no waiting for Enter)
- No automatic echoing of typed characters
- Full control over cursor and screen

**Always restore the terminal:**
```rust
fn setup_terminal() -> Result<Terminal<...>> {
    enable_raw_mode()?;
    // ...
}

fn restore_terminal(terminal: &mut Terminal<...>) -> Result<()> {
    disable_raw_mode()?;
    // Show cursor, leave alternate screen, etc.
}

// Use a guard pattern or explicit cleanup
let result = run_app(&mut terminal);
restore_terminal(&mut terminal)?;  // Always runs
result
```

### Alternate Screen

Terminals support an "alternate screen buffer" — a separate screen that preserves the original content:

```rust
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

execute!(stdout, EnterAlternateScreen)?;  // Switch to alternate
// ... TUI runs here ...
execute!(stdout, LeaveAlternateScreen)?;  // Restore original
```

This is how `vim`, `less`, and other TUI apps work without destroying your terminal history.

---

## ratatui Architecture

### Frame and Buffer

ratatui renders to a `Buffer` — a 2D grid of styled cells:

```rust
terminal.draw(|frame| {
    // frame.area() gives the full terminal size
    let area = frame.area();
    
    // Widgets render to the frame's buffer
    frame.render_widget(my_widget, area);
})?;
```

The `Frame` provides:
- `area()` — Terminal dimensions as `Rect`
- `render_widget()` — Render a widget to a region
- `render_stateful_widget()` — Render with mutable state

### Layout

`Layout` divides space into chunks:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),    // Fixed 3 rows for header
        Constraint::Min(10),      // At least 10 rows, grows
        Constraint::Length(1),    // Fixed 1 row for footer
    ])
    .split(area);

// chunks[0] = header area
// chunks[1] = content area  
// chunks[2] = footer area
```

**Constraint types:**
- `Length(n)` — Exactly n cells
- `Min(n)` — At least n cells
- `Max(n)` — At most n cells
- `Percentage(n)` — n% of available space
- `Ratio(a, b)` — a/b of available space

### Widgets

Widgets implement the `Widget` trait:

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

Widgets are consumed on render (note `self`, not `&self`). This is intentional — widgets are typically cheap to construct and render once.

**Built-in widgets:**
- `Paragraph` — Text with wrapping and alignment
- `Block` — Borders and titles
- `Table` — Rows and columns
- `List` — Selectable items
- `Chart` — Line/scatter plots
- `Gauge` — Progress bars
- `Sparkline` — Mini inline charts

### Composing Widgets

Widgets compose via `Block` borders and `Layout`:

```rust
let block = Block::default()
    .title(" My Section ")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

let paragraph = Paragraph::new("Content here")
    .block(block);  // Paragraph inside a bordered box

frame.render_widget(paragraph, area);
```

---

## Event Loop Pattern

### Two-Thread Architecture

The TUI uses two threads:
1. **Event thread** — Polls for keyboard/terminal events
2. **Main thread** — Renders and handles application logic

```rust
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Spawn event polling thread
        std::thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    // Got an event before timeout
                    if let Ok(evt) = event::read() {
                        tx.send(Event::from(evt)).ok();
                    }
                } else {
                    // Timeout — send tick
                    tx.send(Event::Tick).ok();
                }
            }
        });
        
        Self { rx }
    }
    
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
```

**Why separate threads?**
- `crossterm::event::poll()` is blocking
- We need async for gRPC calls
- Tick events drive periodic updates even without input

### Event Types

```rust
pub enum Event {
    Key(KeyEvent),      // Keyboard input
    Tick,               // Periodic timer
    Resize(u16, u16),   // Terminal resize
}
```

### Action Pattern

Map raw events to semantic actions:

```rust
pub enum Action {
    Quit,
    Up,
    Down,
    Enter,
    Back,
    Help,
    Refresh,
    None,
}

impl From<KeyEvent> for Action {
    fn from(key: KeyEvent) -> Self {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Up | KeyCode::Char('k') => Action::Up,
            KeyCode::Down | KeyCode::Char('j') => Action::Down,
            KeyCode::Enter => Action::Enter,
            KeyCode::Esc => Action::Back,
            KeyCode::Char('?') => Action::Help,
            _ => Action::None,
        }
    }
}
```

**Benefits:**
- Decouple input handling from business logic
- Easy to add alternative keybindings
- Testable without simulating keys

---

## State Management

### Application State

Centralize all UI state in one struct:

```rust
pub struct App {
    pub screen: Screen,
    pub overlay: Overlay,
    pub connection: ConnectionState,
    pub positions: Vec<Position>,
    pub selected_position: usize,
    pub stats_cache: HashMap<String, StatsSnapshot>,
    pub chart_data: HashMap<String, ChartBuffer>,
    pub should_quit: bool,
}
```

**Key principles:**
- State is the single source of truth
- Rendering reads state, never modifies it
- Event handling modifies state, never renders
- Keep derived/computed data minimal

### Screen and Overlay

Use enums for screen navigation:

```rust
pub enum Screen {
    Overview,
    PositionDetail { position_idx: usize },
}

pub enum Overlay {
    None,
    Help,
    Error { message: String },
}
```

Overlays render on top of screens without replacing them.

### Connection State Machine

```rust
pub enum ConnectionState {
    Connected,
    Connecting,
    Disconnected { since: Instant, reason: String },
    Reconnecting { attempt: u32 },
}
```

Render differently based on connection state:

```rust
let status = match &app.connection {
    ConnectionState::Connected => 
        Span::styled("● Connected", Style::default().fg(Color::Green)),
    ConnectionState::Reconnecting { attempt } =>
        Span::styled(
            format!("◌ Reconnecting ({})...", attempt),
            Style::default().fg(Color::Yellow)
        ),
    // ...
};
```

---

## Charts and Sparklines

### Time-Series Charts

ratatui's `Chart` widget needs data as `(f64, f64)` points:

```rust
let datasets = vec![
    Dataset::default()
        .name("Throughput")
        .marker(symbols::Marker::Braille)  // High-resolution dots
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data_points)  // &[(f64, f64)]
];

let chart = Chart::new(datasets)
    .x_axis(Axis::default()
        .bounds([x_min, x_max])
        .labels(vec![Line::from("past"), Line::from("now")]))
    .y_axis(Axis::default()
        .bounds([0.0, y_max])
        .labels(vec![Line::from("0"), Line::from(format!("{:.1}", y_max))]));
```

**Data management:**

```rust
pub struct ChartBuffer {
    data: Vec<(f64, f64)>,
    max_points: usize,
}

impl ChartBuffer {
    pub fn push(&mut self, timestamp: f64, value: f64) {
        if self.data.len() >= self.max_points {
            self.data.remove(0);  // Drop oldest
        }
        self.data.push((timestamp, value));
    }
}
```

### Mini Sparklines

Text-based sparklines using Unicode block characters:

```rust
fn render_mini_sparkline(data: &[(f64, f64)], width: usize) -> String {
    let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    
    // Normalize values to 0-7 range
    let max_val = data.iter().map(|(_, y)| *y).fold(0.0f64, f64::max);
    
    data.iter()
        .map(|(_, val)| {
            let idx = ((val / max_val) * 7.0).round() as usize;
            bars[idx.min(7)]
        })
        .collect()
}
```

Result: `▂▃▅▇█▆▄▃▂▁` — shows trend at a glance.

---

## Styling

### Style and Color

```rust
use ratatui::style::{Color, Modifier, Style, Stylize};

// Method chaining
let style = Style::default()
    .fg(Color::Cyan)
    .bg(Color::Black)
    .add_modifier(Modifier::BOLD);

// Stylize trait shortcuts
let span = "Hello".cyan().bold();
```

**Color options:**
- Named: `Color::Red`, `Color::Green`, etc.
- 256-color: `Color::Indexed(208)` (orange)
- RGB: `Color::Rgb(255, 128, 0)`

### Status Indicators

Use Unicode symbols with colors:

```rust
let indicator = match state {
    Running => Span::styled("●", Style::default().fg(Color::Green)),
    Paused  => Span::styled("●", Style::default().fg(Color::Yellow)),
    Idle    => Span::styled("○", Style::default().fg(Color::DarkGray)),
    Error   => Span::styled("✖", Style::default().fg(Color::Red)),
};
```

---

## Gotchas & Lessons Learned

### 1. Terminal Cleanup on Panic

If your app panics, the terminal stays in raw mode. Use a panic hook:

```rust
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |panic| {
    disable_raw_mode().ok();
    execute!(std::io::stdout(), LeaveAlternateScreen).ok();
    original_hook(panic);
}));
```

### 2. Blocking I/O in Async Context

`crossterm::event::poll()` blocks the thread. Don't call it from async code — use a dedicated thread with channel communication.

### 3. Frame Rate vs Data Rate

Separate concerns:
- **Render rate**: How often you redraw (event-driven + tick)
- **Data rate**: How often you fetch new data (configurable)

Don't fetch data on every render — it's wasteful and can cause UI lag.

### 4. Unicode Width

Some Unicode characters (like CJK or emojis) are "wide" — they take 2 cells. Use `unicode-width` crate if you need precise alignment with user-provided text.

### 5. Color Support Detection

Not all terminals support 256 colors or RGB. ratatui handles this gracefully by falling back, but test on different terminals.

### 6. Stateful Widget Confusion

`StatefulWidget` is for widgets that need to track state across renders (like scroll position in a list). Most widgets are stateless — they render based entirely on input data.

---

## Key Takeaways

1. **Immediate mode is simple** — Redraw everything, don't track diffs.

2. **Separate threads for events** — Keep async code away from blocking event polling.

3. **State → UI is one-way** — Render functions read state, event handlers modify state.

4. **Layout is constraint-based** — Think in terms of min/max/percentage, not absolute positions.

5. **Widgets are cheap and disposable** — Create them fresh each frame.

6. **Always restore the terminal** — Raw mode and alternate screen must be cleaned up.

7. **Unicode enables rich text UI** — Box drawing, symbols, and sparkline chars work in most terminals.

---

## Resources

### Official Documentation
- [ratatui docs](https://docs.rs/ratatui) — Widget reference and examples
- [crossterm docs](https://docs.rs/crossterm) — Terminal manipulation
- [ratatui book](https://ratatui.rs/) — Tutorials and guides

### Reference TUIs
- [bottom](https://github.com/ClementTsang/bottom) — System monitor
- [gitui](https://github.com/extrawurst/gitui) — Git TUI
- [lazygit](https://github.com/jesseduffield/lazygit) — Another Git TUI (Go, but good design reference)

### Patterns
- [ratatui examples](https://github.com/ratatui/ratatui/tree/main/examples) — Official examples
- [tui-rs-tree-widget](https://github.com/EdJoPaTo/tui-rs-tree-widget) — Example of custom widget
