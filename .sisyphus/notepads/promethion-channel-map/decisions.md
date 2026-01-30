# Architectural Decisions: PromethION Channel Map Implementation

## Decision 1: FlowCellType Inference from Channel Count
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Infer device type (MinION, Flongle, PromethION) from channel count rather than requiring explicit device type from API.

**Rationale**:
- Channel count is reliable indicator of device type
- Avoids dependency on device type field that may not be available
- Allows graceful handling of unknown device types
- Boundary cases well-defined:
  - 0 channels → MinION (default)
  - 1-126 channels → Flongle
  - 127-512 channels → MinION
  - 513+ channels → PromethION

**Implementation**:
```rust
pub fn from_channel_count(count: usize) -> Self {
    match count {
        0 => FlowCellType::MinION,
        1..=126 => FlowCellType::Flongle,
        127..=512 => FlowCellType::MinION,
        _ => FlowCellType::PromethION,
    }
}
```

**Verification**: 5 tests covering all boundary cases pass

---

## Decision 2: BlockArrangement Enum for Layout Variants
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Use enum to represent different grid block arrangements (TwoVertical, FourQuadrant) rather than boolean flags or magic numbers.

**Rationale**:
- Type-safe representation of layout variants
- Self-documenting code (enum variant names are clear)
- Extensible for future layout types
- Prevents invalid state combinations

**Implementation**:
```rust
pub enum BlockArrangement {
    TwoVertical,   // MinION: 2 blocks stacked vertically
    FourQuadrant,  // PromethION: 4 blocks in 2×2 grid
}
```

**Verification**: All layout tests use enum variants correctly

---

## Decision 3: GridStructure as Configuration Container
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Create GridStructure type to encapsulate all grid layout information (dimensions, cell width, gap positions, block arrangement).

**Rationale**:
- Single source of truth for grid configuration
- Reduces parameter passing (one struct instead of 5+ parameters)
- Enables caching of calculated values
- Facilitates testing with fixtures

**Implementation**:
```rust
pub struct GridStructure {
    pub flow_cell_type: FlowCellType,
    pub block_arrangement: BlockArrangement,
    pub cols: usize,
    pub rows: usize,
    pub cell_width: usize,
    pub gap_positions: Vec<GapPosition>,
}
```

**Verification**: GridStructure creation and field access tested

---

## Decision 4: Separate Grid vs Display Coordinate Tracking
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Maintain separate counters for grid coordinates (actual cell positions) and display coordinates (screen positions including gaps).

**Rationale**:
- Gaps don't affect grid coordinate mapping
- Display position = grid position + gap offset
- Prevents off-by-one errors in cell rendering
- Makes gap logic explicit and testable

**Implementation**:
```rust
let mut grid_row = 0;
for display_row in 0..display_rows {
    if is_gap_row {
        // Render gap, don't increment grid_row
        continue;
    }
    // Render cell at grid_row
    grid_row += 1;
}
```

**Verification**: All grid structure tests pass; gap position tests verify correctness

---

## Decision 5: Dynamic Cell Sizing Based on Terminal Width
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Calculate cell width (1 or 2 characters) based on grid dimensions and available terminal space.

**Rationale**:
- Maximizes use of available screen space
- Gracefully degrades on narrow terminals
- Maintains readability (2-char cells preferred, 1-char fallback)
- Prevents grid from exceeding terminal width

**Algorithm**:
1. Try 2-character cells: if `grid_width * 2 + gaps ≤ terminal_width`, use 2-char
2. Fall back to 1-character cells
3. If still too wide, show truncation indicator

**Verification**: 4 tests covering MinION and PromethION at various terminal widths

---

## Decision 6: Gap Rendering Only When Scale ≤ 1.5
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Only render gap symbols when scale factor ≤ 1.5 (i.e., cells are at least 1.5 characters wide).

**Rationale**:
- Prevents gaps from dominating display on heavily downscaled grids
- Maintains visual clarity when cells are small
- Reduces visual clutter on narrow terminals
- Scale factor naturally emerges from cell width calculation

**Implementation**:
```rust
if scale <= 1.5 {
    // Render gap symbols
} else {
    // Skip gap rendering, just render cells
}
```

**Verification**: Gap rendering tests verify this behavior

---

## Decision 7: Fallback to Default Layout When Structure Unavailable
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: When GridStructure is None, fall back to default behavior: 2-char cells, no gaps, square grid.

**Rationale**:
- Maintains backward compatibility with existing code
- Graceful degradation if layout calculation fails
- Prevents crashes due to missing layout data
- Allows incremental adoption of new layout system

**Implementation**:
```rust
let cell_width = grid_structure.as_ref().map(|gs| gs.cell_width).unwrap_or(2);
let gaps = grid_structure.as_ref().map(|gs| extract_gaps(&gs)).unwrap_or_default();
```

**Verification**: All existing tests pass without modification (regression guard)

---

## Decision 8: GapPosition Enum for Type-Safe Gap Specification
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Use enum to represent gap positions (Horizontal, Vertical) with associated data.

**Rationale**:
- Type-safe representation of gap types
- Prevents invalid gap specifications
- Self-documenting code
- Enables pattern matching for gap rendering

**Implementation**:
```rust
pub enum GapPosition {
    Horizontal { after_row: usize },
    Vertical { after_col: usize },
}
```

**Verification**: Gap position tests verify correct calculation and rendering

---

## Decision 9: TDD Approach for Grid Calculations
**Status**: ✅ IMPLEMENTED & VERIFIED

**Decision**: Use Test-Driven Development (TDD) for all grid structure calculations.

**Rationale**:
- Ensures correctness before implementation
- Provides regression guard for future changes
- Documents expected behavior through tests
- Reduces debugging time

**Process**:
1. Write failing tests (RED phase)
2. Implement to pass tests (GREEN phase)
3. Refactor for clarity (REFACTOR phase)

**Verification**: 93 tests all pass; test suite provides comprehensive coverage

---

## Decision 10: Coordinate Normalization in Client Layer
**Status**: ✅ VERIFIED (NOT CHANGED)

**Decision**: Keep coordinate normalization in client layer (position.rs), not UI layer.

**Rationale**:
- Separation of concerns: client handles protocol details, UI handles rendering
- Normalization is protocol-specific (MinKNOW API detail)
- UI works with normalized coordinates directly
- Reduces complexity in rendering code

**Verification**: Traced coordinate transformation in client/position.rs; UI code works with normalized coordinates

---

## Summary

All architectural decisions are:
- ✅ Implemented
- ✅ Tested (93 tests pass)
- ✅ Verified (code quality checks pass)
- ✅ Documented (this file)

The implementation maintains backward compatibility while adding support for PromethION's complex 4-quadrant layout. All decisions prioritize clarity, testability, and maintainability.
