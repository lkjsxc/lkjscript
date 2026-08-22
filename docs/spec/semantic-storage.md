# Meaning-graph storage

Status: normative for meaning graph contract 4 and root storage contract
`lkjscript-persistent-root-2`.

## Physical model and authority

A small HEAD is the single accepted visibility point. Immutable roots, persistent map pages, module
objects, revision records, receipts, and exact dependency package objects reconstruct accepted
authority. Drafts are separate non-executable authority. Query indexes, semantic summaries,
compiler caches, lock state, and logs are operational or derived and cannot change accepted
meaning.

The store root is `.lkjscript/meaning`:

```text
HEAD
LOCK
objects/modules/PP/DIGEST.lkjm
objects/roots/PP/DIGEST.lkjr
objects/map-pages/PP/DIGEST.lkjp
revisions/PP/REVISION.lkjv
receipts/PP/DIGEST.lkjt
artifacts/PP/DIGEST.lkja
drafts/DRAFT.lkjd
indexes/PP/REVISION.lkji
indexes/PP/REVISION/local-manifest.lkix
indexes/PP/REVISION/owners/BB.lkix
indexes/PP/REVISION/names/BB.lkix
indexes/PP/REVISION/facts.lkix
indexes/semantic/pages/PP/DIGEST.lksp
indexes/semantic/summaries/PP/DIGEST.lkss
```

`LOCK` and indexes are local operational/derived state. Drafts are local non-executable authority.
HEAD, reachable immutable objects, revision records, receipts, required dependency artifacts, and
retained drafts are transportable authority. Filesystem location and page coordinates are not
semantic identities.

## Persistent root

The accepted root object is a bounded `StoredGraphRoot` manifest. It binds storage and graph
contracts, repository/package identity and name, and six `MapRoot` values: modules keyed by module
ID, module names to module IDs, dependencies keyed by package ID, dependency aliases to package
IDs, targets keyed by target ID, and typed tombstones. Each `MapRoot` binds one page digest and
entry count.

Map pages implement a canonical path-compressed Merkle radix map. A subtree is a leaf when its
canonical encoding fits the 16 KiB target or holds one record; otherwise it branches at the first
byte following the longest common prefix. Equal sorted key/value sets therefore produce equal page
and root digests regardless of insertion order. Pages are immutable and content-addressed.

Storage boundaries limit keys to 256 bytes, values to 48 KiB, and hostile page inputs to 64 KiB.
These limits force larger values into separately addressed objects; they do not impose a module
count on language meaning. A root manifest is at most 64 KiB.

A local root update computes `StoredGraphRootDelta` from logical roots and path-copies affected
map branches in an overlay page store. The overlay retains every generated page, including exact
physical reuse. Extraction starts at each changed final map root, visits only generated reachable
pages, and treats an absent staged digest as an unchanged accepted-base subtree. A map whose final
root equals its accepted base root requires no extraction. Exact module ID/name and dependency
ID/alias lookup follows the appropriate map path. Deterministic exhaustive reconstruction and
full-build equality remain the oracle for delta operations. Eligible pure-body replacement,
independent empty-module creation, module rename, and declaration rename can derive bounded deltas.
A missing disposable index and every fallback transaction still reconstruct and clone the logical
graph before deriving the delta; persistent physical locality does not imply a fully incremental
semantic engine.

## Packed objects and integrity

Ordinary packed objects use an eight-byte domain magic, little-endian envelope version 1, checked
little-endian 64-bit payload length, canonical bincode-2 little-endian variable-integer payload, and
a 32-byte domain-separated BLAKE3 checksum over header and payload. Persistent map pages use their
own fixed, versioned, domain-separated canonical envelope and checksum.

Decoding checks the owning byte bound before length-directed allocation, then checks magic,
version, exact length, checksum, typed identity tags, closed enum values, contract version, sorted
uniqueness, and semantic shape. Unknown contracts, duplicates, malformed values, checksum mismatch,
and trailing bytes reject. The shared packed-decoder payload ceiling is 128 MiB; object-specific
bounds may be smaller. This hostile single-object ceiling is not a public project-size promise.

