# Collector-Free Value Island

## Status

<!-- LKJ-STATUS id=collector-free-value-island status=accepted-contract -->
<!-- LKJ-STATUS id=native-byte-vector-island status=current -->

**The complete listed island remains an Accepted Contract.** The exact
collector-free native `byte-vector`/`byte-slice`/`byte-slice-mut` subset is
**Current** for forced baseline and proof execution, alongside the existing
scalar set and `stdio` capability to borrowed `input-stream`. This promotion
does not include immutable `bytes`, `path`, owned resources, mixed resource and
unique groups, or the complete island.

The evaluator, VM, forced baseline, and forced proof tiers consume the same
verified byte-vector family with whole-group native eligibility and no fallback.
Immutable static/dynamic bytes execute in evaluator and VM; native tiers reject
bytes before entry, so the complete island remains non-Current.

## Type Set

The minimum island is:

- never, unit, bool, complete-range i64, and exact-bit f64;
- capability values;
- all eleven typed external resources;
- byte-vector, byte-slice, byte-slice-mut, bytes, and path.

Dynamic strings, symbols, products, enums, option, result, list nodes, closures,
and other registered structural families remain outside the island.

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
and exactly-once release. Each fixture requires evaluator/VM/native result
identity, nonzero selected-tier entries, zero VM transitions/fallback, zero
collector-capable calls/counters, and zero final owners, loans, or release
backlog. Proof fixtures permit optimizing entries only.

Freeze/thaw and immutable `bytes` have evaluator/VM evidence. Native bytes,
path construction, file I/O, owned resource close, and mixed resource/unique
execution remain required evidence for the complete island.

## Remaining Collector

The existing collector remains only for exact registered non-island structural
families. This capability does not promote the whole runtime to collector-free.
