
## Task 4: Refactoring Notes

### Critical Points Verified

1. **Coordinate Mapping Correctness**
   - Client already normalizes sparse Y coordinates in position.rs
   - ChannelLayout.coords contains dense (normalized) coordinates
   - No additional normalization needed in rendering code
   - Verified by tracing coordinate transformation in client/position.rs lines 785-799

2. **Gap Position Calculation**
   - Gap display position: `((gap_after + 1) / scale).floor()`
   - Only shown when `scale <= 1.5` (prevents gaps dominating small grids)
   - Vertical gaps require separate grid_col tracking to maintain correct mapping

3. **MinION Regression Guard**
   - Existing behavior preserved exactly:
     - 32×16 grid with gap after row 7
     - 2-char cells when space allows
     - No vertical gaps
   - All existing tests pass without modification

### Non-Issues (Initially Suspected)

1. **Y Coordinate Normalization**
   - Initially thought UI needed to normalize sparse Y coords
   - Actually handled by client layer, not UI concern
   - Test helper creates sparse coords but specifies height=25, indicating normalization

2. **Display vs Grid Column Confusion**
   - Carefully traced through loop logic
   - Confirmed grid_col increments correctly only for non-gap columns
   - Gap columns shift display position but don't affect grid coordinate mapping

