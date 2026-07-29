# Narrow Unsafe Mechanism Crates

## Purpose

Define Current ownership of executable/native-reference and Linux host
mechanisms without compatibility aliases.

## Status

**Current architecture decision.** The repository decomposition is Current.
Linux x86-64 execution remains Current; non-Linux paths remain unexecuted and do
not gain a portability claim.

## Decision

Two private crates own narrow reviewed mechanisms:

- `lkjscript-executable` owns the complete implementation formerly under
  `crates/lkjscript-sys/src/executable/` and its executable and native-reference
  tests. It depends only on `lkjscript-native`. `lkjscript-jit` consumes its
  safe root facade directly.
- `lkjscript-linux-host` owns the complete implementation formerly under
  `crates/lkjscript-sys/src/linux_host/` and the Linux host, binder, and bounds
  tests. It depends only on `lkjscript-contracts` and `lkjscript-resource`.
  `lkjscript-app` consumes its safe root facade directly.

Residual `lkjscript-sys` owns only checked host file-descriptor, file, path,
poll, random, socket, time, terminal, and SQLite FFI mechanisms. The app has no
residual dependency or capsule authorization for it. The VM retains its real
host I/O and SQLite dependency.

No former `lkjscript-sys` executable or Linux-host module is re-exported. No
crate alias, compatibility dependency, feature, or fallback preserves either
removed path.

## Unsafe Boundaries

Boundary IDs and reviewed safe caller contracts remain unchanged. Registry file
paths move as follows:

- `linux-executable-entry`, `linux-executable-memory`, and
  `native-runtime-bridge` point into `lkjscript-executable`;
- the Linux affinity ABI member of `linux-host-io` points into
  `lkjscript-linux-host`; and
- residual host I/O and `sqlite-ffi` remain in `lkjscript-sys`.

`lkjscript-linux-host` obtains errno through `std::io::Error::last_os_error`, so
it does not depend on the residual descriptor implementation. Unsafe remains
allowed only in mechanism crates whose files are exact registry members; safe
callers continue to forbid unsafe Rust.

## Dependency Direction

```text
lkjscript-jit -> lkjscript-executable -> lkjscript-native
lkjscript-app -> lkjscript-linux-host -> lkjscript-contracts + lkjscript-resource
lkjscript-vm -> lkjscript-sys
```

The repository retains `crates/capsule.json` as the workspace collection
authority. The nested `crates/mechanisms/capsule.json` owns the two mechanism
capsules, so `crates/` has exactly 16 direct tracked entries without flattening
or deleting its collection contract.

These edges change repository ownership only. They do not alter source,
compiler, process registry, control database, session, GUI, runtime outcome, or
native image contracts.

## Acceptance

Acceptance requires moved rather than copied implementations and tests, no old
module paths, exact capsule and Cargo graphs, updated lock/provenance identity,
exact unsafe registration, formatting, focused tests, strict affected Clippy,
and repository documentation/source/tree/structure gates. Linux x86-64 is the
executed acceptance platform. Non-Linux execution is explicitly untested.
