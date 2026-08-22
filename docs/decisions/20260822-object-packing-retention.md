# Object packing, retention, and small-project policy

Date: 2026-08-22 UTC.

## Status

Accepted decision to defer physical packing, compaction, garbage collection, and history pruning.
No measured current small-to-million-owner comparison supports choosing a live-store pack layout.
Backup contract 4 is a segmented directory that copies payload objects one at a time, but it
retains the complete sorted key set in memory, preserves one file per canonical object, and is not
a live object-pack implementation.

## Current physical reality

Graph contract 3/root storage 2 stores separate immutable files for module objects, persistent-map
pages, root manifests, revision records, receipts, and exact dependency artifacts. It also stores
content-addressed semantic summaries and revision-bound semantic/query indexes as disposable files.
The persistent-map update uses an in-memory overlay while preparing new pages, then publishes the
new reachable immutable pages; it is not an on-disk pack or retained overlay log.

Backup and restore copy and verify canonical objects one at a time under a bounded manifest and
bounded index segments. This removes the predecessor single-container allocation and 128 MiB total
bundle ceiling. It does not provide live-store packing, compaction, GC, or retained evidence for a
backup larger than that predecessor bound.

All accepted ancestors reachable from HEAD are retained. Every live draft additionally retains its
base parent DAG. Identical immutable content shares a digest. Retention contract 1 implements a
read-only `doctor cleanup` inventory over those roots and reports retained/reclaimable candidate
counts, reclaimable bytes, derived counts/bytes, unknown-entry counts, and a plan digest. It always
reports `destructive_ready: false`.
There is no public revision pin, history-pruning policy, garbage collector, segment catalog,
compactor, or deletion path for canonical objects. Derived indexes may be removed and rebuilt;
canonical files may not be deleted as though they were caches.

The current layout is simple at the maintained 3- and 12-module scale. Its inode, directory,
metadata, open, synchronization, backup, restore, and long-history behavior has not been measured
at one million owners. Persistent-map correctness and local page writes do not establish that
one-file-per-object is the right large-scale physical layout.

## Authority and invariants for future selection

Logical module/page/root digests and accepted revision identity must remain independent of physical
pack coordinates. A future layout may use immutable packs or segments only if it preserves:

- exact content-addressed logical object identity and strict bounded decode;
- immutable reachable bytes and one durable HEAD visibility point;
- deterministic lookup, iteration, backup, restore, and deep reconstruction;
- safe duplicate handling, truncated-index rejection, and interruption recovery;
- small-project startup/edit/build behavior; and
- the ability to rebuild operational catalogs without changing accepted meaning.

Pack offsets, filenames, compression, segment generations, and lookup catalogs remain physical or
operational state. They cannot enter semantic references or revision meaning.

## Retention prerequisites

The current preview enumerates HEAD and live-draft base DAGs, revision records and receipts,
canonical roots/pages/modules, exact dependency closures, and draft records. No destructive
cleanup may be implemented until revision pins, active-reader leases, and registered backup roots
are explicit authority as well. Logical history pruning and physical repacking are separate
operations.

The first destructive command must revalidate an exact preview under the exclusive lock, add an
implementation-disjoint reachability oracle, inject interruption before and after its visibility
switch, and prove that every retained revision, artifact, query, and restored authority is
unchanged. Deletion is recoverable only from another retained root or a verified external backup
and must say so.

## Evidence gate and rejected shortcuts

Compare current separate files with at least two credible immutable pack/segment layouts across 3-
and 12-module projects and 10,000, 100,000, and one-million-owner sparse, dense, high-fanout,
large-value, and long-history topologies. Measure files/inodes, opens, metadata operations, fsyncs,
random reads, cold open, local update, backup, restore, CPU, RSS, and bytes. Retain packing only if
complete workflows materially improve without unacceptable small-project overhead.

Rejected without that evidence are one mutable ever-growing pack, in-place mutation of reachable
bytes, one sealed pack per edit, pack coordinates as identity, backing up disposable indexes as
authority, automatic history deletion, cleanup without dry run, and raising monolithic limits as a
substitute for bounded-memory transfer or sharding.
