# Security and trust model

This threat model covers the current source-authored platform and `lkjournal`. It does not grant a
sandbox claim.

## Trust boundaries

The local operator, checked lkjscript source, compiled bootstrap binary, Rust dependency closure,
deployment descriptor, PostgreSQL administrator, object-store administrator, and host OS are
trusted within their named authority. HTTP peers, request bytes, JSON, URL/query fields, headers,
database rows, object metadata, queue records, source/artifact/backup bytes, and environment values
are untrusted inputs. Authored lkjscript is validated and resource-bounded but is not treated as
hostile tenant code.

Application authentication establishes an actor string from a bearer session. Application-owned
queries enforce resource ownership. Deployment grants authorize generic mechanics; they do not
authorize domain access. A process boundary, package digest, or database connection is not an actor
identity.

## Principal threats and controls

| Threat | Current control | Residual assumption |
|---|---|---|
| Malformed or excessive source/artifact/config | strict closed decoders, bounds before retained allocation, digests, exact versions | trusted compiler/runtime correctness |
| Capability confusion | exact interface owner, operation set, requirement alias, limits, sharing domain, authority revision, descriptor digest | deployment author chooses appropriate grants |
| Secret disclosure | environment acquisition into opaque catalog, redacted debug/observations, verifier comparison without serialization | OS process environment and administrator are trusted |
| SQL injection | statements must be `StaticText`; values use typed parameter binding | authored static SQL is trusted and reviewed |
| Cross-actor resource access | session lookup plus application-owned owner checks and deterministic denial tests | one PostgreSQL authority is trusted |
| Password theft | bounded Argon2 hashes, random salt, generic verification, no plaintext persistence | operator selects deployment parameters and transport security |
| Request denial | header/body/count limits, bounded admission queue, per-request deadline, bounded streams | no per-IP rate limiter or TLS proxy is included |
| Object overwrite/traversal | validated opaque keys/prefix, no-replace publication, checksum, exact local root/object-store API | local root and S3 credentials are operator-controlled |
| Unknown external publication | explicit `possible_visibility` class and application reconciliation | provider truth remains external authority |
| Duplicate background work | enqueue idempotency, exact attempt/lease owner, stale completion rejection | handlers must keep domain publication idempotent |
| Shutdown repetition | admission stop, drain, cooperative cancellation, adapter cleanup receipt | blocking drivers may outlive cancellation grace and report infrastructure failure |
| Diagnostic leakage | closed error projections and redacted secret types | application-authored response/log text is trusted |

Tests cover strict/oversized decoding, missing grants, deterministic auth success/failure,
cross-actor denial, transaction rollback and handle lifetime, overload and shutdown, stream
backpressure/cleanup, no-replace object behavior and reconciliation, queue duplicate delivery/lease
loss/stale completion, secret redaction, predecessor rejection, and absence of `lkjournal` product
vocabulary in generic native sources. Live acceptance covers authentication and the same owner path
over a real PostgreSQL process.

Generic HTTP failure responses expose only a bounded closed failure class/code header for
operational diagnosis. Provider messages, connection descriptors, credentials, and application
values are not copied into those headers.

## Dependency and native boundary

First-party Rust forbids `unsafe`. Native dependencies are locked. Axum/Tokio own HTTP and task
mechanics; `postgres` owns the PostgreSQL protocol; `object_store` owns local/S3 protocol mechanics;
Argon2 and OS randomness own cryptographic mechanics. These dependencies may contain audited unsafe
code outside the repository prohibition. The current PostgreSQL adapter uses `NoTls`, and the HTTP
listener does not terminate TLS; production exposure therefore requires a trusted local/private
network or a correctly configured external TLS boundary.

There is no ambient outbound network interface available to lkjscript. The S3 and PostgreSQL
adapters can reach only endpoints explicitly bound by deployment. The S3 endpoint restrictions are
deployment validation, not an application-visible arbitrary socket capability.

## Explicit non-claims

The runtime does not isolate hostile lkjscript programs, hostile native dependencies, a hostile OS
administrator, database superusers, object-store administrators, or co-resident processes. It does
not provide tenant CPU/RSS isolation, encrypted artifacts, artifact signatures, provenance,
certificate management, CSRF middleware, session revocation UI, audit-log durability, or cluster
consensus. These claims require separate implemented consumers and evidence.
