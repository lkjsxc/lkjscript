# Work with owned sequences

Use `(owned-sequence T)` when the number of owned elements is known only at
runtime. Ordinary lists continue to hold ordinary values. Sequences own their
elements, including while empty, and support append, last-element removal and
scoped indexed inspection.

See [status](../status.md) for exact source acceptance and public executable
availability, and the
[semantic contract](../spec/owned-sequences.md) for precise rights and cleanup.

The maintained [example](../../examples/owned-sequences/README.md) has a
[generic library](../../examples/owned-sequences/library.lkjc), independently
authored [carriers](../../examples/owned-sequences/carriers.lkjc), an
[application](../../examples/owned-sequences/application.lkjc) and a
[task consumer](../../examples/owned-sequences/tasks.lkjc). The library is checked
and exported before the example's concrete carrier implementations are authored. Its `Element`
contract declares creation from I64, pure borrowed observation and consuming
completion returning I64.

## Construct and inspect

With the example packages imported, build a runtime-sized cell sequence through
an exact witness:

```text
(implementation-call owned-sequences::build (types OwnedI64Cell)
  (implementations concrete@sequence-carriers::Scalar)
  (local inputs))
```

`inputs` is an ordinary `(list I64)`. The generic builder creates one owned element
per input and appends it in input order. The library does not depend on the cell's
representation. ByteBuffer uses the same algorithm with the `Octets` witness.

Bind the returned owner to an explicitly typed local before reading or consuming
it. Here `values` is a live `(owned-sequence OwnedI64Cell)`:

```text
(borrow-owned-item
  (type (owned-sequence OwnedI64Cell)) (local values)
  (index (i64 0)) (binding view (type OwnedI64Cell))
  (in (call sequence-carriers::cell-read (local view))))
```

The index runs once before the read loan starts. Negative or out-of-range indices
trap. `view` exists only in the body and can be forwarded to exact pure borrowed
helpers. After scope exit, `values` remains available for another read or a
consuming operation. Nested sequences use the same operation at each level, and
ancestor custody persists through nested reads.

The generic `summarize` function loops over scoped indexed reads. Selecting `Scalar`
observes each stored cell. Selecting `Alternate` observes 99 per cell while its
later consuming finish still returns the original stored I64. Witness selection
is explicit and confers no effect authority.

## Append, pop and reuse

These direct expressions are also available through ordinary generic wrappers:

```text
(sequence-empty (type (owned-sequence OwnedI64Cell)))
(sequence-length (type (owned-sequence OwnedI64Cell)) (local values))
(sequence-push (type (owned-sequence OwnedI64Cell)) (local element) (local values))
(sequence-pop (type (owned-sequence OwnedI64Cell)) (local values))
```

Push consumes the element before consuming the sequence. Pop consumes the sequence
and returns an exhaustive owned choice. `empty` carries the reusable empty
sequence; `item` carries an owned product with `rest` and the last `value`. Use
`match-owned`, then `unpack-owned`, to recover their owning bindings. Both outcomes
preserve sequence allocation and capacity.

The example's generic `drain` finishes every popped element in LIFO order. Its
result is an owned product containing an ordinary list of finished I64 values and
the empty sequence. Keeping that empty owner enables the application to append a
new element and drain it again. Dropping it at frame exit instead releases its
storage through normal cleanup.

## Author and run

Each `.lkjc` file is a literal declarations request body. The minimal library first
exports and stages the builtin standard transport, then adds its observed exact
dependency for the ordinary list and arithmetic helpers. Prepend
`request base=REVISION` from current `status`, plan with `change plan`, apply the
returned review token and run `check`. Export the generic library first through
`package current export --kind transport`, stage its exact transport in carriers,
and add its exact dependency with an `owned-sequences` native alias. Check and
export carriers, then stage both exact packages into a command project using
`owned-sequences` and `sequence-carriers` aliases. The
[library guide](native-library.md) describes these operations.

The `owned-sequences` target accepts one ordinary list argument. Write
`[[2,3,7]]` to the argument file. The scalar and ByteBuffer paths both report two
summaries of 12, pop 7, replace it with 11, drain `[11,3,2]`, confirm length zero and
reuse the empty owner to drain `[17]`. The alternate cell witness reports two
summaries of 297, then the same consuming results. The nested sequence reports 12.
The combined example requires octet inputs in `0..255`; the cell carrier itself
retains the existing I64 contract.

Build `owned-sequences.lkja` and place it beside the supplied deployment descriptor.
Run through the public executable using `run --deployment` with argument and result
files. The public acceptance suite deletes source projects and staged transports
before repeating execution. Use canonical `change draft` for unchanged re-entry and
reviewed literal edits; the accepted meaning graph remains the program authority.

## Tasks and limits

The task consumer uses a clock capability in an index expression, returns an
unrelated new owner from the read body and then transfers complete cell and buffer
sequences through joined parallel tasks. Each worker appends 11, and the parent
drains both returned sequences. For `[[2,3,7]]`, both draining results are
`[11,7,3,2]`. This target needs an explicit clock grant and a nonempty input list.

Views cannot be consumed, returned, stored, captured, passed unrestricted or moved
to a task. The source and all ancestors remain protected through the body. A
sequence crosses a task boundary only when its complete element type satisfies
the existing transfer contract. `T: Owned` by itself grants no task permission.

Cancellation, traps, growth refusal and failed transfer finalize loans and owned
storage. They do not undo completed effects or make an invocation safe to retry.
Finite type depth and existing collection/storage limits still apply. Function
extraction containing an indexed scope rejects before publication.
