# Authenticated Interactive Session Broker

## Status

**Current on Linux x86-64** for the authenticated presence protocol, lease
registry, foreground broker, CLI listing, generated Linux composition, and
focused evidence. Interactive cell launch, display access, accessibility,
graphics, and GUI applications remain an Accepted Target.

## Decision

A per-login `lkjscript-session` process registers ephemeral interactive-session
presence with `lkjscriptd` over the existing authenticated local control
transport. The coordinator never presents UI. The broker gains no application,
display, filesystem, network, executable, or environment authority from
registration.

Linux registration authority is the kernel-authenticated Unix peer identity.
No request field supplies a user, group, process, token, display name, socket
path, executable, or capability secret. The Current single-principal daemon
admits only its configured user before decoding a request.

## Protocol

The closed runtime-control protocol adds register, heartbeat, unregister, and
list operations. Register carries one 32-byte non-secret broker-instance
identity and the closed backend value `none`. A successful registration returns
a coordinator-issued monotonic nonzero session identity plus the authenticated
process/user/group, instance identity, backend, and monotonic lease deadline.

Heartbeat and unregister name only the session identity. They succeed only for
the same authenticated process and user. Exact modifying request replay remains
idempotent through the bounded coordinator replay cache. Lists return only live
records in session-identity order.

The registry is bounded to 64 live sessions. A registration lease is ten
seconds. Register, heartbeat, unregister, and list lazily remove expired
records using an explicit monotonic `Clock`; wall time grants no authority.
Broker-instance duplicates are rejected while live, and session identities are
not reused. Daemon restart intentionally loses the registry and requires
registration again.

## Broker Process

`lkjscript-session --foreground --endpoint PATH --backend none` registers once,
heartbeats every two seconds, and exits after three consecutive bounded control
failures. `--heartbeat-limit N` is a bounded operational mode that unregisters
after exactly N successful heartbeats; it exists for finite supervisors and
integration evidence, not as a semantic fallback.

The process does not read desktop environment variables or discover a display.
Its instance identity is a SHA-256 digest of its process identity, endpoint,
and platform revision; it is identity, not a secret or authority token. A PID
reused before lease expiry must wait for reaping rather than replacing a live
instance.

## Service Composition

The generated Linux system service runs the coordinator as the configured user
with a systemd-owned state directory. The graphical-session service invokes the
broker with explicit `/var/lib/lkjscript/control.sock` and backend `none`.
Generated Windows and macOS definitions remain contracts only until native
execution evidence exists.

## Required Next Boundary

Interactive application launch requires a separate accepted cell class and a
broker-to-cell bootstrap bound to coordinator, session, application, and
incarnation identities. Display, event-loop, graphics, and accessibility
providers must be typed and private. Registration presence is never accepted as
proof of those capabilities.

## Rejected Alternatives

- Environment-variable display discovery is rejected as ambient authority.
- Durable session records are rejected because login processes are ephemeral.
- User- or process-supplied authority fields are rejected.
- Arbitrary broker-selected executable launch is rejected.
- Calling generated service text native execution evidence is rejected.
