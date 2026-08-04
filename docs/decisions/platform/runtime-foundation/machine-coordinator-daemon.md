# Machine Coordinator Daemon And Local Control

## Purpose

Define the persistent `lkjscriptd` coordinator, its database-independent boot
store, authenticated local protocol, and installable service boundary.

## Status
<!-- LKJ-F os-resident-runtime-foundation current tv7TmvPjmBID87VD91R-ewgRyhsyYnqTN6Hirq-t9VY -->


**Current on Linux x86-64** for the focused foreground coordinator, exclusive
lease, control journal/snapshot, kernel-peer-authenticated Unix transport,
application lifecycle/invocation client, and deterministic service generation
covered by [exact evidence](../../../current-state/os-resident-runtime-evidence.md).
Windows and macOS definitions remain experimental until built and executed on
native hosts. Privileged installation is never implied by file generation.

## Coordinator Identity And Lease

One state directory has one nonzero coordinator identity and one exclusive
process lease. A second live coordinator fails before opening control or
starting services. On Linux the lease is an atomically created local file that
records the process ID and coordinator identity; stale recovery is one bounded
check that confirms the recorded process is absent before replacement.

The coordinator starts in this order:

1. validate platform revision and control contract digest;
2. acquire the exclusive lease;
3. recover the bootstrap store;
4. mark shutdown unclean;
5. bind authenticated local control;
6. initialize runtime, compiler/code, resource, and database supervisors; and
7. admit application and session work.

Shutdown closes admission, quiesces applications, aborts transactions, closes
sessions and control, checkpoints bootstrap state, writes the clean marker,
synchronizes durable bytes, and releases the lease.

## Bootstrap Control Store

The control store is independent of `lkjscript-database`. It contains bounded
canonical key/value facts for coordinator identity, installed packages,
application manifests and owners, grants, quotas, restart policy, current and
previous incarnations, endpoint routing, durable session facts where needed,
database tenant mappings, code-cache metadata, and clean shutdown.

Records use explicit little-endian widths, one platform revision, exact control
contract digest, monotonic sequence, bounded payload, and SHA-256 checksum. An
append journal is synchronized before publication. A canonical snapshot is
atomically replaced and synchronized before the journal is reset. Replay is
idempotent, rejects sequence gaps and corruption, and ignores only an incomplete
final frame. Unknown fields or operations fail closed.

A platform revision mismatch rejects startup with an exact recovery command
unless one atomic migration is implemented. The Current implementation carries
one parser only.

## Local Control Envelope

The stable schema is `lkjscript.local-control`. Every frame contains:

```text
frame length
platform-revision
control contract digest
request identity
idempotency identity
operation
bounded typed payload
```

Responses repeat request identity and carry one typed success or error. Unknown
operations/fields, stale revision, wrong digest, replay conflict, oversize,
partial frame, deadline expiry, cancellation, and disconnect fail closed.
Modifying requests retain their result under a bounded idempotency key so a
retry cannot repeat an effect.

Bootstrap `describe`, `status`, and shutdown use the fixed envelope. Current
application operations are install, list, start, stop, restart, invoke, and
remove. Current session operations are register, list, heartbeat, and
unregister. Application status/log, grant, revoke, quota update, and database
control operations remain absent; inert variants are forbidden.

## Authentication And Transport

Unix-domain control obtains principal identity from `SO_PEERCRED`; socket path
and permissions restrict connection, but peer credentials remain authority.
User-supplied names grant nothing. The initial Linux deployment accepts only the
configured effective user ID and rejects other principals before decoding an
application operation.

Windows uses an ACL-restricted named pipe and derives the impersonated client
token identity. macOS uses a permission-restricted local socket and peer
identity; XPC remains Deferred. Foreground/bootstrap mode supports one inherited
stdio channel. Unauthenticated TCP is rejected.

Control frames are bounded before allocation. No request or response contains a
raw pointer, host descriptor, capability secret, database file, or mutable guest
reference.

## Service Adapters

`lkjscript system install|uninstall|start|stop|status|describe` is a client or
installer boundary. Linux generates a systemd system unit for `lkjscriptd` and
a systemd user unit for `lkjscript-session`. Windows generates a noninteractive
service contract; UI remains in user sessions because of Session 0 isolation.
macOS generates a LaunchDaemon and per-user LaunchAgent contract; AppKit remains
in the agent. Container execution uses `lkjscriptd --foreground` and maps
termination signals to exact quiesce.

Generation is deterministic and testable without privilege. Installation fails
safely without authority. Native signing, service registration, start/stop, and
shutdown evidence is platform-specific and must be reported separately.

## Privilege And Failure

The coordinator uses minimum deployment rights. Verified application code never
runs in the coordinator's elevated context. Capability-bearing code runs in a
target-principal process cell and receives only authenticated provider proxies
or inherited handles.

Control-store failure preserves the last durable state and publishes no partial
mutation. Database unavailability cannot prevent control recovery or status.
A coordinator crash leaves a recoverable unclean marker. A cell crash cannot
corrupt the control journal or stop unrelated applications.
