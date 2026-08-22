# Canonical persistent graph root

Date: 2026-08-22 UTC.

## Status

Accepted and implemented by meaning graph contract 4 and root storage contract
`lkjscript-persistent-root-2`. The predecessor flat root and root-storage-1 reader are not current.
Complete million-owner workflow evidence and a second measured persistent-structure comparison are
still absent, so this record makes no large-scale latency claim.

## Implemented decision

An accepted revision binds one small `StoredGraphRoot` manifest with repository/package metadata
and exactly six typed persistent maps:

1. module ID to module object reference;
2. module name to module ID;
3. package ID to exact dependency binding;
4. dependency alias to package ID;
5. target ID to exact target binding; and
6. typed deleted identity to tombstone.

These are canonical accepted root data, including the name and alias maps. Query indexes and
semantic-fact maps are separate disposable acceleration. Module objects remain independent
content-addressed semantic/validation units.

Every map is a canonical immutable path-compressed Merkle radix tree over bounded byte keys and
values. A subtree becomes a sorted leaf when its canonical encoding fits the target or holds one
entry; otherwise it branches at the first byte after the longest common prefix. Branch edges are
sorted and unique. Equal logical entries produce equal page and root digests independent of
insertion history, and canonical iteration follows key order rather than hash, filesystem, or edit
order.

Mutation uses an overlay page store and path-copies changed branches. Exact module ID/name and
dependency ID/alias lookup follows one map path. Root delta application updates both sides of each
exact identity/presentation pairing and checks deterministic equality with full root rebuilding in
tests. The overlay records every generated page, including exact physical reuse, so an interrupted
old publication cannot make a reused generated ancestor hide a newly required child. Final
extraction traverses generated pages reachable from changed map roots only, verifies their digests
and generated parent/child links, and omits intermediate mutation roots. Unchanged accepted-base
subtrees are structurally reused under the exclusive publication lock. Required generated pages are
immutable, digest checked, and made durable before HEAD changes.

## Reference locality

Graph contract 4 imports bind exact package/module IDs and targets bind exact
module/component/port IDs. Module rename therefore updates the module object plus the module-name
map without rewriting importers or targets. Declaration exports and references bind exact
package/module/declaration IDs; declaration rename likewise updates only the owning module and
derived persistent paths.

Four precondition-free transaction classes currently exploit the root directly: eligible
pure-body replacement, independent empty-module creation, module rename, and declaration rename.
Every preconditioned, mixed, or other change may still reconstruct and clone complete logical vectors before producing a
root delta. Persistent physical locality is therefore necessary current infrastructure, not proof
of a general incremental semantic engine.

## Integrity, reconstruction, and recovery

- A page binds exact map contract, path prefix, sorted leaf entries or unique branch links, entry
  count, and domain-separated digest. Missing, oversized, noncanonical, truncated, trailing, or
  digest-mismatched reachable pages are canonical corruption.
- Storage boundaries limit keys to 256 bytes, values to 48 KiB, target leaf pages to 16 KiB, and
  hostile page inputs to 64 KiB. These are representation/decoder bounds, not a module-count
  semantic limit.
- Deep doctor walks the accepted revision DAG, roots, pages, modules, receipts, and object-key
  bindings and reconstructs logical roots. It does not trust query or semantic index bytes.
- Backup contract 4 includes reachable canonical map pages. Restore verifies them in a private
  stage before authority becomes visible.
- Equal child digests permit structural reuse and diff skipping. Deep reconstruction remains the
  explicit broad recovery and correctness path.

## Evidence and limits

Persistent-map unit/property tests cover insertion-order equality, thousands of operations against
`BTreeMap`, prefix keys, removal collapse, no-change reuse, exact/digest-skipping diff, bounded
fixture writes, staged/exhaustive parent-link validation, interrupted orphan-page reuse, and
missing/foreign/corrupt/trailing inputs. Repository tests cover root-delta/full-build equality,
physical page/byte reads and retained pages for one local update over 10,000 and 100,000 modules,
local module creation and rename, backup/restore, predecessor rejection, and deep doctor.
Maintained standard and `lkjournal` authorities use graph 4/root 2.

The 10,000/100,000 evidence is an in-process root property test, not a complete public workflow.
Evidence still does not cover one million current logical owners, adversarial key distributions in
complete workflows, crash injection at every page/publication boundary, or a measured B+ tree/HAMT
alternative. Current pages and module versions are separate files; immutable packing, retention
pruning, compaction, and garbage collection are not implemented. External damage to an untouched
accepted-base subtree is detected on later read or deep doctor rather than by every local publish.

## Rejected alternatives and reversal gate

The monolithic sorted root vector is rejected because local updates and exact lookup scale with all
modules. Mutable database pages, hash iteration, pack coordinates, and edit-history-dependent tree
shape are rejected as canonical identity. One filesystem object per page is not selected as the
long-term million-owner layout; it is only current storage pending measured packing evidence.

A future canonical-map replacement requires a direct graph/storage contract cutover and complete
consumer reconstruction. Physical packing may change independently only when logical object bytes,
page/root digests, revisions, deterministic iteration, strict decode, structural sharing, and
recovery remain exact. No alternate reader may remain as a compatibility fallback.
