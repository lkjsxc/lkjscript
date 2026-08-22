# Semantic storage

Status: normative for graph contract 1.

## Physical model

The selected model is a hybrid current root plus content-addressed immutable history. A small HEAD
is the single visibility point. Roots, module shards, revision records, receipts, and dependency
package objects are immutable and sharded by digest prefix. Drafts and query indexes are separate
operational records. Logical graph ownership does not depend on this layout.

The store root is `.lkjscript/meaning`:

```text
HEAD
LOCK
objects/modules/PP/DIGEST.lkjm
objects/roots/PP/DIGEST.lkjr
revisions/PP/REVISION.lkjv
receipts/PP/DIGEST.lkjt
artifacts/PP/DIGEST.lkja
drafts/PP/DRAFT.lkjd
indexes/PP/REVISION.lkji
indexes/PP/REVISION/local-manifest.lkix
indexes/PP/REVISION/owners/BB.lkix
indexes/PP/REVISION/names/BB.lkix
```

`LOCK`, `drafts`, and `indexes` are local/derived by default and ignored by Git. Canonical HEAD,
objects, revisions, receipts, and needed dependency artifacts are transportable authority.

## Packed envelope

Every packed object begins with an eight-byte domain magic, little-endian envelope version 1, a
checked little-endian 64-bit payload length, the bincode-2 payload using little-endian variable
integer encoding, and a 32-byte BLAKE3 domain-separated checksum over the exact header and payload.

Decoding checks the file bound before allocation; magic, version, exact length, checksum, typed
identity tags, closed enum values, contract version, sorted uniqueness, and semantic shape. Unknown
contracts, duplicates, excess, malformed values, checksum mismatch, and trailing bytes reject.
The global packed payload ceiling is 128 MiB; each object type selects a smaller or equal limit.

Object names are typed digests of complete packed bytes. Digest domains for modules, roots,
receipts, records, artifacts, backups, indexes, transactions, diffs, and identities are distinct.
A content digest proves equality only in its domain. It does not prove authority, provenance,
freshness, permission, or revision visibility.

## Publication

Writers acquire the repository lock and reread HEAD. A proposal whose exact base is not current
returns stale without writes to authority. The publisher validates and canonicalizes the complete
candidate, writes dependency artifacts, modules, root, receipt, and revision as immutable files,
makes every object and directory entry durable, writes a unique HEAD stage, syncs it, atomically
renames it over HEAD, and syncs the store directory. Linux batches object durability with `syncfs`
after closing all object files; other targets synchronize each file. HEAD is never included in that
batch and remains the separately synchronized visibility point.

An interruption before HEAD replacement leaves only unreachable immutable objects. An interruption
after replacement exposes the new complete revision. Existing immutable paths must contain equal
bytes; conflicting bytes are corruption. Cleanup never removes a reachable object. A visibility
sync failure is classified as indeterminate and requires HEAD/receipt reconciliation.

Initial creation uses a sibling `.meaning-stage-*` directory with the complete store layout and
atomically renames it into place. Restore uses the same staged-directory rule.

## Reconstruction and indexes

Deep reconstruction starts at HEAD, verifies every reachable parent record and receipt, decodes
each root/module, checks all object-key digests, validates graph identity shape and tombstones, and
runs semantic reconstruction. It does not trust query indexes.

The broad query index is a revision-bound packed derived object containing ordered owner summaries,
semantic owner projections, module facts, sorted relations, and incoming/outgoing adjacency. A
local manifest and 256 deterministic owner and name buckets provide exact lookups without loading
that broad object. Every key binds revision, root, package, repository, and index contract. The
local manifest publishes last; an absent or corrupt expected bucket invalidates the local cache and
rebuilds it. Missing, corrupt, foreign, or stale index bytes are discarded logically and rebuilt
from canonical objects. Index loss cannot make a valid revision unavailable. Disposable index
writes do not receive canonical durability synchronization.

## Backup and restore

A backup locks a single observed HEAD and includes every revision in its reachable DAG, every
bound receipt/root/module, the complete exact dependency artifact closure, and retained drafts. It
uses a sorted unique key table and its own packed/checksummed contract. The receipt binds repository
ID, revision, backup digest, entry count, draft count, and byte count.

Restore accepts an existing empty project directory, verifies the entire backup and every entry,
reconstructs the exact authority in a private stage, runs deep doctor there, and only then renames
the store into visibility. Repository IDs, stable semantic IDs, revision IDs, and history are
preserved exactly. Importing a graph artifact is intentionally different: it creates new history
from a history-free package snapshot.

## Threat model

Persisted bytes and backup/artifact inputs are hostile decoding boundaries. Store traversal uses
fixed domain directories and canonical hex keys. Reads require regular files; layout creation and
restore reject symlinks and incompatible existing types. Temporary names use fresh typed IDs.
Permission and durability behavior is the local-filesystem guarantee reproduced by tests; network
filesystems, hostile concurrent filesystem principals, encrypted storage, and distributed
consensus are not claimed.

## Retention and compaction

All revisions reachable from HEAD are retained. Immutable identical module bodies are shared by
digest across revisions. Query indexes are disposable and may be removed at any time. Contract 1
does not yet expose canonical-history pruning, garbage collection, or segment repacking; those are
explicit current limits, not implicit retention behavior. Any future implementation must preserve
every retained revision/draft/receipt and prove deep reconstruction before and after layout change.
