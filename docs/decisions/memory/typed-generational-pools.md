# Typed Generational Pools

## Status

<!-- LKJ-STATUS id=typed-generational-pools status=accepted-contract -->

**Accepted contract; implementation is not yet Current.** Pools are the selected
domain for mutable identity, individual deletion, and cyclic graphs. They are
not a general source API in the first slice.

## Owner And Identity

A pool has one affine owner. A typed element ID binds pool domain identity,
slot, nonzero generation, layout, semantic type, and Rust/runtime element type.
It is not an integer or pointer and safe source cannot inspect it.

Slots are vacant, initializing, live, removing, or retired. Borrow state may be
represented by exact runtime loans when a borrow crosses an internal call.
Reuse advances generation before publication. Exhaustion retires the slot or
pool epoch; generation never wraps into stale validity.

## Access

Lookup validates pool, slot, generation, layout, semantic type, and state before
payload access. It yields a temporary shared or exclusive borrow bounded by the
pool owner and slot generation. Removal rejects any live loan and invalidates
the old ID before reuse.

Pool elements name peers through typed non-owning IDs. The pool owns storage,
so graph cycles are not ownership cycles. Physical movement is permitted only
when references remain IDs.

## Determinism And Partitioning

Semantic iteration is ascending live slot order. Faster unordered traversal is
explicit and non-semantic. Exact disjoint partitions permit concurrent task
access; partition proofs bind pool generation and ranges.

Pool destruction visits live slots in deterministic slot order, executes exact
element drop, invalidates all IDs, frees storage, and records metrics. It does
not discover liveness by traversing element edges.

## Source Boundary

No general source pool operation becomes Current until source types, Semantic
Source, HIR, verified SSA, borrow checking, bytecode, VM, native execution, and
cleanup are complete. Internal compiler, editor, game, or runtime pools may be
Current first.

## Acceptance

Promotion requires stale and wrong-pool rejection, remove/reinsert, exhaustion
retirement, deterministic iteration/destruction, cyclic mutable graphs, one
ECS-style workload, one compiler/editor identity workload where coherent,
partition validation, allocation failure, and leak-free cleanup.
