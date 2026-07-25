# Isolates And Structured Concurrency

## Purpose

Fix the long-term concurrency and async-runtime direction without implying that
tasks, isolates, or asynchronous sockets are implemented.

## Status

The current runtime is single-owner and process-global host services prevent
concurrent VM supervision. Isolates, tasks, channels, async syntax, `Send` and
`Sync`, and the reactor are **Accepted Targets**, not Current behavior.

## Decision

The concurrency model is:

```text
isolates
+ structured concurrency
+ lightweight tasks
+ typed bounded channels
+ ownership transfer
+ compiler-derived Send and Sync
```

A worker owns a local heap by default. A task may borrow only within a proven
scope. Child tasks are joined or cancelled before their scope ends. Unique
owned values may cross workers by move; immutable shared byte storage may be
shared through an explicit type. Worker-local GC references are not `Send`.
Shared mutation requires an explicit synchronized type.

Ordinary source cannot assert `Send` or `Sync` unsafely. The compiler derives
them from all fields, future enum payloads, closure captures, reference kinds,
GC ownership, synchronization wrappers, and unique containers.

## Accepted Sequence

```text
ownership and derived Send/Sync
  -> Task, Future, Cancellation, and Deadline representations
  -> structured task scopes
  -> single-thread lightweight scheduler
  -> epoll reactor and timers
  -> bounded blocking pool
  -> multi-worker isolates
  -> ownership-transfer bounded channels
  -> work stealing only for Send tasks
```

The source design will record one canonical line-oriented syntax for concepts
equivalent to `async function`, `await`, `scope`, `spawn`, `cancel`, `select`,
`yield`, and `channel` before implementation. Async failure remains exact
`Option`, `Result`, or structured runtime outcome; general exceptions are not
introduced.

## I/O Boundary

Linux `epoll` is the first exact reactor backend. A later `io_uring` candidate
must implement the same async runtime ABI and be adopted only after measurement.
Blocking operations run in a bounded pool and preserve deadlines,
cancellation, resource limits, and backpressure.

## Deferred

All syntax, scheduler, reactor, multi-worker GC, channels, work stealing, and
`io_uring` are **Deferred** in the current JIT cycle.

## Rejected

An unconstrained shared mutable heap, detached unscoped tasks, unbounded
channels, implicit blocking on reactor workers, source-asserted unsafe
`Send`/`Sync`, and making `io_uring` the only API contract are **Rejected**.
