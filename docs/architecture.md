# Architecture

## Current flow and ownership

```text
lkjscript client
    -> typed, framed Unix-socket request
lkjscriptd
    -> one DurableWorkspace writer per workspace
    -> staged typed transaction over an immutable Snapshot
    -> canonical per-revision .lkjscript artifact and atomic HEAD
    -> immutable published Snapshot
    -> revision-bound query or direct SPG-to-Core-IR compilation
    -> Core IR verifier
    -> interpreter
    -> typed response
```

The daemon is synchronous and is the only live writer. The client contains presentation and typed
request construction only. `graph.rs` owns immutable snapshots and retained workspace history;
`schema.rs` owns node and operation contracts; `transaction.rs` owns staged mutation; `validate.rs`
owns graph acceptance; `artifact.rs` owns canonical semantic bytes; `persistence.rs` owns durable
publication; `protocol.rs` owns IPC types and framing; `query.rs` owns derived summaries and blockers;
`compile.rs` and private `core_ir.rs` own lowering and executable verification; `interpret.rs` owns
the one runtime route.

There is no second function body, type authority, evaluator, mutable client workspace, or persisted
compiler representation.

## Durability

A workspace directory retains immutable `revisions/REVISION.lkjscript` files and one small
non-semantic `HEAD` record. HEAD names the committed revision and hash, may retain one typed
idempotency outcome, and has a checksum over the entire record. Workspace creation builds and
flushes a recognized staging directory, renames it atomically, and flushes `workspaces/`; startup
removes only well-formed abandoned staging directories.

Commit order is:

1. encode and validate the candidate snapshot;
2. write a new revision temporary file, flush it, rename it, and flush `revisions/`;
3. write and flush a new HEAD temporary file;
4. atomically replace HEAD and flush the workspace directory;
5. publish the in-memory `Arc<Snapshot>` and acknowledge.

Failure before HEAD commit leaves the old HEAD authoritative; an orphan revision is removed or
ignored during recovery. Injected failures cover every bootstrap publication step. If post-rename
directory sync and HEAD rollback both fail, the daemon reports `CommitOutcomeUnknown` and exits
rather than continuing with uncertain authority.

Restart bounds files before reading, loads every contiguous retained revision through the same
defensive artifact decoder, checks the HEAD checksum/hash, validates monotonic allocator,
tombstone, root, kind, and owner history, restores the semantically checked idempotency outcome,
and then opens IPC. Old revisions remain independently queryable and executable.

## Trust boundaries

- **same-build Rust values:** closed enums and private constructors; no serialization is added for
  Core IR;
- **artifact bytes:** bounded custom decoder, stable tags, graph validation, canonical hash, and
  strict trailing-byte policy;
- **IPC bytes:** private Unix socket, checked frame length, closed message vocabulary, correlation,
  and structured rejection;
- **runtime:** verified Core IR only; no native code or host capabilities exist in this slice.

All graph walks that can scale with user nodes use loops and explicit vectors/sets. The current
schema also has closed containment depth. Resource limits at artifact and IPC boundaries are
reported as policy errors and do not redefine language validity.

## Deliberate restraint

The baseline uses `BTreeMap`, vectors, and full snapshot clones. There is no database, journal,
async runtime, generic graph framework, schema generator, query engine, cache, source projection,
native backend, runtime cell, plugin mechanism, or remote service. A later mechanism must have a
measured producer/consumer and preserve this single authority path.
