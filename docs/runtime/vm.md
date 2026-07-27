# Virtual Machine

## Purpose

Describe the current execution engine, host-resource boundary, and accepted
next repairs.

## Status

**Current** unless a section is labeled **Accepted Target** or **Deferred**.

## Current Shape

- Dense bytecode with contiguous value and frame stacks.
- Tagged `u64` immediates for `unit`, typed empty lists, `option` none,
  booleans, signed 61-bit integers, heap references, and opaque resource tokens; all-zero
  is private invalid/uninitialized state, not a language value.
- Arena objects for wide `i64` values, `f64` values, strings, symbols, pairs,
  closures, buffers, `option` some wrappers, language `result` wrappers, immutable
  nominal products, and boxed enums storing validated layout identity, physical
  tag, and only the active initialized payload.
- `i64`-preserving constants, checked `i64` arithmetic, IEEE `f64` arithmetic,
  exact value/object/`f64`-bit equality, structural list equality
  bounded to 1,000,000 pair nodes, immutable product construction/access/update,
  exact enum construction/test/active projection primitives, and checked narrow
  host domains.
- Precise non-moving mark-sweep collection after 1,024 allocations.
- Return-adjacent frame reuse for tail recursion.
- Synchronous, single-threaded execution with explicit fuel, stack/frame,
  estimated-live-heap, allocation, logical aggregate construction, handle-slot,
  output, and wall-time budgets.
- One compiler invocation and one VM per CLI process.
- Process-global console/terminal behavior, but no process termination from VM
  core.

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
are separate. `OwnedValue` privately retains a reachable heap snapshot, so no
arena index escapes arena lifetime. Resources are dropped, terminal restoration
is attempted, and stdout is flushed before the CLI translates the outcome.

## Current Resource Boundary

- Integer values are never accepted as resources.
- Standard input has the exact borrowed kind `input-stream` and a reserved token
  disjoint from owned slots.
- Owned tokens are monotonic and never reused after close; exact file mode,
  listener/stream, connection/statement, and directory kinds are checked before
  host access.
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
- The terminal guard and stdin/stdout are process-global rather than per-VM
  leases, preventing safe concurrent VM supervision.
- Strings and network/file bytes do not provide a complete lossless byte model.
- The default deadline is cooperative. Poll, wait, stdin/handle read,
  accept, and receive use remaining time. Filesystem, console-write, send/write,
  and cleanup wrappers are not cancellable and can overrun cooperative mode.
  `require_hard_deadline` rejects known unsupported operations before effects
  with `HostFailure`; it does not claim cancellable cleanup.
- Heap byte accounting estimates object and owned-capacity sizes and is checked
  at instruction boundaries; one bounded allocation can transiently occur
  before the aggregate check. `print` currently constructs its temporary host
  formatting string before applying the output-byte limit.

The numeric representation and behavior are specified by
[Exact I64 And F64 Semantics](../decisions/semantics/numeric-semantics.md).

## Current Baseline Tier Boundary

Typed SSA and callable native execution consume the compiler's same verified
semantic program and structured outcome/resource configuration. The Linux
x86-64 allocation-free scalar subset uses exact VM/native boxing only at VM
entries and returns; compatible native calls remain unboxed. Forced mode
compiles before main effects and never falls back. Auto observes bounded
function entries and uses installed code only on later calls. The complete
subset and unsupported reference/allocation/host boundary are in
[Callable Baseline JIT](baseline-jit.md).

Native PollV1 applies cooperative deadline and native-poll fuel limits. VM
instruction fuel and native poll fuel are separately placed safepoint measures;
they preserve the same structured resource category but do not claim identical
instruction-by-instruction exhaustion points.

Host-service injection, instruction quanta, fully cancellable filesystem and
output services, terminal leases, generation-reused handle slots, loop OSR, and
native reference/allocation paths remain **Deferred** to later measured cycles.
