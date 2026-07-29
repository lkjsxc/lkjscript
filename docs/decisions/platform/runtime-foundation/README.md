# Runtime Foundation Decision Capsule

## Purpose

Bound the accepted runtime-foundation direction without promoting research or
this cycle's experiments to Current behavior.

## Status

**Accepted Contract.** The inherited implementation remains Current exactly as
listed below. Portable cells, runtime nodes, and the transactional kernel are
accepted architectural contracts. Only the explicitly named probes are
experimental this cycle.

## Current Inherited State

- Linux x86-64 is the only acceptance platform.
- The compiler, evaluator, reference bytecode VM, baseline JIT, and proof JIT
  consume one resolved typed IR family.
- Current execution, tracing-memory limits, resource support, and native-tier
  evidence remain those in [Current State](../../../current-state.md).
- `LKJ-UNSAFE-BOUNDARY` enforces an exact bounded registry for the inherited
  unsafe-containing sys files; no unsafe code moved in this slice.
- No runtime node, portable Wasm cell, Component Model ABI, transactional
  kernel, Cranelift backend, LeanStore integration, or DuckDB vector engine is
  Current.

## Accepted Contracts

1. Separate semantic target identity from host execution policy.
2. Compose bounded cells under a supervised runtime node; do not introduce
   ambient authority or an independent language semantics.
3. Put durable mutable runtime metadata behind one deterministic transactional
   kernel with an ordered key-value foundation and explicit upper layers.
4. Share only immutable, content-addressed runtime images across cells.
5. Keep the Component Model at the external ABI boundary.
6. Require every third-party implementation candidate to pass the repository's
   dependency, correctness, resource, and measurement gates before adoption.

## This Cycle's Experimental Slices

- implement and test exact lifecycle transitions for capability-free private VMs;
- implement a durable single-writer ordered-key kernel and deterministic faults;
- execute its fake-storage commit/reopen probe on `wasm32-wasip1`;
- share immutable validated chunks while retaining per-app mutable state; and
- retain Wasmtime, Cranelift, LeanStore, and vector engines as reviewed inputs
  with no production dependency.

Experiments produce retained evidence, not compatibility promises. Failed
candidates are removed rather than retained as fallback paths.

## Capsule Map

- [Portable Host and Targets](portable-host-and-targets.md)
- [Runtime Node](runtime-node.md)
- [Transactional Kernel](transactional-kernel.md)

## Metadata Claim Rejected as Evidence

The directive's claimed "WASI 0.3, released June 11 2026" metadata is
**unverified and future-dated** relative to this decision baseline. It cannot be
Current evidence or an adoption premise. Any later use requires a published
upstream release record, pinned interface contract, and fresh acceptance
results.
