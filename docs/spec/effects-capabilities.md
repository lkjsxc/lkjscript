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

Compact change 6 authors this boundary directly. `add.requirement` extends an existing component
with one exact built-in interface, ordered admitted operations, and separately named resource
limits. `create.function effect=task` and `set.function-contract effect=task` name an ordered exact
set of local requirements; the latter changes the existing function contract without replacing its
identity, parameters, or body. `expression.capability-call` names one admitted requirement and exact
operation, while `expression.transaction` creates one lexical database scope. Request labels are
not capabilities, and there is no ambient requirement search or unchecked IO effect.

Validation rejects pure-to-task calls, pure capability use, absent effect requirements, duplicate
requirements, cross-component or foreign references, unadmitted operations, interface mismatch,
escaping transaction bindings, nested transactions, and effect escalation before publication.
Failed planning or apply leaves accepted authority unchanged.

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

Application-owned persistence functions may depend on the generic database interface and typed SQL
values while HTTP routing, request admission, domain validation, and response construction depend
only on domain types and those narrow functions. Statements, migration identity/checksum, row
conversion, and transaction policy are current graph meaning; PostgreSQL coordinates, credentials,
pool and timeout values, and adapter choice are deployment authority. This seam is replaceable:
neither SQL text nor PostgreSQL is a language effect or permanent semantic identity.

## Structured runtime

One resident kernel performs preparation, admission, execution, capability routing, task
ownership, resource accounting, cancellation, shutdown, and observations. Concurrency and queues
are bounded. There are no detached ownerless tasks.

Graceful shutdown stops admission, drains within the configured bound, cancels remaining work,
allows only explicitly non-cancellable publication sections to finish, closes resources, and
returns a classified status. A process boundary is not a hostile-code sandbox or multi-tenant
security boundary.
