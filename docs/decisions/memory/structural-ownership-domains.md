# Structural Ownership Domains

## Status

<!-- LKJ-STATUS id=structural-ownership-domains status=current -->
<!-- LKJ-STATUS id=structural-root-values status=current -->

**Current for the safe core substrate, resource-plane owner-home adapter, and
nonrecursive deterministic language island.** Dynamic strings, paths, eligible
products/enums/results, destinations, and key-free snapshots use this service in
the evaluator, VM, forced baseline, and forced proof tiers. Recursive legacy
families remain outside the island, so this does not support a whole-runtime
collector-free claim.

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

## Runtime Root Table

A dynamic execution session may project a typed `RootKey` to one compact
`StructuralValueKey`. The 64-bit key carries only a table-local slot and
nonzero generation. The selected table binds runtime identity; its entry binds
the complete domain, root, layout, semantic type, and ownership state.

Slots advance generation before reuse and retire instead of wrapping. Owned
roots cannot be duplicated. Shared and exclusive loans use separate stale-safe
tokens and exact conflict checks. A move, drop, sealed release, or static
unregistration requires no live loan and invalidates the old key before reuse.
Table removal returns the typed root to the domain authority that performs the
actual release; the table never determines liveness.

Keys are invocation-private runtime plumbing. They are not source identities,
addresses, serializable values, or substitutes for domain ownership.

## State And Validation

Domain and root-table transitions are closed and checked. Partially initialized
state is private. Independent validation reconstructs live slots, roots, loans,
dependency and drop ledgers, region-level ownership, and metrics without
deciding liveness.

Optional debug facts may record live roots, state transitions, poisoning,
ownership events, and leaks. Debug tracking is observation only. Runtime storage
uses no reachability traversal, finalizer, collector fallback, or source raw
pointer.

## Execution Service Cutover

Each evaluator, VM, or native invocation owns one structural execution service:
a structural runtime, compact root table, deterministic domain stores, exact
limits, and metrics. Publishing a dynamic value creates or selects its domain,
creates one typed root, publishes one table entry, and returns only a
`StructuralValueKey` in runtime `Value` storage.

A move takes and invalidates the old table entry before transfer or
republication. A borrow couples a stale-safe table loan to the exact domain
loan. A drop removes the entry first and then dispatches the returned typed root
to its domain authority for deterministic release. Static artifacts unregister
without dynamic allocation; sealed roots release one region-level owner and do
not count internal nodes. Session completion requires zero live dynamic roots,
loans, private destinations, and release backlog.

The adapter dispatches by typed domain/layout metadata, never source type-name
strings. It rejects wrong runtime, stale generation, wrong layout or semantic
type, conflicting borrow, drop while borrowed, and a domain/root mismatch
before mutation. Structural keys and loan tokens are invocation-private and
never returned through a process codec.

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
