# Measured Execution Portfolio

## Purpose

Define one semantic authority with multiple workload-selected execution
strategies, while preserving the Current callable JIT and proof authority.
## Status

**Accepted Target.** The Current modes remain:

- independent bounded SSA evaluator for host-independent differential evidence;
- validated reference bytecode VM;
- synchronous callable Linux x86-64 baseline JIT;
- conservative baseline-only automatic function-entry promotion; and
- forced proof-checked optimizing JIT with no fallback.

AOT deployment, persistent native cache, automatic optimizing promotion, OSR,
local PGO, direct Wasm, and non-Linux native backends are not Current. This
record changes long-term product policy, not existing engine behavior or
retained benchmark verdicts.

This record supersedes the permanent JIT-only and permanent offline-PGO/cache
rejection in [Runtime JIT Instead Of Offline
PGO](../jit/runtime-jit-instead-of-offline-pgo.md). That record remains authoritative
for the Current VM/JIT implementation, exact forced-mode gates, and historical
measurements.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Decision](execution-portfolio/decision.md)
- [Rejected](execution-portfolio/rejected.md)
