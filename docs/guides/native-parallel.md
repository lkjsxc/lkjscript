# Joined parallel computation

Development 0.1.70 supports two child tasks with owned inputs, joined ordinary
or owned results, and explicit closed generic applications. Check the installed executable's `capabilities --section change`:
it must advertise `parallel`. Check [status](../status.md) for the separate public executable selection.

For two already declared empty-effect graph tasks, the native form is:

```text
(parallel
  (call workers::buffer-job (i64 32) (local buffer))
  (call workers::cell-job (local count) (local cell)))
```

The enclosing function declares `(effect (task))`. Each child is a direct named,
task application with an empty effect row. Generic tasks need their exact concrete
type and implementation arguments at this boundary. Ordinary arguments come first; owned
arguments are exact live locals passed to `consume` parameters. Results can be
closed ordinary data or owned carriers. Both results above are ordinary I64 values,
producing `(record (left I64) (right I64))` in this example.
All argument expressions run once in the parent, left call first. Child bodies
may overlap; the parent receives the pair only after both finish.

The [literal worker library](../../tests/fixtures/parallel-library.lkjc) supplies
buffer and scalar-cell reductions, a generic Owned reduction contract, exact static
implementations, and concrete task wrappers. These wrappers remain supported, but
closed generic tasks no longer need a wrapper solely to become parallel children. The
[literal consumer](examples/parallel-consumer.lkjc) uses two groups: independent
buffer/cell inputs, followed by an owned product and owned choice. Constructors first
bind each owned input to a typed local, preserving the existing transfer rules.

Use the [native library workflow](native-library.md) to create a command-template
worker package, review/apply the worker request at its observed base, and export its
exact transport. Create a minimal consumer, stage that transport, select its observed
package/revision with `dependency.add` and `use`, then review/apply the consumer input.
Run `check`, inspect/draft the accepted module, and build an ordinary detached artifact.
Execute this task with `run --deployment PATH`; its deployment selects the artifact
and target with empty grants. Plain `run TARGET` is the pure differential runner
and rejects tasks even when their effect rows are empty.
The [maintained public test](../../tests/public_cli/native_parallel.rs) preserves the
literal inputs and complete executable sequence, including a third entry package,
identity-preserving edits and execution after source removal.

For the original example and input `[16, -99, false]`, the independent arithmetic is
`T = 16 * 17 / 2 = 136`. The expected result is:

```json
{"simple":{"left":499,"right":37},"aggregate":{"left":187,"right":141}}
```

Editing the first child's argument from 31 to 32 changes only `simple.left` to
531. The test checks this edit, four input cases and both choice cases using
closed-form arithmetic. Its detached runtime admits only one ordinary resident task;
child execution does not need another resident slot held by its parent.

## Returning owners

When either child returns an owner, the joined result is an owned product with
`left` and `right` fields. For a buffer-returning task and a cell-returning task,
bind and unpack the pair using their exact result types. Here `freeze` and `extract`
are the consumer's closed `core.buffer.freeze` and `core.cell.extract` externals:

```text
(let
  (binding pair (type (owned-product (field left ByteBuffer) (field right OwnedI64Cell)))
    (parallel
      (call result-workers::buffer-job (i64 8) (local buffer))
      (call result-workers::cell-job (local count) (local cell))))
  (in (unpack-owned (type (owned-product (field left ByteBuffer) (field right OwnedI64Cell)))
    (local pair)
    (field left (binding data (type ByteBuffer)))
    (field right (binding scalar (type OwnedI64Cell)))
    (in (record structural
      (field bytes (call freeze (local data)))
      (field value (call extract (local scalar))))))))
```

The child declarations are in the [literal result worker package](../../tests/fixtures/parallel-result-workers.lkjc).
They call exported generic Owned functions with exact implementation witnesses.
The [literal carrier library](../../tests/fixtures/parallel-result-library.lkjc)
uses distinct exported buffer and cell function names within their shared package.
Its `nested-job` returns the owned pair from another parallel group; `choice-job`
returns either an ordinary accepted value or a retained cell on a typed rejection.
The [literal result consumer](../../tests/fixtures/parallel-result-consumer.lkjc)
also covers both mixed orientations. An I64/ByteBuffer pair therefore has type
`(owned-product (field left I64) (field right ByteBuffer))` and uses the same unpacking
form. The existing restrictions on borrowing and ordinary aggregate storage apply.

The public test builds library, worker and consumer as three separately exported
or imported packages. In its all-owned case, the initial buffer is exactly
`[0,255,128]`; an identity-preserving edit changes the child's appended octet from
7 to 8, and the parent appends 9 after borrowing the returned buffer. The result
contains the complete Bytes payload `{"$bytes":"AP+ACAk="}`, observed length 4,
the exact input cell value, and the parent's replacement value -81. Both signed
I64 extremes and both choice cases execute with and without authoring sources.
The nested case returns the complete `[0,255,128,64]` buffer and the exact cell.

`run` reports `parallel_scopes`, `parallel_worker_dispatches` and
`parallel_inline_fallbacks` inside its production
observation. They count groups, accepted off-thread jobs and caller fallbacks,
not physical thread creation, simultaneous activity or speedup. Development
0.1.72 reuses auxiliary workers and replaces the former `parallel_workers_spawned`
observation. Worker exhaustion falls back to the calling thread. Optional cumulative
quotas apply to the entire invocation, including both children and later parent work.
On a trap or cancellation, started children are cancelled cooperatively and joined,
and every transferred owner is reclaimed before the invocation returns. A successful
child's returned owner is also reclaimed when its sibling fails or the parent cannot
allocate the joined pair. Failure does not implicitly return inputs or retry work.

No capability grants or external adapters enter these children. Use ordinary parent
task code for external effects before or after the group. Channels, detached tasks,
borrowed child inputs and detached lifetimes are separate future boundaries. See
the [normative contract](../spec/structured-parallel.md) and
[current acceptance status](../status.md) for precise limits and evidence.

## Closed generic child applications

A generic worker can transfer ownership without knowing the concrete carrier. A
worker with an Owned method contract can also forward a selected implementation:

```text
(parallel
  (implementation-call generic-workers::transform (types ByteBuffer)
    (implementations concrete@buffer::Octets) (i64 8) (local buffer))
  (implementation-call generic-workers::forward (types OwnedI64Cell)
    (implementations concrete@cell::Scalar) (local count) (local cell)))
```

The [generic workers](../../tests/fixtures/parallel-generic-workers.lkjc) declare
both tasks; the existing carrier library supplies their exact contracts and
implementations. This pair has the same owned-product type and unpacking rules as
the earlier monomorphic example. A plain generic `call` needs no witness when the
callee has no implementation parameters:

```text
(call generic-workers::transfer (types OwnedI64Cell) (local cell))
```

That call can be either child of `parallel`. It can also instantiate `transfer`
with a complete owned product or choice, moving the entire aggregate rather than
reconstructing its leaves. Ordinary generic results such as `list I64` remain
ordinary data. Every concrete type argument is checked, including unused cases and
phantom arguments. An open caller type parameter, borrowed carrier, callable or
capability is not made transferable by wrapping it in a generic type.

The [generic consumer](../../tests/fixtures/parallel-generic-consumer.lkjc) is a
literal three-package example. Its public test retains package export/import,
identity-preserving edits, complete returned byte payloads, both signed I64 extremes,
both owned-choice cases, and execution after deleting the source projects and
package transports. Another literal witness uses two implementations with the same
Self type but different behavior: their exact selected references, not just Self,
remain distinct through forwarding and return custody.
