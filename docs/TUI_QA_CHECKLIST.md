# TUI Visual QA Checklist

Manual testing checklist for the Termion TUI. Run through this checklist before releases.

## Prerequisites

- MinKNOW running with at least one device (simulated is fine)
- Terminal with 256-color support
- Terminal size at least 80x24

## Overview Screen

### Layout
- [ ] Header shows "Termion" title
- [ ] Connection status indicator visible in header
- [ ] Position table renders with aligned columns
- [ ] Footer shows keybinding hints
- [ ] No visual artifacts or misaligned text

### Position Table
- [ ] All positions listed
- [ ] Columns: Name, State, Reads, Throughput
- [ ] Numbers formatted with K/M/G suffixes
- [ ] Simulated positions marked appropriately

### Status Indicators
- [ ] Running: Green filled circle (●)
- [ ] Paused: Yellow filled circle (●)
- [ ] Idle: Gray hollow circle (○)
- [ ] Error: Red X (✖)

### Selection
- [ ] Current selection highlighted
- [ ] Arrow keys move selection up/down
- [ ] Selection wraps at top/bottom
- [ ] Selection visible even with many positions

### Mini Sparklines
- [ ] Sparklines render next to throughput
- [ ] Uses block characters (▁▂▃▄▅▆▇█)
- [ ] Updates as new data arrives

## Position Detail Screen

### Navigation
- [ ] Enter opens detail for selected position
- [ ] Esc returns to overview
- [ ] Correct position shown after navigation

### Header
- [ ] Position name displayed
- [ ] Run state shown
- [ ] Run ID shown (if active)

### Throughput Chart
- [ ] Chart renders without artifacts
- [ ] X-axis shows time progression
- [ ] Y-axis shows throughput scale
- [ ] Line updates in real-time
- [ ] Axis labels readable

### Stats Grid
- [ ] Reads count displayed
- [ ] Bases passed/failed shown
- [ ] Throughput (Gb/h) displayed
- [ ] Mean quality shown
- [ ] All numbers formatted appropriately

## Help Overlay

- [ ] Press ? opens help overlay
- [ ] Press ? or Esc closes help
- [ ] All keybindings listed
- [ ] Overlay renders on top of content
- [ ] Content behind is dimmed/visible

## Connection States

### Connected
- [ ] Green indicator shown
- [ ] Data updates regularly
- [ ] No error messages

### Disconnected
- [ ] Banner appears at top
- [ ] Shows "Disconnected" with reason
- [ ] Last known data still visible
- [ ] Data appears dimmed/grayed

### Reconnecting
- [ ] Shows reconnection attempt counter
- [ ] Updates attempt number
- [ ] Recovers gracefully on reconnect
- [ ] Data refreshes after reconnect

## Edge Cases

### Empty States
- [ ] No devices: Shows appropriate message
- [ ] No active runs: Shows idle state
- [ ] No stats available: Graceful handling

### Terminal Handling
- [ ] Resize: Layout adjusts correctly
- [ ] Small terminal: Degrades gracefully
- [ ] Wide terminal: Uses extra space well

### Long Content
- [ ] Long position names: Truncated with ellipsis
- [ ] Long run IDs: Handled appropriately
- [ ] Many positions (10+): Scrolling works

## Keyboard Navigation

- [ ] `q` - Quits application
- [ ] `↑`/`↓` - Move selection
- [ ] `Enter` - Open detail
- [ ] `Esc` - Back/close overlay
- [ ] `?` - Toggle help
- [ ] No unexpected key behaviors

## Exit Behavior

- [ ] Clean exit on `q`
- [ ] Terminal restored to normal mode
- [ ] Cursor visible after exit
- [ ] No artifacts left on screen

## Performance

- [ ] Smooth rendering (no flicker)
- [ ] Responsive to keypresses
- [ ] Chart updates without lag
- [ ] No memory growth over time

---

## Test Environment Notes

Date tested: _______________
Terminal: _______________
OS: _______________
MinKNOW version: _______________
Tester: _______________

## Issues Found

| Issue | Severity | Screen | Notes |
|-------|----------|--------|-------|
|       |          |        |       |
|       |          |        |       |
