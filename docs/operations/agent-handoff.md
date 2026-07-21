# Agent Handoff

## Purpose

Capture product intent, locked layout, and known sharp edges so a new session
can continue without rediscovering recent pain.

## Product Intent

- Thin, scratch Rust host; grow capability in `.lkjscript`, not frameworks.
- No new crates.io dependencies without an ADR (`lkjscript2026-sys` owns OS
  wrappers; `unsafe` only there).
- Fat host opcodes are frozen; prefer script libraries for new features.
- Docker and `quiet verify` are the honesty gates for claimed completion.
- AI-friendly sources: many small files, fan-out at most eight visible children.
- Package-root imports only: `std/...`, `lib/...`, `examples/...`, or `./...`.
  Paths containing `..` are rejected.

## Locked Layout

```text
src/std/          # primitives: list buffer io fs ansi term net
src/lib/edit/     # only reusable lib package this sprint
examples/         # hello mandel texteditor http bench
crates/           # Rust host / compiler / VM / sys
```

Do not invent many application crates, sealed modules, or rename the product
again unless the user asks.

## Known Sharp Edges

- Prefer helpers like `maybe-load` over heavy work in a top-level `<if>`
  then-branch (can hang).
- Top-level `<do>` + `<let>` must copy proto locals into the chunk main locals
  (`compile_do`); this is already fixed — do not regress it.
- Raw TTY redraw must emit CR+LF (LF-only caused a staircase display bug).
- Missing file opens as an empty buffer with status `new file`.
- Editor idle must not full-redraw; use `while` + `wait-ms` without paint.
- Command mode must paint `ed-cmd` and CUP onto the cmdline row.
- Flush after final CUP; hide cursor during clear/paint.
- Prefer host `write-str` for bulk TTY output (not per-byte loops).
- Terminal raw/poll are script libraries; do not reintroduce `term-raw` opcodes.
- TCP listen/accept/recv/send are script libraries (`std/net`); do not
  reintroduce fat `tcp-*` opcodes or `std::net` in the VM.
- File open/path-exists are script libraries (`std/fs`); do not reintroduce
  fat `open-read` / `open-write` / `path-exists` or VM `std::fs`.

## Near-Term Focus

Keep editor/HTTP/bench green while migrating toward thinner sys primitives and
`.lkjscript` libraries. Types / GC / JIT stay roadmap-only; see
[vision/performance-roadmap.md](../vision/performance-roadmap.md) and
[decisions/scratch-host.md](../decisions/scratch-host.md).
