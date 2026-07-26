# Semantic Source: Local Session

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Define the one accepted local revisioned session command, framing, pinning,
bounds, invalidation, and shutdown behavior.

## Status

**Current.** `lkjscript semantic serve --stdio` implements this bounded local
session envelope version 1 over the Current one-shot Semantic Source Schema V2
engine. Semantic Source V1 requests are rejected.

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
reason. It never rebases silently. V1 retains one exact immutable fingerprint
set and reports zero query-cache entries; it does not claim incremental caching.
A successful explicit refresh creates a new revision and replaces that bounded
snapshot metadata. Future nonzero cache keys must include exact source units,
declarations, compiler build, schema, profile, and dependencies. Publication
uses the same atomic journaled source transaction authority.

## Bounds And Shutdown

Current Session Limits V1 intersect selected Resource Profile V2 protocol and
`semantic_session_*` ceilings with stricter frame, cumulative-byte, request,
fuel, metadata, and revision maxima. The session exposes the selected node,
snapshot, retained-byte, input/output, lifetime-fuel, and zero-cache bounds in
its pinned state. It meters each request and retains one revision; it does not
claim the Accepted cross-authority shared ledger or nonzero cache.
Exhaustion returns a framed structured error when possible,
publishes nothing partial, and may close the session if safe framing cannot be
guaranteed.

Clean EOF after a complete response and an explicit `shutdown` request both
finish outstanding atomic publication or rollback, release caches/locks, emit
no unframed bytes, and exit success. Signals and malformed framing perform
bounded rollback and non-success exit; no private state is serialized.
