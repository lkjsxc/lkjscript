# Persistent Application Registry And Control

## Status

**Current.** Linux x86-64 durable restart and authenticated local control are
covered by the named evidence below.

## Decision

`lkjscriptd` owns one bounded durable application registry in its bootstrap
control store. The application database is never required to discover or start
registered applications.

A registry identity is a monotonic nonzero `u64` scoped to the machine
coordinator. It is stable across coordinator restart and distinct from the
runtime's current application and incarnation identities. Removed identities
are never reused.

## Durable Record

Each record binds:

- registry identity and canonical application name;
- full package content digest;
- canonical absolute package root and normalized relative package entry;
- isolated-process cell class;
- sorted unique capability grants;
- concurrent and total invocation quotas; and
- desired state, either `stopped` or `running`.

Records use one closed checksummed control-store value with explicit widths and
bounds. Unknown tags, duplicate identities or names, malformed paths, stale
platform state, unsupported capabilities, and impossible quotas fail recovery.
The next registry identity is a separate durable fact advanced before a newly
assigned identity can be reused.

Installation validates and publishes the runnable runtime application before it
commits the durable record. A durability failure removes that unpublished
runtime application. Removal deletes the durable record only after the runtime
application stops and uninstalls successfully.

## Recovery

Coordinator bootstrap decodes all registry facts in identity order, binds the
fixed sibling `lkjscript-cell` worker, reconstructs every installed process
application, and starts records whose desired state is `running`. Any record
that cannot be validated, installed, or started fails coordinator startup; it
is never silently skipped.

A clean or unclean daemon restart creates new runtime application and
incarnation identities behind the stable registry identity. Desired running
state is committed only after start succeeds. Desired stopped state is committed
only after stop succeeds.

## Authenticated Local Control

The existing kernel-authenticated local transport adds closed operations:

- install, list, start, stop, restart, remove, and invoke application.

Every modifying request requires the existing nonzero idempotency identity and
uses the replay cache. Requests carry platform revision and the exact
runtime-control digest. Application arguments and all strings are explicitly
bounded. The application cannot choose the worker executable or environment.

List and lifecycle responses expose stable registry identity, name, desired
state, runtime lifecycle, current incarnation when present, and worker process
when present. Invoke returns the exact closed `ExecutionOutcome` plus only the
stdio bytes produced by that invocation. The control frame bound applies; a
request whose legal result cannot fit is rejected before execution by the
registry's execution limits.

## Initial Bounds

- registry entries: 1,024;
- application name: 64 UTF-8 bytes;
- package root and entry: 4 KiB each;
- invocation arguments: process-cell protocol bounds;
- per-invocation output: 16 KiB;
- returned heap: 32 KiB;
- concurrent invocations: 1..=64;
- total invocations: nonzero `u64`;
- local-control frame: existing 64 KiB maximum.

## Service Binding

`lkjscriptd` resolves one canonical `lkjscript-cell` sibling of its own
executable at bootstrap. Generated service definitions install/launch that
fixed product pair. A package record cannot supply or replace the worker path.

## Current Evidence

A focused Linux integration started the packaged `lkjscriptd`, installed the
locked factorial hello package through `lkjscript system app install`, started
and invoked it, observed exact private stdout and a lossless returned outcome,
then killed the daemon without clean shutdown. It restarted from the same
bootstrap store, observed the unclean marker, recovered registry identity `1` with desired/running state,
started a new worker incarnation, invoked it again, stopped and removed it, and
shut down cleanly. Control codec tests cover install and invoke payloads,
application views, outcomes, malformed frames, revision/digest rejection, and
idempotent modifying replay.

The Current registry is isolated-process only. Trusted in-process artifact
serialization, database tenants, restart-policy automation after a runtime
failure, principal impersonation, and non-Linux control transports remain
outside this cut. A failed durable remove attempts runtime reconstruction, but
injected failure coverage for every multi-step registry rollback is not yet
Current. Coordinator-wide admission totals are process-lifetime accounting and
reset on daemon restart; per-app configured ceilings persist, but consumed-total
counters do not yet persist.

## Rejected Alternatives

- Database-only application discovery is rejected because database failure must
  not prevent bootstrap control.
- Reusing runtime-local application identities after restart as durable public
  identities is rejected.
- Persisting arbitrary commands, environment variables, or loader search paths
  is rejected.
- Silently dropping a malformed or unstartable record is rejected.
