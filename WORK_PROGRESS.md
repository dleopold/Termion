# WORK_PROGRESS.md
Session Progress Log — Termion

This document tracks progress across development sessions. Update after each session.

---

## Current State

**Phase:** 1 — Core Client Library  
**Status:** Not started  
**Blockers:** None

### Ready to Start
- [ ] Initialize Cargo workspace
- [ ] Vendor MinKNOW protobufs from `nanoporetech/minknow_api`
- [ ] Implement basic connection to manager service
- [ ] Build mock gRPC server for testing

### Development Environment
- MinKNOW: Running (localhost:9501)
- Simulated device: MS00001 (IDLE)
- Bulk file: Configured (`/minknow/test-data/...NA12878...fast5`)
- Verify with: `make sim-status`

---

## Session Log

### 2026-01-20 — Project Setup

**Completed:**
- Finalized all design decisions (D0-D7)
- Created detailed specifications:
  - SPEC_CLIENT.md — gRPC client
  - SPEC_TUI.md — TUI screens and design
  - SPEC_CLI.md — CLI commands
  - SPEC_CONFIG.md — Configuration
  - SPEC_TESTING.md — Testing strategy
- Created AGENTS.md with comprehensive coding guidelines
- Set up development environment:
  - dev/scripts/sim_manager.py — Simulation management
  - Makefile with sim-* targets
  - Created simulated MinION (MS00001)
  - Configured bulk file playback
- Updated WORK_PLAN.md, ARCHITECTURE_OVERVIEW.md

**Decisions Made:**
- Project name: `termion`
- Single binary, TUI-first
- Design references: scope-tui, bottom, trippy
- Hybrid design process: text mockups → functional iteration

**Notes:**
- Phase 0 complete
- Ready for Phase 1 scaffolding

---

## Open Questions

None currently.

---

## Blockers

None currently.

---

## Quick Reference

```bash
# Verify dev environment
make sim-status

# Start fresh simulation
make sim-remove && make sim-create

# With bulk file
make sim-setup BULK_FILE=/minknow/test-data/GXB02001_20230509_1250_FAW79338_X3_sequencing_run_NA12878_B1_19382aa5_ef4362cd.fast5
```

**Key docs:**
- What to build: `WORK_PLAN.md`
- How to build: `specs/SPEC_*.md`  
- Why we chose: `DECISIONS.md`
- Code standards: `AGENTS.md`
