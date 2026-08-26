# Semantic repository storage

Status: normative for the current Graph 5 repository. Storage contracts and hostile decoder bounds
are executable-derived in [contracts.md](../generated/contracts.md).

## Project root

A current project root is the Graph 5 repository itself. Its owned entries are:

```text
HEAD                       atomic accepted visibility binding
LOCK                       repository publication lock
packs/                     immutable multi-domain object packs
catalog/current.lkjc       rebuildable physical object locator
staging/                   private incomplete publication state
PACKAGE-TRANSPORTS/        exact immutable dependency transports when present
derived/compiler/          disposable compiler manifests, units, and CURRENT
```

The physical layout is replaceable and is not language meaning. The logical authority is the exact
validated semantic revision and immutable closure selected by `HEAD`. Missing disposable
`staging/` or compiler state may be recreated. Missing or inconsistent accepted `HEAD`, pack,
object, revision, receipt, root, or witness bindings are corruption.

A predecessor `.lkjscript` marker is not a storage edition. Project discovery rejects it before
opening derived state or mutating the destination. A root containing both current and predecessor
markers is rejected as ambiguous; it never selects a fallback.

## Immutable objects and packs

Typed canonical objects occupy distinct domains for semantic owners, types, blobs, persistent-map
pages, roots, witnesses, transactions, diffs, receipts, revisions, idempotency records, package
interfaces/transports, compiler units/manifests, and other exact contracts. Object keys verify
domain-separated content bytes. A digest from one domain cannot authorize another.

Packs are immutable, bounded, canonical collections of keyed objects with checked headers,
footers, order, offsets, lengths, checksums, and closure. Duplicate physical definitions,
noncanonical order, truncation, trailing input, foreign domains, overflow, and digest mismatch
reject. Pack filenames are locators and never semantic identity.

The catalog maps exact object keys to pack positions and may be rebuilt by bounded verification of
packs. A catalog entry is useful only after the referenced pack entry reproduces its exact key.
Catalog disagreement is derived corruption and may not make unavailable or wrong semantic bytes
appear valid.

Persistent maps use canonical typed keys and bounded values over immutable content-addressed pages.
Equal logical maps have equal logical content commitments independent of page splits. Sparse path
copies can share accepted-base subtrees, but full logical reconstruction remains the independent
semantic oracle.

## Accepted revision and publication

`HEAD` strictly binds repository identity, current revision and revision-record object, semantic
root/state, validation witness/certificate, transaction, diff, and receipt under their exact
current contracts. It is the one normal accepted visibility point.

Publication proceeds in this order:

1. validate request components that do not need repository access;
2. prepare and fully validate a candidate against an exact base;
3. acquire the repository lock and recheck that exact base;
4. stage and seal all new immutable canonical objects and packs;
5. synchronize durable object data and containing metadata;
6. publish any required non-authoritative catalog bindings;
7. write and synchronize a private `HEAD` stage; and
8. atomically replace `HEAD`, then synchronize its directory.

Failure before the final visibility point leaves the old accepted revision. Interruption tests must
reopen either the complete old revision or complete new revision, never a partial hybrid. Cleanup
may remove only a stage whose ownership and target are exact.

Accepted immutable objects may exist before visibility and remain unreachable after a failed or
stale attempt. Their existence cannot advance authority. There is currently no public retention
deletion or compaction operation.

## Package transports

`PACKAGE-TRANSPORTS` contains strict immutable dependency material selected by exact package
revision. Installing a transport precedes initial dependent-repository publication, and reopening
validates its canonical bytes and exact dependency binding. A transport is not editable authority,
a lockfile, or ambient resolver input. Missing, foreign, duplicate, noncanonical, stale, or corrupt
transport data rejects dependent lifecycle preparation.

The current command lifecycle supports only the generated built-in standard transport. Arbitrary
transport installation is not a released operation.

## Derived compiler cache

`derived/compiler/CURRENT` is a separate atomic cache visibility point. It binds one repository,
exact accepted revision/state, compiler and unit contracts, optimization policy, compilation
manifest, and object closure. Compiler objects are derived and may be deleted and rebuilt.

Cache publication uses its own lock, private stage, synchronization, and atomic current-head
replacement. Stale or foreign revision cache state is never reused. Missing cache performs a clean
build. Malformed or inconsistent cache state is reported and clean-recovered. An interrupted cache
update either leaves the prior exact cache head or exposes the complete new one; neither state can
alter semantic `HEAD`.

An accepted semantic publication may be followed by incremental cache maintenance using its
in-memory prepared compiler impact. Authority publication is already complete. Cache failure is a
separate derived observation and cannot cause semantic rollback or a failed-write report.

## External artifact output

Artifact files are external immutable derived outputs and never repository `HEAD`. Public build
requires an explicit absent output path with an ordinary existing parent. It rejects traversal,
symlinked parents, existing file/directory/symlink targets, invalid names, and byte exhaustion
before visible publication.

The writer creates a unique owned sibling stage, writes and synchronizes exact bytes, publishes by
create-new visibility, synchronizes the parent, and removes only its own stage. Existing data is
never overwritten. A failure or interruption before visibility leaves no partial target; after
visibility the artifact bytes are complete even if a later durability observation is uncertain.

## Trust and non-goals

Repository paths, packs, catalogs, transports, cache heads/objects, and artifacts are hostile
decoder boundaries. Symlinks and non-regular files reject where publication or exact immutable
reads require ordinary filesystem objects.

The local operator, executable, OS, and filesystem durability implementation are trusted. This
contract does not provide encrypted storage, signatures/provenance, hostile-code sandboxing,
multi-tenant isolation, distributed consensus, leases, garbage collection, live packing, memory
mapping, or cross-filesystem atomic visibility.
