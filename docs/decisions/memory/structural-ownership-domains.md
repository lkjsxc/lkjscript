# Structural Ownership Domains

## Status

<!-- LKJ-STATUS id=structural-ownership-domains status=current -->
<!-- LKJ-STATUS id=structural-root-values status=current -->

**Current for the safe core substrate, resource-plane owner-home adapter, and
deterministic language runtime.** Dynamic strings, paths, eligible products,
enums, results, regular recursive aggregates, copy-leaf segmented lists,
narrow list-bearing ordinary-region products, destinations, and key-free
snapshots execute in all four tiers. No traced object family or tracing
collector remains.

## Decision

A structural value consists of one ownership domain, one typed root, one exact
layout, one semantic type, and one destruction policy. Safe source observes
value semantics, not domain keys, slots, generations, offsets, or addresses.

The closed domain classes are static, unique, region-building, region-owned,
region-sealing, region-sealed, pool, and external. Unknown analysis never
selects legacy tracing. Physical placement does not change resource charges or source semantics.

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

## Current Flat Image Cutover

Production recursive aggregates and lists use one flat typed structural image,
not nested Rust-owned semantic trees. An image contains bounded typed node
records, field cells, local node IDs, immutable byte storage, exact external
dependencies, and an exact side-drop ledger. A node records semantic type,
runtime layout, product or enum identity/tag, field range, and private build or
published state.

A field cell is closed: inline scalar, static artifact, local node reference,
external deterministic dependency, owned leaf payload, or an allowed side-drop
entry. It never stores a runtime root key, session key, raw pointer, collector
index, or untyped handle. Internal node IDs are non-owning. Publication proves
complete initialization, local-edge validity, ordinary-value acyclicity, and an
acyclic external-domain dependency graph.

A unique private builder is backed by one bounded ordinary region. Immutable
escape seals the completed image and publishes one typed root. Final release
runs side drops, releases external dependencies with an iterative bounded
worklist, frees flat storage, and invalidates domain/root generations. It does
not traverse internal payload edges and cannot recurse on the native stack.
Key-free semantic snapshots remain bounded observation and wire values, not
production ownership storage.

Copy-mode products use this image despite copy multiplicity: construction uses
a private destination, field access copies a projection, and immutable update
reconstructs the whole value. An exact-plan and nominal-name content contract
drives bytecode/native identities; validation rejects noncanonical reductions.
Copy roots need no language drop obligation;
dead locals may release early and remaining roots bulk-release at session end.
No collector service participates.

## Current Segmented Lists

Empty lists remain inline. A nonempty list is an invocation-owned ordinary
region of append-only fixed-capacity segments. Each entry stores one value, its
immutable tail key, and exact list-type identity. Adding an entry never changes
an existing logical list, so retained and branched tails share storage without
an element or cons-node count. All segments release in one session-region reset;
list values are copyable region handles and carry no root projection or
per-value drop.

Current ownership selection accepts only `unit`, `bool`, `i64`, and `f64`
elements. Each list witness records whether region ownership is selected.
Residual generic, borrowed, affine, and immutable structural element types
remain `ListElementWitnessRequired`-blocked. Operations in all four engines use
the same physical arena without promoting containing ownership; unsupported
aggregate elements reject rather than materializing through another ownership
domain. Each prepend charges the inherited aggregate-allocation limit.
Capacity 32 is fixed by
[measurement](evidence/segmented-list-capacity-evidence.md).
Returns use a flat key-free codec-stable owned-list table. The pair heap family,
its boundary adapter, and its wire encoding are removed.

Revision 14 adds an Experimental exact-string prerequisite. All four tiers clone
owners into one ledger, compare through structural services, and release before
teardown. Native list sites cross exact frame homes; nodes gain no owner count.

## Current Invocation-Region Products

Products transitively closed over copy-leaf lists, scalar leaves, and acyclic
region-product fields may use invocation-owned records. Canonical identities
guard projection/update; limits preflight and teardown resets without tracing.

Worker-local handles have no codec or main return. Exact internal calls work;
malformed identity, foreign keys, region cycles, nested owners, and unsupported fields reject.
Native region dispatch has no root, publication, barrier, collection, or collector metric.

## Accepted Segmented List Extension

Empty list remains inline. A nonempty root binds an exact element witness, one
sealed or unique front-segment domain, a logical cursor, and at most one older
tail dependency. Each segment stores a bounded block of elements; a uniquely
moved front may be extended in place before sealing. Escape, independent share,
call/task transfer, or capacity seals the builder.

`list-prepend` borrows or shares its element by witness and consumes or shares
the tail from the verified use plan. `list-first` returns a copy, nonescaping
borrow, sealed share, or explicit clone as authorized by the witness.
`list-rest` advances a borrowed cursor, transfers a consumed owner without a
count change, or creates one coarse segment/domain owner for independent escape.
`equal-list` recursively accepts statically comparable nested lists under one
global step bound. Segment layout and size are not source or wire semantics.

A sealed segment/domain may retain a non-atomic region-level owner count. This
is coarse reference counting and is reported as such; no logical list node has
a count. Release is iterative by segment dependencies. Improper pairs cannot be
constructed because no pair object remains.

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
[Ordinary regions](ordinary-regions.md), [sealed regions](sealed-shared-regions.md),
and [pools](typed-generational-pools.md) compose with
[affine aggregates](aggregate-affine-values.md), [deterministic memory](collector-free-deterministic-memory.md), and the
[resource plane](../platform/semantic-resource-plane.md).
