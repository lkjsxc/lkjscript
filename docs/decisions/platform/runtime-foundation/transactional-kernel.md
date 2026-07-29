# Transactional Kernel

## Status

**Accepted Contract with Experimental Implementation.** A bounded safe-Rust
ordered kernel is durable and tested, but it is not yet a node service, source
capability, relational engine, SQLite replacement, or Current public format.

## Current Inherited State

Current compiler and agent transactions retain their existing authorities and
publication rules. Current runtime state is not unified under an ordered store.
This decision does not replace source transactions, package locks, the resource
ledger, SQLite semantics, or Linux filesystem durability.

## Accepted Kernel Contract

The kernel exposes one ordered byte-key/byte-value layer with:

- read-only snapshots and serializable read-write transactions;
- deterministic key ordering, conflict detection, and commit identity;
- explicit byte, key, operation, conflict, retry, and nesting bounds;
- atomic no-publication failure before commit;
- deterministic simulation hooks for clocks, cancellation, faults, and order;
- a replayable bounded journal and content-addressed immutable snapshots; and
- upper layers for lifecycle, names, indexes, queues, and caches rather than
  separate storage engines with independent transaction semantics.

The contract adopts two FoundationDB research lessons: deterministic simulation
as a primary correctness tool and one ordered key-value foundation with database
features layered above it. Sources:

- [FoundationDB testing](https://apple.github.io/foundationdb/testing.html)
- [FoundationDB architecture](https://apple.github.io/foundationdb/architecture.html)

FoundationDB itself, its wire protocol, distributed claims, background behavior,
and compatibility surface are rejected as embedded runtime dependencies.

## Modern OLTP Experiment

[LeanStore](https://github.com/leanstore/leanstore) is adopted as a research
reference for modern OLTP co-design across concurrency, caching, storage layout,
and hardware. This cycle may compare LeanStore-inspired page/layout and
contention hypotheses against a simple ordered baseline. It may not import
LeanStore, weaken deterministic replay, or couple semantic correctness to a
particular storage engine.

## Deferred Vector Work

DuckDB's [vector execution](https://duckdb.org/docs/stable/internals/vector.html)
is **Deferred** to a later analytical/dataflow slice. Vectors are rejected for
this cycle's control-plane kernel because lifecycle and small ordered metadata
transactions need deterministic latency and simple conflict evidence before
batch throughput. Deferral does not reject future vectorized query operators.

## Implemented Experiment

`lkjscript-database` provides tenant-prefixed byte ordering, snapshot reads, one
serialized writer, atomic commit/abort, get/put/delete/range, bounded logical
write buffering, canonical little-endian checksummed WAL frames, sync-before-
success, replay, truncated-tail handling, uncommitted discard, and atomic
checkpoints over `DurableStorage`. Fake storage injects short write, disk full,
sync failure, crash, truncation, and tail corruption. Model and reopen tests pass;
a create/commit/checkpoint/reopen/read probe executes under `wasm32-wasip1`.
There is no page-oriented B+tree, MVCC writer concurrency, node-owned page cache,
node tenant lifecycle, migration, or fair SQLite benchmark. The format and API
remain experimental until those missing guarantees and comparisons pass.
