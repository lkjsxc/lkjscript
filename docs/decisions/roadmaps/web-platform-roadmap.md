# lkjscript Web Platform Roadmap

## Purpose

Keep the eventual first-party Web framework on top of general language/runtime
foundations rather than embedding framework policy in Rust host operations.

## Status

One-shot synchronous HTTP and bounded socket/file/buffer capabilities are
**Current** examples. Async sockets, a streaming HTTP implementation, routing,
middleware, WebSocket, HTTP/2, TLS integration, and HTTP/3 are **Accepted
Targets** and are not Current.

## Decision

The accepted implementation sequence is:

```text
Bytes, Vec, and lifetime-checked Slice
  -> ownership and borrowing
  -> structured tasks, cancellation, and deadlines
  -> epoll reactor and timers
  -> streaming sockets with backpressure
  -> HTTP/1.1 parser and server in lkjscript
  -> typed routing and middleware
  -> streaming request and response bodies
  -> WebSocket
  -> HTTP/2
  -> audited TLS provider
  -> HTTP/3 only after measured need
```

Protocol parsing, routing, middleware, application composition, and framework
policy belong in lkjscript. Native code exposes thin safe capabilities for
sockets, polling, timers, bulk bytes, and audited provider boundaries.

`buf-slice` remains a bounded copying operation. It is not the future borrowed
`Slice T`. `Bytes` is an immutable ownership-aware byte type; `Vec T` is a
unique growable container; `Slice T` is a checked lexical borrow. Their exact
syntax and layout require separate contracts before implementation.

## TLS

Initial deployments may terminate TLS in a reverse proxy. The transport API
must permit a later audited native cryptographic provider without changing
framework semantics. Production cryptographic primitives are not implemented
in lkjscript. Provider ownership, buffer lifetime, cancellation, and versioned
ABI facts are explicit.

## Acceptance Boundaries

A future HTTP server is accepted only with bounded parsing, streaming bodies,
backpressure, cancellation/deadline propagation, malformed-input tests, and
retained application-level measurements. A Rust Web framework wrapper is not a
first-party lkjscript framework.

## Deferred

All framework implementation, async runtime work, TLS providers, HTTP/2,
HTTP/3, QUIC, and production deployment claims are **Deferred** in the current
JIT cycle.

## Rejected

A fat Rust Web framework hidden behind host calls, unbounded body buffering,
calling copying `buf-slice` a zero-copy borrow, implementing unaudited
cryptography in lkjscript, and adopting HTTP/3 without measured need are
**Rejected**.
