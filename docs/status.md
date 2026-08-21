# Current status

Status date: 2026-08-21 UTC. This file describes implemented checkout reality, not future intent.

## Maintained authorities

| Authority | Current identity |
|---|---|
| `packages/standard` | package `10000000000000000000000000000001`, revision 5, record `9766c0b08788b8326757091526fddd50bceb4ec618874e4023d0fc972a3e8c49`, semantic `d0fe09ea464351240e77248f35b293653b198b238f0688b05c372903cd04630a` |
| `applications/lkjournal` | package `20000000000000000000000000000001`, revision 7, record `e85d84f4bd23c1cb768b8387805710939fd1347a6604bd2ac7d3886b2aaf7beb`, semantic `cdbd18f4be55897dcd27b6601cf03e818d666e8ca90789ad02d86f4c4a4bce53` |
| checked service artifact | component artifact digest `eec4c68b121bfa4bdf4af2b01e712040d9e40907e92bf490e345da75bb682af4`, 41,587 bytes |
| checked standard dependency | package artifact digest `a09712fe34ccc0315fdf6e55bbddf8e4ba433093a140075fa2d704d3876a8cab`, 9,602 bytes |

`lkjournal` exposes HTTP target `serve` and worker target `work`. It has 11 accepted package tests
across its exact closure. `standard` has three package tests.

## Current contract identities

All listed identities have only version 1. Unknown fields, wrong versions, trailing input, foreign
identity, and contract-specific excess reject.

| Domain | Identity or encoding |
|---|---|
| Source authority | source project contract 1, `.lkjscript/source-v1` |
| Package | package contract 1, `lkjscript.package.json` |
| Workspace projection | workspace and CLI contract 1, strict JSON output |
| Component artifact | artifact contract 1, canonical JSON `.lkja` |
| Capability grant | capability grant contract 1 |
| Deployment | deployment contract 1, strict JSON descriptor |
| Execution | `bytecode_v1` production and `reference_ast_v1` oracle |
| Resident topology | resident runtime contract 1 |
| HTTP server | HTTP adapter contract 1 |
| Typed JSON | JSON contract 1 |
| Streams | stream contract 1 |
| PostgreSQL | PostgreSQL adapter contract 1 |
| Object storage | object adapter contract 1 |
| Durable queue and worker | queue contract 1 and worker contract 1 |
| Configuration | configuration adapter contract 1 |
| Secrets | secret catalog and verifier contract 1 |
| Clock/random/identifier/password | security adapter contract 1 |

Runner kinds are `command`, `http`, `interactive`, `batch`, `worker`, and `test`. They are target
metadata over one component/port model, not separate application formats. HTTP and worker have live
generic runners; pure command target execution and tests use the same prepared functions. No
current interactive or batch transport adapter is maintained.

## Implemented language

The value/type surface includes unit, bool, checked signed i64, immutable bytes, UTF-8 text,
compile-time `StaticText`, opaque secret/resource values, nominal and structural records, nominal
variants, homogeneous lists, deterministically ordered maps, option, result, stream, and function
types. Map keys are bool, i64, bytes, or text; iteration follows their defined total order. Runtime
values are bounded to depth 256 and 1,000,000 collection items.

Declarations include records, variants, interfaces, closed externs, pure/task functions, constants,
components, and tests. Expressions include lazy conditionals, lexical binding, sequencing, calls,
records/fields, variants/matching, lists, maps, function references, capability performance, and
lexical transactions. There is no implicit coercion, floating point, mutation-visible collection,
closure capture, generics, traits, set, or user-visible scheduler primitive.

## Implemented service mechanics

- Strict HTTP/1 request decoding, query decoding, header validation, bounded request-body streams,
  whole bounded responses, overload mapping, in-memory dispatch, and graceful shutdown.
- Strict JSON with duplicate-field, unknown typed-field, non-i64 number, depth, item, string, total
  byte, and trailing-input rejection.
- Parameterized PostgreSQL statements using unforgeable `StaticText`, typed parameters and row
  schemas, bounded pools/rows, lexical transactions, checksummed ordered migrations, rollback, and
  classified PostgreSQL failures. PostgreSQL-backed deployments establish a reusable connection
  under bounded retry before publishing readiness.
- Typed configuration, environment-bound redacted secrets, constant-time secret verification,
  wall clock, deterministic test clocks, OS secure randomness, deterministic test randomness,
  canonical UUID v4, and bounded Argon2 password hashing.
- Task-scoped bounded byte streams with backpressure and cleanup; memory, local, and S3-compatible
  named object adapters with no-replace publication, checksum facts, streaming multipart upload,
  range/whole read operations, and reconciliation.
- Memory and PostgreSQL durable queues with exact idempotency key, attempt identity, lease,
  heartbeat, completion, failure/retry, cancellation, inspection, and stale-completion rejection;
  bounded resident workers.

Global hard maxima include 128 MiB artifacts, 1 MiB package descriptors, 4,096 modules, 1,024
dependencies/targets/grants, 4,096 active runtime tasks, 65,536 queued tasks, 64 MiB HTTP bodies,
1 MiB stream chunks, 65,536 live streams, 1,024 database connections, 1,000,000 rows, 16 GiB
objects, 16 MiB queue payloads/results, 24-hour leases, 64 KiB secrets, 1 MiB random requests, and
1,024-byte passwords. Deployment descriptors select smaller values for `lkjournal`.

## Direct cutover

The active tree contains no graph-authored project engine, semantic proposal mutation path,
application profile reader, stateful instance format, predecessor runtime protocol, `lkjedit`,
`lkjwork`, application-specific native binary, vendored terminal stack, or compatibility alias.
Opening a predecessor `.lkjscript/project` rejects with `source_predecessor_rejected`; artifact and
descriptor decoders reject predecessor versions directly. Historical code and evidence remain in
Git and the campaign ledger, not in a current reader.

The two old products were deliberately deleted rather than kept on incompatible parallel profiles.
Their user value is not claimed as migrated. The reproduced editor append, viewport, explorer,
status, visual-boundary, split, and large-file debts therefore remain historical unresolved product
evidence, not fixed defects.

## Known limits

- The language and current native adapters are trusted; this is not a hostile-code sandbox.
- HTTP is currently HTTP/1 without TLS termination, response streams, trailers, multipart, cookies,
  compression, WebSocket, or outbound client authority.
- URL support is bounded query decoding, not a complete URI/form library. HTML support is text
  escaping plus literal authored markup, not a typed multi-context template system. Markdown is
  stored as text; no parser or sanitizer is implemented.
- PostgreSQL uses `NoTls`; driver cancellation is cooperative only at operation boundaries. No
  production PostgreSQL integration is run without the explicit service profile.
- S3-compatible code has deterministic adapter tests but no retained live S3 conformance receipt.
  The local object backend validates but cannot persist provider content-type attributes.
- Configuration is a validated deployment map, not a schema declaration language with watches.
  Secret rotation is observed only by restart.
- There is no general filesystem, terminal, subprocess, HTTP client, metrics exporter, structured
  logging sink, distributed tracing, calendar scheduler, package registry, FFI, or GUI.
- Only Linux x86-64 has fresh-checkout evidence. No sandbox, multi-node coordination, distributed
  transaction, or multi-tenant operator isolation is claimed.
