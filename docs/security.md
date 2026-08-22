# Security and trust model

This threat model covers the canonical graph platform and `lkjournal`. It does not grant a
hostile-code sandbox or distributed-system guarantee.

## Trust boundaries

The local repository operator, compiled bootstrap, locked Rust dependency closure, accepted graph
meaning, deployment author, host OS, PostgreSQL administrator, and object-store administrator are
trusted within their named authority. Packed graph/backup/artifact bytes, semantic requests,
continuation handles, deployment descriptors, HTTP input, JSON, database rows, object responses,
queue records, and environment values are hostile decoding inputs.

Accepted lkjscript meaning is validated and resource-bounded but is not isolated as hostile tenant
code. A process, package digest, repository ID, database connection, or runtime task is not an
application actor identity.

## Canonical graph threats

| Threat | Implemented control | Residual assumption |
|---|---|---|
| Malformed or excessive packed objects | typed domain magic, exact version/length, domain-separated digest, trailing rejection, checked bounds before decode | Rust and dependency correctness |
| Identity-domain confusion | tagged opaque IDs, typed selectors, graph shape validation, foreign-domain rejection tests | callers preserve full durable IDs |
| Partial or torn publication | exact-base lock, immutable objects first, filesystem durability before one synchronized atomic HEAD rename | local filesystem honors documented operations |
| Corrupt current authority | object-key verification, deep reconstruction independent of indexes, writes blocked on observed corruption | operator restores or repairs from trusted backup |
| Index substitution or loss | revision/root/contract-bound digests; local/full indexes are disposable and rebuild | rebuild cost can be broad |
| Stale concurrent write | HEAD reread under exclusive lock and exact base comparison | one local repository lock domain |
| Replay confusion | optional bounded idempotency key binds one exact transaction digest | caller scopes keys appropriately |
| Oversized query or traversal | item, byte, work, depth, fanout, and continuation bounds | elapsed-time cancellation is process-level, not a hard realtime guarantee |
| Tampered continuation | revision/query/cursor binding plus domain-separated integrity digest | handle is integrity protected, not confidential |
| Draft execution | drafts use distinct IDs/storage and build/run paths accept only accepted revisions | local operator controls draft retention |
| Review text becoming authority | projection is span-free, marked non-authoritative, and has no apply/import path | reviewer understands it is a projection |
| Backup path/symlink attack | fixed canonical key spaces, regular-file checks, private restore stage, full validation before rename | hostile co-resident filesystem administrator is out of scope |

On Linux, publication closes all new immutable files, uses `syncfs` on the containing filesystem,
then writes and synchronizes the HEAD stage and store directory. This batches data durability
without weakening the visibility point. Non-Linux builds use per-file synchronization. Network
filesystems and platforms not covered by retained evidence are not claimed.

## Capability and service threats

| Threat | Implemented control | Residual assumption |
|---|---|---|
| Capability confusion | exact interface/operation/alias/limit requirement and deployment grant equality | deployment author chooses appropriate authority |
| Secret disclosure | environment acquisition into opaque redacted values; no durable graph or artifact secret | OS environment and administrator are trusted |
| SQL injection | graph-owned statements require `StaticText`; values use typed parameters | authored static SQL is trusted and reviewed |
| Cross-actor access | graph-owned session and owner checks with deterministic denial tests | one PostgreSQL authority is trusted |
| Password theft | bounded Argon2 hashes, random salt, generic verification | deployment protects transport and database |
| Request denial | bounded headers/body/admission/tasks/streams and operational deadlines | no per-IP limiter or TLS proxy is included |
| Object overwrite or traversal | validated opaque keys/prefixes, no-replace publication, checksums | object root and credentials are operator-controlled |
| Possibly visible external write | closed `possible_visibility` failure plus application reconciliation | provider truth remains external authority |
| Duplicate background work | idempotency key, exact attempt/lease owner, stale completion rejection | handlers keep domain publication idempotent |
| Diagnostic leakage | closed projections, bounded excerpts, secret redaction | graph-authored response text is trusted |

Artifacts contain typed requirements, not grants, host coordinates, credentials, or live handles.
Deployment descriptors bind concrete adapters and secrets after artifact verification. Generic Rust
does not own lkjournal routes, tables, roles, object keys, or queue transitions.

## Native and test boundaries

First-party Rust forbids `unsafe`. `rustix` supplies the safe Linux `syncfs` wrapper; Axum/Tokio own
HTTP and task mechanics; `postgres` owns the PostgreSQL protocol; `object_store` owns local/S3
mechanics; Argon2 and OS randomness own cryptographic mechanics. Locked dependencies may contain
unsafe code outside the first-party prohibition.

The old source parser and semantic builder compile only in Rust tests as an independent migration
and execution oracle. Test fixture text is not maintained program authority. Source-era project
markers and artifact contracts reject at public boundaries; there is no fallback decoder.

## Explicit non-claims

The system does not isolate hostile lkjscript programs, native dependencies, an OS administrator,
database superusers, object-store administrators, or co-resident processes. It does not provide
tenant CPU/RSS isolation, encrypted stores/backups, artifact signatures, authenticated provenance,
distributed consensus, multi-node publication, cross-authority transactions, certificate
management, CSRF middleware, audit-log durability, or production portability beyond retained
Linux x86-64 evidence. Current PostgreSQL uses `NoTls`, and the HTTP listener does not terminate
TLS; production exposure requires an appropriate trusted network or external TLS boundary.
