# PromethION Channel Map: Adaptive Layout with Scrolling

## TL;DR

> **Quick Summary**: Add adaptive layout that switches to 1×4 vertical stack when terminal is too narrow for 2×2 layout, with arrow key scrolling.
> 
> **Deliverables**:
> - New `BlockArrangement::FourVertical` layout variant
> - Adaptive layout selection based on terminal width
> - Scroll offset state in App with arrow key controls
> - Viewport clipping for tall vertical layouts
> - Scroll position indicator
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 3 waves
> **Critical Path**: Task 1 → Task 2 → Task 3 → Task 4 → Task 5

---

## Context

### Original Request
The PromethION channel map (126 columns × 25 rows) doesn't fit in most terminals. User requested:
- Adaptive layout: 2×2 when wide, 1×4 vertical stack when narrow
- Arrow key scrolling for the vertical layout

### Interview Summary
**Key Discussions**:
- Current 2×2 layout needs ~127-254 chars depending on cell width (too wide)
- Vertical 1×4 layout would be ~63 chars wide (fits any terminal)
- User wants arrow keys (up/down) for scrolling
- Layout should adapt based on terminal width

**Research Findings**:
- Current code: `BlockArrangement::FourQuadrant` with gaps at col 62, row 11
- No scrolling infrastructure exists in current TUI
- Width threshold: 254+ chars for 2×2 with 2-char cells, 127+ for 1-char cells

### Metis Review
**Identified Gaps** (addressed):
- Layout threshold: Default to 254 chars (2×2 with 2-char) vs 127 chars (2×2 with 1-char)
- Scroll granularity: Default to 1 row at a time
- Scroll feedback: Default to text indicator in title
- MinION handling: No changes needed (already fits at ~65 chars)
- Horizontal scroll: Out of scope, show "too small" message
- Arrow key context: Smart detection (only scroll when content > viewport)

---

## Work Objectives

### Core Objective
Enable PromethION channel map to display in narrow terminals by adding adaptive 1×4 vertical layout with arrow key scrolling.

### Concrete Deliverables
- New `BlockArrangement::FourVertical` enum variant in `src/tui/ui.rs`
- Modified `calculate_grid_structure()` with width-based layout selection
- New `channel_map_scroll_offset: usize` field in `App` struct
- Scroll handling in `handle_action()` for Panel 3 (Pore Activity)
- Viewport clipping in `render_pore_grid_from_states()`
- Scroll position indicator: "Rows X-Y of Z" in panel title

### Definition of Done
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] PromethION displays 1×4 vertical layout in narrow terminal
- [ ] Arrow keys scroll the channel map in Panel 3
- [ ] Scroll position indicator shows current viewport
- [ ] 2×2 layout still works in wide terminals

