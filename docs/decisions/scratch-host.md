# Scratch host

## Context

High-level Rust wrappers (`term-raw`, `tcp-listen`, …) and crates.io deps
(e.g. former `rustix`) pull behavior out of `.lkjscript` and fight the goal of
a self-owned, eventually JIT-fast stack.

## Decision

1. **No third-party Rust crates** unless an ADR explicitly allows one.
   Rust `std` / `core` remain allowed.
2. **`unsafe` only in `lkjscript2026-sys`** — owned Linux-first syscall / libc
   extern wrappers. All other crates keep `unsafe_code = "forbid"`.
3. **Fat-opcode freeze** — do not add new high-level host features. New
   capability should land as `.lkjscript` under `src/std` or `src/lib`, or as
   thinner primitives after an ADR.
4. **Migration direction** — shrink today’s fat ops toward syscall-shaped
   primitives; reimplement policy in `.lkjscript` over time.

## Fat opcode inventory and backlog

Keep as **language/VM core** (JIT-friendly, not “OS features”):

- arithmetic / compare / logic
- `cons` / `car` / `cdr` / `null?`
- string ops (`str-len`, `str-ref`, `str-append`, `str-slice`, `str-from-byte`)
- `print` / control (`if`, `while`, `call`, …)

**Fat host ops to migrate later** (ranked):

1. ~~Terminal policy~~ — done in `.lkjscript`
2. ~~Sockets~~ — done in `.lkjscript`
3. ~~Filesystem~~ — done in `.lkjscript`
4. ~~Time~~ — done in `.lkjscript` (`wait-ms` / `now-ms` on `sys-wait-ms` / `sys-now-ms`)
5. Bulk stdout: `write-str` / `flush` **intentionally kept** as thin byte-pump intrinsics

## This sprint

Time demotion: Rust `Instant` / `thread::sleep` removed from the VM time path.
`lkjscript2026-sys` owns `clock_gettime(CLOCK_MONOTONIC)` and `nanosleep`.
Script wrappers live in `src/std/io/wait-ms.lkjscript` and `now-ms.lkjscript`.

Demotion backlog for OS feature opcodes is complete aside from intentional thin
`write-str` / `flush`.

## Consequences

- No further fat OS demotion required unless new fat ops appear
- Docker/Linux remains the acceptance platform
- Performance roadmap (types → GC → JIT → adaptive) stays the next major arc

## Rejected

- Growing a Rust TUI / networking framework beside the language
- Adding crates.io deps for convenience without an ADR
- Turning `write-str` into per-byte `.lkjscript` loops
