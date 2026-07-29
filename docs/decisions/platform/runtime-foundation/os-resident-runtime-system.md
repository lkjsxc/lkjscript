# OS-Resident Runtime System

## Purpose

Define the daemon-first product boundary that supervises installed lkjscript
applications without requiring one process or one address space.

## Status

**Accepted Contract with a Current Linux Coordinator Foundation.** The focused
foreground daemon, exclusive lease, bootstrap store, authenticated Unix control,
CLI, and service-definition slice are Current by their named evidence. The
inherited capability-free application runtime remains experimental. Process
cells, session brokers, GUI execution, database attachment, and other native
transports become Current only with their own execution evidence.

## Product Authority

One installed operating-system environment has one logical lkjscript runtime
authority. Its machine coordinator is `lkjscriptd`. The coordinator may
supervise subordinate processes but remains the sole durable control authority.
“Node” is not a Current product name.

The daemon owns package and application registries, grants, quotas,
incarnation routing, immutable compiler/code leases, the semantic resource
root, database supervision, logs, metrics, session registration, process-cell
supervision, recovery, and shutdown. It never presents user interface and never
executes application code with coordinator elevation.

Standalone execution is explicit bootstrap, recovery, differential-test, CI,
diagnosis, or development behavior. It never silently starts a competing
runtime authority.

## Principals And Deployment

Each control request derives its principal from authenticated local transport,
not request data. Application ownership, grants, resources, database tenants,
logs, sessions, and metrics are principal and application scoped. Until that
is completely proven, deployment admits exactly one configured principal and
rejects every other principal.

Linux machine deployment uses a system service plus a graphical-session user
service. Windows uses a noninteractive service plus a broker in each authorized
login session; Session 0 never owns UI. macOS uses a LaunchDaemon where
machine-wide deployment is available and a per-user LaunchAgent or login item
for interactive work. Container deployment runs one foreground coordinator.
Unexecuted adapters remain experimental, never accepted platform evidence.

## Applications And Incarnations

An installed application has kind `command`, `service`, or `interactive` and
one closed lifecycle:

```text
installed -> loading -> starting -> running -> quiescing -> stopping -> stopped
                |          |          |            |           |
                +----------+----------+------------+----------> failed
stopped|failed -> loading
installed|stopped|failed -> uninstalled
```

An `ApplicationIncarnationId` identifies one successful start or replacement
realization. It contains an application identity and an internal nonzero slot
counter. Public stale-identity diagnostics say `incarnation`; generation
remains only an internal reusable-slot safety mechanism.

Start validates package, owner, grants, quotas, immutable artifacts, selected
cell, private providers, and tenant bindings before atomic routing publication.
Stop closes admission, quiesces routing, cancels the root scope, aborts active
transactions, closes resources, destroys private memory, stops the cell,
releases leases, removes routing, and persists cleanup outcomes.

Restart policies are `never`, bounded `on-failure`, and bounded `always`, with
explicit attempt and window limits. Rolling replacement starts and health
checks a new immutable package incarnation before routing changes and old-cell
quiescence. Mutable memory is never migrated implicitly.

## Cells And Sharing

Current cell classes are `trusted-in-process`, `isolated-process`, and
`interactive-process`. A trusted in-process cell is legal only for verified
code and reviewed in-process-safe providers. Capability-bearing applications
use isolated process cells until their exact in-process provider boundary is
independently proven. Interactive cells run in an authenticated user session
and own their native main-thread event loop and frame loop.

Immutable content-addressed source closures, HIR, verified SSA, proof-checked
SSA, code images, literals, and metadata may be shared under bounded leases.
Mutable heaps, capabilities, ledgers, transactions, task scopes, metrics,
resources, and traced guest references remain incarnation private. No writable
application static or traced reference crosses a cell boundary.

## Host And Database Boundaries

Each cell receives one application-private composition of implemented typed
provider families. Package capability requirements are requests; daemon grants
are authority. Ambient roots, raw host descriptors, command-line secrets, and
user-supplied principals grant nothing.

The daemon bootstrap journal is independent of the application database. The
database starts later as one supervised service. Tenant identities are
principal and application scoped; transactions are typed affine resources and
abort on stop, cancellation, deadline, or cell loss. SQLite remains a separate
compatibility provider and oracle.

## Resource Hierarchy

Admission and accounting follow:

```text
system -> principal -> application -> incarnation -> scope -> task
```

Reservation precedes publication. Per-application concurrency caps, bounded
weighted deficit, reserved interactive/service share, and bounded bursts are
the initial explainable policy. Interactive preference cannot starve control,
cleanup, or durable database work. Coordinator, control, GUI host, database,
scheduler, and cache state never enters `GcHeap`.

## Failure And Evidence

A cell crash fails only its incarnation, closes provider leases, aborts its
transactions, applies bounded restart policy, and leaves daemon control and
other applications usable. Exact process, database, GUI, and platform execution
evidence is required before each corresponding Current claim.

The inherited experiment installed, started, stopped, restarted, removed,
listed, and invoked private capability-free VMs with bounded admission and
immutable chunk leases. It was not a daemon or persistence boundary. Git
history preserves that experiment; the obsolete public node and application
generation names provide no alias.
