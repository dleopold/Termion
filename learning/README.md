# Learning Documents

This folder contains educational documents generated at the end of each development phase. Each document explains the concepts, patterns, and implementation details from first principles.

## Purpose

Termion is both a product and a learning opportunity. These documents capture:
- **Concepts** — The theory behind what we built
- **Patterns** — Reusable solutions and idioms
- **Implementation** — How theory becomes code
- **Gotchas** — Pitfalls encountered and how to avoid them
- **Resources** — Further reading and references

## Document Index

| Phase | Document | Topics |
|-------|----------|--------|
| 1 | `PHASE_1_CLIENT.md` | gRPC fundamentals, tonic/prost, async Rust, protocol buffers, connection management |
| 2 | `PHASE_2_TUI.md` | TUI architecture, ratatui, immediate mode rendering, event loops, widget patterns |
| 3 | `PHASE_3_CLI.md` | CLI design, clap, output formatting, exit codes |
| 4 | `PHASE_4_TESTING.md` | Rust testing, mocking, integration tests, CI |

## Document Structure

Each phase document follows this structure:

```markdown
# Phase N — [Title]

## Overview
What we built and why.

## Concepts
### [Concept 1]
Theory, mental models, how it works.

### [Concept 2]
...

## Patterns
### [Pattern 1]
Problem → Solution → Code example → When to use.

## Implementation Walkthrough
Step-by-step explanation of key code.

## Gotchas & Lessons Learned
What tripped us up, how we solved it.

## Key Takeaways
Bullet summary of most important learnings.

## Resources
- Links to docs, articles, books
- Related projects to study
```

## How These Are Generated

At the end of each phase:
1. Review what was built
2. Identify core concepts that were applied
3. Explain from first principles (assume reader is learning)
4. Include real code examples from the project
5. Document pitfalls and solutions encountered

These are **not** API docs — they're educational material for understanding *why* and *how*, not just *what*.
