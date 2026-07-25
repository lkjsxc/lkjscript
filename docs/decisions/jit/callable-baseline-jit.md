# Callable Linux x86-64 Baseline JIT Cycle

## Purpose

Fix the completion boundary and prerequisite contracts for the first native
runtime tier without describing planned work as current behavior.
## Status

**Current** for the Linux x86-64 allocation-free scalar MVP: machine code
lowered from verified typed SSA for canonical lkjscript programs is installed in
W^X memory and actually called. Exact current coverage and unsupported native
semantics are recorded in [Current State](../../current-state.md) and
[Callable Baseline JIT](../../runtime/baseline-jit.md). Native references,
allocation, host IO, recursion, OSR, background work, and optimizing tiers are
not made Current by this completion.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Platform And Tier Decision](callable-baseline-jit/platform-and-tier-decision.md)
- [Typed SSA Authority](callable-baseline-jit/typed-ssa-authority.md)
- [Rejected For This Cycle](callable-baseline-jit/rejected-for-this-cycle.md)
