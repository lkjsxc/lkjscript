# Ordered data, object, and durable-queue capabilities

Status: normative.

This specification owns the generic application-data, object, and durable-queue capability
families. Application spaces, schemas, indexes, authorization, object-key policy, payload meaning,
retry policy, and cross-capability coordination remain authored graph meaning.

## First-party ordered data store

The only production data adapter is `data`. It binds one confined deployment-relative local root,
one strict namespace, one sharing domain and authority revision, and independently bounded data
limits. The root is trusted-host operational application-data authority. It is separate from the
typed meaning `GraphRepository`, package material, artifacts, compiler caches, object bytes,
deployment descriptors, and receipts. Opening or mutating it never selects or advances program
meaning.

The physical format is `lkjscript-data-store-1`. One root has a random physical identity, immutable
canonical revision objects, retained parent history, private staging, one cross-process writer lock,
and one atomic durable `HEAD`. A transaction reads one immutable base snapshot. Commit locks,
rechecks that exact base, synchronizes one complete immutable revision, and changes visibility once.
A changed base is a retryable conflict with no visibility. Failure before head visibility reopens
the old state; interruption after visibility is a distinct possible-visibility reconciliation
case. Readers pin their revision. This format performs no compaction, garbage collection,
in-place repair, accepted-file rewriting, replication, or remote coordination.

Runtime cancellation is checked before transaction commit. A cancelled lexical transaction drops
its staged state; the bounded visibility critical section itself is not interrupted after it
begins, so cancellation cannot expose a partial revision.

Format, head, revisions, keys, schemas, continuations, and backups use bounded canonical encodings
and independent digest domains. Opening rejects foreign versions/domains, invalid links or lengths,
duplicate/noncanonical facts, corruption, trailing bytes, excess, path escape, symbolic links, and
non-regular files. All accepted revision objects reachable from `HEAD` are authoritative and must
verify. `catalog/CURRENT` is a bounded sorted acceleration only: read-only verification reconstructs
objects without it, and a later write rebuilds it after missing, stale, or corrupt catalog bytes.
Catalog damage cannot hide or redefine canonical authority.

## Logical interface

The exact standard interface is `DataStore`. `DataKeyPart` has exactly `Bool`, `I64`, `Text`, and
`Bytes`. A key is a nonempty bounded sequence. Ordering is lexicographic by part sequence: tag order
is Bool, I64, Text, Bytes; values use `false < true`, signed numeric order, validated UTF-8 byte
order, and unsigned byte order; a strict prefix precedes its extension. One static graph-owned
space name plus one key selects a record.

Pure `data-encode<T>` and `data-decode-or<T>` use canonical typed-value contract 1. The envelope
binds the complete nominal/runtime layout identity and checksum. Decoding rejects a foreign type or
layout, malformed or noncanonical values, invalid UTF-8, duplicate/out-of-order map keys, trailing
bytes, and item/depth/byte exhaustion. `decode-or` returns its exact typed fallback on any rejected
encoding. JSON, SQL rows, Rust layout, serde shape, and host filesystem representation are not data
authority. Production and canonical-reference codecs are separate implementations and must agree.

`schema-read` returns missing or one exact schema identity/digest. Transaction-only `schema-set`
requires missing or an exact prior schema. An equal exact retry is a no-op; a mismatch marks the
whole transaction uncommittable. Application migrations perform ordinary reads and writes, then
stage the next schema marker last.

`get` returns missing or value plus an opaque entry revision. Every accepted put receives a new
revision bound to physical store, base, mutation position, key, and value, so delete/recreate does
not validate a predecessor expectation. Transaction-only `put` and `delete` require `Missing` or an
exact entry revision. A false expectation returns false and guarantees that commit publishes none
of the transaction's earlier or later staged changes. Maintained code performs its primary
conditional mutation before dependent index mutations.

`scan` reads a static space and key prefix in forward or reverse canonical order from the exact
transaction snapshot. It has separate item, returned-byte, and examined-work limits. Its opaque
continuation contains no mutable cursor and binds store, revision, namespace, space, prefix,
direction, all selected limits, and an exclusive resume key. A foreign, stale, corrupt, or
selector-mismatched continuation rejects.

Secondary indexes are ordinary application spaces updated in the same transaction as their
primary. The interface has no joins, arbitrary predicates, dynamic schema language, optimizer,
network access, ambient filesystem access, or SQL compatibility.

## Executable limits

The implementation ceilings are 128 bytes per space or namespace name, 16 key parts, 4 KiB per
encoded key, 4 MiB per value, 4,096 mutations and 16 MiB of mutation bytes per transaction, 10,000
items, 16 MiB returned bytes and 1,000,000 examined records per scan, and 1,024 live transactions
per prepared adapter. One retained revision or logical backup is at most 1 GiB; verification admits
at most 1,000,000 retained revisions and 1,000,000 immutable objects. Deployment may select only
equal or smaller logical limits. These are admissions, not scale claims.

