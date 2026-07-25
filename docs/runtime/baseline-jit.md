# Callable Baseline JIT

## Purpose

Describe the implemented Linux x86-64 baseline tier without implying full-
language native coverage or future OSR behavior.
## Status

**Current on Linux x86-64** for allocation-free scalar execution and the exact
host-independent allocation/recursion subset below. Ordinary `run` uses
baseline-only automatic tiering; reference-typed functions remain
conservatively in the VM there. The reference VM remains the cold tier,
full-language oracle, and deterministic explicit engine. The next synchronous
automatic baseline-to-proof-optimizing slice is an **Accepted Implementation
Selection**, not yet Current.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Implemented Pipeline](baseline-jit/implemented-pipeline.md)
- [Native-Reference Boundary](baseline-jit/native-reference-boundary.md)
