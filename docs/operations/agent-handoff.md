# Agent Handoff

## Purpose

Capture product intent, locked layout, and known sharp edges so a new session
can continue without rediscovering recent pain.

## Product Intent

- Thin Rust host; grow capability in `.lkjsxc`, not in host frameworks.
- Docker and `quiet verify` are the honesty gates for claimed completion.
- AI-friendly sources: many small files, fan-out at most eight visible children.
- Package-root imports only: `std/...`, `lib/...`, `examples/...`, or `./...`.
  Paths containing `..` are rejected.

## Locked Layout

```text
src/std/          # primitives: list buffer io fs ansi term
src/lib/edit/     # only reusable lib package this sprint
examples/         # hello mandel texteditor http bench
crates/           # Rust host / compiler / VM
```

Do not invent many crates, sealed modules, or rename the product again unless
the user asks.

## Known Sharp Edges

- Prefer helpers like `maybe-load` over heavy work in a top-level `<if>`
  then-branch (can hang).
- Top-level `<do>` + `<let>` must copy proto locals into the chunk main locals
  (`compile_do`); this is already fixed — do not regress it.
- Raw TTY redraw must emit CR+LF (LF-only caused a staircase display bug).
- Missing file opens as an empty buffer with status `new file`.

## Near-Term Focus

Keep editor, HTTP demo, and bench scripts green. Types, advanced GC, baseline
JIT, and adaptive/PGO-style opts stay roadmap-only; see
[vision/performance-roadmap.md](../vision/performance-roadmap.md).
