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

1. ~~Terminal policy~~ — done in `.lkjscript` (`enter-raw` / `leave-raw` / `poll-byte`)
2. ~~Sockets~~ — done in `.lkjscript` (`tcp-*` on `sys-socket` / `sys-bind` / …)
3. ~~Filesystem~~ — done in `.lkjscript` (`open-*` / `path-exists` on `sys-open-*`)
4. Time: `wait-ms`, `now-ms` → thin clock/sleep primitives + script
5. Bulk stdout: `write-str` / `flush` may stay as thin byte-pump intrinsics

## This sprint

Filesystem demotion: fat `open-read` / `open-write` / `path-exists` and VM
`std::fs` paths are gone. Policy lives in `src/std/fs` on thin
`sys-open-read` / `sys-open-write` / `sys-path-exists` over
`lkjscript2026-sys` (`OwnedFd`, open/read/write/access). Byte IO stays as
thin `read-byte-fd` / `write-byte-fd` / `close`. Line helpers live under
`src/std/fs/text/` to keep fan-out ≤8.

Earlier: socket demotion (`src/std/net`); terminal demotion (`src/std/term`).

## Consequences

- Next demotion target is time (`wait-ms` / `now-ms`)
- Docker/Linux is the acceptance platform; other OS support is deferred
- Performance roadmap (types → GC → JIT → adaptive) stays compatible: specialize
  hot thin primitives, not opaque frameworks

## Rejected

- Growing a Rust TUI / networking framework beside the language
- Adding crates.io deps for convenience without an ADR
