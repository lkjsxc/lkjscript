# Effects and capabilities

Status: normative.

## Effect boundary

Pure meaning is independent of deployment, time, randomness, scheduling, network, storage,
credentials, and process state. A pure function cannot perform a capability operation or call a
task function. A task function has an explicit task effect containing stable requirement identities
and aliases. Effect checking is transitive through exact calls.

A component declares the requirements needed by its ports. Preparation proves that every task
capability use resolves to the required interface and operation. Deployment grants bind those
requirements to adapter instances; omission, foreign interface, operation mismatch, or excess
authority rejects before admission.

## Interfaces and operations

An interface owns stable operation identities, input parameters, output type, failure contract,
and semantic limits. A capability relation records the exact use site, requirement, interface, and
operation. Rename does not change identity. Interface evolution exposes requirements, callers,
components, grants, adapters, tests, targets, and artifacts through impact queries.

Closed external functions are pure or explicitly task-bound compiler/runtime intrinsics. Unknown
or forged intrinsic names reject during semantic validation; there is no ambient host call escape.

## Resources and visibility

Every live resource has exact acquisition, owner task, allowed operation, close, cancellation,
timeout, and cleanup semantics. Handles are runtime-only and cannot serialize into graph authority,
artifacts as values, backups, queues, objects, or logs. Streams use bounded chunks and backpressure.
Database transactions and queue leases are lexical/task-owned.

Operations that may have committed externally before visibility loss return the distinct possible
visibility class. Callers may retry only where the graph-owned idempotency contract permits it.
Cancellation and resource exhaustion are distinct from typed application failure.

## Generic adapters

Current adapters cover strict HTTP server dispatch, typed JSON, PostgreSQL and lexical
transactions, configuration, redacted secrets, clocks, secure/deterministic randomness, UUID,
Argon2 password hashing, bounded streams, memory/local/S3-compatible objects, and memory/PostgreSQL
durable queues and workers.

Production and deterministic test adapters share public behavior contracts but use disjoint
implementations. Tests use explicit scripted or deterministic grants and never ambient production
credentials. Deployment descriptors own adapter selection and limits; semantic artifacts own only
typed requirements.

## Structured runtime

One resident kernel performs preparation, admission, execution, capability routing, task
ownership, resource accounting, cancellation, shutdown, and observations. Concurrency and queues
are bounded. There are no detached ownerless tasks.

Graceful shutdown stops admission, drains within the configured bound, cancels remaining work,
allows only explicitly non-cancellable publication sections to finish, closes resources, and
returns a classified status. A process boundary is not a hostile-code sandbox or multi-tenant
security boundary.
