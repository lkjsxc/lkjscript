# Runtime JIT Instead of Offline PGO

## Purpose

Replace offline profile-guided optimization with a runtime-JIT-first strategy
for adaptive native performance while preserving one typed semantic pipeline
and the reference VM as the cold tier and correctness oracle.
## Status

**Current** for the reference VM, synchronous scalar and host-independent
allocation/recursion Linux x86-64 baseline tier, and the forced first
proof-based optimizing pipeline. The synchronous automatic baseline-to-proof-
optimizing slice is an **Accepted Implementation Selection**, not yet Current.
Handle/host-capability allocation, lexical ownership adapters, native/VM
reference transitions, broader proof passes, and later OSR remain **Accepted
Targets**.
Offline PGO was **Rejected by Product Decision**, not by measurement; optional
explicit local PGO is now a **Deferred Optional Target** under the superseding
execution portfolio. Canonical source/verified-SSA linkage, bounded code objects,
VM/native transfer, `vm`/`auto`/`baseline-jit`/`optimizing-jit`, PollV1, and actual generated
calls are implemented. Closed machine plans also have canonical native contract exact typed
reference frames/maps and a collecting Buf-reference slot. Source-level host-independent native
references/allocation and recursive SCCs
are implemented. Handle/host calls, lexical ownership adapters, native/VM
reference transitions, automatic optimizing promotion, broader proof passes,
OSR, speculative tiers, background work, and deoptimization are absent.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Supersession](runtime-jit-instead-of-offline-pgo/supersession.md)
- [Tier 1: Baseline JIT](runtime-jit-instead-of-offline-pgo/tier-1-baseline-jit.md)
- [Executable Code Objects](runtime-jit-instead-of-offline-pgo/executable-code-objects.md)
- [Rejected](runtime-jit-instead-of-offline-pgo/rejected.md)
