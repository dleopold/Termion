# Task 2: calculate_grid_structure() Implementation

## TDD Approach
- Created test helpers `create_minion_layout()` and `create_promethion_layout()` first
- Wrote 4 failing tests before implementation (RED phase)
- Implemented function to pass all tests (GREEN phase)
- Tests passed on first try after implementation

## Grid Structure Calculations
- MinION: 32×16 grid, TwoVertical blocks, gap after row 7
- PromethION: 126×25 grid, FourQuadrant blocks, gaps at col 62 and row 11
- Cell width determined by `calculate_cell_dimensions()` helper (from Task 3)

## Test Data Fixtures
- MinION: Sequential coordinate mapping (0,0) to (31,15)
- PromethION: Sparse Y coordinates every 4 units (0, 4, 8, ..., 96)
- Both fixtures critical for testing grid normalization logic

## Code Quality
- Added `#[allow(dead_code)]` since function not yet called in rendering (future task)
- Kept function private (internal helper)
- Minimal documentation per AGENTS.md guidelines

## Task 4: Refactored render_pore_grid_from_states()

### Key Implementation Decisions

1. **Grid Column Tracking for Vertical Gaps**
   - Added `grid_col` variable (similar to existing `grid_row`)
   - Only increment when rendering actual cells, not gaps
   - Ensures correct coordinate mapping: display columns → grid coordinates
   - Example: cols 0-62, [gap], 63-125 map to display positions 0-62, 63(gap), 64-126

2. **Gap Detection Logic**
   - Horizontal gaps: check at start of each row, skip entire row if gap
   - Vertical gaps: check within column loop, render gap symbol and continue
   - Preserve `scale <= 1.5` guard from original (only show gaps when not heavily downscaled)

3. **Coordinate Normalization**
   - Already handled by client code in `src/client/position.rs` (lines 785-799)
   - Sparse physical coordinates (e.g., Y=0,12,24,36,48) mapped to dense indices (0-24)
   - Rendering code works with normalized coordinates directly
   - No additional normalization needed in UI layer

4. **Fallback Behavior Preservation**
   - When no layout available: grid_structure = None
   - Falls back to: cell_width=2, no gaps, square grid
   - Maintains backward compatibility with existing behavior

5. **Display Dimensions Include Gaps**
   - `total_display_width = grid_width + num_vertical_gaps`
   - `total_display_height = grid_height + num_horizontal_gaps`
   - Scale calculation accounts for total display size including gaps
   - Ensures correct fitting to screen

### Code Patterns

**Pattern**: Track separate grid coordinate counters for rows/columns when gaps are involved
```rust
let mut grid_row = 0;
for display_row in 0..display_rows {
    if is_gap_row {
        continue;  // Don't increment grid_row
    }
    // render row at grid_row
    grid_row += 1;
}
```

**Pattern**: Extract configuration from Option<GridStructure> with fallbacks
```rust
let cell_width = grid_structure.as_ref().map(|gs| gs.cell_width).unwrap_or(2);
let gaps = grid_structure.as_ref().map(|gs| extract_gaps(&gs)).unwrap_or_default();
```

### Testing Strategy

- Unit tests for grid structure calculation already existed
- Integration verified through:
  1. All existing tests pass (regression guard for MinION)
  2. Compilation with no warnings
  3. Manual testing required for visual verification on live PromethION

### Potential Future Improvements

- Consider extracting gap rendering logic into helper functions
- Could optimize gap position lookups (currently O(n) per cell) with HashSet
- Dynamic gap width based on screen space available

