# Coherent Traits And Static Dispatch: Current Slice Evidence

[Authority](../traits-and-static-dispatch.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Current Slice Evidence

The Current marker slice is covered by compiler declaration/coherence/solver
tests, IR canonical/malformed-witness verification tests, and evaluator/VM
equivalence for an exact bounded generic marker call. The implementation tree based on `5c6ba38`
passed focused compiler/IR/app
tests, rustfmt check,
strict Clippy for the touched compiler/IR/JIT/app crates, the 151-test canonical
workspace gate, locked release build, all current runtime smokes, and Docker
`result=ok` on Linux x86-64 with Rust/Cargo 1.96.0. Full Brainfuck Mandelbrot,
performance, Miri, sanitizers, methods, associated items, ownership, package
coherence, and native generic monomorphization were not tested or implemented.
Exact commands are recorded in
[Current State](../../../current-state.md#evidence).
## Deferred And Rejected

Dynamic trait objects, specialization, higher-kinded types, overlapping
instances, negative bounds, source-asserted `Send`/`Sync`, and unbounded solving
are **Deferred** or **Rejected**. An inert accepted declaration, declaration-
order dispatch, and backend source-name interpretation are **Rejected**.
