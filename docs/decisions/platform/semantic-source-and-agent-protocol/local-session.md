# Semantic Source: Local Session

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Define the one accepted local revisioned session command, framing, pinning,
bounds, invalidation, and shutdown behavior.

## Status

**Accepted Target, not Current.** The Current command remains the bounded
one-shot `lkjscript.semantic-source/1` interface.

## Command And Framing

The only session transport is exactly:

```text
lkjscript semantic serve --stdio
```

Each request frame is an 8-byte big-endian unsigned payload length followed by
exactly that many bytes containing one strict UTF-8 JSON envelope. Every
response uses the same framing. The envelope is closed and contains exactly
`schema`, `version`, `request_id`, `revision`, and one typed `request` or
`response` field as applicable. Duplicate/unknown/missing fields, unknown
variants/versions, malformed UTF-8/JSON, and trailing bytes fail.

The unsigned length is checked with checked conversion against the pinned
profile's frame and aggregate byte ceilings before payload allocation or read.
Zero bytes before a new header is clean EOF; a partial header or payload is a
protocol error. stdout contains framed protocol bytes only. Guest output,
logs, diagnostics text, and private model reasoning never mix with stdout.
There is no network listener or network fallback.

## Pinned Session

Startup pins compiler build, source and diagnostic schemas, resource profile,
and canonical repository root. The revision is monotonic and checked for
overflow. Each accepted transaction produces a new immutable snapshot;
queries pin one revision. Derived trees, facts, indexes, and caches are immutable
per snapshot and keyed by all semantic inputs.

A stale request or externally changed file/root/dependency rejects with a typed
reason. It never rebases silently. Typed cache keys record exact source units,
declarations, compiler build, schema, profile, and query dependencies. A
successful explicit refresh creates a new revision and invalidates every
transitive dependent while retaining only bounded immutable revisions.
Publication uses the same atomic journaled source transaction authority.

## Bounds And Shutdown

A session-owned bounded grant covers lifetime and retained state; each request
receives one request-owned hierarchical child ledger. Together they bound fuel,
input/output bytes, decoded nodes, compiler work, snapshots, cached
bytes/entries, retained revisions, transactions, and staged publication. Child
work receives lower-only grants. Exhaustion returns a framed structured error
when possible,
publishes nothing partial, and may close the session if safe framing cannot be
guaranteed.

Clean EOF after a complete response and an explicit `shutdown` request both
finish outstanding atomic publication or rollback, release caches/locks, emit
no unframed bytes, and exit success. Signals and malformed framing perform
bounded rollback and non-success exit; no private state is serialized.
