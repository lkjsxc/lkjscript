# Isolates And Structured Concurrency

## Purpose

Fix the long-term concurrency and async-runtime direction without implying that
tasks, isolates, or asynchronous sockets are implemented.

## Status

The current language runtime is single-owner and process-global host services
prevent concurrent VM supervision. The internal
[semantic resource plane](semantic-resource-plane.md) is an Accepted Contract;
source tasks, isolates, channels, async syntax, `Send` and `Sync`, and the
reactor remain **Accepted Targets**, not Current behavior.

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

A worker owns bounded scratch and participates in deterministic memory homes;
it does not gain a worker-local tracing heap. A task may borrow only within a
proven scope. Child tasks are joined or cancelled before their scope ends.
Unique owned values may cross workers only by verified boundary move with no
live loan; immutable shared byte storage requires an explicit sharing contract.
Task-access verification derives portability, dependencies, and result
ownership. Legacy traced references are not portable. Shared mutation requires
an explicit synchronized type.

Ordinary source cannot assert `Send` or `Sync` unsafely. The compiler derives
them from all fields, future enum payloads, closure captures, reference kinds,
GC ownership, synchronization wrappers, and unique containers.

## Accepted Sequence

```text
internal verified task graph and deterministic scheduler
  -> topology-aware bounded workers and deterministic memory homes
  -> ownership and derived Send/Sync for source values
  -> Task, Future, Cancellation, and Deadline representations
  -> source structured task scopes
  -> epoll reactor and timers
  -> bounded blocking pool
  -> ownership-transfer bounded channels
  -> source work stealing only for derived-portable tasks
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

All source syntax, reactor, multi-worker legacy tracing, channels, and
`io_uring` are **Deferred**. The internal semantic resource plane does not
promote them.

## Rejected

An unconstrained shared mutable heap, detached unscoped tasks, unbounded
channels, implicit blocking on reactor workers, source-asserted unsafe
`Send`/`Sync`, and making `io_uring` the only API contract are **Rejected**.