Object names are typed digests of complete bytes. Module, root, map-page, receipt, revision,
artifact, backup, index, transaction, diff, and identity domains are distinct. A digest proves
equality or integrity only in its exact domain, not provenance, authority, freshness, permission,
or visibility.

## Preparation and publication

The common change path prepares semantic validation once before locking publication. Four
precondition-free transaction classes may prepare locally: eligible pure-function body
replacement validates selected modules and their recursive local import dependencies; independent
creation validates new empty modules; and module rename validates renamed modules plus their
outgoing import dependencies without rewriting importers or targets. Declaration rename validates
owning modules plus outgoing imports without rewriting exact-reference callers. Preconditions,
mixed operations, and all other requests reconstruct current logical meaning, apply operations,
canonicalize relations, and fully validate the candidate. The resulting prepared validation binds:

- exact expected revision and base root;
- exact result root;
- canonical root delta;
- changed module objects;
- changed semantic summaries and path-local edits to three persistent semantic-fact maps;
- a revision-independent semantic certificate for the exact fact roots; and
- validation facts.

Publication acquires the repository lock, rereads the current binding, and rejects a stale base
before replaying the root delta. It verifies that the delta result, semantic-diff binding, summary
delta, and semantic certificate equal the prepared values and reuses the prepared validation
facts; it does not repeat semantic validation. Unprepared internal publication paths retain
complete validation. A root delta contains typed logical values, never caller-selected page
digests. Generated parent/child links and bytes are checked while extracting staged pages. Reused
subtrees inherit exact references from digest-checked accepted-base pages under the same exclusive
lock and are not reopened merely to reprove the complete retained store.

The publisher writes newly required dependency artifacts, changed module objects, new map pages,
the fixed root manifest, receipt, and revision as immutable files. It also writes disposable
content-addressed summary objects, semantic-fact pages, and the revision-bound fact manifest. New canonical
bytes and directory entries become durable before a unique HEAD stage is synchronized and
atomically renamed over HEAD. Linux batches immutable-object durability with `syncfs`; other
targets synchronize individual files. HEAD remains the separately synchronized visibility point.

An interruption before HEAD replacement can leave only unreachable immutable bytes. An
interruption after replacement exposes the complete new revision. An existing immutable path must
contain equal bytes or corruption is reported. A visibility-sync failure is indeterminate and
requires HEAD/receipt reconciliation.

The repository itself has no canonical deletion path, and future cleanup or compaction must share
the publication lock. Under that model, an accepted-base subtree cannot disappear during a local
publication. External filesystem damage outside repository operations may remain unobserved when an
untouched subtree is structurally reused; the first read of the damaged path or exhaustive deep
doctor rejects it. This is a stated local-store integrity assumption, not a claim that every write
rescans all accepted bytes.

Initial project creation constructs a complete store in a private sibling stage and exposes the
destination through one rename. Restore uses a private store stage under the selected destination
and makes verified authority visible only after complete reconstruction.

## Reconstruction and derived state

Deep doctor starts at HEAD, verifies reachable parents and receipts, decodes each root and all
referenced map pages/modules, checks object-key digests, reconstructs logical root sets, validates
root/module identity and tombstone shape, and loads or rebuilds the current query and semantic
indexes. It does not trust disposable index or summary bytes: a rebuilt semantic certificate must
equal the value in the current revision. It does not currently rerun complete cross-package
semantic validation for every historical revision. Initial publication runs complete direct and
packed validation; focused differential tests retain the full validator for local changes. Restore
runs deep structural/history verification but does not rerun complete cross-package semantic
validation.

The query system stores a revision-bound broad relation index plus exact-index contract 3. The
exact index consists of revision-independent content-addressed owner and name shard objects and a
revision-bound manifest with 256 optional owner digests and 256 optional name digests. The manifest
binds repository, package, revision, canonical root, owner count, graph contract, and index
contract. A local accepted change derives touched buckets from old and new module projections,
rewrites only changed content objects, and reuses all other digests. Initial and complete-candidate
publication may seed all exact shards from graph values already in memory. Shards are written and
verified before the manifest; a failed disposable write never changes accepted publication.

