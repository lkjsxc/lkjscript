# Resident runtime, HTTP, and streams

This specification owns generic resident admission, structured request/worker task scopes, HTTP/1
transport adaptation, byte-stream lifetime, overload, and shutdown. Exact route meaning is owned
by [http-route-topology.md](http-route-topology.md); this adapter owns only bounded preparation and
lookup. It does not own middleware order, actor policy, response content, data-space/index policy,
or job meaning.

## Resident tasks

A prepared deployment has one exact program/target/grant set. Limits require 1 through 4,096 active
tasks, 0 through 65,536 queued tasks, and positive request, shutdown, and cancellation bounds no
greater than one hour. Admission capacity is active plus queued. A nonblocking acquire rejects excess
as `resident_overloaded`; it never forms another hidden queue.

Each admitted call receives a nonreused process-local u64 task identity, fresh capability counters,
an operational deadline, parent deployment cancellation, one worker permit, and one active guard.
Scheduler order and worker count are not language values. Independent calls may overlap; external
authority ordering remains the adapter/data/object/queue contract. Pure results must agree with
serial execution.

Runtime observations include current queued/active and admitted, completed, failed, cancelled,
overloaded, post-shutdown rejection, and maximum queue/active counts. They are disposable
operational evidence, not application state.

Shutdown atomically stops admission, closes admission/worker semaphores, wakes queued tasks, and
waits the declared drain grace. If work remains, it requests cooperative cancellation and waits the
cancellation grace. It then calls every owned adapter's idempotent shutdown exactly once, retains
that cleanup outcome for repeated shutdown calls, and reports admission stop, whether drain
preceded cancellation, cancellation count, remaining tasks, cleanup failures, and elapsed time. A
stalled blocking task is infrastructure failure; possibly visible work is not replayed. Process
restart reloads artifact and durable adapters and discards tasks/queues/caches.

## Byte streams

A stream is an opaque task-scoped resource ID owned by one `StreamRegistry`. Global limits are at
most 1 MiB per chunk, 1,024 buffered chunks, 65,536 live streams, and a positive total byte limit;
deployments select smaller limits. Pipe producers block asynchronously when the exact chunk queue
is full, so backpressure reaches transport producers. Reads block cooperatively and check
cancellation/deadline. EOF, producer failure, consumer close, cancellation, size excess, and
shutdown wake the other side and close owned state.

`read` returns one bounded chunk plus completion, `read-all` is a convenience with its own maximum,
and `close` is idempotent. A lease closes its source on scope drop. Streams cannot be compared,
serialized, returned as durable data, or used by another registry/process. Memory streams and
faultable bounded pipes are the test oracles.

## HTTP adapter

The HTTP adapter accepts a validated method, path, raw query, deterministically decoded map
of query names to ordered values, validated bounded headers, and a `Stream Bytes` body. After
transport validation it selects the unique most-specific graph-owned exact or whole-segment
pattern route and invokes that route's component port once. Pattern captures preserve the validated
raw segment spelling and become the handler's ordered `Text` arguments after the request; query
values never select a route. The component returns signed status, validated headers, and bounded whole bytes.
Current global maxima are 64 MiB body, 256 KiB headers, and 1,024 headers; deployment chooses
request/response limits.

Axum/Hyper own socket acceptance, HTTP/1 parsing, connection lifecycle, transport backpressure, and
disconnect. The adapter owns percent/query decoding, header validation/canonicalization,
transport-owned header rejection, exact-index plus literal-before-capture trie lookup,
request-body streaming into a bounded pipe, response validation, and safe protocol error mapping.
The prepared matcher bounds ordinary work by visited nodes rather than scanning the route set. Graph route owners select
handlers; application functions own authentication, authorization, request decoding, status
selection, and content.

Malformed method/URI/query/header, excess, disconnect, closed body, component failure, overload,
deadline, and shutdown have distinct stable adapter/runtime outcomes. A valid unmatched pair
returns an empty header-free 404 without task admission or application effects; the body remains
transport-owned and is drained or closed within bounds. Other request rejection occurs before
component admission where transport facts suffice. Response emission cannot roll back prior
data/object/queue publication. Adapter-generated failure responses contain a bounded
`x-lkjscript-failure-class` and, when representable in at most 128 bytes, an
`x-lkjscript-failure-code`; they never contain the provider diagnostic message.

The in-memory dispatcher constructs the same `HttpRequest`, registers the same body stream, and
invokes the same prepared port used by the live listener. Tests may replace grants but not handlers.
Current inbound responses are whole bounded byte values; response streaming, trailers, HTTP/2/3,
multipart, and compression are not part of this request/response boundary. Structured
`interactive` targets use the separate relational session contract and RFC 6455 adapter specified
in [structured-sessions.md](structured-sessions.md); they do not turn an HTTP response into a graph
stream. The listener is plaintext. Inbound TLS termination, certificate management, and ACME
remain outside the current product scope; encrypted inbound deployments require an external
trusted transport boundary.

## Outbound HTTP client

The outbound client is a separate deployment adapter and standard interface, not a mode of the
listener. It performs one whole-response HTTP/1.1 GET to an immutable deployment endpoint. Graph
meaning supplies only bounded ordered request headers and receives status, ordered headers, and
bounded whole body bytes. Request-time DNS/address-set validation, outbound TLS trust, transport-
owned headers, response parsing, redirects, retries, deadlines, cancellation, and cleanup are
specified in [outbound-http-client.md](outbound-http-client.md). An inbound disconnect cancels its
owning task and therefore any active outbound request; no partial response is returned as success.
