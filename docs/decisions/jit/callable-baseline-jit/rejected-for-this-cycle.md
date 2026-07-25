# Callable Linux x86-64 Baseline JIT Cycle: Rejected For This Cycle

[Authority](../callable-baseline-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Rejected For This Cycle

- observation-only or inert engine seams;
- machine-code fixtures unrelated to typed SSA;
- emitted or disassembled but uncalled code;
- hard-coded benchmark expressions or output;
- silent VM fallback in forced mode;
- bytecode or untyped-syntax reinterpretation in the backend;
- RWX memory or unchecked executable bytes;
- process exit from the execution core or generated code;
- imprecise native GC roots;
- background compilation, OSR, speculative optimization, guards, and
  deoptimization;
- persistent profiles, persistent code cache, and offline PGO;
- non-Linux or non-x86-64 completion claims.
