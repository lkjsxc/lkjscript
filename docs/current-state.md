# Current State

## Purpose

Separate observed behavior in this checkout from remaining work.

## Evidence Boundary

Docker verification is the acceptance path for claimed completion.

## Current Implementation

- Standalone repo: `https://github.com/lkjsxc/lkjscript2026`
- Language name: **lkjscript2026**; sources use **`.lkjsxc`**
- Layout: `src/std` (primitives), `src/lib/edit` (editor package), `examples/`
- Imports: `std/...`, `lib/...`, `examples/...` (package-root; no `../`)
- Hardcoded limit constants (no user-facing JSON limits)
- Bytecode VM; host IO, FD, TCP, wait, TTY raw/poll
- Terminal editor under `src/lib/edit` with CRLF redraw and new-file open
- Examples: `hello`, `mandel`, `texteditor`, `http`, `bench`
- Honest C comparison script: `meta/scripts/bench-compare.sh`
- Session intent and pitfalls: [operations/agent-handoff.md](operations/agent-handoff.md)

## Open Work

See [vision/performance-roadmap.md](vision/performance-roadmap.md):
static types, advanced GC, baseline JIT, adaptive PGO-style opts.

## Sprint Board

| Area | Status |
| --- | --- |
| Editor display / new-file | done |
| Rebrand + `.lkjsxc` | done |
| Hardcoded limits | done |
| Minimal HTTP + bench vs C | done |
| Rust-like `src/std` + `src/lib` | done |
| Standalone GitHub repo | done |
| Types / GC / adaptive JIT | roadmap only |
