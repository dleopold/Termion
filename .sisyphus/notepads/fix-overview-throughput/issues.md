# Issues - Fix Overview Throughput

## [2026-01-30] Issues Encountered

### Issue 1: Throughput Values Flickering
**Symptom**: Throughput appeared briefly then reverted to "--", cycling every second

**Root Cause**: 
- refresh_data() calls get_stats() every tick (~1s)
- get_stats() returns fresh StatsSnapshot with throughput_gbph = 0.0
- update_stats() was overwriting the entire cache entry
- Calculated throughput values (updated every 5s) were being overwritten

**Resolution**: Modified update_stats() to preserve existing throughput values

**Time to Resolve**: ~10 minutes (quick fix once identified)

### Issue 2: Cargo Not in PATH
**Symptom**: `zsh: command not found: cargo`

**Root Cause**: Rust environment not sourced in shell

**Resolution**: 
```bash
source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"
```

**Prevention**: Added to shell rc file for persistence

### Issue 3: Missing protoc Dependency
**Symptom**: Build failed with "Could not find `protoc`"

**Root Cause**: protobuf compiler not installed

**Resolution**: Downloaded and installed protoc from GitHub releases to ~/.local/bin

**Time to Resolve**: ~5 minutes
