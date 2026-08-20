# Runtime kernel, resource, telemetry, and foreground-session contract

This specification owns topology-neutral operational composition of exact applications, durable
instances, resource admission, trusted adapter routing, bounded stage observations, and the
foreground runtime session. It does not own semantic application meaning or durable instance state.

## Kernel boundary

`RuntimeKernel` is constructed explicitly from one validated `RuntimePolicy` and, for instance
operation, one exact instance-store root. It calls the existing application and instance owners for
all validation, execution, publication, replay, inspection, and host outcomes. One-shot CLI commands
and the foreground session use this same kernel; neither topology contains a second semantic path.

The kernel may read exact application bytes, open an instance store, admit an operation, prepare a
typed mutation or pure query, coordinate an adapter call, and collect disposable observations. It is not a
workspace `Engine`, package resolver, application registry, grant registry, instance authority,
semantic scheduler, durable queue, deployment manager, or sandbox. There is no implicit current
application, instance, grant, or revision.

## Retained deployment policy

Runtime contract version 2 owns these operational maxima:

- JSON request 8 MiB and response 32 MiB;
- application artifact 256 MiB;
- one loaded-application allowance and one open instance store;
- one active transition, one active host operation, and one concurrent compilation; and
- zero queued requests, compiled-unit bytes, cache bytes, and profile bytes.

Every nonzero limit is validated against its owning global maximum. A policy requesting a queue,
cache, profile, compiled unit, multiple stores, or a zero/over-limit active resource rejects. The
current implementation is synchronous: a request either acquires the applicable single slot or
returns `authority_busy`; no accepted request waits in a hidden queue.

Request bytes are encoded and checked before semantic work. Application bytes are checked after a
bounded read and before validation. Transition, host, compilation, and store reservations have one
release path on success or error. Durable state/history/blob bytes remain accounted by their
instance or grant owners; the kernel does not pretend logical counts enforce process RSS, open-file
limits, or OS CPU shares.

## Stage observations

Runtime observations are bounded disposable counters, never semantic results, durable authority, or
cache identity. Monotonic durations and applicable byte/work counters separate:

- runtime startup, request decode, peer authentication, authority resolution, and admission;
- application read, envelope decode, canonical re-encode, release-graph validation, release tests,
  and closure flattening;
- lowering, Core verification, execution, and public-value materialization;
- instance-store open, instance open, record-chain validation, replay, transition preparation, and
  state publication;
- grant validation, adapter preparation, host action, and outcome publication; and
- queue delay, cache lookup/population/eviction, and response encoding.

Stages with no retained implementation remain present with exact zero observations. Inspection also
reports requests, rejected admissions, application reads, instance/adapter operations,
compilations, releases decoded, flattened items, replay records/history bytes, cache counters,
current reservations, supported topology/adapter names, and explicit omissions. Timing values may
vary and cannot be consumed by applications.

`runtime orientation` is the compact discovery surface. It reports contract versions, exact
interface identities, supported topologies/adapters, default limits, exact root categories, and
commands for bounded expansion. `runtime inspect --store DIRECTORY` reports one kernel snapshot.

## One-shot topology

Application and instance CLI commands construct one kernel, execute one operation, flush one
bounded response, release resources, and exit. Process cleanup is operational only; published
authority remains in its owner. An output failure cannot undo a completed publication. Exact
idempotency or inspection is the retry route.

Current-revision operations delegate to the instance owner's HEAD-bound current manifest and exact
fallback. Historical queries, history pages, and deep audit delegate to the full chain oracle. The
kernel does not duplicate either loader or turn operational reuse into semantic authority.

## Foreground session protocol version 2

`runtime session --store DIRECTORY` opens one kernel and store lock for a caller-owned lifetime.
It accepts one strict line-delimited JSON envelope per request:

```json
{"version":2,"request_id":1,"request":{"kind":"inspect_runtime"}}
```

Version must be 2 and request ID must be nonzero. Every request names its exact application path,
instance, base, command, grant, or inspection target as applicable. Supported variants are create,
validate/apply event, pure query, execute host, fake outcome, validate/resume, inspect instance,
history, delete, inspect runtime, and shutdown.

