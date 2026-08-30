# Security and trust model

This threat model covers the canonical graph platform and `lkjournal`. It does not grant a
hostile-code sandbox or distributed-system guarantee.

## Trust boundaries

The local repository operator, compiled bootstrap, locked Rust dependency closure, accepted graph
meaning, deployment author, host OS/filesystem administrator, first-party data-root operator, and
object-store administrator are trusted within their named authority. Packed graph/artifact bytes,
data-format/revision/head/logical-backup bytes, public change and query requests, continuation
handles, deployment descriptors, HTTP input, JSON, object responses, queue records, and environment
values are hostile decoding inputs.

Accepted lkjscript meaning is validated and resource-bounded but is not isolated as hostile tenant
code. A process, package digest, repository ID, data-store identity, or runtime task is not an
application actor identity.

## Canonical graph threats

| Threat | Implemented control | Residual assumption |
|---|---|---|
| Malformed or excessive packed objects | typed domain magic, exact version/length, domain-separated digest, trailing rejection, checked bounds before decode | Rust and dependency correctness |
| Persistent-root page substitution | root-bound page digests, canonical page encoding, bounded changed-path traversal, generated parent-link checks, missing/corrupt-page rejection on access, exhaustive deep reconstruction | repository operations never delete accepted-base objects; external damage to an untouched reused subtree may remain latent until read or deep doctor |
| Identity-domain confusion | tagged opaque IDs, typed selectors, graph shape validation, foreign-domain rejection tests | callers preserve full durable IDs |
| Partial or torn publication | exact-base lock, immutable objects first, filesystem durability before one synchronized atomic HEAD rename | local filesystem honors documented operations |
| Corrupt current authority | object-key verification, deep reconstruction independent of indexes, writes blocked on observed corruption | operator restores or repairs from trusted backup |
| Index substitution or loss | revision/root/contract-bound manifests; content-addressed local exact shards; local/full indexes are disposable and rebuild | loss may widen the next local change or query to broad reconstruction |
| Semantic-fact substitution | typed module/validator/input summaries, persistent fact-page digests and typed keys, a revision/root-bound manifest, and a revision-core certificate over the exact map roots | summary, page, and manifest bytes are persisted but disposable; loss or malformed bytes rebuild, while a rebuilt certificate mismatch is canonical corruption |
| Name-driven reference drift | canonical imports, exports, declaration references, and targets bind typed stable IDs; module name uniqueness uses the persistent name map; module and declaration rename have full-oracle differential locality tests | declaration move remains a complete-path operation because exact references include the owning module identity |
| Stale concurrent write | HEAD reread under exclusive lock and exact base comparison | one local repository lock domain |
| Replay confusion | optional bounded idempotency key binds one exact transaction digest | caller scopes keys appropriately |
| Oversized query or traversal | item, byte, work, depth, fanout, and continuation bounds | elapsed-time cancellation is process-level, not a hard realtime guarantee |
| Tampered continuation | revision/query/cursor binding plus domain-separated integrity digest | handle is integrity protected, not confidential |
| Review text becoming authority | projection is span-free, marked non-authoritative, and has no apply/import path | reviewer understands it is a projection |
| Embedded bootstrap drift | built-in artifact is exact, inspected through the ordinary artifact decoder, exported for reproduction, and checked against maintained standard authority | the built executable and its locked build inputs are trusted |

On Linux, publication closes all new immutable files, uses `syncfs` on the containing filesystem,
then writes and synchronizes the HEAD stage and store directory. This batches data durability
without weakening the visibility point. Non-Linux builds use per-file synchronization. Network
filesystems and platforms not covered by retained evidence are not claimed.

## Capability and service threats

| Threat | Implemented control | Residual assumption |
|---|---|---|
| Deployment artifact substitution | strict relative regular path, symlink-component rejection, artifact bundle checksum/digest validation, exact root target and requirement closure before secrets or readiness | deployment directory and descriptor author are trusted operational authority |
| Capability confusion | exact interface/operation/alias/limit requirement and deployment grant equality | deployment author chooses appropriate authority |
| Secret disclosure | environment acquisition into opaque redacted values; no durable graph or artifact secret | OS environment and administrator are trusted |
| Typed data substitution | nominal/runtime layout digest, envelope checksum, independent production/reference codecs, canonical/trailing/bounds rejection | graph-owned schema and index policy is reviewed application meaning |
| Data corruption or torn commit | immutable complete revisions, cross-process writer lock, exact-base recheck, data-before-head synchronization, old-or-new fault tests, complete verify | local filesystem and operator honor the supported trusted-host boundary |
| ABA or stale application write | opaque per-put entry revisions plus exact/missing conditional mutation; any failed expectation rolls back the whole transaction | application performs the primary conditional mutation before dependent indexes |
| Cross-actor access | graph-owned session and owner checks with deterministic denial tests | one confined first-party data authority is trusted |
| Password theft | bounded Argon2 hashes, random salt, generic verification | deployment protects transport and local data/object roots |
| Request denial | bounded headers/body/admission/tasks/streams and operational deadlines | no per-IP limiter or TLS proxy is included |
| Object overwrite or traversal | existing real local-root components, validated opaque keys/prefixes, no-replace publication, checksums | object root and credentials are operator-controlled |
| Possibly visible write | post-head data interruption is separately reconcilable; object effects use closed `possible_visibility` plus application reconciliation | data head and object-provider truth remain distinct authorities |
| Duplicate background work | idempotency key, exact attempt/lease owner, stale completion rejection | handlers keep domain publication idempotent |
| Diagnostic leakage | closed projections, bounded excerpts, secret redaction | graph-authored response text is trusted |

