# Incremental rebuildable object catalog

Date: 2026-09-03 UTC.

## Status

Accepted and implemented by campaign `202609031354` for unreleased product snapshot 0.1.19. This
record refines the derived-catalog portion of
[`20260825-canonical-store.md`](20260825-canonical-store.md); immutable pack contract 1, object-store
contract 1, Graph 7 meaning, and atomic `HEAD` authority remain unchanged.

## Problem and authority

Catalog contract 1 encoded one complete object-to-pack map. Every process open independently
enumerated and decoded every pack footer before consulting it, and every accepted pack set repeated
that reconstruction and rewrote the complete map. The work was crash-safe but physical cost grew
with all prior packs rather than the accepted delta. A valid million-owner public lifecycle was
therefore dominated by disposable locator work.

Immutable typed objects and packs remain the recovery authority for physical locations. `HEAD` and
its exact validated immutable closure remain the only accepted semantic visibility. A catalog may
accelerate finding those bytes, but neither its manifest, segments, generations, ranges, filters,
work counters, paths, nor commitment may select meaning.

## Decision

- Catalog contract 2 consists of one atomically replaced `catalog/current.lkjc` manifest and
  immutable content-addressed sorted files beneath `catalog/segments/`.
- A segment binds contract, level, generation, exact count and key range, canonical locations,
  referenced pack descriptors, fixed-size block summaries and filters, checksums, and a closing
  marker. The manifest binds one segment per level, all selected segment digests, aggregate counts,
  cumulative physical work, and a packing-independent logical catalog commitment.
- Healthy open reads only the bounded manifest and selected metadata. Lookup uses ranges and filters
  to read at most one 64-entry block per candidate segment, then verifies the exact selected pack
  footer and typed entry before returning bytes.
- Each accepted pack set produces one level-zero delta from sealed metadata already in memory.
  Equal levels merge deterministically as a binary counter through streaming readers. Segment
  identity may vary with physical placement; the aggregate logical commitment may not.
- Segment bytes and their directory become durable before a manifest references them. The manifest
  becomes durable before a new `HEAD` may require the objects. Exact unselected segments and owned
  stages are removed only after that manifest durability point.
- Missing, contract-1, malformed, stale, or current-closure-incomplete catalog state is rechecked
  under the exclusive repository lock, reconstructed at most once from immutable pack footers,
  atomically published as contract 2, and validated again. Canonical pack/object corruption rejects
  and never advances `HEAD`.
- Contract 1 is not a compatibility input. Its healthy decoder, writer, dual-reader path, and
  whole-catalog-per-seal publication are deleted; one synthetic predecessor fixture proves that
  first open treats it as disposable and rebuilds.

## Bounds and observation

The current owner admits at most 8,000,000 entries, 100,000 packs, 32 live segments/levels,
125,000 blocks, 64 entries and 128 filter bytes per block, a 64 KiB manifest, 64 MiB of metadata and
1 GiB per segment, and 128 classified derived leftovers. Decoding validates exact lengths, counts,
ranges, order, arithmetic, names, file types, checksums, and trailing input before use.

Contributor evidence records healthy-session metadata/lookup/footer work and persistent delta,
merge, reconstruction, and scan history. The scale receipt advances independently to contract 3 so
it can bind that observation and a footer-oracle commitment. These counters and receipts are
derived evidence, not runtime telemetry or program meaning.

## Alternatives

- Retaining the monolithic file with a faster decoder was rejected because accepted batches would
  still rewrite all prior entries and process open would still scale with complete catalog bytes.
- SQLite or another mutable database was rejected because its pages, transactions, and recovery
  state would become a second mutable physical authority beside immutable packs.
- A resident daemon or session was rejected because ordinary development must remain correct
  through separate invocations of the distributed executable.
- Mutable in-place segments, pack redesign, pack deletion, compaction, and garbage collection were
  rejected because none is needed to contract locator work and each adds a new recovery or
  authority problem.
- Trusting segment keys without rechecking their exact pack entries was rejected because derived
  corruption could then fabricate or redirect canonical objects.

## Consequences

Normal open cost is bounded by selected metadata, and normal seal cost is one delta plus
deterministic logarithmic merge work. Recovery and deep verification remain intentionally broad and
observable. Interrupted publication may leave bounded derived files or unreachable immutable
packs, but old or new `HEAD` remains complete; a later exact retry or recovery can clean only owned
derived paths.

Catalog arrangement can change without renaming accepted meaning, artifacts, packages, or runtime
inputs. This decision provides no compaction, canonical deletion, mutable semantic database,
encryption, signing, sandboxing, multi-tenant isolation, consensus, reproducible-build,
cross-platform durability, or service-level claim.

## Evidence and reversal condition

Storage codec/lookup/merge tests, repository recovery and interruption tests, maintained-project
verification, the exact copied-binary capacity gate, and the disjoint footer oracle are recorded in
[`202609031354-incremental-object-catalog.json`](../evidence/202609031354-incremental-object-catalog.json).

Replace this structure only when a maintained workload demonstrates a material correctness or
resource failure that bounded immutable segments cannot meet. A successor must keep packs and
accepted `HEAD` canonical, preserve a complete implementation-disjoint rebuild path, validate
hostile bytes before use, provide bounded lookup/update/cleanup, prove old-or-new interruption
visibility, migrate every maintained consumer in one cutover, and delete this reader and writer.
Convenience, conventional database familiarity, or an isolated microbenchmark is insufficient.