A missing, stale, foreign, predecessor, or corrupt manifest or shard invalidates derived state and
triggers reconstruction from canonical authority. Rebuild output must equal delta output for the
same revision. Disposable index writes are not canonical publication. The broad relation index is
still rebuilt lazily rather than updated by accepted deltas.

Semantic-summary contract 2 defines integrity-bound per-module facts under validator contract 2.
Module summaries bind module object, package, validator, exact input, signatures, implementations,
effects, and dependency edges. Semantic-fact contract 3 binds summary input/content digests,
graph-owned test owners, and flat typed reverse edges in three persistent maps. Summary objects are
content-addressed under `indexes/semantic/summaries`, map pages under `indexes/semantic/pages`, and
one `facts.lkix` manifest is bound to each accepted revision and canonical root. Local transaction
paths replace or remove exact keys and path-copy changed map branches. Missing or malformed cache
state rebuilds from canonical modules; a certificate mismatch against the revision core is
canonical corruption. These bytes remain derived acceleration and cannot alter accepted meaning.
The implemented frontier is not yet used to select general validation.

## Backup, restore, retention, and recovery

Backup locks one observed HEAD and includes the reachable revision DAG, receipts, stored roots,
map pages, modules, exact dependency artifact closure, and retained drafts under a globally sorted
unique key table. Backup contract version 4 publishes a directory with `MANIFEST.lkjb`, bounded
checksummed index segments, and individually copied canonical objects. The manifest binds segment
order/digests/counts and aggregate payload bytes; each segment binds the exact key, length, and
digest of every entry. Backup and restore process entries one at a time instead of allocating one
complete bundle value, while retaining the complete O(object-count) sorted key set in memory. The
manifest is bounded to 32 MiB, each index segment to 4 MiB and 4,096 entries, and retained history
traversal to 10,000 revisions. Those hostile/implementation bounds do not establish a tested
maximum total backup size or fully bounded memory.

Restore accepts an explicit destination without current authority, verifies the manifest,
consecutively ordered segments, every entry, and the exact recomputed retained closure; it
reconstructs authority in a private stage, runs deep object/history and draft validation, and only
then exposes it. Missing, reordered, corrupt, or predecessor monolithic inputs reject before
visibility. Repository/stable/revision identities and history are preserved. Current
restore does not rerun the complete cross-package semantic validator; that is an implementation
limit rather than a weaker meaning-graph invariant. Package staging is intentionally different: it
verifies a package artifact as unreachable dependency data and cannot publish HEAD.

All revisions reachable from HEAD are retained. Each live draft additionally retains its base and
that base's parent DAG. Identical immutable content is shared by digest. Retention contract 1
exposes `doctor cleanup`, an exact read-only inventory that compares those roots with canonical
store files, reports retained/reclaimable candidate counts and bytes, derived counts/bytes, unknown
entry counts, and an integrity-bound plan digest. It always reports `destructive_ready: false` and
identifies missing revision-pin, active-reader-lease, and registered-backup-root authority.

No public history pruning, garbage collection, canonical deletion, or segment repacking exists.
Derived indexes may be removed and rebuilt; canonical objects may not be deleted by the preview or
treated as caches.

## Security assumptions, failure classes, and non-goals

Persisted bytes, artifacts, and backups are hostile decoding boundaries. Traversal uses fixed
domain directories and canonical keys; reads require ordinary non-symlink files. Creation, output,
and restore reject unsafe path types and use fresh private stages.

Missing or malformed canonical bytes observed on a selected base path or generated closure are
corruption and block writes. External damage in an untouched structurally reused subtree may remain
latent until that subtree is read or deep doctor walks it. Missing derived bytes are rebuildable.
Stale base, invalid graph, resource exhaustion, corruption, cancellation, and
infrastructure/durability failure remain distinct.

The guarantee assumes a trusted local operator and a filesystem that honors the documented
operations. Network filesystems, a hostile concurrent filesystem administrator, encrypted graph
storage, artifact signatures, distributed consensus, and multi-node publication are not claimed.
Physical page or pack coordinates never become authoring syntax.
