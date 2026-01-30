# Decisions - Fix Overview Throughput

## [2026-01-30] Implementation Decisions

### Decision 1: Single Commit vs Multiple Commits
**Choice**: Single comprehensive commit
**Rationale**: 
- The fix required an additional change (preserving throughput in update_stats) that was discovered during testing
- All changes are tightly coupled and part of the same bug fix
- Splitting into multiple commits would create intermediate states where the feature is broken

### Decision 2: Throttling Interval
**Choice**: Hardcoded 5 seconds
**Rationale**:
- User explicitly requested "cache throughput longer" approach
- 5 seconds balances freshness vs API call overhead
- No configuration option added (per guardrails)

### Decision 3: Preserve Throughput in update_stats
**Choice**: Check for existing values and preserve them
**Rationale**:
- get_stats() returns throughput_gbph = 0.0 by default
- Throughput is calculated separately from yield history every 5 seconds
- Without preservation, values would flicker (show briefly, then disappear)
- This pattern is cleaner than modifying get_stats() or StatsSnapshot

### Decision 4: Error Handling Strategy
**Choice**: Keep stale values on error, log at debug level
**Rationale**:
- Showing stale throughput is better than showing "--"
- Debug logging doesn't clutter user logs
- Follows existing pattern in fetch_detail_data()
