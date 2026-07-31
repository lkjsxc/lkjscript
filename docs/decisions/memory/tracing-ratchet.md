# Tracing Family Ratchet

## Status

<!-- LKJ-STATUS id=memory-tracing-ratchet status=current -->
<!-- LKJ-STATUS id=no-tracing-runtime status=accepted-target -->

**Current intermediate migration gate; it is not the final
no-tracing-collector gate.** `check-sources` verifies the exact registry and
`lkjscript memory traced [--json]` exposes it.

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

## Current Migration

Complete-range i64 and exact-bit f64 scalar families have left the allowed set.
The unused `HeapObj::Builtin` representation has also been removed: operation
identity is validated bytecode/static registry data, and no runtime allocation
site existed. Capture-free function values carry an inline validated prototype
ID; captured closure graphs remain unsupported. Symbol constants carry bounded
artifact indexes and returned snapshots copy only reachable symbol text. The
Current validated constant-byte ceiling bounds that text; artifact identity and
result transfer do not consume tracing-heap allocation or live-byte limits.
The exact three registered legacy families are `enum`, `pair`, and `product`.
Collector infrastructure remains available only to those families, and the
complete collector-free value island is not yet Current.

## Completed Leaf Decrements

The buffer decrement removed source `buf`, `HeapObj::Buf`, buffer opcodes,
native layouts and helpers, host owner access, codec identities, metrics,
packages, and collector-root producers. Immutable `bytes`, affine
`byte-vector`, and checked whole-owner slices execute without tracing.

The path and string decrement removed `HeapObj::Path`, `HeapObj::Str`, their
wire tags, native heap layouts, collector traversal, root production, and
registry entries. Dynamic path and string leaves use bounded structural owners;
static string artifacts remain inline identities. Neither removed family may
regain a traced representation. The ratchet gate fixes the remaining registry
at exactly `enum`, `pair`, and `product`.

Eligible nonrecursive product/enum instantiations may execute structurally while
the broad family remains registered. Their independently verified closure must
produce zero collector interactions; this does not remove the broad family from
the registry. A completed leaf cannot regain a legacy memory plan, heap variant,
collector allocation, root, barrier, or safepoint.

## Final Gate

`LKJ-RUNTIME-NO-TRACING-COLLECTOR` is implemented as the zero-registry closure
of `check-sources` and remains disabled while any family is registered. At zero
it rejects collector directories; `LegacyTraced`, `HeapObj`, and `GcHeap`;
collector services, root materialization, collecting safepoints, barriers,
configuration, and metrics. Deterministic dependency validation, release
worklists, typed pools, root tables, and debug observation are not liveness
tracing and remain permitted. Passing the intermediate ratchet cannot support a
whole-runtime collector-free claim.
