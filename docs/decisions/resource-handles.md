# Resource Handles And Terminal ABI

## Purpose

Define a memory-safe, stale-safe language boundary for descriptors and terminal
state before multi-process or untrusted execution work begins.

## Status

**Current.** Bounded terminal operations and monotonic stale-safe handles are
implemented. Generation-based reusable slots and per-process terminal leases
remain deferred.

## Handle Decision

- A language `Handle` is never accepted from an integer value.
- Handle payloads are opaque tokens, not raw operating-system descriptors.
- Borrowed standard streams occupy reserved tokens disjoint from owned files
  and sockets.
- Owned resource tokens are allocated monotonically in this cycle. Closing a
  resource invalidates its token permanently; a later open receives a new token.
- Closing a borrowed, unknown, already closed, wrong-kind, or stale token is an
  explicit error.
- Every operation resolves a token through one resource table before reaching
  an OS descriptor. No operation reinterprets the payload independently.

Monotonic tokens intentionally trade bounded metadata growth for the simplest
proof that stale handles cannot alias later resources. Reusable slot plus
generation encoding is deferred until long-running workload measurements show
that token metadata matters.

## Terminal Decision

Remove script-controlled `sys-ioctl Handle I64 Buf`. Replace it with exactly:

```text
sys-tty-get Handle Buf -> Result Nil Str
sys-tty-set Handle Buf -> Result Nil Str
```

The Linux backend selects `TCGETS` and `TCSETS` internally. Both safe Rust
wrappers require the exact Linux termios buffer size before entering FFI. A
request number never crosses the language boundary. The language standard
library continues to own raw-mode flag manipulation over the checked buffer.

The process-exit terminal guard accepts only an exact-size saved buffer. It is
still process-global and therefore cannot serve the future supervisor; moving
it behind a per-process terminal lease is deferred to the process-safe VM
cycle.

## Related Result Decisions

`sys-poll` becomes `Handle I64 -> Result I64 Str`. Terminal and poll wrappers
unwrap or propagate these values explicitly. Ordinary invalid/stale handle and
OS failures are data at `sys-*` language boundaries, not VM termination.

Non-`sys-*` descriptor helpers remain direct VM-error surfaces and are migrated
to explicit Results in the following conformance slice.

## Verification

Focused tests must prove:

- integers cannot masquerade as handles;
- stdin cannot collide with an owned slot;
- close then open does not revive the stale token;
- repeated close and close of a borrowed token fail;
- file/socket kind mismatches fail;
- 59-byte and 61-byte terminal buffers fail before FFI, while the exact size
  reaches only the fixed request wrapper;
- `sys-ioctl` no longer typechecks;
- lkjedit and one-shot HTTP acceptance remain green.

## Rejected

- Keeping legacy integer descriptors.
- Allowing arbitrary ioctl with a user-provided byte slice.
- Reusing slots without generations.
- Encoding raw OS descriptors directly in language values.
- Claiming process isolation while the terminal guard remains global.
