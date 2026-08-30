# First-party ordered application-data authority

Date: 2026-08-30 UTC.

## Status

Accepted and implemented by campaign `202608300840`. This decision completes the reversal gate in
`20260828-public-task-capability-storage-seam.md` for maintained production consumers. That older
record remains the historical SQL/PostgreSQL seam decision; it is not current production guidance.

## Problem and authority

The copied BBS and `lkjournal` service/worker required a network PostgreSQL process, SQL-shaped
standard types, application-authored statements and row codecs, deployment secrets, and a second
durable-queue backend. That mechanism was replaceable but prevented the one-binary product from
owning its maintained operational-data boundary.

Program meaning and application data are different authorities. `GraphRepository` must remain the
sole editable authority for typed program meaning. Runtime application facts require their own
deployment-selected identity, transaction, durability, recovery, and resource contract without
becoming graph objects, artifacts, deployment policy, or object bytes.

## Decision

- Select the repository-owned local ordered store `lkjscript-data-store-1` as the sole maintained
  production application-data authority.
- Store immutable complete canonical revisions beneath one random physical store identity. Readers
  pin a revision; one cross-process writer lock and exact-base recheck serialize commits; durable
  immutable data precedes one atomic `HEAD` visibility change.
- Retain all reachable history for this generation. Reject corrupt canonical authority. Provide no
  compaction, garbage collection, in-place repair, or accepted-file rewrite.
- Expose one exact provider-independent standard `DataStore` interface with typed ordered keys,
  schema compare/set, ABA-resistant entry revisions, snapshot prefix scans, revision-bound stateless
  continuations, conditional put/delete, and lexical exact-base transactions.
- Encode application values through canonical typed-value contract 1, binding nominal/runtime
  layout identity. Maintain separate production and canonical-reference implementations.
- Make application indexes ordinary graph-owned spaces updated atomically with their primaries.
  SQL, joins, arbitrary predicates, dynamic schemas, and hidden optimization are not retained.
- Expose public `data initialize`, `verify`, `backup`, and `restore`. Logical backup contract 1 is
  physical-layout independent; restore creates an absent root under a new physical identity.
- Use `durable_queue_data` in a separate internal namespace of the same engine. Keep object bytes in
  object storage and move only metadata/reconciliation facts.
- Move PostgreSQL 16.15 and its Rust client into contributor-only differential tooling. It may
  produce neutral fixtures and measurements but cannot be selected by a deployment or product
  binary.

## Consumer cutover

The standard SQL declarations and all product database implementations are deleted. The BBS stores
one primary post and one `(created-at, id)` index fact in a single transaction. `lkjournal` maps
actor, session, resource, immutable snapshot, object metadata, lookup, and durable-job facts to
explicit spaces and indexes. Both changes were authored through reviewed public graph changes;
there is no frozen replacement artifact or private application builder. Service and worker
descriptors share one confined root through `data` and `durable_queue_data` grants and carry no
database URL or connection secret. Predecessor adapter tags and fields reject during strict
descriptor decoding.

## Evidence and measured reversal conditions

Correctness requires an implementation-disjoint ordered-map transaction model, production/reference
typed-codec equality, randomized mixed-key transactions, insertion-order equality, conditional/ABA
tests, snapshot and continuation tests, concurrent readers, cross-process writers, schema retry and
rollback, resource limits, strict corruption/path decoding, every retained commit interruption
checkpoint, logical backup/restore, and durable-queue restart/stale-attempt behavior.

The contributor oracle uses exact PostgreSQL 16.15 image
`postgres@sha256:485935f94cc7165afa896978809c37b592dc07f0a37d2c8f645f12412d0212c8`
with config digest
`sha256:80f4c7a5e91618546dce5b4fe60cf03b14c0f9efa7e40157278d122772ced8d2`.
After one warm-up and three fresh samples per workload, first-party medians above five times
PostgreSQL wall time, twice its peak RSS, or four times its durable logical-data bytes block this
selection. The admission dataset is bounded evidence, not a general service-level objective or
million-key claim.

## Consequences and reversal

The supported provider is local trusted-host storage. There is no connection pool, database
credential, remote service, replication, consensus, encryption, tenant isolation, online
compaction, lock-free progress, exactly-once queue delivery, or cross-capability atomicity.

Replace this engine only after a maintained workload demonstrates a material correctness,
durability, recovery, portability, or resource failure. A replacement must preserve the
provider-independent logical contract or perform another dependency-closed graph/application
cutover, migrate every maintained consumer, pass neutral and public-workflow differential evidence,
retain old-or-new crash visibility and absent-root restore, reject predecessor production input,
and delete its displaced production path. A faster microbenchmark or provider preference alone is
not a reversal condition.
