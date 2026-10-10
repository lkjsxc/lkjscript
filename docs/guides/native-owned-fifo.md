# A native owned FIFO

The current standard package defines `fifo-empty`, `fifo-length`, `fifo-push`,
`fifo-pop`, `fifo-front` and `fifo-discard` as ordinary pure generic graph
functions. They add no FIFO-specific host intrinsic, opcode, capability or
scheduler. Their accepted graph is the meaning authority; the literal
[operation request](../../packages/standard/requests/20261011-owned-fifo.lkjc)
and [fixed-test request](../../packages/standard/requests/20261011-owned-fifo-tests.lkjc)
retain the reviewed native authoring inputs.

These are development-source additions, not a claim about an older frozen
executable. [Status](../status.md) owns source acceptance and publication.

## Exact shape and operations

For an eligible `T: Owned`, the queue type is the structural owned product:

```lisp
(owned-product
  (field incoming (owned-sequence T))
  (field outgoing (owned-sequence T)))
```

`Queue<T>` below is explanatory notation, not a new language type constructor.
A concrete native consumer can give that exact shape a local `type-alias`.
The representation is public, not an opaque nominal type. Any two correctly
typed sequences have a defined logical order: the reverse of `outgoing`,
followed by `incoming`. No cached count or hidden representation invariant can
be forged through ordinary construction.

```text
fifo-empty<T: Owned>() -> Queue<T>
fifo-length<T: Owned>(queue: borrow Queue<T>) -> I64
fifo-push<T: Owned>(value: consume T, queue: consume Queue<T>) -> Queue<T>
fifo-pop<T: Owned>(queue: consume Queue<T>) ->
  owned-choice {empty: Queue<T>, item: owned-product {rest: Queue<T>, value: T}}
fifo-front<T: Owned>(queue: borrow Queue<T>) -> T read-from queue
fifo-discard<T: Owned>(queue: consume Queue<T>) -> Unit
```

Push appends at the logical back. Pop removes the logical front. Both pop
outcomes return the remaining queue, including its empty sequence owners;
a caller can reuse them without replacing the queue with an unrelated empty
value. Discard joins ordinary recursive cleanup, including populated nested
queues. It is not secure erasure.

`T` is Owned even for an empty queue. ByteBuffer, OwnedI64Cell and eligible
owned products, choices or sequences can be elements. Plain I64 is not an
eligible element in this first contract. A scalar-cell wrapper has its own
allocation and admission costs; it is not a free replacement for an ordinary
integer collection.

## Borrowing the first element

`fifo-front` does not remove, freeze or copy the payload. Its exact borrowed
result is tied to the queue input. Use a lexical `borrow-call`:

```lisp
(borrow-call
  (call std::fifo-front (types ByteBuffer) (local queue))
  (binding view (type ByteBuffer))
  (in (call std::buffer-length (local view))))
```

The scope protects the queue and all ancestor owners. The view cannot become
an owning result or be frozen, stored or consumed. The queue cannot be pushed,
popped or discarded while that view is live, even when the view is unused.
After the scope ends, the owning queue is available again. Matching pure
`borrow-from` functions can forward the view without losing source custody.

An empty front operation traps through existing owned-sequence index
validation. Guard with `fifo-length` when absence is normal. The function does
not invent a nullable borrowed result or silently convert operational errors
into absence. The [borrowed-result contract](../spec/owned-read-results.md)
defines the lifetime and cleanup boundary.

## Transfer cost and limits

Enqueue pushes onto `incoming`. Dequeue first uses `outgoing`; only when that
stack is empty does the private tail-recursive helper reverse `incoming` into
it. An enqueued element crosses from incoming to outgoing at most once before
removal. This gives an amortized constant number of sequence transfers per
queue operation; it does **not** promise constant end-to-end VM work, zero
metadata allocation or bounded dequeue latency.

One refill can move the complete pending input. Length and front do not
perform that refill. Both sequence allocations can retain spare capacity, so
retained storage can exceed the current logical length. Normal instruction,
allocation, call-depth and cancellation limits still apply. A failed consuming
operation returns no partial queue; invocation cleanup owns transferred inputs.
This pure local structure does not supply transactions, durable delivery or
an asynchronous work scheduler.

Replacing the public structural representation would be an explicit API
change. An opaque generic owned type may eventually provide a stronger
abstraction boundary; adding a queue-specific runtime carrier merely to hide
two existing sequences is not required for this increment.

## Independent consumer

The [native consumer](../../examples/owned-fifo/README.md) first exports a
carrier-independent operation trace and drain. A separate package then supplies
scalar cells, two-byte buffers and stamped owned packets. It compares every
observable operation and the final drainage against an independent host
`VecDeque` model, then repeats through a detached command artifact after
removing the authored projects and package transports. Fixed graph tests cover
empty reuse, refill order, repeated front reads, mixed stacks and nested custody.

This is standard-library adoption and a maintained consumer, not a claim that
the compiler now uses a native graph planner. The current planner's bounded
input and the compiler's much larger admission envelope remain different
contracts. A production planner replacement needs its own scale, correctness
and resource evidence rather than inheriting acceptance from this FIFO.
