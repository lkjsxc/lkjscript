# Scratch Host

## Purpose

Define the owned Linux-first host boundary and reject framework-shaped policy
inside the VM.

## Status

**Current direction**, with safety repairs in the foundation target.

## Context

High-level Rust wrappers (`term-raw`, `tcp-listen`, …) and crates.io deps
(e.g. former `rustix`) pull behavior out of `.lkjscript` and fight the goal of
a self-owned, eventually JIT-fast stack.

## Decision

1. **No third-party Rust crates** unless an ADR explicitly allows one.
   Rust `std` / `core` remain allowed.
2. **`unsafe` only in registered mechanism files** — executable/native-runtime
   mechanisms live in `lkjscript-executable`, topology/affinity in
   `lkjscript-linux-host`, residual host/SQLite FFI in `lkjscript-sys`, and peer
   identity in `lkjscript-host`. Safe callers keep `unsafe_code = "forbid"`.
3. **Fat-opcode freeze** — do not add new high-level host features. New
   capability should land as `.lkjscript` under `src/std` or `src/lib`, or as
   thinner primitives after an ADR.
4. **Migration direction** — shrink today’s fat ops toward syscall-shaped
   primitives; reimplement policy in `.lkjscript` over time.

## Fat opcode inventory and backlog

Keep as **language/VM core** (JIT-friendly, not “OS features”):

- arithmetic / compare / logic
- `list-prepend` / `list-first` / `list-rest` / `is-empty-list`
- string ops (`string-byte-length`, `string-byte-at`, `append-string`,
  `copy-string-byte-slice`, `convert-byte-to-string`)
- `print` / control (`if`, `while`, `call`, …)

**Fat host ops to migrate later** (ranked):

1. ~~Terminal policy~~ — done in `.lkjscript`
2. ~~Sockets~~ — done in `.lkjscript`
3. ~~Filesystem~~ — done in `.lkjscript`
4. ~~Time~~ — done in `.lkjscript` (`wait-ms` / `now-ms` on `wait-milliseconds` / `current-time-milliseconds`)
5. Bulk stdout: `write-string` / `flush` **intentionally kept** as thin byte-pump intrinsics

## This sprint

Time demotion: Rust `Instant` / `thread::sleep` removed from the VM time path.
`lkjscript-sys` owns `clock_gettime(CLOCK_MONOTONIC)` and `nanosleep`.
Script wrappers live in `src/std/io/wait-ms.lkjscript` and `now-ms.lkjscript`.

Demotion backlog for OS feature opcodes is complete aside from intentional thin
`write-string` / `flush`.

## Consequences

- No further fat OS demotion required unless new fat ops appear
- Docker/Linux remains the acceptance platform
- Performance roadmap continues through semantic/outcome prerequisites,
  typed SSA, shared native code objects, baseline runtime JIT, and loop OSR;
  runtime JIT is the adaptive path

## Rejected

- Growing a Rust TUI / networking framework beside the language
- Adding crates.io deps for convenience without an ADR
- Turning `write-string` into per-byte `.lkjscript` loops


## Multi-OS note

Linux backend is current. Keep `sys-*` behind a portable façade so
future native ports do not force another language rewrite.
