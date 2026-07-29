# Authenticated Session Broker Evidence

## Status

**Current on Linux x86-64** for authenticated ephemeral broker presence. This
record does not claim an interactive application cell, display connection,
native window, graphics, accessibility tree, or GUI application.

## Authority Boundary

`lkjscript-session` is a real foreground per-login process. It connects only to
an explicit coordinator endpoint and names the closed backend `none`. It does
not read `DISPLAY`, `WAYLAND_DISPLAY`, desktop configuration, a home directory,
or an application-selected endpoint.

Linux Unix control authenticates every connection with `SO_PEERCRED` before
request decoding. The configured user is checked by the server. Registration,
heartbeat, and unregister bind the session to the kernel-supplied process, user,
and group; requests carry none of those authority fields.

## Exact Protocol

Runtime control at platform revision 9 and digest
`5ca07048677f164ef9d25b8fc65a455b670e7e385b94d850692204fafe1a3305`
contains register, heartbeat, unregister, and list operations. Register carries
a nonzero 32-byte non-secret broker-instance digest and backend `none`.
Responses carry a monotonic nonzero session identity, authenticated peer,
backend, and monotonic lease deadline.

The in-memory registry admits at most 64 sessions. A ten-second lease is renewed
by heartbeat. Every session operation lazily reaps expiration through an
explicit host `Clock`. Session identities are never reused during a daemon
process. A duplicate live broker instance, wrong peer process, zero identity,
unknown session, malformed backend, and bound overflow fail closed.

The registry is intentionally absent from durable control. Daemon restart clears
all sessions and requires re-registration. Coordinator shutdown clears sessions
before application database checkpoint and process-cell shutdown.

## Broker Lifecycle

The broker heartbeats every two seconds and exits after three consecutive
control failures. A finite nonzero heartbeat limit unregisters after exactly
that many successful heartbeats. Exact modifying replays return the cached
result and conflicting replays fail.

`lkjscript system session list --endpoint PATH` prints only live records in
session-identity order. It cannot register or impersonate a broker process.

Generated Linux service composition runs `lkjscriptd` as the configured user,
uses `StateDirectory=lkjscript`, and starts the graphical-session broker with
explicit `/var/lib/lkjscript/control.sock` and backend `none`. Generated Windows
and macOS definitions remain unexecuted contracts, not platform evidence.

## Executed Evidence

Executed on Linux x86-64 at the implementing worktree:

```text
cargo test --locked -p lkjscript-runtime
cargo test --locked -p lkjscript-app --test cli_contract session_broker
```

The runtime suite round-tripped all session request and response forms and
proved monotonic identity, duplicate rejection, peer ownership, heartbeat lease
renewal, exact expiration, lazy reaping, unregister, clear, and the aggregate
bound. Existing Unix control evidence also proved kernel peer authentication
and exact replay behavior.

The integration started the real coordinator and broker under the test kernel
user after clearing the broker environment. It observed registration as session
1, listed the broker's exact kernel process with backend `none`, completed one
heartbeat, observed clean unregister and zero list output, then stopped the
daemon cleanly.

## Non-Claims

- No broker-to-application bootstrap or interactive cell class exists.
- No display, window, event-loop, graphics, GPU, input, or accessibility
  provider exists.
- No GUI application executed.
- No native Windows or macOS broker executed.
- Generated service files were not privileged-installed or supervisor-started.
