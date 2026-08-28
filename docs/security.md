# Security and trust model

This threat model covers the canonical graph platform and `lkjournal`. It does not grant a
hostile-code sandbox or distributed-system guarantee.

## Trust boundaries

The local repository operator, compiled bootstrap, locked Rust dependency closure, accepted graph
meaning, deployment author, host OS, PostgreSQL administrator, and object-store administrator are
trusted within their named authority. Packed graph/backup/artifact bytes, public change and query
requests, continuation handles, deployment descriptors, HTTP input, JSON, database rows, object
responses, queue records, and environment values are hostile decoding inputs.

Accepted lkjscript meaning is validated and resource-bounded but is not isolated as hostile tenant
code. A process, package digest, repository ID, database connection, or runtime task is not an
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
| Semantic-fact substitution | contract-2 module/validator/input summaries, contract-3 persistent fact-page digests and typed keys, a revision/root-bound manifest, and a revision-core certificate over the exact map roots | summary, page, and manifest bytes are persisted but disposable; loss or malformed bytes rebuild, while a rebuilt certificate mismatch is canonical corruption |
| Name-driven reference drift | canonical imports, exports, declaration references, and targets bind typed stable IDs; module name uniqueness uses the persistent name map; module and declaration rename have full-oracle differential locality tests | declaration move remains a complete-path operation because exact references include the owning module identity |
| Stale concurrent write | HEAD reread under exclusive lock and exact base comparison | one local repository lock domain |
| Replay confusion | optional bounded idempotency key binds one exact transaction digest | caller scopes keys appropriately |
| Oversized query or traversal | item, byte, work, depth, fanout, and continuation bounds | elapsed-time cancellation is process-level, not a hard realtime guarantee |
| Tampered continuation | revision/query/cursor binding plus domain-separated integrity digest | handle is integrity protected, not confidential |
| Draft execution | drafts use distinct IDs/storage and build/run paths accept only accepted revisions | local operator controls draft retention |
| Review text becoming authority | projection is span-free, marked non-authoritative, and has no apply/import path | reviewer understands it is a projection |
| Backup path/symlink or segment substitution | fixed canonical key spaces, regular-file checks, manifest-bound consecutive segment digests, per-entry length/digest checks, exact reachability comparison, private restore stage, and deep structural/history verification before rename | hostile co-resident filesystem administrator is out of scope; restore does not yet rerun complete cross-package semantic validation |
| Cleanup-plan misuse | `doctor cleanup` is read-only, hashes the observed plan, reports unknown entries, and always sets `destructive_ready: false` | revision pins, active-reader leases, and registered backup roots are not represented, so canonical deletion remains unavailable |
| Embedded bootstrap drift | built-in artifact is exact, inspected through the ordinary artifact decoder, exported for reproduction, and checked against maintained standard authority | the built executable and its locked build inputs are trusted |

On Linux, publication closes all new immutable files, uses `syncfs` on the containing filesystem,
then writes and synchronizes the HEAD stage and store directory. This batches data durability
without weakening the visibility point. Non-Linux builds use per-file synchronization. Network
filesystems and platforms not covered by retained evidence are not claimed.

## Capability and service threats

| Threat | Implemented control | Residual assumption |
|---|---|---|
| Deployment artifact substitution | strict relative regular path, symlink-component rejection, artifact-10 contract/checksum/digest validation, exact root target and requirement closure before secrets or readiness | deployment directory and descriptor author are trusted operational authority |
| Capability confusion | exact interface/operation/alias/limit requirement and deployment grant equality | deployment author chooses appropriate authority |
| Secret disclosure | environment acquisition into opaque redacted values; no durable graph or artifact secret | OS environment and administrator are trusted |
| SQL injection | graph-owned statements require `StaticText`; values use typed parameters | authored static SQL is trusted and reviewed |
| Cross-actor access | graph-owned session and owner checks with deterministic denial tests | one PostgreSQL authority is trusted |
| Password theft | bounded Argon2 hashes, random salt, generic verification | deployment protects transport and database |
| Request denial | bounded headers/body/admission/tasks/streams and operational deadlines | no per-IP limiter or TLS proxy is included |
| Object overwrite or traversal | existing real local-root components, validated opaque keys/prefixes, no-replace publication, checksums | object root and credentials are operator-controlled |
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

## Prepared static distribution boundary

The pending release path selects one `x86_64-unknown-linux-musl` candidate and treats every build,
handoff, archive, manifest, checksum, receipt, image, and downloaded asset as hostile or mutable
input until validated. First-party ELF inspection requires little-endian ELF64 x86-64 with no
runtime interpreter, `DT_NEEDED` library, or GLIBC symbol-version requirement. Strict archive
verification rejects links, traversal, duplicates, extras, noncanonical metadata, target/linkage
contradiction, checksum corruption, candidate substitution, and unsafe extraction targets.

Exact candidate execution has been prepared for one pinned musl userland and one pinned older-glibc
userland with network unavailable and no host library mounts. Transferred application verification
receives only exact candidate/verifier bytes, an explicit create-new private root, loopback HTTP, and
the exact isolated PostgreSQL prerequisite required by the stateful oracle. It does not gain a
checkout, Cargo, application helper, raw retained secret, or publication permission. The separate
publication job receives no verifier and executes no repository code.

These remain source-candidate controls until the additive `v0.1.9` recovery completes immutable
publication and anonymous exact/latest behavior. They establish static runtime linkage and only the
named Linux/amd64 userland boundary; they do not establish a minimum kernel, every CPU feature or
filesystem, container independence, reproducible-build provenance, binary signing, or universal
Linux compatibility. Immutable `v0.1.8` is externally committed but not closed after a final
workflow-only cross-application equality defect; `v0.1.7` remains the documented supported release
during recovery.

## Explicit non-claims

The system does not isolate hostile lkjscript programs, native dependencies, an OS administrator,
database superusers, object-store administrators, or co-resident processes. It does not provide
tenant CPU/RSS isolation, encrypted stores/backups, artifact signatures, authenticated provenance,
distributed consensus, multi-node publication, cross-authority transactions, certificate
management, CSRF middleware, audit-log durability, or production portability beyond retained
exact Linux x86-64 target and userland evidence.

The HTTP listener is plaintext, and PostgreSQL connections use `NoTls`. lkjscript does not plan to
implement HTTP TLS termination, PostgreSQL TLS, certificate parsing or management, certificate
issuance or rotation, ACME, or speculative TLS language and capability hooks. A deployment that
requires encrypted transport must use an appropriate external trusted transport boundary or a
different adapter outside the current product scope. External termination does not make the
runtime a hostile-code sandbox or provide multi-tenant isolation.