Artifacts contain typed requirements, not grants, host coordinates, credentials, or live handles.
Deployment descriptors bind concrete adapters and secrets after artifact verification. Generic Rust
does not own lkjournal routes, spaces/indexes, roles, object keys, or queue transitions.

## Native and test boundaries

First-party Rust forbids `unsafe`. `rustix` supplies safe Linux filesystem/process wrappers;
Axum/Tokio own HTTP and task mechanics; `object_store` owns local/S3 mechanics; Argon2 and OS
randomness own cryptographic mechanics. The `postgres` crate exists only in contributor differential
tooling and is absent from the product dependency graph. Locked dependencies may contain unsafe code
outside the first-party prohibition.

The predecessor parser has no public CLI or application-development path. Its remaining callers are
Rust-test fixtures that provide an independent structural language oracle. Fixture text is not
maintained program authority. The predecessor value, VM, capability, artifact, and adapter stack is
deleted. Source-era project markers and predecessor artifact/store contracts reject at public
boundaries; there is no fallback decoder.

Explicit rank-1 type substitution and named pure function values are validated before preparation.
Type parameters have distinct stable IDs, calls require exact type-argument arity, generic task
functions reject, and changing type arguments through polymorphic recursion rejects. Function
values contain stable named-function identity, not a runtime address or captured lexical
environment. These checks provide language consistency, not hostile-code isolation.

## Current static distribution boundary

The current release path selects one `x86_64-unknown-linux-musl` candidate and treats every build,
handoff, archive, manifest, checksum, receipt, image, and downloaded asset as hostile or mutable
input until validated. First-party ELF inspection requires little-endian ELF64 x86-64 with no
runtime interpreter, `DT_NEEDED` library, or GLIBC symbol-version requirement. Strict archive
verification rejects links, traversal, duplicates, extras, noncanonical metadata, target/linkage
contradiction, checksum corruption, candidate substitution, and unsafe extraction targets.

Exact v0.1.10 candidate execution passed in pinned Alpine 3.22.5/musl 1.2 and Debian 11/glibc 2.31
userlands with network unavailable and no host library mounts. Current-source transferred
application verification receives only exact candidate/verifier bytes, an explicit create-new
private root, loopback HTTP, and isolated local data/object roots. It does not gain a checkout,
Cargo, application helper, raw retained secret, database server, or publication permission. The
separate publication job receives no verifier and executes no repository code.

Immutable v0.1.10 and its independent anonymous exact/latest checks completed the predecessor
release controls. They
establish static runtime linkage and only the named Linux/amd64 userland boundary; they do not
establish a minimum kernel, every CPU feature or filesystem, container independence,
reproducible-build provenance, binary signing, or universal Linux compatibility. Immutable v0.1.8
remains an unclosed historical recovery point after a workflow-only cross-application equality
defect; none of its external objects was changed during additive recovery.

## Explicit non-claims

The system does not isolate hostile lkjscript programs, native dependencies, an OS administrator,
data-root/object-store administrators or co-resident processes. It does not provide
tenant CPU/RSS isolation, encrypted stores/backups, artifact signatures, authenticated provenance,
distributed consensus, multi-node publication, cross-authority transactions, certificate
management, CSRF middleware, audit-log durability, or production portability beyond retained
exact Linux x86-64 target and userland evidence.

The HTTP listener is plaintext, and first-party data roots/backups are not encrypted. lkjscript does
not plan to implement HTTP TLS termination, certificate parsing or management, certificate issuance
or rotation, ACME, or speculative TLS language and capability hooks. A deployment that requires
encrypted network transport or storage must use an appropriate external trusted boundary outside
the current product scope. External protection does not make the runtime a hostile-code sandbox or
provide multi-tenant isolation.
