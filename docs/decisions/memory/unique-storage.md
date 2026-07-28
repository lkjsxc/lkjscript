# Deterministic Unique Storage

## Status

**The safe core service and exact byte-vector and immutable-bytes evaluator/VM
families are Current; collector-free path evaluator/VM/native integration and
the remaining source families are not Current.** The core path layout supports
bounded allocation, exact access and equality, structural copy, release, stale
and wrong-layout rejection, and returned-backing transfer. Current source path
values remain traced because their constructor result is an aliasable aggregate.
Transitional `buf` remains independently traced. This service is not a
collector and does not decide liveness by tracing.

## Representation

`UniqueStore` owns bounded slots addressed by opaque keys containing store
identity, slot index, and nonzero generation. Each occupied slot has exactly one
verified affine key and one closed `UniqueLayout` identity. The layouts are
mutable byte-vector, immutable dynamic bytes, and immutable path bytes.

An execution runtime may project a typed key to one canonical 64-bit
`UniqueKeyWord`: the low 32 bits are the slot index and the high 32 bits are the
nonzero generation. The word contains neither store identity nor layout. Only a
typed key exports it, and only a selected `UniqueStore` imports it into a
requested exact typed layout after stale and layout validation. Import binds the
selected store identity; using the returned typed key through another store
fails store validation.

The word is runtime-local plumbing, not source identity, an owner constructor,
a capability, or a serializable reference. The execution-owned store selection
is the authority boundary: because the packed bits intentionally omit store
identity, the word alone cannot distinguish equal slot/generation/layout tuples
in different stores. Runtimes must not move words across that boundary.

Payloads may use safe Rust `Vec<u8>` or `Box<[u8]>`. Rust allocation identity is
not source identity. Native code carries a typed key or its runtime-local packed
projection, not a direct pointer or collector handle.

## Publication

Before publication the store checks allocation count, requested and retained
bytes, index, generation, layout, profile ceilings, and arithmetic. Partial
construction remains private. Failure publishes no key and creates no drop
obligation. Explicit dynamic-bytes clones, static-bytes thaw, path construction,
and path structural copy each perform one bounded copy and one accounted owner
publication.

## Access

Every read, mutation, transfer, and deallocation validates generation and
layout before payload access. Mutation additionally requires a verified
exclusive loan. Static values use separate immutable storage.

Ranged access first checks `start + length` for arithmetic overflow and then
checks the computed end against payload length. Overflow and out-of-bounds are
distinct structured errors. Byte-vector ranged copy validates source and
destination completely before mutation and has memmove-like overlap semantics.

Byte-vector resize is transactional. Growth beyond retained capacity constructs
replacement backing privately, validates its exact retained capacity and the
live-byte ceiling, and only then publishes payload and metric changes. Positive
retained growth increases cumulative allocated bytes by that exact growth;
shrinking length does not release retained capacity. Fill and ranged copy do not
change retained metrics. Bounds, ceiling, arithmetic, and storage failures
change neither payload nor metrics; stale and layout rejection counters remain
the intentional validation-failure metrics.

## Deallocation And Reuse

Drop removes the slot exactly once and updates exact metrics. A freed slot may
be reused only after generation changes. Stale and wrong-layout keys fail
safely. Generation exhaustion retires the slot rather than wrapping.

Freeze transfers compatible mutable backing into immutable bytes. Thaw
transfers uniquely owned dynamic bytes back into a vector. Neither operation
copies when layout permits, and both preserve the slot, generation, packed key
word, retained bytes, and owner-publication accounting while incrementing the
transfer count. Exact counters cover allocations, frees, transfers, live and
peak objects and bytes, reuse, retirement, rejected stale and layout accesses,
and cumulative allocated bytes.

## Limits And Completion

At completion of each first-family evaluator/VM invocation there are no
untransferred unique owners, live loans, cleanup flags, or release backlog.
Normal lexical completion releases owners directly. Trap and early outcomes
run deterministic execution cleanup before the store leak assertion; teardown
only verifies the result. Explicit byte-vector and bytes returned-owner transfer remains a bounded
execution-boundary case and does not promote the wider collector-free island.
Core path returned-backing transfer is executable foundation evidence only; no
Current evaluator or VM path owner reaches that boundary. Path-bearing general
aggregates and every native path group remain outside the collector-free subset,
so neither the complete island nor whole-runtime collector removal is Current.
