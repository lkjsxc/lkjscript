# Coherent Traits And Static Dispatch

## Purpose

Define the minimal bounded trait system required by ownership, generic
collections, static method dispatch, and future self-hosting.
## Status

The bounded **Initial Coherent Slice** below is **Current**: declaration-only
marker traits, exact nominal-product impls, generic marker bounds, structural
`Copy`/`Send`/`Sync` facts, and verified erased witness identities. Trait
methods, associated types/values, generic or blanket impls, package orphan
rules, specialization, dynamic dispatch, and native monomorphization remain an
**Accepted Target** or **Deferred** as identified below.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Syntax Candidates](traits-and-static-dispatch/syntax-candidates.md)
- [Current Slice Evidence](traits-and-static-dispatch/current-slice-evidence.md)
