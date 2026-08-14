# Local daemon protocol specification

## Endpoint and framing

`lkjscriptd` requires an explicit absolute state path and listens on
`STATE_DIRECTORY/lkjscript.sock`, never TCP. The state directory is mode 0700 and the socket is mode
0600; this filesystem boundary is the bootstrap local access policy.
The daemon holds an exclusive lock for the state directory and removes a stale socket only after it
owns that lock.

Each connection carries one request and one response, then closes. A frame is:

```text
u32 little-endian body length | u16 protocol version | u64 request ID | typed message
```

The request ID is copied into the response. Version 1 bounds a frame to 8 MiB and each encoded
collection to 100,000 items as IPC policy.
Truncated headers or bodies, invalid booleans or UTF-8, unknown message/operation/type tags, invalid
lengths, and forbidden trailing bytes reject. The persistent artifact format is decoded by a
separate boundary and is never used as an IPC frame.

## Implemented requests

- `CreateWorkspace`
- `ApplyTransaction`
- revision-bound `WorkspaceSummary`
- revision-bound `Node` summary with optional exact-record expansion
- revision-bound package-wide `Blockers`
- revision-bound `Run(entry NodeId)`
- `Shutdown`

Responses are corresponding typed variants, `Acknowledged`, or `Error`. Errors carry a stable code
and typed optional workspace, revision, transaction operation index, target Node ID, expected and
actual node kind/type, related Node IDs, and retryability. The message is presentation only.

`ApplyTransaction` carries the exact typed operation batch; arbitrary JSON fields and `add_edge`
requests do not exist. The daemon retains one typed idempotency outcome per workspace in the atomic
HEAD metadata. An exact retry returns that prior outcome without reapplying mutation even though its retained,
fingerprint-bound original base is no longer the head. Unseen requests still require the current
head; conflicting key reuse rejects. A newer keyed mutation replaces the retained record.

## Connection behavior

A clean connection close before a frame is ignored. Per-connection reads and writes have a five
second operational deadline, so a partial client cannot block the synchronous daemon indefinitely.
Malformed input receives a structured error with correlation ID zero when the request ID could not
be decoded, then the connection closes. An oversized frame is a policy error and cannot reach
workspace mutation. A dropped response connection does not stop the daemon. `Shutdown` exits only
after its acknowledgment is written.

The `lkjscript` binary is a protocol client. It owns no mutable graph, artifact writer, compiler, or
interpreter implementation.
