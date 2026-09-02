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

Compact change records author this boundary directly. `add.requirement` extends an existing component
with one exact built-in interface, ordered admitted operations, and separately named resource
limits. `create.function effect=task` and `set.function-contract effect=task` name an ordered exact
set of local requirements; the latter changes the existing function contract without replacing its
identity, parameters, or body. `expression.capability-call` names one admitted requirement and exact
operation, while `expression.transaction` creates one lexical data-transaction scope. Request labels are
not capabilities, and there is no ambient requirement search or unchecked IO effect.

Validation rejects pure-to-task calls, pure capability use, absent effect requirements, duplicate
requirements, cross-component or foreign references, unadmitted operations, interface mismatch,
escaping transaction bindings, nested transactions, and effect escalation before publication.
Failed planning or apply leaves accepted authority unchanged.

## Interfaces and operations

An interface owns stable operation identities, input parameters, canonical parameter-use modes,
output type, failure contract, and semantic limits. Ordinary parameters are unrestricted. A direct
`CapabilityResource<Interface>` operation parameter is explicitly borrow or consume, and its
interface must be the operation's exact owning interface. The one admitted task-helper resource
parameter is final and consume-only and carries an exact requirement binding in the callee effect.
A parameter-requirement relation makes that binding part of summaries, impact, package validation,
compilation, artifacts, and inspection. A capability relation records the exact use site,
requirement, interface, and operation. Resource types additionally retain an exact interface
relation, so interface evolution reaches requirements, callers, components, grants, adapters,
tests, targets, package interfaces, compilation, and artifacts through impact queries. Rename does
not change identity.

Closed external functions are pure or explicitly task-bound compiler/runtime intrinsics. Unknown
or forged intrinsic names reject during semantic validation; there is no ambient host call escape.

## Resources and visibility

Every live resource has exact acquisition, owner task, interface, acquiring requirement, allowed
operation, close, cancellation, timeout, and cleanup semantics. Exact-interface capability
resources are acquired only by a call through that same requirement. Borrow preserves the local
right; consume removes it before the external operation. A foreign requirement, interface, scope,
kind, closed slot, duplicate consume, or post-consume use rejects. Handles are runtime-only and
cannot serialize into accepted literals, graph data, artifacts as values, caches, backups, queue
payloads, objects, or logs. Streams use bounded chunks and backpressure. Data transactions and
queue leases are lexical/task-owned.

The compiler records borrow/consume on local loads and retains the exact requirement on a resource
parameter. Strict artifact loading and normalized preparation recheck the private task signature,
exact requirement/interface, immediate final consume load, direct call, and acyclic resource-call
graph. Each VM and reference call frame revalidates the same live task scope, kind, requirement,
interface, and slot, so hostile derived input cannot copy, rebind, revive, or indirectly invoke a
transition right. Resource-bearing variants move as one outer value; matching transfers the direct
payload only to the selected arm. Task cleanup drops local handles without implicitly completing,
failing, cancelling, or otherwise mutating an external queue lease.

Operations that may have committed externally before visibility loss return the distinct possible
visibility class. Callers may retry only where the graph-owned idempotency contract permits it.
Cancellation and resource exhaustion are distinct from typed application failure.

## Generic adapters

Current adapters cover strict inbound HTTP server dispatch, deployment-bound exact-endpoint
outbound HTTP/1.1 GET, typed JSON, the first-party ordered data store and lexical transactions,
configuration, redacted secrets, clocks, secure/deterministic randomness, UUID, Argon2 password
hashing, bounded streams, memory/local/S3-compatible objects, and the first-party durable queue and
workers.

Production and deterministic test adapters share public behavior contracts but use disjoint
implementations. Tests use explicit scripted or deterministic grants and never ambient production
credentials. Deployment descriptors own adapter selection and limits; semantic artifacts own only
typed requirements.

Application-owned persistence functions depend on the exact standard `DataStore` interface while
HTTP routing, request admission, domain validation, and response construction depend only on domain
types and those narrow functions. Space and index policy, typed encodings, schema identities,
expectations, and transaction ordering are graph meaning. Deployment owns the confined first-party
root, namespace, sharing domain, authority revision, and independent limits. There is no production
provider selector, SQL surface, connection credential, network database, or fallback backend.

Outbound graph meaning depends on the exact standard `HttpClient` interface. It supplies only a
bounded ordered header list and consumes status, ordered headers, and whole body bytes. The exact
endpoint, address class, TLS trust, deadlines, and independent limits belong to one deployment
grant. The operation is idempotent with possible external visibility, but the adapter does not
redirect or retry. Exact destination admission, cancellation, and nonclaims are normative in
[outbound-http-client.md](outbound-http-client.md).

## Structured runtime

One resident kernel performs preparation, admission, execution, capability routing, task
ownership, resource accounting, cancellation, shutdown, and observations. Concurrency and queues
are bounded. There are no detached ownerless tasks.

Graceful shutdown stops admission, drains within the configured bound, cancels remaining work,
allows only explicitly non-cancellable publication sections to finish, closes resources, and
returns a classified status. A process boundary is not a hostile-code sandbox or multi-tenant
security boundary.
