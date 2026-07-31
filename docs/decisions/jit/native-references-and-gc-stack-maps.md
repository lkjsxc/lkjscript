# Native References, Frames, And Exact GC Stack Maps

## Purpose

Define the safety boundary that must be Current before generated code may
allocate or carry heap references across a collecting safepoint.
## Status

The closed machine-plan boundary and remaining host-independent traced
allocation slice are **Current** on Linux x86-64. The native contract carries
typed stable reference words, derives exact typed maps, registers generated
frames, and reaches `GcHeap` through safe copied-root/typed-operation callbacks
only for the registered legacy `enum`, `pair`, and `product` families. Dynamic
strings, paths, and eligible nonrecursive products, enums, options, and results
use compact structural roots instead and carry no GC stack-map obligation.
Bytes and byte vectors use the separate noncollecting unique runtime.
Reference-capable lowering remains Current for persistent lists and remaining
legacy aggregate instantiations. Handle/host-capability allocation, lexical
ownership references, and native/VM reference transitions remain **Accepted
Targets**.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Selected Implementation Contract](native-references-and-gc-stack-maps/selected-implementation-contract.md)
- [Collection Acceptance](native-references-and-gc-stack-maps/collection-acceptance.md)
