# Virtual Machine

## Purpose

Describe the current execution engine, host-resource boundary, and accepted
next repairs.

## Status
<!-- LKJ-F typed-vm-scalars current C1VP7M79Zi03E75dMS2NE11LnOZ2WF6kFxUMBAjaEzg -->

**Current** unless a section is labeled **Accepted Target** or **Deferred**.

## Current Shape

- Dense bytecode with contiguous value and frame stacks.
- Safe closed 16-byte `Value` records with a 64-bit payload and explicit
  metadata for private invalid storage, unit, bool, complete i64, exact-bit f64,
  typed empty lists, capabilities, resources, opaque unique keys, structural
  roots/views/destinations, typed segmented-list handles, and invocation-scoped
  region-product keys.
- Products, enums, strings, paths, and results use deterministic structural
  stores. Lists use segmented invocation regions. Acyclic products closed over
  selected copy lists, scalar leaves, and region products use a bounded
  invocation-owned typed record arena.
- `i64`-preserving constants, checked `i64` arithmetic, IEEE `f64` arithmetic,
  exact value/structural/`f64`-bit equality, segmented structural list equality
  bounded to 1,000,000 entries, flat copy-product construction/projection/update,
  region-product construction/projection/immutable update and exact calls, exact
  enum construction/test/active projection primitives, and checked narrow
  host domains.
- Exact scalar parameter/return metadata seeds validator kinds across builds and calls.
  Direct-call type-variable metadata binds copy arguments and returns to one
  representation; mixed repeats reject. It is not a hidden witness ABI.
- Return-adjacent frame reuse for tail recursion.
- Synchronous, single-threaded execution with explicit fuel, stack/frame,
  runtime-storage bytes, allocation, logical aggregate construction, handle-slot,
  output, and wall-time budgets.
- One validated program and fresh VM execution scope per invocation. The
  standalone CLI hosts that scope in its process; daemon-supervised applications
  use isolated cells and app-private providers.
- VM core never terminates its host process. Console and terminal authority is
  explicit at the host adapter boundary.

Mutable `Chunk` remains the compiler/test builder. The only executable boundary
is opaque immutable `ValidatedChunk`, created by `validate_chunk`; public VM,
disassembly, and runtime-tiering APIs require it. The validator decodes all code
before effects and checks encoding/table/metadata limits, operands and indexes,
function/main/local/global shape, zero captures, products, enum identities,
layouts, tags, variants, fields, constructor substitutions, jump boundaries, CFG
stack joins, definite local initialization, return/fallthrough shape, and
statically known operation categories. Compiler output uses this same path.

Execution returns `Returned(OwnedValue)`, `Exited`, `Trapped`,
`DeadlineExceeded`, `ResourceLimitExceeded`, or `HostFailure`. Validation errors
are separate. `OwnedValue` retains only key-free structural images, canonical
owned-list tables, unique bytes, symbols, and scalars; no invocation key escapes
its arena lifetime. Resources are dropped, terminal restoration is attempted,
and stdout is flushed before the CLI translates the outcome.

## Current Resource Boundary

- Integer values are never accepted as resources.
- Standard input has the exact borrowed kind `input-stream` and a reserved token
  disjoint from owned slots.
- Owned tokens combine a reusable slot with a nonzero generation. Close
  invalidates the generation; stale tokens reject even after the slot is reused.
  Exact file mode, listener/stream, connection/statement, and directory kinds
  are checked before host access.
- All raw descriptor resolution is centralized in the VM resource table.
- Close rejects borrowed, unknown, stale, wrong-kind, and repeatedly closed
  tokens. File readers cannot write, sync, or truncate; listener and stream
  operations are disjoint.
- Arbitrary ioctl is absent. `get-terminal-state` and `set-terminal-state`
  select fixed Linux requests and validate exactly 60 state bytes before FFI.
- Every fallible host primitive returns a language `result`.
- Public operations use canonical domain words. `sys-*` labels are internal
  stable/runtime details, not accepted source names.
- Missing paths return `Ok(false)` only for absence-class errors; other
  `access(2)` failures return errors.
- Network send returns its real byte count and uses `MSG_NOSIGNAL`.

## Current Host Risks

- Core console IO and terminal-restoration failures are `HostFailure`, while
  ordinary fallible host-operation errors remain language results.
- The standalone terminal guard and direct stdin/stdout adapter remain
  process-global. Daemon-supervised process cells isolate those host effects;
  in-process concurrent terminal leases are not Current.
- Strings and network/file bytes do not provide a complete lossless byte model.
- The default deadline is cooperative. Poll, wait, stdin/handle read,
  accept, and receive use remaining time. Filesystem, console-write, send/write,
  and cleanup wrappers are not cancellable and can overrun cooperative mode.
  `require_hard_deadline` rejects known unsupported operations before effects
  with `HostFailure`; it does not claim cancellable cleanup.
- Runtime-storage byte accounting estimates owned capacities and is checked at
  instruction boundaries; one bounded allocation can transiently occur before
  the aggregate check. `print` currently constructs its temporary host
  formatting string before applying the output-byte limit.

The numeric representation and behavior are specified by
[Exact I64 And F64 Semantics](../decisions/semantics/numeric-semantics.md).

## Current Baseline Tier Boundary

Typed SSA and callable native execution consume the compiler's same verified
semantic program and structured outcome/resource configuration. The Linux
x86-64 scalar subset transfers complete i64 values and exact f64 bits directly
at VM entries, calls, and returns without scalar heap allocation. Forced mode
compiles before main effects and never falls back. Auto observes bounded
function entries and uses installed code only on later calls. The complete
subset and unsupported reference/allocation/host boundary are in
[Callable Baseline JIT](baseline-jit.md).

Native `Poll` applies cooperative deadline and native-poll fuel limits. VM
instruction fuel and native poll fuel are separately placed bounded measures;
they preserve the same structured resource category but do not claim identical
instruction-by-instruction exhaustion points.

Complete host-service injection, instruction quanta, fully cancellable
filesystem and output services, in-process terminal leases, loop OSR, and broader invocation-region witnesses
remain **Deferred** to
later measured cycles.
