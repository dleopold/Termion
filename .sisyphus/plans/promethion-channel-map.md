# PromethION Channel Map Support

## TL;DR

> **Quick Summary**: Add PromethION flow cell support to the channel status map, displaying 3000 channels in a 2×2 quadrant layout with dynamic sizing.
> 
> **Deliverables**:
> - Refactored `render_pore_grid_from_states()` supporting both MinION and PromethION layouts
> - New layout calculation functions with unit tests
> - Dynamic cell sizing that fills available terminal space
> - Visual indicator when terminal is too small
> - Updated SPEC_TUI.md documentation
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 → Tasks 2,3 (parallel) → Task 4 → Task 5 → Task 7

---

## Context

### Original Request
Support PromethION flow cell channel map in the TUI. The current implementation is designed around MinION's 512 channels in a 32×16 grid. PromethION has 3000 channels that need to display as 4 quadrants like the MinKNOW GUI, with dynamic sizing to accommodate different flow cell types.

### Interview Summary
**Key Discussions**:
- Layout strategy: User chose **2×2 quadrant grid** for PromethION (matching MinKNOW GUI)
- Cell sizing: User chose **dynamic sizing** to fill available space
- Quadrant separation: **Gap/spacing** between quadrants
- Small terminal handling: **Show partial with indicator**
- Device type detection: **Infer from channel count** (512=MinION, 3000=PromethION)
- Testing: **TDD with unit tests** for layout logic

**Research Findings**:
- MinION: 512 channels, 32×16 grid, 2 vertical blocks with gap after row 7
- PromethION (live device probe): 3000 channels, 126×25 physical grid, sparse Y coordinates (0,4,8...96)
- Current code hardcodes MinION: `let has_two_blocks = grid_height == 16;` (ui.rs:894)
- ChannelLayout struct already has width, height, coords fields
- DeviceType enum exists but isn't currently used in rendering

### Metis Review
**Identified Gaps** (addressed):
- Device type detection: Resolved - infer from channel count
- GridION handling: Auto-resolved - follows MinION path (same flow cell type)
- Quadrant boundaries: Default applied - physical coordinate midpoints (X=63, Y=48)
- Minimum terminal size: Default applied - at least 1 char per quadrant
- Performance budget: Default applied - render < 16ms for 60fps TUI
- Function signature: Resolved - no signature changes needed with inference approach

---

## Work Objectives

### Core Objective
Enable the channel map widget to correctly display both MinION (512 channels, 2 blocks) and PromethION (3000 channels, 4 quadrants) flow cells with dynamic sizing that fills the available terminal space.

### Concrete Deliverables
- Modified `render_pore_grid_from_states()` in `src/tui/ui.rs`
- New `FlowCellType` enum (MinION, PromethION) for layout classification
- New `GridStructure` struct representing calculated layout (blocks, gaps, cell size)
- New `calculate_grid_structure()` pure function for testable layout logic
- Unit tests for layout calculations
- Updated SPEC_TUI.md section 2a with PromethION documentation

### Definition of Done
- [ ] `cargo test` passes (all 74+ tests including new ones)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] MinION channel map displays identically to current behavior (regression guard)
- [ ] PromethION channel map displays as 4 quadrants with gaps
- [ ] Dynamic sizing fills available space appropriately
- [ ] Partial display indicator shows when terminal too small

### Must Have
- MinION layout unchanged (2 blocks, gap after row 7)
- PromethION 4-quadrant layout with gaps
- Dynamic cell sizing (1-char or 2-char based on space)
- Device type inference from channel count
- Unit tests for layout logic
- Documentation updates

### Must NOT Have (Guardrails)
- No support for Flongle, P2, or other device types
- No zooming, panning, or channel selection features
- No changes to channel state data fetching or client code
- No new dependencies
- No premature abstraction (keep layout logic in ui.rs unless >200 lines)
- No configuration options for quadrant arrangement (hardcode MinKNOW-standard)
- No quadrant labels (Q1, Q2, etc.) - keep simple like MinION
- No changes to ChannelLayout struct (use inference approach)

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (cargo test, 74 existing tests)
- **User wants tests**: TDD
- **Framework**: cargo test (Rust built-in)

### TDD Structure

Each core task follows RED-GREEN-REFACTOR:

1. **RED**: Write failing test first in `src/tui/ui.rs` test module
2. **GREEN**: Implement minimum code to pass
3. **REFACTOR**: Clean up while keeping tests green

### Automated Verification

**For all tasks:**
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --lib -- tui::ui::tests

