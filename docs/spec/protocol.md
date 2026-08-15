# Local daemon and machine protocol specification

## Endpoint and binary framing

`lkjscriptd` requires an explicit absolute state path and listens only on
`STATE_DIRECTORY/lkjscript.sock`. State-directory mode 0700, socket mode 0600, an exclusive daemon
lock, and OS filesystem ownership form the bootstrap local access boundary. There is no HTTP, TCP,
or public JSON listener.

Each connection carries exactly one request and one response and then closes. Protocol version 3
directly replaces prior versions; old versions reject and no legacy reader remains. A binary frame is:

```text
u32 little-endian body length | u16 version=3 | nonzero u64 request ID | closed typed message
```

The response repeats the request ID. Frames are limited to 8 MiB and encoded collections to 100,000
items as transport policy. Checked decoders reject truncated or oversized frames, zero request IDs,
invalid lengths/counts/booleans/UTF-8/IDs/indexes, unknown stable tags, payload trailing data, and
any bytes after the one request or response frame. Artifact and HEAD bytes use separate decoders.

## Request and response families

The complete public request families are:

- `CreateWorkspace`;
- `ApplyTransaction`, carrying the typed transaction, commit/validate-only mode, optional commit
  idempotency key, and bounded response projection;
- `QueryBatch`, binding independent read items to one workspace and exact retained revision;
- `Run`, naming workspace, exact revision, entry function Node ID, ordered invocation arguments, and
  a closed positive bounded policy containing `fuel` and `maximum_frames`;
- `DescribeSchema`;
- `Shutdown`.

Responses are `WorkspaceCreated`, compact `TransactionReceipt`, `QueryBatchResult`, typed `Run`,
`SchemaDescription`, `Acknowledged`, or structured `Error`. Errors include a stable code and typed
optional workspace, revision, public transaction operation index, offending explicit local handle,
target, expected/actual kind or type, at most 64 deterministically ordered related IDs, and
retryability; prose is presentation only. Exact larger blocker detail remains available through the
paginated blockers query. Run argument mismatch, execution-fuel exhaustion, and execution-frame exhaustion have distinct stable
error codes; arithmetic overflow remains a runtime trap.

Initial public construction uses structured `CreateFunction`, optional structured bodies,
`DefineFunctionBody`, and base-block-only `InsertExpression`. Parameter, expression, loop-index,
and loop-carried handles are explicit; implied regions, blocks, block arguments, and terminators
are expanded once into canonical nodes and the draft is discarded. The displaced low-level
parameter/region/block/operation creation messages are not public variants. Expansion scans and
allocates every explicit and implicit identity before applying canonical edits, so forward and
mutual function references resolve deterministically. Structured requests allow at most 16 nested
structured bodies and 65,536 exactly counted draft items; the conservative depth remains below the
strict JSON parser recursion boundary and binary decoding enforces the same value. The item total is
the checked aggregate of top-level transaction operations, function parameters, function bodies,
yielding bodies, expressions, and call arguments. Deeper semantic graphs remain constructible by inserting into blocks retained by prior transactions.

A transaction receipt never contains the full semantic diff or every allocation by default. It
contains bounded identity/publication/completeness facts, exact change count/digest, total explicit
and implicit created-node count, and only requested explicit-handle bindings. The response
projection is part of the idempotency fingerprint. Validate-only returns an identical predicted
allocation/hash with `published=false` and does not publish.

A query batch contains at most 32 client-labelled items and an aggregate requested-item budget of
2048. Every item observes the same immutable revision and results remain in request order with the
same query IDs. A bad workspace, revision, duplicate query ID, batch shape, page limit, or aggregate
budget rejects the batch. Once the batch boundary is valid, a target-specific error affects only
that item and other items can succeed.

The closed query families are workspace summary, compact or exact node view, completeness blockers,
owner chain, block body slice, incoming value uses, incoming definition references, outgoing
dependencies, visible values, legal constructors, semantic diff, and repair context. Pages contain
typed items, `next` when more exists, and total count when provided. Page size is 1..=256. Cursors
bind workspace, revision, query family/purpose, target and options as applicable, and deterministic
next position; cross-revision, cross-target, cross-purpose, malformed, or out-of-range cursors
reject. A diff query binds exact `from` and batch `to` revisions and repeats exact total
change-count/digest facts on every page. Repair context composes bounded structural facts using up
to 64 items per category. Full scans and recomputation are the correctness oracle.

`DescribeSchema` is derived from executable enums and descriptors and exposes stable names/tags,
operation contracts, structured expression/value variants, depth-first allocation and implicit-node
facts, transaction/query/error/request/response vocabularies, unit/newtype/record payload shapes,
exact required and optional field names with stable type expressions, request/response/boundary
envelopes, required Run and RunPolicy fields, runtime value tags and payload forms, and active
boundary limits. Runtime limits are 1,024 ordered arguments, 10,000,000 fuel units, 100,000 frames,
and 65,536 aggregate live frame-value slots. This specification does not duplicate its exhaustive
payload catalogue.

## Strict generic JSON CLI

`lkjscript --state DIRECTORY rpc [--pretty]` reads exactly one strict JSON version-3 request envelope
from stdin, projects it to the same typed binary request, invokes the private daemon, and writes
exactly one JSON response envelope to stdout. `lkjscript schema [--pretty]` emits the same runtime
description locally; `DescribeSchema` provides it through the daemon. JSON is transport only and
is never persisted as program authority.

The envelope fields are `version`, nonzero `request_id`, and typed `request`. Tagged variants use
stable lowercase snake-case names. Workspace and idempotency IDs are exactly 32 lowercase hex
characters; Node IDs are `workspace:nonzero-canonical-decimal-serial`; hashes/digests are
fixed-width lowercase hex; revisions, query IDs, handles, indexes, and counts are JSON integers in
their checked domains. There is one canonical representation.

Unknown fields and variants, duplicate fields, wrong case, malformed or uppercase hex, zero node
serials, invalid numeric domains, invalid UTF-8, excessive nesting, trailing JSON values, and input
over 8 MiB reject locally. JSON output is streamed through a 32 MiB limit rather than first
allocating an unbounded value. Boundary error messages are bounded. Semantic and policy validation
that belongs to the daemon is returned as typed `Response::Error`, not reclassified as JSON syntax
failure.

Machine stdout contains exactly one compact JSON value plus newline (pretty output is explicit).
Diagnostics do not contaminate stdout and belong on stderr. Exit status is:

- `0`: a syntactically valid daemon response, including typed semantic rejection;
- `2`: CLI usage, stdin, or JSON boundary error;
- `3`: daemon transport failure;
- `4`: response conversion, output-limit, serialization, or write failure.

## Connection behavior

A clean close before a frame is ignored. After writing its request, the production client shuts down
the connection write half. The daemon proves request-side EOF before dispatch, so a second frame or
any connection-level trailing byte cannot reach mutation. The client likewise proves response-side
EOF before accepting the reply. The server enforces one absolute five-second connection deadline;
the client uses a 30-second I/O timeout. Every well-formed request
echoes its nonzero request ID. A malformed binary request receives a structured error with
correlation ID zero. A dropped response does not stop the daemon. `Shutdown` exits only after
acknowledgement is written. The client owns no mutable graph, artifact writer, compiler, or
interpreter.