### Must Have
- Adaptive layout selection (2×2 or 1×4 based on width)
- Arrow key scrolling (up/down)
- Scroll position indicator
- Scroll bounds clamping (can't scroll past content)
- Scroll reset on position/panel change
- All existing tests pass

### Must NOT Have (Guardrails)
- No horizontal scrolling (out of scope)
- No smooth scrolling or momentum
- No Page Up/Down, Home/End keys (unless trivial to add)
- No MinION layout changes
- No mouse scroll support
- No scroll "bounce" effects
- No changes to channel-to-coordinate mapping

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (cargo test, 93 existing tests)
- **User wants tests**: TDD
- **Framework**: cargo test (Rust built-in)

### TDD Structure

Each task follows RED-GREEN-REFACTOR:
1. **RED**: Write failing test first
2. **GREEN**: Implement minimum code to pass
3. **REFACTOR**: Clean up while keeping tests green

### Automated Verification

**For all tasks:**
```bash
cargo test
cargo test --lib -- tui::ui::tests
cargo test --lib -- tui::app::tests
cargo fmt --check && cargo clippy -- -D warnings
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately):
└── Task 1: Add FourVertical layout variant and adaptive selection

Wave 2 (After Wave 1):
├── Task 2: Add scroll state to App
└── Task 3: Implement viewport clipping in render function

Wave 3 (After Wave 2):
├── Task 4: Wire up arrow key scroll handling
└── Task 5: Add scroll position indicator

Wave 4 (Final):
└── Task 6: Integration testing and verification

Critical Path: Task 1 → Tasks 2,3 (parallel) → Tasks 4,5 (parallel) → Task 6
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2, 3 | None |
| 2 | 1 | 4 | 3 |
| 3 | 1 | 4, 5 | 2 |
| 4 | 2, 3 | 6 | 5 |
| 5 | 3 | 6 | 4 |
| 6 | 4, 5 | None | None (final) |

---

## TODOs

- [ ] 1. Add FourVertical layout variant and adaptive selection

  **What to do**:
  - Add `FourVertical` variant to `BlockArrangement` enum in `src/tui/ui.rs`
  - Modify `calculate_grid_structure()` to select layout based on `screen_width`:
    - If `screen_width >= 254`: Use `FourQuadrant` with 2-char cells
    - Else if `screen_width >= 127`: Use `FourQuadrant` with 1-char cells  
    - Else: Use `FourVertical` with appropriate cell width
  - For `FourVertical` layout:
    - `grid_cols = 63` (half of 126)
    - `grid_rows = 53` (12+1+12+1+13+1+13 = content + gaps, showing 4 quadrants stacked vertically)
    - Quadrant order (top to bottom): TL, TR, BL, BR (left-to-right row order)
      - TL: display rows 0-11 (original coords x=0-62, y=0-11) — 12 rows
      - [gap row 12]
      - TR: display rows 13-24 (original coords x=63-125, y=0-11) — 12 rows
      - [gap row 25]
      - BL: display rows 26-38 (original coords x=0-62, y=12-24) — 13 rows
      - [gap row 39]
      - BR: display rows 40-52 (original coords x=63-125, y=12-24) — 13 rows
    - Total display rows: 12 + 1 + 12 + 1 + 13 + 1 + 13 = 53 rows
    - Gaps: horizontal after display rows 12, 25, 39 (between quadrants)
  - Update `FlowCellType::PromethION` match arm to call layout selection logic
  - Write unit tests for layout selection at various widths

  **Must NOT do**:
  - Do not change MinION layout
  - Do not modify actual coordinate mapping (that happens in render)
  - Do not add scrolling yet (that's Task 2-4)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core algorithm change with math complexity
  - **Skills**: []
    - Standard Rust

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (solo)
  - **Blocks**: Tasks 2, 3
  - **Blocked By**: None

  **References**:
  - `src/tui/ui.rs:1608-1665` - Current `calculate_grid_structure()` function
  - `src/tui/ui.rs:130-145` - `BlockArrangement` enum definition
  - `src/tui/ui.rs:1639-1665` - PromethION match arm to modify

  **Acceptance Criteria**:

  **TDD Tests:**
  ```rust
  #[test]
  fn test_promethion_uses_quadrant_in_wide_terminal() {
      let layout = create_promethion_layout();
      let result = calculate_grid_structure(&layout, 260, 30);
      assert_eq!(result.block_arrangement, BlockArrangement::FourQuadrant);
      assert_eq!(result.cell_width, 2);
  }

  #[test]
  fn test_promethion_uses_quadrant_1char_in_medium_terminal() {
      let layout = create_promethion_layout();
      let result = calculate_grid_structure(&layout, 150, 30);
      assert_eq!(result.block_arrangement, BlockArrangement::FourQuadrant);
      assert_eq!(result.cell_width, 1);
  }

  #[test]
  fn test_promethion_uses_vertical_in_narrow_terminal() {
      let layout = create_promethion_layout();
      let result = calculate_grid_structure(&layout, 100, 30);
      assert_eq!(result.block_arrangement, BlockArrangement::FourVertical);
  }
  ```
  - [ ] `cargo test --lib -- tui::ui::tests::test_promethion_uses_quadrant_in_wide` → PASS
  - [ ] `cargo test --lib -- tui::ui::tests::test_promethion_uses_vertical_in_narrow` → PASS

  **Commit**: YES
  - Message: `feat(tui): add FourVertical layout and adaptive layout selection`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 2. Add scroll state to App

  **What to do**:
  - Add `channel_map_scroll_offset: usize` field to `App` struct in `src/tui/app.rs`
  - Initialize to 0 in `App::new()`
  - Add method `reset_channel_map_scroll(&mut self)` that sets offset to 0
  - Call `reset_channel_map_scroll()` when:
    - Switching positions (in `select_position()` or similar)
    - Switching away from Panel 3 (in `set_detail_chart()` or similar)
  - Add method `clamp_channel_map_scroll(&mut self, total_rows: usize, visible_rows: usize)`:
    ```rust
    fn clamp_channel_map_scroll(&mut self, total_rows: usize, visible_rows: usize) {
        let max_offset = total_rows.saturating_sub(visible_rows);
        self.channel_map_scroll_offset = self.channel_map_scroll_offset.min(max_offset);
    }
    ```
  - Write unit tests for scroll state management

  **Must NOT do**:
  - Do not handle key events yet (Task 4)
  - Do not modify rendering yet (Task 3)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple state addition with straightforward logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3)
  - **Blocks**: Task 4
  - **Blocked By**: Task 1

  **References**:
  - `src/tui/app.rs` - App struct definition
  - `src/tui/app.rs:select_position()` - Where to call reset
  - `src/tui/ui.rs:DetailChart` enum - Panel tracking

  **Acceptance Criteria**:

  **TDD Tests:**
  ```rust
  #[test]
  fn test_scroll_offset_clamps_to_bounds() {
      let mut app = create_test_app();
      app.channel_map_scroll_offset = 100;
      app.clamp_channel_map_scroll(28, 20); // total=28, visible=20
      assert_eq!(app.channel_map_scroll_offset, 8); // max = 28-20
  }

  #[test]
  fn test_scroll_reset_on_position_change() {
      let mut app = create_test_app();
      app.channel_map_scroll_offset = 10;
      app.reset_channel_map_scroll();
      assert_eq!(app.channel_map_scroll_offset, 0);
  }
  ```
  - [ ] `cargo test --lib -- tui::app::tests::test_scroll_offset` → PASS

  **Commit**: YES
  - Message: `feat(tui): add scroll state for channel map`
  - Files: `src/tui/app.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 3. Implement viewport clipping in render function

  **What to do**:
  - Modify `render_pore_grid_from_states()` to accept scroll_offset parameter
  - Update function signature or pass through App state
  - When `BlockArrangement::FourVertical`:
    - Calculate `visible_rows` from `inner_area.height`
    - Only render rows from `scroll_offset` to `scroll_offset + visible_rows`
    - Adjust y-coordinate in rendering loop
  - For `FourQuadrant` layout, ignore scroll_offset (no scrolling)
  - Handle coordinate remapping for vertical layout:
    - Original: (x, y) where x ∈ [0, 125], y ∈ [0, 24]
    - Vertical stack order: TL, TR, BL, BR (left-to-right rows)
    - Coordinate mapping (accounting for asymmetric split at y=12):
      - TL (x < 63, y < 12): display_row = y, display_col = x
      - TR (x >= 63, y < 12): display_row = y + 13, display_col = x - 63
      - BL (x < 63, y >= 12): display_row = (y - 12) + 26, display_col = x
      - BR (x >= 63, y >= 12): display_row = (y - 12) + 40, display_col = x - 63
    - Section sizes: TL/TR = 63×12, BL/BR = 63×13
    - Total: 53 display rows (12 + 1gap + 12 + 1gap + 13 + 1gap + 13)

  **Must NOT do**:
  - Do not handle key events (Task 4)
  - Do not add scroll indicator yet (Task 5)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Complex coordinate remapping logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 2)
  - **Blocks**: Tasks 4, 5
  - **Blocked By**: Task 1

  **References**:
  - `src/tui/ui.rs:930-1130` - Current `render_pore_grid_from_states()` function
  - `src/tui/ui.rs:1023-1033` - `coord_to_channel` mapping to modify
  - `src/tui/ui.rs:1044-1100` - Rendering loop to add viewport clipping

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  # Build and run with verbose logging
  cargo build --release
  
  # Verify no panics with various scroll offsets
  cargo test --lib -- tui::ui::tests
  ```

  **Manual Verification (agent-executable):**
  ```
  1. Run: ./target/release/termion (in narrow terminal, e.g., 100 chars)
  2. Navigate to PromethION position
  3. Press '3' for Pore Activity
  4. Verify: Vertical layout displays (4 stacked sections)
  5. Verify: Only portion of grid visible (not all 53 rows)
  ```

  **Commit**: YES
  - Message: `feat(tui): implement viewport clipping for vertical layout`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 4. Wire up arrow key scroll handling

  **What to do**:
  - In `App::handle_action()` or equivalent event handler:
    - When in detail view AND Panel 3 (PoreActivity) AND FourVertical layout:
      - `Action::Up` → decrement `channel_map_scroll_offset` by 1 (min 0)
      - `Action::Down` → increment `channel_map_scroll_offset` by 1 (max = total - visible)
    - Otherwise: pass through to existing navigation behavior
  - Add detection for "needs scrolling" (total_rows > visible_rows)
  - Only consume arrow keys when scrolling is applicable
  - Write tests for scroll action handling

  **Must NOT do**:
  - Do not add Page Up/Down or other keys
  - Do not add momentum or smooth scrolling

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple event routing logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 5)
  - **Blocks**: Task 6
  - **Blocked By**: Tasks 2, 3

  **References**:
  - `src/tui/app.rs:handle_action()` - Event handler to modify
  - `src/tui/event.rs:Action::Up/Down` - Existing action definitions
  - `src/tui/app.rs` - Current view/panel state tracking

  **Acceptance Criteria**:

  **TDD Tests:**
  ```rust
  #[test]
  fn test_down_arrow_increments_scroll() {
      let mut app = create_test_app_with_vertical_layout();
      app.channel_map_scroll_offset = 0;
      app.handle_scroll_down(53, 20); // total=53 (vertical layout), visible=20
      assert_eq!(app.channel_map_scroll_offset, 1);
  }

  #[test]
  fn test_scroll_does_not_exceed_max() {
      let mut app = create_test_app_with_vertical_layout();
      app.channel_map_scroll_offset = 33; // max = 53-20 = 33
      app.handle_scroll_down(53, 20);
      assert_eq!(app.channel_map_scroll_offset, 33); // unchanged
  }
  ```
  - [ ] `cargo test --lib -- tui::app::tests::test_scroll_` → PASS

  **Manual Verification:**
  ```
  1. Run termion in narrow terminal
  2. Navigate to PromethION, Panel 3
  3. Press Down arrow → grid scrolls down
  4. Press Up arrow → grid scrolls up
  5. Scroll to bottom → further Down has no effect
  6. Scroll to top → further Up has no effect
  ```

  **Commit**: YES
  - Message: `feat(tui): add arrow key scrolling for channel map`
  - Files: `src/tui/app.rs`, `src/tui/event.rs` (if needed)
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 5. Add scroll position indicator

  **What to do**:
  - Modify channel map title to show scroll position when scrollable:
    - Current: `" Channel Map "`
    - With scroll: `" Channel Map [1-20/53] "`
  - Only show indicator when `total_rows > visible_rows`
  - Pass scroll info to `render_pore_grid_from_states()` or construct title externally
  - Format: `[start_row-end_row/total_rows]`

  **Must NOT do**:
  - Do not add graphical scrollbar
  - Do not add percentage indicator

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple string formatting
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 4)
  - **Blocks**: Task 6
  - **Blocked By**: Task 3

  **References**:
  - `src/tui/ui.rs:937-940` - Block title definition

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  cargo test --lib -- tui::ui::tests
  ```

  **Manual Verification:**
  ```
  1. Run termion in narrow terminal
  2. Navigate to PromethION, Panel 3
  3. Verify: Title shows " Channel Map [1-20/53] " (or similar)
  4. Scroll down → numbers update
  5. In wide terminal → indicator not shown (no scrolling needed)
  ```

  **Commit**: YES
  - Message: `feat(tui): add scroll position indicator to channel map`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 6. Integration testing and verification

  **What to do**:
  - Run full test suite: `cargo test`
  - Run clippy: `cargo clippy -- -D warnings`
  - Run fmt check: `cargo fmt --check`
  - Manual verification with live PromethION:
    - Narrow terminal: Verify vertical layout and scrolling
    - Wide terminal: Verify 2×2 layout unchanged
  - Test edge cases:
    - Terminal resize during scroll
    - Switching panels while scrolled
    - Switching positions while scrolled

  **Must NOT do**:
  - Do not modify code (verification only)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Verification task
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (final)
  - **Blocks**: None
  - **Blocked By**: Tasks 4, 5

  **References**:
  - All previous tasks
  - Live device: P2S-03597-A (PromethION)

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  cargo test                        # All tests pass
  cargo clippy -- -D warnings       # No warnings
  cargo fmt --check                 # No formatting issues
  cargo build --release             # Build succeeds
  ```

  **Evidence to Capture:**
  - [ ] Test output showing all tests pass
  - [ ] Screenshot of narrow terminal with vertical layout
  - [ ] Screenshot showing scroll indicator

  **Commit**: NO (verification only)

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `feat(tui): add FourVertical layout and adaptive selection` | ui.rs | cargo test |
| 2 | `feat(tui): add scroll state for channel map` | app.rs | cargo test |
| 3 | `feat(tui): implement viewport clipping for vertical layout` | ui.rs | cargo test |
| 4 | `feat(tui): add arrow key scrolling for channel map` | app.rs | cargo test |
| 5 | `feat(tui): add scroll position indicator` | ui.rs | cargo test |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test
# Expected: 93+ tests, 0 failures

# Lints pass
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Build succeeds
cargo build --release
```

### Final Checklist
- [ ] All "Must Have" features implemented
- [ ] All "Must NOT Have" guardrails respected
- [ ] All tests pass (existing + new)
- [ ] PromethION vertical layout works in narrow terminal
- [ ] Arrow key scrolling works
- [ ] Scroll position indicator shows
- [ ] 2×2 layout unchanged in wide terminal
- [ ] MinION layout unchanged