# Check formatting and lints
cargo fmt --check && cargo clippy -- -D warnings
```

**For visual verification (manual with live device):**
```bash
# Run TUI against live PromethION
./target/release/termion

# Navigate to P2S-03597-A position
# Press 3 for Pore Activity panel
# Verify 4-quadrant layout displays correctly
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately):
├── Task 1: Add FlowCellType enum and GridStructure types
└── Task 6: Update SPEC_TUI.md documentation (no code deps)

Wave 2 (After Wave 1):
├── Task 2: Implement calculate_grid_structure() with TDD
└── Task 3: Implement dynamic cell sizing logic (can start in parallel)

Wave 3 (After Wave 2):
├── Task 4: Refactor render_pore_grid_from_states()
└── Task 5: Add terminal-too-small indicator

Wave 4 (After Wave 3):
└── Task 7: Integration testing and regression verification

Critical Path: Task 1 → Tasks 2,3 (parallel) → Task 4 → Task 5 → Task 7
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2, 3, 4 | 6 |
| 2 | 1 | 4 | 3, 6 |
| 3 | 1 | 4 | 2, 6 |
| 4 | 2, 3 | 5, 7 | None |
| 5 | 4 | 7 | None |
| 6 | None | None | 1, 2, 3 |
| 7 | 4, 5 | None | None (final) |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Approach |
|------|-------|---------------------|
| 1 | 1, 6 | Parallel: types + docs |
| 2 | 2, 3 | Parallel: layout calc + sizing |
| 3 | 4, 5 | Sequential: refactor depends on 2,3 |
| 4 | 7 | Final verification |

---

## TODOs

