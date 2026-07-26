# Native References, Frames, And Exact GC Stack Maps

## Purpose

Define the safety boundary that must be Current before generated code may
allocate or carry heap references across a collecting safepoint.
## Status

The closed machine-plan boundary and host-independent source allocation slice
are **Current** on Linux x86-64. Native ABI 2 carries typed stable reference
words, derives exact typed maps, registers generated frames, and reaches
`GcHeap` through safe copied-root/typed-operation callbacks. Reference-capable
SSA/JIT lowering, initialization/scalar-store classification, and direct/mutual
recursive source groups are Current for Str, legacy Buf, Product, List, Option,
Result, and monomorphic host-independent enums. Handle/host-capability allocation, lexical ownership references,
and native/VM reference transitions remain **Accepted Targets**.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Selected Implementation Contract](native-references-and-gc-stack-maps/selected-implementation-contract.md)
- [Collection Acceptance](native-references-and-gc-stack-maps/collection-acceptance.md)
