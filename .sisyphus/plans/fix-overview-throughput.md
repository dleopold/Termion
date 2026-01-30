# Fix Throughput Display in Overview Table

## TL;DR

> **Quick Summary**: Fix bug where throughput shows "--" in overview table even for active runs. Move throughput calculation from detail-view-only to main fetch loop with 5-second throttling.
> 
> **Deliverables**:
> - Throughput values visible in overview table for all active positions
> - Performance-optimized with 5-second calculation interval per position
> 
> **Estimated Effort**: Quick
> **Parallel Execution**: NO - sequential (2 dependent tasks)
> **Critical Path**: Task 1 (add tracking field) -> Task 2 (implement calculation)

---

## Context

### Original Request
Throughput metric shows correctly in the run details run info panel, but not in the overview table. Even with an active run, throughput in the table shows "--".

### Interview Summary
**Key Discussions**:
- Root cause: Throughput calculation only runs in `fetch_detail_data()` which is detail-view-only
- User preference: Cache throughput longer (calculate every ~5 seconds) to reduce gRPC calls

**Research Findings**:
- `get_stats()` returns `throughput_gbph = 0.0` by default
- `format_throughput_gbph(0.0)` returns "--" (ui.rs line 1396)
- Throughput calculation at mod.rs lines 385-397 requires `run_id` and yield history
- Refresh interval is 1000ms by default

### Metis Review
**Identified Gaps** (addressed):
- Need to track last calculation time per-position (use HashMap in App)
- Need to call `get_current_run_id()` before `get_yield_history()`
- Handle edge case: runs with < 2 yield points should show "--"
- Error handling: keep stale value on failure, don't overwrite with 0.0

---

## Work Objectives

### Core Objective
Calculate throughput for all active positions in the overview table, not just the selected detail view position.

### Concrete Deliverables
- Modified `App` struct with throughput calculation tracking
- Modified `refresh_data()` function with throttled throughput calculation
- Throughput values visible in overview table for active runs

### Definition of Done
- [x] Active runs show throughput values (e.g., "1.23 Gb/h") instead of "--" in overview table
- [x] Throughput calculation runs approximately every 5 seconds per position (not every tick)
- [x] Detail view throughput continues to work correctly (regression check)
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo test` passes

### Must Have
- Per-position time tracking for calculation throttling
- 5-second interval between calculations
- Identical calculation formula to existing detail view logic
- Graceful handling of < 2 yield points

### Must NOT Have (Guardrails)
- DO NOT modify `ui.rs` - the rendering is correct, only data population is wrong
- DO NOT modify `StatsSnapshot` struct - keep domain type clean
- DO NOT add configuration option for the interval - hardcode 5 seconds
- DO NOT change `fetch_detail_data()` behavior - detail view should continue as-is
- DO NOT calculate throughput on every tick - performance guardrail
- DO NOT add new dependencies

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **User wants tests**: Manual verification (TUI visual behavior)
- **Framework**: cargo test (existing)

### Automated Verification

Each TODO includes executable verification via:
1. **Compilation**: `cargo build` passes
2. **Linting**: `cargo clippy -- -D warnings` passes
3. **Existing tests**: `cargo test` passes
4. **Log verification**: grep patterns for calculation frequency

---

## Execution Strategy

### Dependency Flow

```
Task 1: Add tracking field to App
    |
    v
Task 2: Implement throughput calculation in refresh_data()
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2 | None |
| 2 | 1 | None | None |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents |
|------|-------|-------------------|
| 1 | 1 | delegate_task(category="quick", load_skills=[]) |
| 2 | 2 | delegate_task(category="quick", load_skills=[]) |

---

## TODOs

