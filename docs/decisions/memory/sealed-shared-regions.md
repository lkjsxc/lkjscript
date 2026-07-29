# Sealed Immutable Shared Regions

## Status

<!-- LKJ-STATUS id=sealed-shared-regions status=accepted-contract -->

**Accepted contract; implementation is not yet Current.** Sealed regions are
the preferred immutable sharing domain. They do not justify per-node reference
counting or whole-runtime collector-free status.

## Build And Seal

A sealed region starts as one uniquely owned ordinary builder. Mutation,
internal-cycle construction, root creation, dependency registration, and drop
registration are private to that owner. No public root exists during build.

One seal transition checks complete initialization, exact layouts and semantic
types, absence of live mutable loans, supported side drops, bounded storage, and
an acyclic strong inter-region dependency graph. Failure publishes nothing and
returns the private builder. Success invalidates mutation authority and exposes
immutable typed roots.

## Ownership

Ownership is region-level. Internal objects have no counts. Structured child
tasks borrow roots without owner traffic. The first Current cross-scope policy
may use checked non-atomic owner counts for worker-local lifetime; independent
cross-worker lifetime needs separately validated synchronization evidence.
Overflow fails before mutation.

A weak sealed root binds domain and root generations, layout, and semantic type,
but never retains. Upgrade returns explicit absence when the region or root is
gone and otherwise acquires one checked owner.

## Dependencies And Cycles

Internal references may cycle freely. Strong dependencies between sealed,
static, or otherwise eligible shared domains must form a DAG. Batch sealing
uses deterministic ordering and returns one canonical closed cycle witness.
Backlinks between independently owned domains use a weak root or pool ID.

## Release

Final release executes exact side drops, releases dependency-ledger entries by
a bounded iterative worklist, frees region storage, invalidates generation, and
records metrics. It does not traverse internal object edges. One region may
retain unreachable internal bytes until final release; measurements must expose
that over-retention.

## Compact Images

A deterministic compact image may order objects, validate relative offsets,
and retain root, dependency, and drop tables. Compaction is optional until it
has same-commit semantic and performance evidence. It does not reopen a
persistent native-image cache.

## Acceptance

Promotion requires immutable internal cycles, multiple roots, stale weak
upgrade, owner overflow, dependency release, deterministic cycle witnesses,
structured-scope borrowing, cleanup failure, over-retention metrics, and no
internal traversal on final release.
