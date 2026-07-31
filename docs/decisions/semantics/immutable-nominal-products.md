# Immutable Nominal Products

## Purpose

Define the first user-defined aggregate type needed to replace mutable singleton
state with explicit values passed through helpers.
## Status

**Current.** The compiler, resolved typed HIR, verified SSA, evaluator,
validated bytecode, disassembler, reference VM, and supported native tiers
implement this contract. Deterministic products use flat structural images or
selected invocation regions; no product uses tracing storage. Brainfuck,
terminal, and editor workload state is product-threaded, and the obsolete
mutable-singleton migration is complete. General product equality,
ownership-generic storage, and broader host/native behavior remain outside this
record. [Current State](../../current-state.md) is exact.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each
capsule preserves a cohesive part of the record. Current means implemented and
evidenced; accepted selections and targets are future work. Deferred and
Rejected remain non-current.

## Strict Capsule Manifest

- [Decision](immutable-nominal-products/decision.md)
- [Required Conformance](immutable-nominal-products/required-conformance.md)
