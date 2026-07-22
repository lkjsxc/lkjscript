# Virtual Machine

## Purpose

Describe the current execution engine, host-resource boundary, and accepted
next repairs.

## Status

**Current** unless a section is labeled **Accepted Target** or **Deferred**.

## Current Shape

- Dense bytecode with contiguous value and frame stacks.
- Tagged `u64` immediates for nil, booleans, small integers, heap references,
  and opaque handle tokens.
- Arena objects for floats, strings, symbols, pairs, closures, buffers, and
  language Result wrappers.
- Precise non-moving mark-sweep collection after 1,024 allocations.
- Return-adjacent frame reuse for tail recursion.
- Synchronous, single-threaded execution with blocking host operations.
- One compiler invocation and one VM per CLI process.
- Process-global console/terminal behavior and direct process exit.

The VM/compiler boundary is `Chunk`, `FunctionProto`, `Constant`, and `Op` in
`lkjscript-core`. Public malformed chunks are not fully validated and can reach
panic-prone assumptions; compiler-produced chunks are the supported path.

## Current Resource Boundary

- Integer values are never accepted as resource handles.
- Stdin has a reserved borrowed token disjoint from owned files and sockets.
- Owned tokens are monotonic and never reused after close.
- All raw descriptor resolution is centralized in the VM resource table.
- Close rejects borrowed, unknown, stale, and repeatedly closed tokens.
- Socket-only operations reject file handles.
- Arbitrary ioctl is absent. `sys-tty-get` and `sys-tty-set` select fixed Linux
  requests internally and validate exactly 60 state bytes before FFI.
- Every fallible resource, filesystem, time, socket, polling, terminal, and
  terminal-guard primitive returns a language Result.
- Descriptor-era aliases are removed in favor of handle-explicit `sys-*` names.
- Missing paths return `Ok(false)` only for absence-class errors; other
  `access(2)` failures return errors.
- Network send returns its real byte count and uses `MSG_NOSIGNAL`.

## Current Host Risks

- Core stdout/stdin operations outside the resource API still treat host IO
  failure as VM failure.
- The terminal exit guard is process-global rather than a per-process lease.
- Monotonic handle metadata grows until the VM ends.
- Strings and network/file bytes do not provide a complete lossless byte model.
- Blocking calls and process exit prevent safe multi-VM supervision.

## Accepted Target

- Preserve complete I64 values with immediate signed 61-bit values and boxed
  wide integers, while keeping F64 as a distinct heap value.
- Preserve numeric types in bytecode and dispatch checked I64 versus IEEE F64
  arithmetic without routing integers through floating point.
- Make every numeric host consumer accept full I64 and check narrower domains.
- Validate public chunks before dispatch.

The numeric representation and behavior are specified by
[Exact I64 And F64 Semantics](../decisions/numeric-semantics.md).

Process-safe outcomes, host-service injection, instruction quanta, blocking
wait objects, generation-reused handle slots, and per-process budgets are
**Deferred** to later measured cycles.
