# Summary - Fix Overview Throughput

## Completion Status: ✅ COMPLETE

**Date**: 2026-01-30
**Session**: ses_3f34faecdffeNJiK1iWOv3oF7k
**Commit**: e55bb82

---

## What Was Fixed

**Original Bug**: Throughput metric showed "--" in the overview table even for active runs, but displayed correctly in the run details panel.

**Root Cause**: Throughput calculation only ran in `fetch_detail_data()` which is called only when viewing position details, not in the main overview refresh loop.

---

## Changes Made

### 1. Added Throughput Tracking Infrastructure (`src/tui/app.rs`)
- Added `throughput_last_calc: HashMap<String, Instant>` field to App struct
- Added `should_calc_throughput(&self, position: &str) -> bool` method
- Added `mark_throughput_calculated(&mut self, position: &str)` method
- **Bonus Fix**: Modified `update_stats()` to preserve throughput values

### 2. Implemented Throttled Calculation (`src/tui/mod.rs`)
- Added throughput calculation in `refresh_data()` for all active positions
- Throttled to run every ~5 seconds per position (not every tick)
- Uses identical formula to existing detail view calculation
- Proper error handling (keeps stale values, logs at debug level)

---

## Verification Results

| Check | Result |
|-------|--------|
| cargo build | ✅ Passed |
| cargo clippy -- -D warnings | ✅ Passed (0 warnings) |
| cargo test | ✅ Passed (74/74 tests) |
| Manual testing | ✅ Throughput persists in overview |

---

## Files Modified

1. `src/tui/app.rs` - 11 lines added (tracking + preservation)
2. `src/tui/mod.rs` - 25 lines added (throttled calculation)

**Total**: 36 lines added, 1 line modified

---

## Performance Impact

- **API calls**: +1 call per active position every 5 seconds (get_yield_history)
- **Memory**: +1 HashMap with ~1-5 entries (typical device count)
- **CPU**: Negligible (simple calculation every 5s)

---

## Testing Recommendations

To verify the fix works:

```bash
# Run with debug logging
cargo run -- -vv --log /tmp/termion.log

# In another terminal, monitor logs
tail -f /tmp/termion.log | grep "throughput"
```

Expected behavior:
- Throughput values appear in overview table for active runs
- Values persist (don't flicker to "--")
- Log shows "Calculated throughput for overview" every ~5 seconds per position

---

## Lessons Learned

1. **Cache Invalidation**: When data is updated at different frequencies, preserve the slower-updating values
2. **Testing**: Always test the actual UI behavior, not just that code compiles
3. **Debugging**: User-reported flickering led to discovering the update_stats overwrite issue
