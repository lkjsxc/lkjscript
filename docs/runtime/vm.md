# Virtual Machine

## Purpose

Describe the execution engine implemented in the baseline and its accepted
foundation repairs.

## Status

**Current** unless a bullet is labeled **Accepted Target**.

## Current Shape

- Dense bytecode with contiguous value and frame stacks.
- Tagged `u64` immediates for nil, booleans, small integers, heap references,
  and handle payloads.
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

## Current Host Risks

- Arbitrary ioctl requests and buffer sizes cross an unsound safe wrapper.
- Handle payloads can mean raw descriptors or reusable resource-table indexes.
- Ordinary OS failures often become VM errors despite Result-typed prelude
  signatures.
- Network send reports zero after success.
- Strings and network/file bytes do not provide a complete lossless byte model.

## Accepted Foundation Target

- Replace arbitrary ioctl with fixed, size-validated terminal operations.
- Use opaque namespace-separated handles whose stale values cannot alias new
  resources.
- Represent ordinary fallible host outcomes as language `ResultOk` or
  `ResultErr`; reserve VM errors for language/runtime contract violations.
- Report actual send counts and suppress process-killing SIGPIPE behavior.
- Make the currently executable numeric contract match static typing exactly.

Process-safe outcomes, host-service injection, instruction quanta, blocking
wait objects, and per-process budgets remain **Deferred** to the supervisor
cycle.
