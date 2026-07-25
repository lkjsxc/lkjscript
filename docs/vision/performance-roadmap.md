# Performance Roadmap

## Purpose

Define a measured runtime-JIT-first path toward category-leading performance
without turning aspiration into a current release claim.
## Status

The reference bytecode VM, exact I64/F64 execution, precise mark-sweep, resolved
typed HIR, verified normalized SSA, selected owned Linux x86-64 backend, bounded
code objects, callable host-independent baseline JIT, and the forced first
proof-based optimizing pipeline are **Current**. The old observation hook is
removed. Broader ownership/traits, Handle/host native calls, native/VM
reference transitions, later loop OSR, and direct Wasm are **Accepted Targets**.
The synchronous automatic baseline-to-proof promotion slice is an **Accepted
Implementation Selection**, not yet Current and no longer the immediate
repository priority. Guarded specialization is **Deferred** until justified.
Production AOT and a content-addressed cache are **Accepted Targets**; optional
explicit local PGO is a **Deferred Optional Target** under the execution
portfolio.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Sequence](performance-roadmap/sequence.md)
- [Phase 7: Loop Hotness And OSR — Later Cycle](performance-roadmap/phase-7-loop-hotness-and-osr-later-cycle.md)
