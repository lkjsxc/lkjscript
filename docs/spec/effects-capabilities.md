# Effects and capabilities

Status: normative.

## Effect boundary

Pure meaning is independent of deployment, time, randomness, scheduling, network, storage,
credentials, and process state. A pure function cannot perform a capability operation or call a
task function, including one with an empty requirement row. It may construct and return task
descriptors without executing them. A task function has an explicit row containing stable exact
requirement operands and function-owned effect parameters. Effect checking is transitive through
direct and indirect calls after explicit substitution.

A component declares the requirements needed by its ports. Preparation proves that every task
capability use resolves to the required interface and operation. Deployment grants bind those
requirements to adapter instances; omission, foreign interface, operation mismatch, or excess
authority rejects before admission.

Compact change records author this boundary directly. `add.requirement` extends an existing component
with one exact built-in interface, ordered admitted operations, and separately named resource
limits. `create.function effect=task` and `set.function-contract effect=task` name an ordered exact
set of exact requirements and in-scope effect parameters; the latter changes the existing function contract without replacing its
identity, parameters, or body. `expression.capability-call` names one admitted requirement and exact
operation, while `expression.transaction` creates one lexical data-transaction scope. Request labels are
not capabilities, and there is no ambient requirement search or unchecked IO effect.

Validation rejects pure-to-task calls, pure capability use, absent effect requirements, duplicate
requirements, cross-component or foreign references, unadmitted operations, interface mismatch,
escaping transaction bindings, nested transactions, and effect escalation before publication.
Failed planning or apply leaves accepted authority unchanged.

## Explicit effect applications and authority-free descriptors

An effect parameter is descriptive meaning. It grants no authority to its declaration, descriptor,
holder or importing library. Pure factories may be effect-parametric and return bound
`TaskFunction` values. Named task values and calls require exact ordered ordinary type, effect and requirement
arguments; omitted/excess/foreign arguments reject even if unused. Rows normalize as set union by
stable identity; equal names or interfaces never merge atoms. Callable type equality compares exact
rows and has no effect subtyping. Signature traversal substitutes task rows nested in pure callable
types, aggregates and nominal applications. Unknown symbolic rows permit forwarding and invocation
of equally typed callbacks, not guessed capability operations.

Ordinary generic calls and named function values obey the finite parameter-flow rule in
[language.md](language.md). A recursive task may permute both ordinary type arguments and ordered
effect arguments simultaneously, including swapping callbacks with different rows. Every new
activation still checks its substituted allowance and exact grants. Effect unions range over the
finite exact requirements in the closure; they do not create ordinary type constructors or cancel
an expanding ordinary-parameter cycle. This rule changes no resource-signature, capture, transaction
or pure/task restriction.

Three checks stay separate: the target's declared resolved row, the current activation's declared
resolved allowance, and the selected component's actual checked grants. Every nested activation
receives its own allowance. A concrete required atom is covered by itself or a validated requirement
with the same package, name and exact interface, a superset of required operations, and compatible
declared limits. Symbolic inclusion requires the same exact parameter. Runtime coverage additionally
requires the references to resolve to the same canonical grant. Shared grants cannot widen a narrow
activation, merge row identities, reset counters or clone authority. Preparation closes HTTP, worker
and structured-session task ports before live execution.

Creation, binding and returning retain only an exact prepared target, closed ordered type/effect/requirement
arguments and a flat capture-safe prefix. Foreign preparation, forged kind/row/target, wrong arity,
mismatched prefixes and captured live resources reject before target execution. Descriptor copies
hold no adapter, credential, resource, transaction or frame. Both evaluators enforce activation
allowances independently; component-wide grant search cannot replace this check.

Indirect and imported calls use the same canonical grant counters, resource provenance and active
lexical transaction as direct calls. Nested transactions remain forbidden. A later trap,
cancellation or exhaustion stops subsequent callbacks and emits no successful partial traversal;
earlier effects remain visible unless the applicable transaction rolls back its staged work. An
ordinary returned `Result` remains a value until graph control flow branches on it. No evaluator
replays live effects or converts these runtime failures into application results.

