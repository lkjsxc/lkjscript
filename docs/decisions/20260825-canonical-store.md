# Canonical semantic store and immutable packs

Date: 2026-08-25 UTC.

## Status

Accepted and implemented in the normalized repository path. Public bootstrap uses this path, but
the remaining released commands have not all cut over, so this record makes no complete public
workflow or production-portability claim. For the normalized path, this record supersedes the
physical-store selections in `20260822-persistent-graph-root.md` and
`20260822-object-packing-retention.md`; those records describe predecessor checkout history.

## Problem and authority

Accepted semantic meaning must survive page splitting, pack assembly, catalog rebuilding, cache
changes, and validation upgrades. At the same time, sparse publication needs authenticated local
reads and writes, and a copied binary must reopen durable authority without an external database.

The selected authority is a small accepted revision binding a storage-independent semantic-state
digest and a physical semantic-root locator. Canonical owner, type, dependency, retirement, and
history objects are immutable. Persistent-map pages and immutable packs carry those objects, but
page roots, pack identities, offsets, target sizes, catalog generations, and staging names are not
semantic meaning.

## Decision

- Each persistent map carries a `MapContentRoot`: an entry count and domain-separated digest of a
  canonical logical radix commitment over sorted key/value records. It excludes page envelopes,
  page split thresholds, and child coordinates.
- The page tree separately authenticates physical navigation. Every page commits its logical
  summary, bounded canonical payload, child page digests, child counts, and child summaries.
  Point reads verify each traversed physical link; complete reconstruction is the independent
  oracle for the logical commitment.
- Semantic state commits package identity and presentation plus logical owner, dependency, and
  retirement content roots. It excludes repository location, physical page roots, packs,
  catalogs, witnesses, indexes, caches, and receipts.
- Immutable objects are grouped into bounded deterministic packs. Pack indexes, per-entry checks,
  complete-pack checks, strict offsets and lengths, and typed object domains are verified before
  use. Packs are never mutated after publication.
- The object catalog is derived. Open scans exact pack footers, independently rebuilds the catalog,
  and may continue from the rebuilt in-memory catalog if persisting the acceleration fails. A
  missing or corrupt catalog cannot redefine or hide authority.
- Publication writes immutable semantic and evidence objects into private stages, seals and
  synchronizes required packs, publishes their directory entries, then atomically advances HEAD
  only after the exact base is rechecked. A failure before HEAD leaves the prior accepted state;
  a response is emitted only after the new binding is durable or an indeterminate durability error
  is reported for reconciliation.
- Physical limits remain typed implementation or decoder bounds. The current target pack size is
  an operational parameter, not a semantic rule.

## Alternatives

- One file per logical object was rejected as the normalized default because inode and metadata
  growth are avoidable physical costs.
- A mutable database or ever-growing in-place pack was rejected because crash recovery and
  immutable reachability would depend on mutable page state.
- Physical `MapRoot` or pack identity in semantic revision identity was rejected because repacking
  or page-layout changes would rename unchanged meaning.
- A catalog as authority was rejected because catalog loss or corruption must be independently
  recoverable from immutable packs.
- A monolithic semantic snapshot was rejected because sparse edits and point reads would require
  whole-project rewriting or decoding.
- Hashing only a sorted byte stream without authenticated local summaries was rejected because it
  would make each sparse publication recompute the complete commitment.

## Evidence

The persistent-map tests compare incremental summaries with a separately implemented full sorted
oracle across alternate page splits, broad batches, randomized mutations, insertion orders,
prefix keys, removals, and sparse path copies. They reject malformed envelopes, foreign digest
tags, false root/child summaries, missing pages, trailing bytes, and digest mismatches.

The storage tests cover deterministic multi-domain packs, strict offset/length/order decoding,
duplicate physical objects, payload corruption, footer scans, symlink rejection, catalog rebuild,
and every retained pack-seal interruption checkpoint. Publication tests exercise stale bases,
idempotent retry, concurrent readers, path-copied maps, and interruptions before and after the HEAD
visibility boundary. Commit `ba833645` binds semantic revision identity to logical map content and
recreates maintained authority under the current page/root contracts.

This evidence establishes correctness and crash behavior for maintained fixtures. It does not yet
establish ideal pack sizing, million-owner complete-workflow performance, bounded backup key
inventory, cross-platform durability, or safe canonical garbage collection.

## Consequences

Semantic revisions can remain equal across repacking and validation-evidence replacement, while
revision records still locate the exact physical and evidentiary objects used for acceptance.
Sparse updates pay for their changed map paths and object frontier plus pack/catalog publication.
Deep verification remains intentionally broad and can detect damage outside paths touched by an
ordinary read.

Catalogs and compiler/query caches may be discarded and rebuilt. Canonical deletion remains
disabled until exact roots, active-reader pins, registered backup roots, an independent
reachability oracle, reviewed deletion plans, and interruption-safe reclamation all exist.

## Reversal condition

Replace the radix commitment or pack layout only when complete workflows show a material
correctness, locality, or resource benefit. A replacement must preserve storage-independent
semantic state, strict typed decoding, immutable reachability, an independent rebuild path, and
old-or-new atomic visibility. A physical parameter or faster microbenchmark alone is not a reason
to change semantic identity.
