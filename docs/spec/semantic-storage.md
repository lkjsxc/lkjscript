# Semantic repository storage

Status: normative for the current typed meaning graph repository. Internal storage compatibility
identities and hostile decoder bounds remain at their typed source owners.

## Current schema epoch

`canonical_schema::SCHEMA_EPOCH: u16 = 1` is the sole product encoding epoch. Graph,
type, package, compiler, bytecode, artifact, publication, storage and data contract constants
alias it. Every reader admits exactly that epoch; numeric ordering grants no feature support.
Canonical object framing stores the kind, a two-byte little-endian epoch and payload length.
Associated-owned inventories and source-tied read views are inline in declaration and type
records; no optional legacy sidecar or predecessor reader exists.

Source edits do not change the schema epoch or canonical meaning identities. Generated validator
implementation commitments partition process admission and compiler caches only. Independent
content digests, closure checks, canonical re-encoding and complete semantic revalidation still
admit every artifact and source closure. Changing validation code cannot make an old cached proof
current, and a digest alone never supplies semantic acceptance.

The standard transport and artifact are compressed with deflate during the build, decoded once
into shared immutable process storage, and then fully admitted. Public exports are the exact
original uncompressed bytes and retain their normal transport and artifact identities.

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

Historical acceptance within the current epoch and current validation are separate claims. A canonical
historical witness authenticates its original envelope, certificate, semantic root, revision,
parents and idempotency receipt even when its validator differs. Opening it must not replace HEAD,
rewrite history, or invoke catalog repair merely for that difference. Foreign epochs and structural
corruption reject explicitly.

Current execution, semantic export and incremental proof require a current validation context bound
to the same exact root, publication record, dependency closure and validator. Rebuilding that context uses full canonical
validation and does not mint a semantic revision. Derived facts and current proof are distinct: an
invalid historical body may still have authenticated, newly reconstructed names and ownership for
read-only inspection and request resolution. A failed current proof never authorizes execution.

Process proof reuse is limited to 64 entries and 512 MiB of conservatively modeled retained storage.
Every hit rechecks the complete original object read set and exact used dependency transport
selections; a changed selection requires fresh admission. Disk objects are reread and verified.
Weak identities may establish unchanged immutable memory bytes without retaining their project
arena, while charging the same object and byte admission limits. Oversized proofs are not cached.
One owning export shares its complete admitted source and current proof through compilation without
retaining its private dependency maps in the global cache. Prepared execution retains the bounded
read-set metadata and exact acceptance binding. Every invocation rereads and verifies the complete
physical source and used selections, checks cancellation and rejects changed acceptance. This proof
does not depend on global cache capacity or replace independent reference execution.

When no valid current base proof exists, ordinary reviewed edits use complete post-change validation
and fresh witness maps within the existing publication owner. Only the valid candidate can publish;
the lock rechecks the exact historical parent before visibility. Earlier history and idempotency
bindings remain intact. A historical retry returns its original result when it can be revalidated;
otherwise it reports current-validation rejection of that original result, without freeing its key.

A predecessor `.lkjscript` marker is not a storage edition. Project discovery rejects it before
opening derived state or mutating the destination. A root containing both current and predecessor
markers is rejected as ambiguous; it never selects a fallback.

Operational application data is not stored in this root and does not reuse any repository,
revision, pack, object, map, or digest identity. A deployment-selected `lkjscript-data-store`
root has its own physical identity and atomic head as specified in
[data-capabilities.md](data-capabilities.md). Runtime data writes cannot author, select, or advance
semantic `HEAD`; semantic publication cannot silently mutate operational data.

## Immutable objects and packs

Typed canonical objects occupy distinct domains for semantic owners, types, blobs, persistent-map
pages, roots, witnesses, transactions, diffs, receipts, revisions, idempotency records, package
interfaces/transports, compiler units/manifests, and other exact contracts. Object keys verify
domain-separated content bytes. A digest from one domain cannot authorize another.