Input is bounded to the workspace JSON maximum before allocation. Unknown/duplicate fields,
unknown variants, wrong versions, invalid IDs, malformed UTF-8/JSON, trailing JSON, and oversized
lines reject. A malformed or oversized complete line produces one bounded error and does not
desynchronize later lines. Success and error envelopes repeat the exact request ID when decode made
it available. Each line retains its own publication boundary.

EOF stops accepting work and releases the store lock after the current synchronous request. An
accepted shutdown response is encoded, written, and flushed before exit. Output failure ends the
process; it does not roll back authority. Because the retained kernel has no queue, worker, or
concurrent operation, there are no queued or active background tasks to drain or cancel.

The session retains only kernel counters and the store handle. It does not cache application
meaning, grant decisions, HEAD, pending commands, or authorization. Restart reconstructs all
semantic and instance state from durable owners.

The application-specific `lkjwork session` is a separate caller-owned product transport. Its
`InstanceStore::open_session` may retain one prepared application and one current state/manifest
object keyed by exact HEAD. Every hit rereads and compares HEAD; publication updates the entry only
after success; a miss, eviction, process restart, missing manifest, or corrupt manifest remains fully
correct. This reuse is bounded to one project and does not create a persistent cache or second query
owner.

## Foreground interactive topology

The interactive topology is a caller-owned process over immutable application-format-8 bytes. It is
separate from durable instance authority: initialization creates one ephemeral application state;
each accepted event or host outcome produces the next state; render observes that state; process
exit discards it. There is no checkpoint, journal, recovery file, daemon, or publication per key.

`PreparedInteractiveApplication` validates and compiles every profile role once. `InteractiveSession`
serially invokes the exact semantic initialize, update, resume, and render owners. It allows one
unresolved typed action and rejects further events as `authority_busy`. Host outcomes are bounded
closed values. The application cannot call an adapter directly and the runner cannot inspect
private state to invent behavior. Candidate state and pending-action changes become observable only
after the candidate frame also validates and renders; any update, resume, fuel, value, or frame
failure preserves the prior ephemeral state and pending action.

Headless replay contract 3 is the independent deterministic product route. One strict JSON request
binds dimensions, at most 10,000 events, and at most 10,000 ordered outcomes within 8 MiB. The runner
uses the same session owner as live execution and reports event/action/change counts, initial/final
frame digests, final frame, and a digest over the complete replay. Missing or extra outcomes,
oversized input, trailing JSON, invalid event data, or an unresolved final action reject. Replay
publishes no semantic, instance, file, or project state.

The generic terminal runner owns terminal acquisition, event decoding, full-frame projection,
action dispatch, signal observation, and cleanup under terminal contract 3. `WorkbenchHost` owns a
process-local composition of one exact semantic-project grant and an optional selected-filesystem
grant. It routes the application's closed actions to `ProjectHost` contract 2 and
`SelectedFilesystem` contract 1; it has no private graph/store mutation route. Exact external
publication survives terminal output failure.

The retained loop is synchronous and sequential. A potentially slow target action blocks input; no
background job, queue, cancellation token, thread pool, or async runtime exists. This topology was
selected because the complete workbench and 10,000-event replay meet their current gates without a
second scheduling authority. Background work may be reconsidered only after a named operation
misses a measured responsiveness gate and a bounded stale-result/cancellation contract is proved.

## Topology and acceleration exclusions

The supported topologies are one-shot, durable foreground session, and ephemeral interactive
foreground session. There is no Unix-socket supervisor, automatic spawn, daemon, worker pool,
request multiplexing, scheduler, persistent cache, generic application/Core cache, bytecode, native
compiler, or JIT. Generic runtime cache counts and bytes are exactly zero. The product-local
exact-HEAD reuse above is disposable and independently differential. The explicit-frame interpreter
remains the sole execution route and oracle.

A resident supervisor requires a demonstrated multi-client or centralized-admission consumer beyond
the foreground session. A cache requires repeated validation/lowering to cross the documented
complete-workload gate. A new execution tier requires execution itself to dominate and must retain
the interpreter differential. Until then, these are explicit absences rather than dormant formats.
