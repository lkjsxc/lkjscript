# Runtime Foundation Decision Capsule

## Purpose

Bound the accepted runtime-foundation direction without promoting research or
this cycle's experiments to Current behavior.

## Status

**Accepted Contract.** The inherited implementation remains Current exactly as
listed below. The OS-resident daemon authority, subordinate cells, portable host
providers, and transactional kernel are accepted architectural contracts. Only
the explicitly named probes are experimental.

## Current Inherited State

- Linux x86-64 is the only acceptance platform.
- The compiler, evaluator, reference bytecode VM, baseline JIT, and proof JIT
  consume one resolved typed IR family.
- Current execution, tracing-memory limits, resource support, and native-tier
  evidence remain those in [Current State](../../../current-state.md).
- `LKJ-UNSAFE-BOUNDARY` enforces an exact bounded registry for the inherited
  unsafe-containing sys files; no unsafe code moved in this slice.
- No persistent coordinator, local control transport, process cell, session
  broker, GUI cell, Component Model ABI, transactional service, Cranelift
  backend, LeanStore integration, or DuckDB vector engine is Current.

## Accepted Contracts

1. Separate semantic target identity from host execution policy.
2. Use one `lkjscriptd` authority and bounded subordinate application cells;
   never introduce ambient authority or independent language semantics.
3. Keep bootstrap control metadata in a dedicated journal independent of the
   application database.
4. Layer the application database service over the ordered transactional
   kernel with exact tenant and transaction ownership.
5. Share only immutable, content-addressed runtime images across cells.
6. Keep Wasm and the Component Model at later external boundaries.
7. Require every third-party implementation candidate to pass the repository's
   dependency, correctness, resource, and measurement gates before adoption.

## This Cycle's Experimental Slices

- the inherited capability-free private VM lifecycle and immutable chunk lease;
- the inherited durable single-writer ordered kernel and deterministic faults;
- the inherited retained `wasm32-wasip1` fake-storage probe;
- daemon control, process-cell, tenant, and interactive-cell vertical slices as
  they gain named execution evidence; and
- retained Wasmtime, Cranelift, LeanStore, and vector research with no production
  dependency.

Experiments produce retained evidence, not compatibility promises. Failed
candidates are removed rather than retained as fallback paths.

## Capsule Map

- [Global Platform Revision](platform-revision.md)
- [Portable Host and Targets](portable-host-and-targets.md)
- [OS-Resident Runtime System](os-resident-runtime-system.md)
- [Application Cells And Host Providers](application-cells-and-host-providers.md)
- [Isolated Process Cells](isolated-process-cells.md)
- [Persistent Application Registry And Control](persistent-application-control.md)
- [Machine Coordinator And Local Control](machine-coordinator-daemon.md)
- [Transactional Kernel](transactional-kernel.md)

## Metadata Claim Rejected as Evidence

The directive's "WASI 0.3, released June 11 2026" metadata was not
corroborated by the official WASI pages inspected for this cycle. It cannot be
Current evidence or an adoption premise without a published upstream release
record, pinned interface contract, and fresh acceptance results.