Packed decoding checks canonical payload bytes directly against the encoder output;
domain owners separately check identity and local meaning. Encoding an artifact
retains its strict standalone admission within that result. Consuming it checks the
complete encoded digest and metadata again; mutated results cannot reuse admission.

Packs are immutable, bounded, canonical collections of keyed objects with checked headers,
footers, order, offsets, lengths, checksums, and closure. Duplicate physical definitions,
noncanonical order, truncation, trailing input, foreign domains, overflow, and digest mismatch
reject. Pack filenames are locators and never semantic identity.

Current physical codecs are raw (header/footer flags 0) and deflate with raw fallback
(flags 1). In the latter, equal encoded/canonical lengths mean raw bytes; shorter
entries contain one zlib-deflate stream. Compression never expands a payload.
Their flags must agree; all other flags reject. Entries separately bind encoded and
canonical lengths. Checksums cover physical payloads; typed identities cover exact
decoded canonical bytes. The domain, caller and aggregate read limits apply to the
decoded length before payload I/O or allocation. Compressed entries
require exactly one complete stream and reject trailing streams,
truncation, expansion beyond the declared size and unknown codecs. Both codecs belong
to the current epoch; neither supplies a predecessor reader.

The current-epoch catalog is the sole healthy physical locator. `catalog/current.lkjc` is an atomically
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

An open store may retain at most 64 completely admitted blocks, keyed by immutable
segment identity and block index, until its catalog and segment handles change.
Full optional retention falls back to ordinary block reads; it does not reject
valid objects or bypass any request allowance. Reuse preserves duplicate-key checks
across live segments. Each object read still checks actual pack length, payload
checksum and typed content identity. Catalog block-read and decoded-entry work
counts describe successful physical reads and full admission, excluding searches
of retained entries. This transient policy changes no persistent format or history.

Each accepted pack set creates one level-zero delta directly from the sealed pack metadata already
in memory. Equal levels merge as a deterministic binary counter through streaming sorted readers;
the healthy path neither materializes the complete catalog nor rewrites every prior entry per
batch. New segment files and their directory are durable before the manifest selects them. The
manifest is durable before a new `HEAD` may require the objects. Only after manifest durability may
exact unselected segment and owned staging paths be removed.

Missing, foreign-epoch, malformed, current-closure-incomplete, or lookup-inconsistent
catalog state is rechecked under the repository's exclusive publication lock and reconstructed at
most once from strict immutable pack footers. Reconstruction uses a disjoint sorted footer oracle,
atomically publishes only the current schema epoch, then retries current-closure validation. Pack
or accepted object corruption rejects with `HEAD` unchanged. No foreign catalog reader is retained.

A healthy read retains its shared publication lock through the catalog/HEAD observation.
If recovery required exclusive ownership, that stronger lock remains owned until the caller
finishes the observation and releases its handle, including on errors. It is not downgraded
between capturing the catalog and reading HEAD: lock-mode conversion is not an atomic
observation boundary. The exceptional recovery path can temporarily exclude other readers;
healthy shared-read concurrency, the one-reconstruction bound and all object checks remain.

Current decoder and resource bounds are 8,000,000 entries, 100,000 packs, 32 segments and levels
0–31, 125,000 blocks, 64 entries and 128 filter bytes per block, 64 KiB per manifest, 64 MiB of
metadata and 1 GiB per segment, and 128 classified derived leftovers. Counts, lengths, offsets,
arithmetic, names, file types, order, ranges, checksums, trailing bytes, and exact selected paths are
validated before allocation, lookup, merge, cleanup, or publication.

Persistent maps use canonical typed keys and bounded values over immutable content-addressed pages.
Equal logical maps have equal logical content commitments independent of page splits. Sparse path
copies can share accepted-base subtrees, but full logical reconstruction remains the independent
semantic oracle.