- [x] 1. Add FlowCellType enum and GridStructure types

  **What to do**:
  - Add `FlowCellType` enum with `MinION` and `PromethION` variants
  - Add helper function `FlowCellType::from_channel_count(count: usize) -> FlowCellType`
    - 512 or fewer channels → MinION
    - Greater than 512 channels → PromethION
  - Add `BlockArrangement` enum:
    ```rust
    enum BlockArrangement {
        TwoVertical,    // MinION: 2 blocks stacked vertically
        FourQuadrant,   // PromethION: 2×2 grid of quadrants
    }
    ```
  - Add `GapPosition` enum:
    ```rust
    enum GapPosition {
        Horizontal { after_row: usize },   // Gap after specified row
        Vertical { after_col: usize },     // Gap after specified column
    }
    ```
  - Add `GridStructure` struct to represent calculated layout:
    - `flow_cell_type: FlowCellType`
    - `grid_cols: usize` (normalized column count)
    - `grid_rows: usize` (normalized row count)
    - `block_arrangement: BlockArrangement`
    - `gap_positions: Vec<GapPosition>` (where to draw gaps)
    - `cell_width: usize` (1 or 2 characters)
  - Write unit tests for `FlowCellType::from_channel_count()`
  - Write test for edge case: `from_channel_count(0)` returns `MinION` (default fallback)

  **Must NOT do**:
  - Do not add Flongle, P2, or other device types
  - Do not modify existing ChannelLayout struct
  - Do not add to separate module (keep in ui.rs for now)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused type definitions with simple tests
  - **Skills**: []
    - No special skills needed for basic Rust types

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 6)
  - **Blocks**: Tasks 2, 3, 4
  - **Blocked By**: None (can start immediately)

  **References**:
  - `src/tui/ui.rs:863-976` - Current render function, add types near top of file
  - `src/client/types.rs:43-78` - DeviceType enum as reference for enum style
  - `src/client/types.rs:499-507` - ChannelLayout struct (do NOT modify)

  **Acceptance Criteria**:

  **TDD Tests:**
  - [ ] Test file: add `#[cfg(test)] mod tests` in ui.rs if not exists
  - [ ] Test: `FlowCellType::from_channel_count(512)` returns `MinION`
  - [ ] Test: `FlowCellType::from_channel_count(3000)` returns `PromethION`
  - [ ] Test: `FlowCellType::from_channel_count(126)` returns `MinION` (Flongle treated as MinION for now)
  - [ ] `cargo test --lib -- tui::ui::tests::test_flow_cell_type` → PASS

  **Commit**: YES
  - Message: `feat(tui): add FlowCellType enum and GridStructure types for multi-device support`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [x] 2. Implement calculate_grid_structure() with TDD

  **What to do**:
  - Create test helper functions (for use in tests only):
    ```rust
    #[cfg(test)]
    fn create_minion_layout() -> ChannelLayout {
        // Create 32×16 grid with 512 channels
        // Coords: (0,0) to (31,15), sequential mapping
        ChannelLayout {
            channel_count: 512,
            width: 32,
            height: 16,
            coords: (0..512).map(|i| ((i % 32) as u32, (i / 32) as u32)).collect(),
        }
    }
    
    #[cfg(test)]
    fn create_promethion_layout() -> ChannelLayout {
        // Create 126×25 grid with 3000 channels (120 per row)
        // Sparse Y: 0, 4, 8, ... 96 (normalized to 0-24)
        ChannelLayout {
            channel_count: 3000,
            width: 126,
            height: 25,
            coords: generate_promethion_coords(), // See below
        }
    }
    ```
  - Create pure function: `fn calculate_grid_structure(layout: &ChannelLayout, screen_width: usize, screen_height: usize) -> GridStructure`
  - **Integration note:** Implement cell_width calculation inline in this function. Task 3 runs in parallel and extracts a reusable `calculate_cell_dimensions()` helper. Task 4's render loop can use either (prefer the extracted function for clarity).
  - Implement MinION logic:
    - 32×16 grid, 2 vertical blocks
    - Gap after row 7
    - Cell width based on available space (prefer 2-char, fall back to 1-char)
  - Implement PromethION logic:
    - Normalize sparse Y coordinates to dense 25 rows
    - 2×2 quadrant arrangement (2 columns × 2 rows of blocks)
    - Quadrant boundaries: X midpoint at col 63, Y midpoint at row 12
    - Horizontal gap between left/right quadrants
    - Vertical gap between top/bottom quadrants
    - Cell width dynamic based on 126 columns fitting in screen
  - Write comprehensive unit tests FIRST (TDD)

  **Must NOT do**:
  - Do not handle screen sizes < 20 chars (defer to Task 5)
  - Do not render anything (pure calculation only)
  - Do not access global state

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core algorithm requiring careful calculation logic and extensive testing
  - **Skills**: []
    - Standard Rust, no special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3)
  - **Blocks**: Task 4
  - **Blocked By**: Task 1

  **References**:
  - `src/tui/ui.rs:887-914` - Current scaling logic to preserve for MinION
  - `src/tui/ui.rs:894-900` - Current MinION gap detection (replace with new logic)
  - Research finding: PromethION physical coords X=[0,125], Y=sparse(0,4,8...96), 120 channels/row

  **Acceptance Criteria**:

  **TDD Tests (write FIRST):**
  ```rust
  #[test]
  fn test_minion_layout_512_channels_two_blocks() {
      let layout = create_minion_layout(); // Helper to create 32×16 layout
      let result = calculate_grid_structure(&layout, 80, 24);
      assert_eq!(result.flow_cell_type, FlowCellType::MinION);
      assert!(matches!(result.block_arrangement, BlockArrangement::TwoVertical));
      assert_eq!(result.gap_positions.len(), 1); // Gap after row 7
  }

  #[test]
  fn test_promethion_layout_3000_channels_four_quadrants() {
      let layout = create_promethion_layout(); // Helper to create 126×25 layout
      let result = calculate_grid_structure(&layout, 120, 30);
      assert_eq!(result.flow_cell_type, FlowCellType::PromethION);
      assert!(matches!(result.block_arrangement, BlockArrangement::FourQuadrant));
      assert_eq!(result.gap_positions.len(), 2); // H and V gaps
  }

  #[test]
  fn test_dynamic_cell_width_narrow_terminal() {
      let layout = create_promethion_layout();
      let result = calculate_grid_structure(&layout, 60, 30); // Narrow
      assert_eq!(result.cell_width, 1); // Must use 1-char cells
  }

  #[test]
  fn test_dynamic_cell_width_wide_terminal() {
      let layout = create_minion_layout();
      let result = calculate_grid_structure(&layout, 120, 30); // Wide
      assert_eq!(result.cell_width, 2); // Can use 2-char cells
  }
  ```
  - [ ] `cargo test --lib -- tui::ui::tests::test_minion_layout` → PASS
  - [ ] `cargo test --lib -- tui::ui::tests::test_promethion_layout` → PASS
  - [ ] `cargo test --lib -- tui::ui::tests::test_dynamic_cell_width` → PASS

  **Commit**: YES
  - Message: `feat(tui): implement calculate_grid_structure() for MinION and PromethION layouts`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [x] 3. Implement dynamic cell sizing logic

  **What to do**:
  - **Integration note:** This task extracts cell sizing as a reusable helper function. Task 2 may implement similar logic inline; this provides a clean abstraction for Task 4's render loop to use.
  - Create function: `fn calculate_cell_dimensions(grid_cols: usize, grid_rows: usize, screen_width: usize, screen_height: usize, block_arrangement: &BlockArrangement) -> (usize, usize)`
  - Returns `(cell_width, cell_height)` where cell_width is 1 or 2
  - Logic:
    - Calculate space needed with 2-char cells
    - If doesn't fit, calculate with 1-char cells
    - Account for gaps in block arrangements
    - Maximize use of available space
  - Write unit tests for edge cases

  **Must NOT do**:
  - Do not allow cell_width < 1
  - Do not add scrolling or truncation logic (that's Task 5)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Focused calculation function with clear inputs/outputs
  - **Skills**: []
    - Standard Rust math

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 2)
  - **Blocks**: Task 4
  - **Blocked By**: Task 1

  **References**:
  - `src/tui/ui.rs:902-909` - Current cell width constant and scaling
  - Task 2's `GridStructure` for integration

  **Acceptance Criteria**:

  **TDD Tests:**
  - [ ] Test: 32×16 grid in 80-char terminal → cell_width=2
  - [ ] Test: 126×25 grid in 80-char terminal → cell_width=1
  - [ ] Test: 126×25 grid in 260-char terminal → cell_width=2
  - [ ] `cargo test --lib -- tui::ui::tests::test_cell_dimensions` → PASS

  **Commit**: YES
  - Message: `feat(tui): add dynamic cell sizing calculation for channel map`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 4. Refactor render_pore_grid_from_states() to use new layout system

  **What to do**:
  - Replace hardcoded `has_two_blocks = grid_height == 16` logic
  - Call `calculate_grid_structure()` to get layout parameters
  - Implement rendering loop that respects `GridStructure`:
    - For `TwoVertical`: render 2 blocks with single horizontal gap
    - For `FourQuadrant`: render 4 quadrants with H and V gaps
  - Use dynamic cell width from GridStructure
  - Maintain coordinate-to-channel mapping for both layouts
  - Normalize PromethION's sparse Y coordinates to dense rows

  **Must NOT do**:
  - Do not change function signature
  - Do not change how channel_states or channel_layout are passed
  - Do not modify ChannelStatesSnapshot or ChannelLayout types
  - Do not break existing MinION rendering

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core refactoring with regression risk, requires careful integration
  - **Skills**: []
    - Standard Rust, no special skills

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential)
  - **Blocks**: Tasks 5, 7
  - **Blocked By**: Tasks 2, 3

  **References**:
  - `src/tui/ui.rs:863-976` - Current implementation to refactor
  - `src/tui/ui.rs:916-926` - Coordinate mapping logic to preserve/extend
  - `src/tui/ui.rs:928-972` - Rendering loop to refactor

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  # All existing tests must pass (regression guard)
  cargo test
  
  # New layout tests must pass
  cargo test --lib -- tui::ui::tests
  
  # Lints must pass
  cargo clippy -- -D warnings
  ```

  **Manual Verification (with live device):**
  ```
  # Agent executes via terminal:
  1. Run: ./target/release/termion
  2. Select position P2S-03597-A (the live PromethION)
  3. Press '3' to switch to Pore Activity panel
  4. Verify: 4-quadrant layout visible with gaps between quadrants
  5. Verify: All 3000 channels represented (check Statistics panel total)
  6. Screenshot: .sisyphus/evidence/task-4-promethion-layout.png
  
  # Also verify MinION doesn't regress:
  1. Select position X1 or X2 (GridION with MinION flow cell)
  2. Press '3' for Pore Activity
  3. Verify: 2-block layout with gap (unchanged from before)
  ```

  **Commit**: YES
  - Message: `refactor(tui): support PromethION 4-quadrant layout in channel map`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [ ] 5. Add terminal-too-small indicator

  **What to do**:
  - Detect when terminal is too small to display meaningful channel map
  - Threshold: if any quadrant would have < 1 visible row or column
  - Display message: "Terminal too small - resize for full view"
  - Show partial content with "[...more]" indicator at truncation edge
  - Graceful degradation: show what fits, indicate what's hidden

  **Must NOT do**:
  - Do not add scrolling or panning
  - Do not crash or panic on small terminals
  - Do not show empty widget area

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small feature addition with clear behavior
  - **Skills**: []
    - Standard Rust/ratatui

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (after Task 4)
  - **Blocks**: Task 7
  - **Blocked By**: Task 4

  **References**:
  - `src/tui/ui.rs:878-883` - Current early return for zero-size
  - `src/tui/ui.rs:839-851` - Placeholder text pattern to follow

  **Acceptance Criteria**:

  **TDD Tests:**
  - [ ] Test: 10×5 screen shows partial indicator
  - [ ] Test: 5×3 screen shows "Terminal too small" message
  - [ ] `cargo test --lib -- tui::ui::tests::test_small_terminal` → PASS

  **Manual Verification:**
  ```
  # Agent executes:
  1. Resize terminal to very small (e.g., 40×10)
  2. Run: ./target/release/termion
  3. Navigate to PromethION position, Pore Activity panel
  4. Verify: Partial display with indicator OR "too small" message
  5. Screenshot: .sisyphus/evidence/task-5-small-terminal.png
  ```

  **Commit**: YES
  - Message: `feat(tui): add graceful degradation for small terminals in channel map`
  - Files: `src/tui/ui.rs`
  - Pre-commit: `cargo test && cargo clippy -- -D warnings`

---

- [x] 6. Update SPEC_TUI.md documentation

  **What to do**:
  - Update channel map section to cover both MinION and PromethION layouts
  - If section 2a exists for MinION, create section 2b for PromethION, or expand 2a to cover both
  - Document:
    - PromethION: 3000 channels, 4-quadrant layout
    - Quadrant arrangement: 2×2 grid with gaps
    - Dynamic cell sizing behavior
    - Small terminal handling
  - Add ASCII art mockup of PromethION layout
  - Update MinION section if needed for consistency

  **Must NOT do**:
  - Do not document Flongle, P2, or other unsupported devices
  - Do not add implementation details (keep it spec-level)

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Documentation task
  - **Skills**: []
    - No special skills

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: None
  - **Blocked By**: None (can start immediately)

  **References**:
  - `specs/SPEC_TUI.md` - Existing TUI specification
  - Research findings in `.sisyphus/drafts/promethion-channel-map.md`

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  # Check file exists and has new content
  grep -q "PromethION" specs/SPEC_TUI.md && echo "PASS" || echo "FAIL"
  grep -q "quadrant" specs/SPEC_TUI.md && echo "PASS" || echo "FAIL"
  ```

  **Commit**: YES
  - Message: `docs(spec): add PromethION channel map layout specification`
  - Files: `specs/SPEC_TUI.md`
  - Pre-commit: None (docs only)

