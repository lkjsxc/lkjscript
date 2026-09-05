# Semantic repository storage

Status: normative for the current typed meaning graph repository. Internal storage compatibility
identities and hostile decoder bounds remain at their typed source owners.

## Project root

A current project root is the typed meaning graph repository itself. Its owned entries are:

```text
HEAD                       atomic accepted visibility binding
LOCK                       repository publication lock
packs/                     immutable multi-domain object packs
catalog/current.lkjc       rebuildable physical object locator
catalog/segments/          immutable content-addressed locator segments
staging/                   private incomplete publication state
PACKAGE-TRANSPORTS/        exact immutable dependency transports when present
derived/compiler/          disposable compiler manifests, units, and CURRENT
```

The physical layout is replaceable and is not language meaning. The logical authority is the exact
validated semantic revision and immutable closure selected by `HEAD`. Missing disposable
`catalog/`, `staging/`, or compiler state may be recreated. A missing `LOCK` may be atomically
recreated only after the accepted `HEAD` and its immutable closure validate; a non-regular,
nonempty, or symlinked lock rejects. Missing or inconsistent accepted `HEAD`, pack, object,
revision, receipt, root, or witness bindings are corruption.

A predecessor `.lkjscript` marker is not a storage edition. Project discovery rejects it before
opening derived state or mutating the destination. A root containing both current and predecessor
markers is rejected as ambiguous; it never selects a fallback.

Operational application data is not stored in this root and does not reuse any repository,
revision, pack, object, map, or digest identity. A deployment-selected `lkjscript-data-store-1`
root has its own physical identity and atomic head as specified in
[data-capabilities.md](data-capabilities.md). Runtime data writes cannot author, select, or advance
semantic `HEAD`; semantic publication cannot silently mutate operational data.

## Immutable objects and packs

Typed canonical objects occupy distinct domains for semantic owners, types, blobs, persistent-map
pages, roots, witnesses, transactions, diffs, receipts, revisions, idempotency records, package
interfaces/transports, compiler units/manifests, and other exact contracts. Object keys verify
domain-separated content bytes. A digest from one domain cannot authorize another.

Packs are immutable, bounded, canonical collections of keyed objects with checked headers,
footers, order, offsets, lengths, checksums, and closure. Duplicate physical definitions,
noncanonical order, truncation, trailing input, foreign domains, overflow, and digest mismatch
reject. Pack filenames are locators and never semantic identity.

Catalog contract 2 is the sole healthy physical locator. `catalog/current.lkjc` is an atomically
replaced manifest selecting at most 32 immutable, content-addressed, sorted segments under
`catalog/segments/`, with at most one segment at each level. The manifest binds the ordered segment
identities, generations, key ranges, entry and pack totals, cumulative physical work, and one
packing-independent logical catalog commitment. Each segment binds its level and generation,
canonical key/location entries, exact pack descriptors, 64-entry block ranges and filters,
per-block checksums, and an authenticated metadata tail and closing marker.

A healthy open reads the bounded manifest and selected segment metadata. It does not enumerate old
packs, read every catalog entry, or scan pack footers. Point lookup searches each live segment's
range and fixed-size filter, reads at most one bounded block per candidate segment, and accepts a
location only after the selected pack footer reproduces the exact typed key, encoded length,
checksum, domain, and descriptor. `GraphRepository` additionally reads the current `HEAD` closure
through that path before returning a healthy repository. Catalog disagreement is derived
corruption and may not make unavailable or wrong semantic bytes appear valid.

Each accepted pack set creates one level-zero delta directly from the sealed pack metadata already
in memory. Equal levels merge as a deterministic binary counter through streaming sorted readers;
the healthy path neither materializes the complete catalog nor rewrites every prior entry per
batch. New segment files and their directory are durable before the manifest selects them. The
manifest is durable before a new `HEAD` may require the objects. Only after manifest durability may
exact unselected segment and owned staging paths be removed.

Missing, predecessor-contract, malformed, current-closure-incomplete, or lookup-inconsistent
catalog state is rechecked under the repository's exclusive publication lock and reconstructed at
most once from strict immutable pack footers. Reconstruction uses a disjoint sorted footer oracle,
atomically publishes only contract 2, then retries current-closure validation. Pack or accepted
object corruption rejects with `HEAD` unchanged. Contract 1 has no healthy reader or writer; its
bytes are merely one disposable recovery trigger.

Current decoder and resource bounds are 8,000,000 entries, 100,000 packs, 32 segments and levels
0–31, 125,000 blocks, 64 entries and 128 filter bytes per block, 64 KiB per manifest, 64 MiB of
metadata and 1 GiB per segment, and 128 classified derived leftovers. Counts, lengths, offsets,
arithmetic, names, file types, order, ranges, checksums, trailing bytes, and exact selected paths are
validated before allocation, lookup, merge, cleanup, or publication.

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

`PACKAGE-TRANSPORTS/CURRENT` is one atomic operational readiness inventory of exact logical
package revisions and physical transports. Its version binds the code-complete validation contract.
Canonical source objects live in the ordinary immutable pack owner, not shadow projects or separate
editable repositories. The strict uncompressed container carries exactly one current transitive
closure, including private implementations and current retirements, without historical bodies,
operational data, grants, host paths, compiler caches, or executable units.

Staging fully reconstructs and validates the entire container before installation, synchronizes
immutable material, verifies its installed bytes, and exposes one complete readiness inventory.
The input read, interface and semantic validation, existing-object comparison, and durable readback
share the fixed aggregate admission. Failure before visibility preserves prior readiness and HEAD;
owned stages are removed, while unreachable immutable objects may remain. Lost acknowledgement
after visibility is reported separately, and exact restaging identifies the complete staged result.
Readiness is not a semantic dependency binding or a name registry. Acceptance rechecks the complete
exact union and canonical availability under the publication lock. Missing or corrupt canonical
source requires exact restaging, never a compiler-cache, embedded-revision, or checkout fallback.

Predecessor per-revision selection/candidate files and bare interface-pack public input reject.
Physical migration does not allocate semantic, package, or logical-revision identities. The embedded
standard supplies ordinary exact source through this same owner and admission contract.

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
