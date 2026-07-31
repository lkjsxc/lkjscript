# Callable Baseline JIT: Native-Reference Boundary

[Authority](../baseline-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Native-Reference Boundary

**Current through host-independent source lowering.** Native canonical native contract
provides typed stable handle words, exact non-empty maps, bounded registered
generated frames, and contract-identified runtime frame and collection calls.
`CollectReferenceV1` remains valid for its closed traced-reference
certificates. `HeapDispatchV1` adds verified frame-home dispatch for source Str,
Product, List, Option, and Result operations. Collector-free bytes and byte
vectors use the separate checked unique runtime. Session-owned stable handles use
exact category/layout checks and zero only for EmptyList/None. Swept indices
are never reused in a session, so same-layout stale handles cannot exhibit ABA.
Returned snapshots retain only the transitive reachable graph. Transactional
heap mutation preserves the old object and counters on closure or estimated-
heap-limit failure. Host
capabilities, lexical ownership references, native/VM reference transitions,
OSR, and optimization are not smuggled into this slice.