Point lookups may retain 4,096 decoded pages within a 64 MiB modeled process bound.
Every hit reads current store bytes, verifies their digest and exact equality, charges
the original page/byte/entry admission, and rechecks parent summaries. Only actual
decodes increment `pages_decoded`; independent full traversal bypasses this cache.
Private immutable memory payloads retain their verified object key; replacements
start unverified and every caller still pays its object and byte admission.

## Reachability compaction

`lkjscript gc --project PATH` retains the complete immutable HEAD ancestry, acceptance
receipts, transactions, diffs, supplied types, idempotency maps, semantic objects and
witness maps. Exact operational package selections, their complete source/interface
closures, and the selected compiler manifest/units are additional roots. Embedded
package witness commitments are rebuilt during ordinary package admission; they do
not imply that imported containers store the originating repository's witness maps.
Compiler type tables also contain inferred identities reconstructed by the linker;
their canonical storage roots come from the retained source closure.
Historical binding checks preserve recorded acceptance without claiming current
semantic permission. Application data and unrelated files are outside collection.

Each disk store holds a shared advisory lease on its root directory's open file
description for its lifetime. GC takes the publication lock, then tries an exclusive
directory lease. Existing readers cause `gc_readers_active` without waiting or
changing storage; retry after they finish. These leases require no new lock file.
The in-memory backend instead installs a filtered copy-on-write index atomically;
existing readers/forks keep their original immutable allocations alive.

GC rereads canonical objects and walks complete maps without using semantic proof
caches. Preparation admits at most 1,000,000 distinct objects, 512 MiB of retained
payloads plus modeled per-object overhead, 16,000,000 object reads and 4 GiB of read
bytes. The reference queue is separately limited to 1,000,000 entries. Exhaustion,
missing/corrupt references and cancellation reject before compaction. This finite
preparation capacity does not limit semantic validity or truncate history.

Disk GC partitions deduplicated objects in deterministic key order using a 32 MiB
target for raw payload plus pack framing. Compression uses the current deflate codec
with raw fallback, so a partition cannot grow during encoding. A single admitted
object larger than the target remains an indivisible singleton under the unchanged
object-domain and per-pack bounds; the target is physical output policy, not semantic
capacity. Partitioning happens before per-pack sealing, so retained history does not
need to fit one pack.

GC installs every replacement pack and synchronizes the pack and staging directories,
then durably selects one complete catalog segment before unlinking old owned
packs/segments. Cleanup protects every selected replacement identity, including a
pack that already existed before this collection. It preserves HEAD, publication
identities, operational selections and canonical object bytes.
Interruption before the catalog switch leaves the old selection valid; interruption
after it leaves a complete new selection and possibly extra files for the next GC.
Cleanup errors report failure even if the catalog switch already happened. A compact
single-line JSON receipt reports retained/reclaimed object counts and physical pack
bytes; memory receipts omit physical pack statistics. New publications can append
ordinary raw packs until the next explicit collection.

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
multi-tenant isolation, distributed consensus, distributed leases, automatic packing, memory
mapping, or cross-filesystem atomic visibility.

## Scalar and transaction encoding

All current owner, type, compiler and artifact records use schema epoch 1, including transaction
outcomes, symbolic requirements and binary64 values. These forms are ordinary inline tagged
variants; no content-dependent predecessor envelope or feature-generation dispatch remains.
Canonical re-encoding, bounded lengths, exact dependency selections and independent full source/code
validation admit the entire closure, including unused types and untaken expressions.

F64 literal payloads are exactly eight little-endian binary64 bytes. Canonical writers normalize
NaN to `0x7ff8000000000000`; strict scalar readers reject other NaN bit patterns. Decimal formatting
and equivalent proposal spellings do not enter numeric identity. Source and artifact readers
independently check scalar types, expression kinds and instruction correspondence.

A changed implementation may revalidate accepted history within the current epoch without changing
its identities. A foreign epoch rejects before execution or accepted writes. Installation does not
migrate graph authority or operational data.
