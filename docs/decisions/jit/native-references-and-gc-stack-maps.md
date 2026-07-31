# Native References, Frames, And Exact GC Stack Maps

## Purpose

Retain the historical safety boundary used while generated code could carry
traced references across collection calls.
## Status

**Superseded by the zero-family cutover.** The machine-plan and typed-frame
parts remain Current, but exact liveness maps, root publication, collection
callbacks, barriers, and the traced reference domain were deleted. Dynamic
strings, paths, products, enums, options, results, and errors use structural
storage. Selected products and copy-leaf lists use invocation-local region
keys. Bytes and byte vectors use the unique runtime. Generated frames retain
only typed homes, cleanup obligations, bounds, and structured outcome state.
Handle/host-capability allocation, complete lexical ownership references, and
additional native/VM ownership transitions remain **Accepted Targets**.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Selected Implementation Contract](native-references-and-gc-stack-maps/selected-implementation-contract.md)
- [Collection Acceptance](native-references-and-gc-stack-maps/collection-acceptance.md)
