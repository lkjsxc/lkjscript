# Structured interactive sessions

This specification defines structured session contract 1. It is the only supported meaning and
runtime model for an `interactive` target. The initial transport is a plaintext RFC 6455 server
over the resident HTTP/1.1 listener.

## Authority and port relation

Accepted graph meaning owns the handler, the single concrete state type, phase policy expressed by
the handler, authentication and path policy, application message grammar, data selection and
ordering, output values, and requested close outcomes. The resident runtime owns transport
validation, framing, scheduling, accounting, cancellation, and observation. A deployment
descriptor owns listener coordinates and finite operational bounds. Operational data and queues do
not select or advance semantic authority.

An interactive target has exactly one port whose function type is structurally equal to:

```text
(Option<State>, SessionEvent) -> SessionDecision<State>
```

All occurrences of `State` bind one exact concrete type object. The type must be closed and
ordinary: unit, boolean, integer, bytes, text, finite records or variants, lists, maps, and options
may be composed recursively. Static text, secrets, results, streams, capability resources,
functions, unresolved parameters, and types containing any of them reject. Cycles, missing type
objects, and foreign standard declarations reject. Authoring validation, accepted full validation,
compiler lowering, strict artifact loading, and deployment preparation each reconstruct this
relation rather than trusting a stored assertion.

The standard package owns `SessionEvent`, its payload records, `SessionMessageKind`,
`SessionDecisionKind`, `SessionOutbound`, `SessionReject`, and `SessionClose`. The parameterized
decision is the canonical structural record:

```text
{
  kind: SessionDecisionKind,
  state: Option<State>,
  messages: List<SessionOutbound>,
  rejection: Option<SessionReject>,
  closing: Option<SessionClose>
}
```

`SessionEvent` has exactly `open`, `message`, `tick`, `peer-close`, and `shutdown` cases. Open
carries normalized path, query, and ordered headers. Message carries text or binary kind and one
task-scoped `Stream<Bytes>` for a complete application message. Peer-close carries an optional
validated code and bounded UTF-8 reason. Tick and shutdown have no payload.

The decision kind has exactly `accept`, `continue`, `reject`, `close`, and `finish`. Outbound values
have exactly text and binary cases. Reject carries an ordinary bounded HTTP status, ordered
headers, and body. Close carries a valid RFC 6455 code and bounded UTF-8 reason.

## Phases and atomic transition

The runtime enforces these pairs mechanically:

| Event | Input state | Allowed decisions | Installed state |
|---|---|---|---|
| `open` | none | `accept`, `reject` | accept installs some; reject installs none |
| `message`, `tick` | some | `continue`, `close` | continue installs some; close installs none |
| `peer-close`, `shutdown` | some | `finish` | none |

Every inactive field in a decision must be the canonical empty value. Accept and continue require
one state. Reject requires only rejection. Close requires only closing plus its final outbound
batch. Finish carries no state or output. A wrong phase, missing or extra decision field, invalid
status or close value, foreign runtime shape, or retained-state violation produces a stable session
diagnostic and never becomes next state.

At most one graph transition runs for a session. Before the next event observes a result, the
runtime validates the entire next state and output batch, commits the entire batch to capacity that
was reserved before the callback, and only then installs the next state. Callback failure,
cancellation, deadline or work exhaustion, invalid output, or protocol failure installs neither
state nor partial output and terminates the session. Effects that were already possibly visible are
not replayed.

## Structured parent and bounds

One admitted parent session scope exclusively owns the upgraded connection, transport reader,
ordered driver, writer, optional tick source, cancellation lineage, state, stream leases, permits,
mailbox accounting, and child joins. Children cannot detach. Each graph callback is a separate
finite resident task and retains the resident one-hour maximum; session idle and total lifetimes are
separate descriptor bounds and may admit at least 24 hours.

Inbound transport events enter one ordered, item- and byte-bounded mailbox. A completed data
message becomes one task-scoped byte stream and must be consumed or closed during that callback.
Outbound messages enter one ordered item- and byte-bounded mailbox. Writer acknowledgement, or an
equivalent reservation, prevents another potentially effectful callback until the configured
maximum transition batch is available. A coalesced tick is pending at most once and never overtakes
an already accepted transport event. There is no silent drop, hidden queue, or unaccounted
whole-message copy.

Deployment and global policy independently validate positive, checked limits for active sessions,
pending handshakes, header/frame/message bytes, stream chunk/buffer/total bytes, inbound and
outbound mailbox items and bytes, retained state nodes and bytes, outbound messages and bytes per
transition, callback work and deadline, tick interval, idle and total lifetime, close and
cancellation grace, and process-wide session-buffer bytes. Descriptor values cannot exceed global
ceilings. Products and sums used for reservations are checked before listener readiness.

Peer close, read/write or transport failure, callback failure, timeout, overload, service shutdown,
or parent completion stops new callbacks, cancels siblings, joins every child, closes callback
streams, drops state and queued messages, attempts at most one bounded close handshake where valid,
and releases every permit once. Listener overload rejects before upgrade and does not cancel an
admitted session. Readiness follows exact artifact, target relation, descriptor, grant, adapter, and
reservation validation.

## RFC 6455 boundary

The server validates the HTTP/1.1 upgrade and accept key, client masking, reserved bits and opcodes,
fragmentation, UTF-8 text, control-frame size and finality, ping/pong, close codes and reasons, and
configured sizes. It negotiates no extension or subprotocol. Malformed continuation, unmasked
client data, invalid text or control frames, oversize or truncated input, stalled peers, abrupt
disconnect, and overload remain connection-local and cannot change semantic `HEAD`.

Graph `reject` returns bounded HTTP without upgrade. Graph `accept` completes the upgrade before
its initial outbound batch is written. The runtime may fragment an outbound message without
changing its application-message identity.

The transport provides no TLS, confidentiality, origin, proxy, browser, authentication, hostile
code, or multi-tenant isolation claim. Graph policy may inspect the normalized path, query, and
headers. Diagnostics, readiness, and retained evidence exclude secrets and complete payloads.

## Contract evolution

Stored target, compiler, artifact, deployment, resident, and public-control contracts advance when
their bytes or behavior encode this runner. Predecessor forms reject at their owning boundary; no
`websocket` alias or second session type family is accepted. General resource-bearing results and
detached tasks remain prohibited. Reconsider live resource results only if a maintained workload
must transfer a capability across ordinary graph calls and this structured runner is proved
insufficient.
