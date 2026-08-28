# Relational, object, and durable-queue contracts 1

This specification owns three generic data capability families. Application schemas, statements,
authorization, object-key policy, payload meaning, retry policy, and cross-authority coordination
remain authored application meaning.

## PostgreSQL

The PostgreSQL adapter accepts only source-origin `StaticText` statements plus ordered typed
parameters: nullable bool, i64, text, and bytes. Queries also carry an exact ordered row schema and
maximum rows. Returned rows must match column count, nullability variant, scalar type, UTF-8, and
bounds before the handler receives them. Global maxima are 1,024 connections, 300,000 ms pool wait,
1,000,000 rows, and 4,096 columns; component and deployment limits further restrict calls, bytes,
rows, connections, waits, and statement time.

The bounded pool either returns an idle connection, opens within its maximum, waits until the
earliest runtime/pool deadline, or reports resource exhaustion. Shutdown closes admission and idle
clients. A failed/protocol-broken client is discarded. Before a deployment publishes readiness,
each PostgreSQL-backed grant establishes one reusable connection, retrying only the retryable
no-publication connection class within the declared pool-wait bound. Individual connection attempts
are bounded to one quarter of that interval, clamped to 250 through 5,000 ms. When invoked from an
async runtime thread, synchronous driver connection establishment runs on one named blocking thread;
thread creation and panic are closed infrastructure failures. Current transport uses `NoTls`.
PostgreSQL TLS is deliberately out of scope and not planned; encrypted database transport requires
an external trusted boundary or a different adapter outside current scope. Malformed connection
descriptors reject immediately as a nonretryable capability failure and are never included in
diagnostics.

Lexical transactions execute multiple parameterized statements on one connection. Normal body
success commits; body error rolls back; dropped scope attempts rollback; commit connection loss is
`possible_visibility`. Transaction control text and nested transaction/migration operations reject.
Constraint/syntax/serialization/deadlock/cancellation/connection/protocol classes are mapped to
stable capability codes and retryability; unknown commit visibility is never retryable.

Migration takes positive integer identity, 64-hex checksum, and static SQL without transaction
control. The adapter transaction creates/locks `lkjscript_schema_migrations`, rejects checksum
divergence, atomically applies a new migration and records it, or reports unchanged. Migration order
is application-owned; deployments must invoke exact required migrations before dependent traffic.

These existing operations are now reachable from public compact task/capability authoring as
specified in [effects-capabilities.md](effects-capabilities.md). That authoring cutover does not
change the database contract, elevate SQL or PostgreSQL into language semantics, or add an ambient
database. Applications retain statements and row conversion behind explicit requirement-scoped
persistence functions; deployment retains provider selection and connection authority.

## Named object storage

An object grant binds memory, confined local root, or explicit S3-compatible endpoint/region/bucket
and prefix. Keys are validated relative opaque slash names beneath the prefix, at most 1,024 bytes;
application ownership/visibility is not inferred. Global object maximum is 16 GiB and whole-read
maximum may not exceed the selected object bound.

Operations are no-replace streaming `put-new`, bounded whole `get`, bounded `range`, `head`,
`reconcile-put`, and `delete`. Upload reads bounded chunks, computes BLAKE3, uses multipart at 5 MiB
parts when needed, validates content type, closes/aborts on failure, and returns key, size, digest,
provider version, and cleanup-pending fact. Reads verify selected bounds and close their provider
stream. Provider timeout/connection after publication becomes possible visibility; reconciliation
reads provider truth before application retry or domain commit.

Memory and S3 preserve provider attributes. The local backend validates content type but cannot
persist provider attributes; its durable truth is bytes/path plus returned integrity facts. A local
filesystem root is deployment authority, not a general selected-filesystem grant. Object and
database publication are never implicitly atomic: `lkjournal` records pending state and exposes an
application reconciliation route.

## Durable queue

Queue contract 1 provides memory and PostgreSQL stores with payload/result maxima up to 16 MiB,
lease up to 24 hours, and at most 1,000,000 attempts. A job has application-supplied job identity,
enqueue idempotency key, opaque payload, availability time, state, attempt count, optional live
lease/attempt identity, result, and safe last-error class.

Operations are initialize, enqueue, claim, heartbeat, complete, fail/retry, cancel, and inspect.
Enqueue is exact-idempotent by key. Claim atomically assigns one attempt identity and finite lease.
Heartbeat, complete, and fail require exact job, attempt, worker, live lease, and time. Lease loss or
replacement makes stale completion return false without publishing. Completion is single-success;
duplicate delivery cannot become a second completion. Failure chooses retry/no-retry and next
availability as application input. Cancellation makes future stale publication harmless.

PostgreSQL queue tables and atomic claim mechanics are generic adapter state under an explicit
namespace. Job payload, priority/order beyond ready-time/identity, retry class mapping, and domain
publication stay in lkjscript. Worker contract 1 runs a bounded number of structured tasks, waits a
bounded idle interval, stops on deployment shutdown, and reports iterations/results/failures without
logging payload content.

None of these capabilities promises cross-database/object/queue atomicity, exactly-once delivery,
distributed consensus, blind retry after possible visibility, or preservation of live handles
across restart.
