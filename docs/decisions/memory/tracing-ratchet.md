# Tracing Family Ratchet

## Status
<!-- LKJ-F memory-tracing-ratchet superseded bdaumT2umNgHfR4PJEhamvu7kWz29TK0EFboRoK6HOg -->
<!-- LKJ-F no-tracing-runtime current G3iFIfhkJeXDVrUevwng0sXnqvrZGdJwsIUz1qcii1Q -->


**Superseded migration gate.** This document retains the decrement history.
The registry and `memory traced` command were deleted when the final
no-tracing rule became unconditional.

## Rule

`LKJ-MEMORY-TRACING-RATCHET` owns one sorted closed set of object families that
may select `legacy-traced`. It derives the implementation, inventory, Current
State count, and island exclusions from one registry.

The gate fails when:

- a new family becomes traced;
- a migrated family regresses;
- an unregistered family selects legacy tracing;
- a migrated island type allocates, roots, barriers, or collects;
- source, documentation, and implementation counts differ.

The set may decrease in an ordinary accepted migration. It may increase only
through an explicit accepted architectural reversal with retained evidence and
a changed ratchet contract. Analysis failure is never such a reversal.

## Completed Migration

Complete-range i64 and exact-bit f64 scalar families have left the allowed set.
The unused `HeapObj::Builtin` representation has also been removed: operation
identity is validated bytecode/static registry data, and no runtime allocation
site existed. Capture-free function values carry an inline validated prototype
ID; captured closure graphs remain unsupported. Symbol constants carry bounded
artifact indexes and returned snapshots copy only reachable symbol text. The
Current validated constant-byte ceiling bounds that text; artifact identity and
result transfer do not consume tracing-heap allocation or live-byte limits.
The final `enum` family moved to bounded structural images. Copy-leaf list
execution uses segmented invocation regions, and returned or nested lists use
a flat key-free owned-list table. Pair, product, and enum traced storage,
allocation, traversal, wire tags, native helpers, and test-only construction
are removed. No collector infrastructure remains.

## Completed Leaf Decrements

The buffer decrement removed source `buf`, `HeapObj::Buf`, buffer opcodes,
native layouts and helpers, host owner access, codec identities, metrics,
packages, and collector-root producers. Immutable `bytes`, affine
`byte-vector`, and checked whole-owner slices execute without tracing.

The path and string decrement removed `HeapObj::Path`, `HeapObj::Str`, their
wire tags, native heap layouts, collector traversal, root production, and
registry entries. Dynamic path and string leaves use bounded structural owners;
static string artifacts remain inline identities. The pair decrement removed
its heap variant after segmented list execution and flat owned-list snapshots.
The product decrement removed `HeapObj::Product`, native traced-product
references, collector traversal, snapshot/wire support, and runtime fallback;
all products now use verified structural or invocation-region storage, or reject.
No removed family may regain tracing. The registry itself is removed.

All accepted products and enum instantiations execute structurally. Copy
products construct, project, update, and return with
zero collector interactions; validated VM calls also transport copy-polymorphic
values. Acyclic products closed over selected copy-leaf lists, exact scalar leaves,
and region products use invocation-owned typed
ordinary-region records in all four tiers. Their native dispatch sites have no
collecting safepoint, root, collector call, or barrier, and process-boundary
escape is invalid. Native polymorphism remains blocked without the final witness
ABI. Non-deterministic or unsupported product closures reject before SSA. A
completed family cannot regain a legacy plan, heap variant, collector allocation,
root, barrier, or safepoint.

## Final Gate

`LKJ-RUNTIME-NO-TRACING-COLLECTOR` is an unconditional `check-sources` rule. It
rejects collector directories, traced object/storage symbols, collection
services, liveness-root materialization, collecting call sites, barriers,
collector configuration, and collector metrics. Deterministic dependency
validation, release worklists, typed pools, structural root tables, and debug
observation are not liveness tracing and remain permitted.

The final integrated revision moved recursive/generic enums, errors, results,
options, process outcomes, VM execution, and both native tiers to verified
deterministic storage before deleting the registry and collector. No disabled
or private collector compatibility path remains.
