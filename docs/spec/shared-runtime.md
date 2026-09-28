# Shared service runtime

Availability: development v0.1.53. This is not part of the immutable public
v0.1.52 binary. Discover the actual executable with
`lkjscript capabilities --section deployment` and `lkjscript capabilities serve`.
The shared-runtime observation contract is version 1; the existing deployment
descriptor, graph and artifact encodings are unchanged.

## Explicit process ownership

```sh
lkjscript serve \
  --deployment alpha/service.deployment.json \
  --deployment beta/service.deployment.json
```

Repeated `--deployment` selects a finite service group in argument order. Each
descriptor still selects an exact immutable artifact, target, listen address,
configuration, secrets and grants. Relative artifact and adapter paths resolve
against that descriptor's own directory. The host does not open the author's
mutable project, select latest code, migrate application data, invoke another
application process or install a persistent process-global selection database.

A single descriptor retains the previous single-service command and output.
Two through 64 descriptors use the shared host. HTTP and interactive/WebSocket
targets may coexist, including different exact versions of the same application.
Workers and foreground commands are not members of this first service group.
Each member owns its listener; this is not host-name routing behind one listener.

The running host has one executable ABI and one Tokio runtime. This is genuine
same-process execution, not merely a shared runtime installation or a supervisor
that launches one process per application.

## Share code, not authority

Each supplied artifact path is read through the existing bounded, regular-file,
strict artifact admission path. A hit never skips reading or admitting that
input. Only a matching digest of that strictly admitted complete artifact may
reuse the same `Arc<NormalizedProgram>` in this executable. The admission proof,
normalized functions, type/layout tables, immutable constants and origin remain
bound to that exact program. A descriptor is validated even when code is shared.

Pooling is scoped to preparation of this explicit group. It is not keyed by a
mutable pathname, target name, package name, guessed compatibility or an unverified
user-provided digest. Distinct complete artifacts get distinct program objects.
Identical dependencies inside different whole-artifact versions are not yet
separately deduplicated.

Every instance independently prepares its configuration, secret catalog, grant
bindings, adapters, request state, resident kernel, task accounting, queues and
cancellation. Sharing a program does not reuse one instance's configuration or
give another instance its grants. Handled request failures, fuel exhaustion and
one instance's overload do not consume a sibling's admission permits.

This is a trusted hosted runtime, not an OS-process or hostile-tenant sandbox.
An operator can deliberately bind the same external data root, queue, endpoint
or secret into multiple descriptors; those resources then retain their ordinary
external sharing semantics. Different descriptor files alone do not promise
different databases. Runtime/native failures, process termination and the address
space remain common failure domains.

## Admit, bind, announce, run, join

The host first registers process termination handlers. It strictly admits every
member, validates service topology, and checks aggregate declared capacities
before opening any member's secrets or live adapters. This keeps an invalid later
artifact or target from triggering earlier live preparation.

It then prepares each member's private adapters. A typed failure closes already
prepared, still-uninvoked members and retains cleanup diagnostics. It creates
service wrappers and binds every listener before publishing one group `ready`
event. A bind or readiness-output failure closes uninvoked adapters and drops all
acquired listeners. No member is served before the whole group is ready.

Preparation runs under an explicitly owned blocking task. A startup signal
requests cooperative cancellation and then joins that owner, rather than assuming
that dropping or aborting its task handle stops native work. Existing bounded
artifact reads/admission still finish at their supported cancellation checkpoints.
There is no new universal startup-latency or immediate-interruption guarantee.

After readiness, member service futures run concurrently. SIGINT or SIGTERM asks
every member to stop admission, cancel/drain work and finish its existing adapter
cleanup. An infrastructure failure of a member also requests group stop. The
host keeps polling every started member to completion before returning; it never
returns the first error while abandoning siblings. Ordinary handled HTTP request
errors do not terminate the group. Cleanup cannot undo already visible
application effects.

Within each resident kernel, acceptance and queue registration share a short
critical section with stopping admission. Stop cannot report idle after accepting
an invocation but before counting its captured resources. The section contains
neither an asynchronous wait nor application execution or adapter cleanup;
semaphore wakeups also happen outside it. Execution remains independently
scheduled, not serialized behind this registration lock.

The fixed-group CLI has no dynamic load, hot reload, individual-stop command or
cross-instance messaging operation. The library's independently owned prepared
applications retain their separate shutdown boundaries. Dropping the preparation
set does not pin code: after the last owning application/reference is released,
the program is reclaimed. Outstanding explicit clones remain legitimate owners.
This is ownership reclamation, not forced unload of live references.

## Bounds and observations

A group admits at most 64 instances. The sum of declared
`maximum_concurrent_tasks` may not exceed 4096; the sum of declared
`maximum_queued_tasks` may not exceed 65536. These are the existing executable
resident maxima applied to a group, not permission to bypass each instance's
limits. All existing execution, stream, HTTP, session and capability limits
remain in force. In particular, these totals are not CPU fairness reservations,
a process-wide live-memory limit, or a promise that all accepted limits can be
physically exhausted simultaneously.

The multi-service `ready` event contains `process_id`, `shared_runtime`, and
`instances`. Each instance has a zero-based argument ordinal, concrete bound
`local_address` and the existing redacted deployment observation. Neither
configuration values nor secret values are included. The `stopped` event includes
the same selection/accounting observation and one joined receipt for each
instance. A failed group returns a diagnostic after cleanup, not a successful
`stopped` receipt.

`shared_runtime` reports the executable version, observation contract version,
instance count, exact program digests, instances per program, loader-reported
artifact object bytes, function/type counts, summed resident capacities, and
the count of private configuration fields. Program entries are digest ordered;
instance entries are argument ordered. These are structural observations, not
RSS/PSS, allocator bytes, peak/live heap, a complete count of adapter storage,
current retained memory after stopping, or measured speedup. Strict loading still
does work for every descriptor even on a normalized-code hit.

## Evidence and next boundary

The owning unit tests prove pointer identity for equal programs, non-aliasing for
different programs, descriptor/corruption rejection on cache hits, group bounds,
cancelled admission, independent shutdown, continued sibling execution, and
reclamation after the last owner across repeated start/stop cycles.

The public tests use copied executables and ordinary authored applications after
removing the authoring project. They cover four editor instances sharing two exact
versions, private origins/secrets/data, per-instance execution failure, a
separate-process persisted-state baseline, busy-sibling isolation, connected
WebSocket shutdown, both Unix termination signals, failed later admission, bind
conflict, closed readiness output and subsequent clean startup. The
[campaign](../campaigns/202609281356.md) records actual runs and limitations.

The next language boundary is explicit owned transfer and scoped read access,
including package-level generic producer/transformer/consumer tests. Queue
reservation, cancellation and receiver failure must preserve one valid owner or
a defined cleanup owner before any cross-instance zero-copy claim. This host does
not establish a new borrow checker, trait solver, region collector or parallel
speedup, and none is a prerequisite for this first code-sharing boundary.