- [x] 1. Add throughput calculation tracking to App struct

  **What to do**:
  - Add `throughput_last_calc: HashMap<String, std::time::Instant>` field to `App` struct
  - Initialize as empty `HashMap::new()` in `App::new()`
  - Add helper method `should_calc_throughput(&self, position: &str) -> bool` that returns true if:
    - Position is not in the map, OR
    - Time since last calc >= 5 seconds
  - Add method `mark_throughput_calculated(&mut self, position: &str)` that updates the timestamp

  **Must NOT do**:
  - Do not modify any other fields in App
  - Do not add configuration for the 5-second interval

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single file modification, straightforward field addition
  - **Skills**: `[]`
    - No special skills needed for this task
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not UI work

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (Wave 1)
  - **Blocks**: Task 2
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/tui/app.rs` - App struct definition and existing HashMap fields (e.g., `stats_cache`, `run_states`)

  **Type References**:
  - `std::time::Instant` - Standard library type for time tracking
  - `std::collections::HashMap` - Already used extensively in App

  **WHY Each Reference Matters**:
  - `app.rs` shows the pattern for HashMap fields and how they're initialized in `new()`

  **Acceptance Criteria**:

  **Compilation Check**:
  ```bash
  cargo build 2>&1
  # Assert: Exit code 0, no errors
  ```

  **Linting Check**:
  ```bash
  cargo clippy -- -D warnings 2>&1
  # Assert: Exit code 0, no warnings
  ```

  **Test Check**:
  ```bash
  cargo test 2>&1
  # Assert: All existing tests pass
  ```

  **Commit**: YES
  - Message: `fix(tui): add throughput calculation tracking to App`
  - Files: `src/tui/app.rs`
  - Pre-commit: `cargo test`

---

- [x] 2. Implement throttled throughput calculation in refresh_data()

  **What to do**:
  - In `refresh_data()` function (mod.rs), after getting stats for active positions (line ~341):
    1. Check `app.should_calc_throughput(&pos.name)`
    2. If true, get `run_id` via `pos_client.get_current_run_id().await`
    3. If `run_id` exists, call `pos_client.get_yield_history(&run_id).await`
    4. If yield history has >= 2 points, calculate throughput using existing formula:
       ```rust
       let recent = &points[points.len() - 1];
       let prev = &points[points.len() - 2];
       let time_delta = (recent.seconds - prev.seconds).max(1) as f64;
       let bases_delta = recent.bases.saturating_sub(prev.bases) as f64;
       stats.throughput_bps = bases_delta / time_delta;
       stats.throughput_gbph = stats.throughput_bps * 3600.0 / 1_000_000_000.0;
       ```
    5. Call `app.mark_throughput_calculated(&pos.name)`
  - Add appropriate error handling (debug logs on failure, keep stale value)
  - DO NOT call `get_yield_history()` if we already calculated within 5 seconds

  **Must NOT do**:
  - Do not modify `fetch_detail_data()` - let it continue working independently
  - Do not calculate on every tick (only every 5 seconds)
  - Do not set `throughput_gbph = 0.0` on error (keep stale value)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single function modification with clear pattern to follow
  - **Skills**: `[]`
    - No special skills needed
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not UI work, this is data fetching logic

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (Wave 2)
  - **Blocks**: None
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/tui/mod.rs:385-408` - Existing throughput calculation in `fetch_detail_data()` (EXACT formula to copy)
  - `src/tui/mod.rs:339-346` - Current stats fetching location in `refresh_data()` (where to add new code)
  - `src/tui/mod.rs:370-383` - Pattern for getting `run_id` with `get_current_run_id()`

  **API References**:
  - `pos_client.get_current_run_id()` - Returns `Result<Option<String>, ClientError>`
  - `pos_client.get_yield_history(&run_id)` - Returns `Result<Vec<YieldPoint>, ClientError>`
  - `app.stats_cache.get_mut(&position_name)` - Get mutable reference to update throughput

  **Error Handling References**:
  - `src/tui/mod.rs:405-407` - Pattern for debug logging on yield history failure

  **WHY Each Reference Matters**:
  - Lines 385-408 contain the EXACT calculation logic to reuse - don't reinvent
  - Lines 339-346 show where the new code should be inserted (after `update_stats`)
  - Lines 370-383 show the `run_id` retrieval pattern with proper error handling

  **Acceptance Criteria**:

  **Compilation Check**:
  ```bash
  cargo build 2>&1
  # Assert: Exit code 0, no errors
  ```

  **Linting Check**:
  ```bash
  cargo clippy -- -D warnings 2>&1
  # Assert: Exit code 0, no warnings
  ```

  **Test Check**:
  ```bash
  cargo test 2>&1
  # Assert: All existing tests pass
  ```

  **Log Verification** (manual - requires running TUI with MinKNOW):
  ```bash
  # Start TUI with debug logging
  termion --log /tmp/termion.log -vv &
  TUI_PID=$!
  
  # Wait for data collection
  sleep 30
  
  # Check for yield history calls (should be ~6 calls for one position over 30s)
  grep -c "Got yield history" /tmp/termion.log
  # Assert: Count approximately (30s / 5s) = ~6 per active position
  
  # Verify timestamps are ~5 seconds apart
  grep "Got yield history" /tmp/termion.log | head -5
  # Assert: Timestamps should be ~5 seconds apart, not 1 second
  
  kill $TUI_PID
  ```

  **Evidence to Capture**:
  - [ ] Terminal output from `cargo build`
  - [ ] Terminal output from `cargo clippy`
  - [ ] Terminal output from `cargo test`

  **Commit**: YES
  - Message: `fix(tui): calculate throughput for overview table with 5s throttling`
  - Files: `src/tui/mod.rs`
  - Pre-commit: `cargo test`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `fix(tui): add throughput calculation tracking to App` | src/tui/app.rs | cargo test |
| 2 | `fix(tui): calculate throughput for overview table with 5s throttling` | src/tui/mod.rs | cargo test |

---

## Success Criteria

### Verification Commands
```bash
cargo build        # Expected: exit 0
cargo clippy -- -D warnings  # Expected: exit 0, no warnings
cargo test         # Expected: all tests pass
```

### Final Checklist
- [x] Throughput shows values (not "--") in overview table for active runs
- [x] Throughput calculation is throttled to ~5 second intervals
- [x] Detail view throughput continues to work
- [x] No clippy warnings introduced
- [x] All existing tests pass
- [x] No modification to ui.rs or StatsSnapshot
