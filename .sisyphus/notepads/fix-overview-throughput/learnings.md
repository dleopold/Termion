# Learnings - Fix Overview Throughput

## [2026-01-30] Task Execution

### Issue Discovered During Testing
**Problem**: After implementing the throttled throughput calculation, throughput values appeared briefly then disappeared, cycling every second.

**Root Cause**: The `update_stats()` method was overwriting the entire `StatsSnapshot` including the calculated throughput values. Since `get_stats()` returns `throughput_gbph = 0.0` by default, and this runs every tick (~1s), it was overwriting the calculated values.

**Solution**: Modified `update_stats()` to preserve existing throughput values:
```rust
pub fn update_stats(&mut self, position_name: &str, mut stats: StatsSnapshot) {
    // Preserve existing throughput values (calculated separately from yield history)
    if let Some(existing) = self.stats_cache.get(position_name) {
        stats.throughput_bps = existing.throughput_bps;
        stats.throughput_gbph = existing.throughput_gbph;
    }
    self.stats_cache.insert(position_name.to_string(), stats);
}
```

### Key Insight
When data is calculated separately from the main fetch cycle (like throughput from yield history), the update method must preserve those values to avoid race conditions.

### Pattern to Remember
For cached data structures where some fields are updated at different frequencies:
- Main data: Updated every tick
- Derived/expensive data: Updated less frequently
- Solution: Preserve the derived data when updating the main data

### Files Modified
1. `src/tui/app.rs` - Added tracking field, helper methods, and fixed update_stats
2. `src/tui/mod.rs` - Added throttled throughput calculation in refresh_data

### Verification
- ✅ cargo build - passed
- ✅ cargo clippy -- -D warnings - passed (no warnings)
- ✅ cargo test - passed (74 tests)