The standard's graph-owned `task-fold-left` uses a private sequential index/state tail loop over
the original persistent list. `task-map` binds a mapper into a task fold step and appends each
result. Each input invokes its callback once in order; empty input invokes none. Traversal uses
constant additional control space. Indexed reads and persistent append retain their existing
costs; cumulative allocations, payload retention and operation quotas are not refunded.

The public nominal `iteration-step<State,Output>` has ordered unconstrained type parameters and
stable `continue(State)` and `done(Output)` cases. The graph function
`task-iterate<State,Output;E>(State, TaskFunction(State)->iteration-step<State,Output> ! E)->Output ! E`
invokes its callback once, returns the `done` payload, or tail-calls itself with the `continue`
payload and the same checked callback. Immediate completion invokes once. State and output need
no capture-safe constraint; ordinary transient callable-containing states retain their existing
containment, equality and durability restrictions. Case names add no special encoding or effects.
There is no iteration-count argument, scheduler, implicit yielding or exception-to-result
conversion. Runtime failure/cancellation/exhaustion stops later callbacks; a returned `Result`
remains ordinary data. Earlier effects keep possible visibility, and an enclosing transaction
rolls back staged effects under its original lexical owner.

Task tail admission checks the outgoing activation's row and actual canonical grant mapping
before its scopes disappear, including aliases with distinct operation allowances. The canonical
reference evaluator passes an internal admitted target/application/argument handoff to its
trampoline; that single-transition object is neither a callable value nor transferable authority.

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

## Operations and transactions through requirement parameters

An ordinary imported library may perform operations through a caller's explicitly supplied
requirement without owning or importing the caller's component. Its formal constraint names an
exact interface and a minimum operation set. Selection performs no operation and supplies no
deployment authority. Concrete activation coverage retains its previous narrow semantics;
minimum constraints neither widen an older narrow activation nor synthesize grants.

A DataStore formal admitting the existing transaction operation may own a lexical transaction.
The actual supplied requirement identifies its transaction and canonical accounting. Direct calls,
imported helpers and callbacks through that same resolved requirement share the transaction.
Nested transactions remain rejected. Cancellation and failure stop later effects and join cleanup.
A second store or a Configuration/HTTP operation is not included in the first store's atomicity.

A false conditional expectation suppresses publication of the entire transaction. Later operations
can still return their local results; normal lexical completion can mean committed or unchanged.
Commit-time base conflicts retain their execution failure. A graph library may return an ordinary
`UpdateAttempt<T>` record with `candidate: T` and `primary_condition_matched: Bool` after normal
lexical completion. The boolean reports only its primary conditional operation, and neither field
certifies durable publication. An effectful callback may already have caused a false expectation,
so the primary flag can be true while the whole transaction publishes nothing. No automatic retry,
exception conversion or changed expectation semantics follows from this API.

Local affine operations are also valid through a constrained requirement parameter. An acquired
resource carries the symbolic operand and exact interface during generic validation. R1 and R2
remain different provenance even if one concrete application aliases them. Matching, borrowing,
consuming and dropping require the same operand under existing local rules; runtime provenance
resolves to the original concrete requirement, canonical grant and task scope. This does not admit
generic resource-bearing function parameters/results, capture, aggregate escape or cross-function
transfer. The private concrete consume-helper exception remains unchanged.

Typed-cell libraries use the existing typed-data encoder and decoder, not JSON. Decode-or may use
a supplied default on absence or incompatible encoding/layout, but does not swallow cancellation,
resource exhaustion or execution failures. Closed intrinsically unencodable applications reject at
the existing pre-effect admission boundary. Capture safety and typed-data eligibility are separate;
only retained values need the existing capture constraint. Space/schema ownership and migration
remain caller policy. An application result or cleanup/output failure never authorizes effect replay.