## Public lifecycle and logical backup

The top-level operation is closed:

```text
data initialize --root PATH
data verify --root PATH
data backup --root PATH --output PATH
data restore --backup PATH --root PATH
```

`initialize` creates only an absent confined root through private sibling staging and one durable
visibility rename. An equal valid root reports unchanged; a foreign, corrupt, symlinked, or
non-directory root rejects. `verify` reconstructs every retained revision and accepted link without
mutation. `backup` pins one head and create-new publishes a sorted logical schema/key/value/revision
snapshot in `lkjscript-data-backup-1`, independent of physical object layout. `restore` accepts only
a strict backup and absent destination, creates a new physical store identity with equivalent
logical facts, verifies it, and publishes the destination once. Neither operation overwrites,
repairs, imports SQL, or silently switches a deployment descriptor.

## Named object storage

An object grant binds memory, confined local root, or explicit S3-compatible endpoint/region/bucket
and prefix. Keys are validated relative opaque slash names beneath the prefix, at most 1,024 bytes;
application ownership/visibility is not inferred. Global object maximum is 16 GiB and whole-read
maximum may not exceed the selected object bound.

Operations are no-replace streaming `put-new`, bounded whole `get`, bounded `range`, `head`,
`reconcile-put`, and `delete`. Upload validates bounds and content type, computes BLAKE3, closes or
aborts on failure, and returns provider facts needed by application reconciliation. Object bytes
remain in object storage; `lkjournal` keeps only metadata and reconciliation state in `DataStore`.
Object and data publication are never implicitly atomic.

## Durable queue

The only production durable queue adapter is `durable_queue_data`. It uses a separate internal
namespace of the same first-party root and has its own queue limits. Service and worker may share
the root through separately validated grants without sharing task handles or gaining program
authoring authority.

The exact standard `DurableQueue` interface has nine operations:

- unchanged `initialize`, `enqueue`, `cancel`, and `inspect`;
- `claim(worker-id, now-milliseconds, lease-milliseconds) -> QueueLeaseState`;
- `lease-info(lease borrow) -> QueueLeaseInfo`;
- `heartbeat(lease consume, now-milliseconds, lease-milliseconds) -> QueueLeaseState`;
- `complete(lease consume, now-milliseconds, result) -> Bool`; and
- `fail(lease consume, now-milliseconds, retry, retry-at-milliseconds, error-class) -> Bool`.

`QueueLeaseState` is the nominal `absent | live(CapabilityResource<DurableQueue>)` variant used by
both claim and heartbeat. `QueueLeaseInfo` is an unrestricted nominal record containing job ID,
attempt number, lease-until time, and payload. It reports acquisition metadata, not a liveness
probe, and deliberately omits attempt and worker identity. Those raw fields remain private to the
runtime resource entry and queue engine; they are not graph transition authority.

Enqueue is exact-idempotent by job and idempotency identity. Ready order is availability time,
creation time, then job identity. Claim assigns one nonreused attempt identity and finite lease.
`lease-info` borrows. Heartbeat consumes the old handle and returns a fresh live handle or absent;
complete and fail consume on every reached result, including stale, cancellation, and possible
visibility. A stale transition returns absent or false without recreating the old right.
Completion is single-success; retry timing and terminal attempt policy are explicit application
inputs. Cancellation makes future stale publication harmless. Restart reconstructs all durable job
and attempt facts from the ordered store.

The task resource scope reserves bounded handle capacity and identity before claim or heartbeat can
mutate queue state. Empty, stale, failed, or cancelled effects release that reservation; successful
effects commit it without avoidable post-effect allocation. Each handle is bound to its exact task
scope, resource kind, interface, and acquiring requirement. Borrow preserves it, consume closes it,
and scope cleanup drops only local authority. Possible visibility is nonretrying and preserves no
exactly-once claim.

The physical `lkjscript-durable-queue-data-1` primary/index encoding is unchanged. It still stores
private job, attempt, and worker fields needed by the engine, so existing operational roots and
logical backups require no migration. Predecessor graph signatures that return a list lease record
or accept raw job/attempt/worker transition tokens are not current input and reject.

The queue does not promise exactly-once delivery or atomicity with an independently executed object
effect. No memory or PostgreSQL production backend, hidden fallback, or provider selector remains.

## Trust and nonclaims

The supported data boundary is one local trusted host with filesystem and process-lock semantics
proved on the current Linux target. It does not claim encryption at rest, tenant isolation,
replication, consensus, remote service, lock-free progress, online compaction, million-key
admission, destructive repair, general portability, or cross-capability atomicity. Recovery is
logical backup, restore into a new root, verification, and an explicit operator descriptor switch.
