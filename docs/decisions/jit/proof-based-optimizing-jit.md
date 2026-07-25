# Proof-Based Optimizing JIT

## Purpose

Define a distinct non-speculative optimizing tier whose transformations are
proved by verified typed-SSA facts and whose generated code is actually called.
## Status

Deterministic baseline SSA normalization and the forced first proof-based
pipeline described in Selected First Delivery are **Current** on Linux x86-64.
The exact automatic baseline-to-proof-optimizing slice below is an **Accepted
Implementation Selection**, not yet Current. Every broader optimization listed
below remains an **Accepted Target**. No baseline code is labeled optimizing,
and the implemented `auto` remains baseline-only.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Selected First Delivery](proof-based-optimizing-jit/selected-first-delivery.md)
- [Accepted Implementation Selection: Automatic Baseline-To-Proof
  Promotion](proof-based-optimizing-jit/accepted-implementation-selection-automatic-baseline-to-proof-pr.md)
- [Deferred And Rejected](proof-based-optimizing-jit/deferred-and-rejected.md)
