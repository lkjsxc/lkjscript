# Collector-Free Value Island

## Status

<!-- LKJ-STATUS id=collector-free-value-island status=accepted-contract -->
<!-- LKJ-STATUS id=native-byte-vector-island status=current -->
<!-- LKJ-STATUS id=native-bytes-island status=current -->

**The complete listed island remains an Accepted Contract.** The exact
collector-free native `byte-vector`/`byte-slice`/`byte-slice-mut` subset and the
separate immutable `bytes` subset are **Current** for forced baseline and proof
execution, alongside the scalar set and `stdio` capability to borrowed
`input-stream`. This promotion does not include `path`, owned resources, mixed
resource and unique groups, transitional legacy removal, or the complete
island.

The evaluator, VM, forced baseline, and forced proof tiers consume the same
verified byte-vector and immutable-bytes operations with whole-group native
eligibility and no fallback. Static bytes use exact immutable image-data tokens;
dynamic bytes use affine owner words and shared loans in the invocation-owned
unique service.

## Type Set

The minimum island is:

- never, unit, bool, complete-range i64, and exact-bit f64;
- capability values;
- all eleven typed external resources;
- byte-vector, byte-slice, byte-slice-mut, bytes, and path.

The integrated cutover extends the island with owned `string`, internal
borrowed UTF-8 views, `path`, and every eligible monomorphized nonrecursive
product or enum whose transitive fields already belong to this set. General
products, enums, list/pair nodes, captured closures, unknown generic arguments,
and recursive aggregate SCCs remain outside.

## Eligibility

A function is eligible only when every parameter, result, local, constant,
instruction result, call signature, runtime operation, owner, loan, allocation,
and cleanup edge has an island plan. No legacy-traced value may be reachable.
Eligibility is computed before source effects.

Missing operation support is a verifier or engine error. VM, baseline JIT, and
proof JIT cannot use tracing or fall back after eligibility succeeds.

## Execution Domains

The independent evaluator uses execution-owned byte-vector/dynamic-bytes keys
and fake resource providers; resource-operation dispatch remains absent. The VM
uses typed scalar slots, unique storage, bounded loan records, resource slots,
and explicit cleanup. Native tiers consume the same verified SSA and a closed
unique/runtime call table. The invocation-owned noncollecting service contains a
bounded
`UniqueStore`, a generation-bearing owner set, and a bounded generation-bearing
loan table. Exact owner, shared-loan, and exclusive-loan machine categories are
opaque words, never collector references or integers. Every operation validates
the key layout/generation, loan identity/range/kind, and exclusivity. Island
native frames contain owners, loans, flags, supported borrowed resources,
budgets, deadlines, outcomes, and transitions, but no collector root map,
collection poll, heap dispatch, or barrier.

## Required Evidence

The implemented native subset selects a separate noncollecting call state and
closed dispatch table before entry. Its images contain no safepoints or root
maps, and invocation does not construct `GcHeap` or `JitHeapServices`.

Each island execution records zero deltas for:

- `GcHeap` allocations;
- collections;
- root materializations;
- collector barriers;
- VM fallback;
- collector fallback.

Completion additionally requires zero untransferred unique owners, live loans,
owned resource obligations, cleanup flags, and release backlog.

The Current byte-vector subset has forced baseline/proof fixtures for allocation,
move, shared and exclusive borrow, length, read, mutation, loan end, drop,
transferred return, trap cleanup, allocation failure, stale/forged identities,
and exactly-once release. The immutable-bytes fixtures add literal image data,
static and dynamic length/read, checked slice copy, clone independence,
zero-copy freeze/dynamic thaw, one-copy static thaw, returned payloads, failed
preflight ownership preservation, wrong-layout identities, and trap cleanup.
Each fixture requires evaluator/VM/native result identity, nonzero selected-tier
entries, zero VM transitions/fallback, zero collector-capable calls/counters,
and zero final owners, loans, or release backlog. Proof fixtures permit
optimizing entries only.

Current evidence covers path construction/borrow/equality, dynamic string
construction/view/equality/return, generic nonrecursive product/enum
construction, field/tag borrow, whole-value move/drop, destination abort,
`option string`, `result path system-error`, `result bytes system-error`, nested
deterministic aggregates, owned resource close, mixed resource/unique
execution, and semantic process snapshots. Eligible forced tiers retain
nonzero selected-tier entries, zero fallback, zero collector metadata/counters,
and exact final root/loan/destination/release accounting.

Recursive products and enums, persistent pairs/lists, captured closures, and
unknown generic graphs remain outside the Current island. Completed leaf,
aggregate, resource, and borrowed-host evidence does not promote those families
or the whole runtime.

## Remaining Collector

The existing collector remains only for exact registered non-island structural
families. This capability does not promote the whole runtime to collector-free.
