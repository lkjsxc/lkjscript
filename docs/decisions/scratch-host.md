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
3. Filesystem helpers: `open-read`, `open-write`, `path-exists`, fd byte IO policy
4. Time: `wait-ms`, `now-ms` → thin clock/sleep primitives + script
5. Bulk stdout: `write-str` / `flush` may stay as thin byte-pump intrinsics

## This sprint

Socket demotion: fat `tcp-listen` / `tcp-accept` / `tcp-recv` / `tcp-send`
opcodes and `std::net` paths are gone. Policy lives in `src/std/net` on thin
`sys-socket` / `sys-bind` / `sys-listen` / `sys-accept` / `sys-recv` /
`sys-send` primitives over `lkjscript2026-sys` Linux socket wrappers. Scripts
still use fd-table indices (not raw OS fds).

Earlier: terminal demotion — `term-raw` / `term-cooked` / `poll-byte` removed;
policy in `src/std/term` and `src/std/io` on `buf-*` / `sys-ioctl` / `sys-poll`.
Exit still restores a guarded termios blob via `lkjscript2026-sys`.

## Consequences

- Next demotion target is filesystem (`open-read` / `std::fs`)
- Docker/Linux is the acceptance platform; other OS support is deferred
- Performance roadmap (types → GC → JIT → adaptive) stays compatible: specialize
  hot thin primitives, not opaque frameworks

## Rejected

- Growing a Rust TUI / networking framework beside the language
- Adding crates.io deps for convenience without an ADR
