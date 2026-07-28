# Deterministic Unique Storage

## Status

**Accepted contract; implementation is not yet Current.** This service is not a
collector and does not decide liveness by tracing.

## Representation

`UniqueStore` owns bounded slots addressed by opaque `UniqueKey { index,
generation }`. Each occupied slot has exactly one verified affine key and one
closed `UniqueLayout` identity. Initial layouts are mutable byte-vector,
immutable dynamic bytes, and immutable path bytes.

Payloads may use safe Rust `Vec<u8>` or `Box<[u8]>`. Rust allocation identity is
not source identity. Native code initially carries the typed key, not a direct
pointer or collector handle.

## Publication

Before publication the store checks allocation count, requested and retained
bytes, index, generation, layout, profile ceilings, and arithmetic. Partial
construction remains private. Failure publishes no key and creates no drop
obligation.

## Access

Every read, mutation, transfer, and deallocation validates generation and
layout before payload access. Mutation additionally requires a verified
exclusive loan. Static values use separate immutable storage.

## Deallocation And Reuse

Drop removes the slot exactly once and updates exact metrics. A freed slot may
be reused only after generation changes. Stale and wrong-layout keys fail
safely. Generation exhaustion retires the slot rather than wrapping.

Freeze transfers compatible mutable backing into immutable bytes. Thaw
transfers uniquely owned dynamic bytes back into a vector. Neither operation
copies when layout permits.

## Limits And Completion

At island completion there are no untransferred unique owners, live loans,
cleanup flags, or release backlog. Explicit returned-owner objects may retain a
slot for the host and release it on host-owner drop.
