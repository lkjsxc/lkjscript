# Structural Ownership Domains

## Status

<!-- LKJ-STATUS id=structural-ownership-domains status=current -->

**Current for the safe core substrate and resource-plane owner-home adapter.**
It does not migrate a language family, change an execution backend, or support
a collector-free-runtime claim.

## Decision

A structural value consists of one ownership domain, one typed root, one exact
layout, one semantic type, and one destruction policy. Safe source observes
value semantics, not domain keys, slots, generations, offsets, or addresses.

The closed domain classes are static, unique, region-building, region-owned,
region-sealing, region-sealed, pool, and external. Unknown analysis never
selects legacy tracing. Physical placement does not change logical resource
charges or source semantics.

## Identity

Every runtime, domain, root, layout, and semantic type has a typed identity.
A domain key binds runtime identity, class, slot, and nonzero generation. A root
also binds root class, location generation, layout, and semantic type. Safe
constructors do not fabricate live keys.

Reuse changes generation before publication. Exhausted slots retire permanently;
wrap cannot validate a stale key. Reservations, arithmetic, and capacity are
checked before mutation or publication.

## State And Validation

Domain transitions are closed and checked. Partially initialized state is
private. Independent validation reconstructs live slots, roots, dependency and
drop ledgers, region-level ownership, and metrics without deciding liveness.

Optional debug facts may record live roots, state transitions, poisoning,
ownership events, and leaks. Debug tracking is observation only. Runtime storage
uses no reachability traversal, finalizer, collector fallback, or source raw
pointer.

## Thread And Scheduler Boundary

Movable domains may carry a home worker, group, portability, loan epoch, and
transfer epoch. These are placement facts. The semantic resource runtime may
move a domain only with a fresh no-live-loan proof. It never decides domain
liveness or silently copies an affine owner.

## Release

Exact owners, region-level ownership, pools, resources, and static lifetime are
the only liveness authorities. Release work is bounded and iterative. Failure
runs every already-registered exact cleanup action once and reports one stable
failure. Process exit is not ordinary reclamation.

## Rejected

Rejected designs are a renamed universal heap, process-lifetime arenas,
universal or atomic per-node reference counting, raw pool indices, wrapping
generations, hidden tracing, source lifetime names, manual retain/release,
general free, and allocator selection in ordinary source.

## Related Contracts

- [ordinary regions](ordinary-regions.md)
- [sealed shared regions](sealed-shared-regions.md)
- [typed generational pools](typed-generational-pools.md)
- [aggregate affine values](aggregate-affine-values.md)
- [collector-free destination](collector-free-deterministic-memory.md)
- [semantic resource plane](../platform/semantic-resource-plane.md)