---

- [ ] 7. Integration testing and regression verification

  **What to do**:
  - Run full test suite: `cargo test`
  - Run clippy: `cargo clippy -- -D warnings`
  - Run fmt check: `cargo fmt --check`
  - Manual visual verification with live devices:
    - MinION/GridION: verify 2-block layout unchanged
    - PromethION: verify 4-quadrant layout correct
  - Verify channel counts match between TUI and Statistics panel
  - Test with different terminal sizes

  **Must NOT do**:
  - Do not skip MinION regression testing
  - Do not modify code in this task (verification only)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Verification task, no implementation
  - **Skills**: []
    - No special skills needed (manual terminal screenshots for visual verification)

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (final)
  - **Blocks**: None (final task)
  - **Blocked By**: Tasks 4, 5

  **References**:
  - All previous tasks
  - Live devices: P2S-03597-A (PromethION), X1-X5 (GridION)

  **Acceptance Criteria**:

  **Automated Verification:**
  ```bash
  # All tests pass
  cargo test
  # Expected: 74+ tests, 0 failures
  
  # No lint warnings
  cargo clippy -- -D warnings
  # Expected: 0 warnings
  
  # Format check
  cargo fmt --check
  # Expected: no formatting issues
  ```

  **Evidence to Capture:**
  - [ ] Test output showing all tests pass
  - [ ] Screenshot of MinION channel map (regression check)
  - [ ] Screenshot of PromethION channel map (new feature)
  - [ ] Screenshot of small terminal handling

  **Commit**: NO (verification only)

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `feat(tui): add FlowCellType enum and GridStructure types` | src/tui/ui.rs | cargo test |
| 2 | `feat(tui): implement calculate_grid_structure()` | src/tui/ui.rs | cargo test |
| 3 | `feat(tui): add dynamic cell sizing calculation` | src/tui/ui.rs | cargo test |
| 4 | `refactor(tui): support PromethION 4-quadrant layout` | src/tui/ui.rs | cargo test |
| 5 | `feat(tui): add graceful degradation for small terminals` | src/tui/ui.rs | cargo test |
| 6 | `docs(spec): add PromethION channel map specification` | specs/SPEC_TUI.md | grep check |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test
# Expected: 74+ tests, 0 failures

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
- [ ] All 74+ tests pass (including new layout tests)
- [ ] MinION rendering unchanged (regression guard)
- [ ] PromethION 4-quadrant layout displays correctly
- [ ] Dynamic sizing fills available space
- [ ] Small terminal graceful degradation works
- [ ] SPEC_TUI.md updated with PromethION documentation
