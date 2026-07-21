# Current State

## Purpose

Separate observed behavior in this checkout from remaining work.

## Evidence Boundary

Docker verification is the acceptance path for claimed completion.
Local `quiet verify`, editor/http smokes, and
`docker compose -f meta/docker-compose.yml --profile verify run --rm verify`
all passed on this checkout.

## Current Implementation

- Standalone repo: `https://github.com/lkjsxc/lkjscript2026`
- Language name: **lkjscript2026**; sources use **`.lkjscript`**
- Layout: `src/std` (primitives), `src/lib/edit` (editor package), `examples/`
- Imports: `std/...`, `lib/...`, `examples/...` (package-root; no `../`)
- Scratch OS layer: `lkjscript2026-sys`; terminal + TCP + FS policy in `.lkjscript`
  (`enter-raw` / `poll-byte`; `tcp-*`; `open-*` / `path-exists` on `sys-open-*`)
- Hardcoded limit constants (no user-facing JSON limits)
- Bytecode VM; remaining fat host: time; bulk `write-str` + `flush`
- Language special `while`; bit ops for flag poking
- Terminal editor: idle without full redraw, visible cmdline, cursor clamp/hide
- Examples: `hello`, `mandel`, `texteditor`, `http`, `bench`
- Honest C comparison script: `meta/scripts/bench-compare.sh`
- Laws: [decisions/scratch-host.md](decisions/scratch-host.md),
  [operations/agent-handoff.md](operations/agent-handoff.md)

## Open Work

See [vision/performance-roadmap.md](vision/performance-roadmap.md):
thin primitives + script libs (time next), then types, GC, JIT, adaptive opts.

## Sprint Board

| Area | Status |
| --- | --- |
| Editor display / new-file | done |
| Rebrand + `.lkjscript` extension | done |
| Hardcoded limits | done |
| Minimal HTTP + bench vs C | done |
| Rust-like `src/std` + `src/lib` | done |
| Standalone GitHub repo | done |
| Editor beauty (idle/cmdline/while/flush) | done |
| Scratch host law + drop rustix | done |
| Terminal policy in `.lkjscript` | done |
| TCP sockets in `.lkjscript` (`src/std/net`) | done |
| Filesystem open/path-exists in `.lkjscript` | done |
| Move time fat ops into `.lkjscript` | backlog |
| Types / GC / adaptive JIT | roadmap only |
