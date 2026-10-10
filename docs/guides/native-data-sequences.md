# Work with ordinary data in owned sequences

Use `(owned-sequence T)` for mutable working storage whose elements are ordinary
immutable data. The storage remains an affine owner even when `T` is I64, Bool or
an immutable aggregate, and even when the sequence is empty. Use an ordinary list
when retained immutable versions are the desired interface.

The [semantic contract](../spec/owned-sequences.md) defines precise admission,
custody and failure behavior. [Status](../status.md) owns exact source acceptance
and public executable availability. Maintained literal inputs live in
[the data-sequence example](../../examples/data-sequences/README.md); the
[dependency planner](../../examples/dependency-plan/README.md) supplies a maintained
working-storage consumer.

## Build, read and replace

These direct expressions operate on an explicitly typed live local `values`:

```text
(sequence-empty (type (owned-sequence I64)))
(sequence-push (type (owned-sequence I64)) (i64 17) (local values))
(sequence-length (type (owned-sequence I64)) (local values))
(sequence-get (type (owned-sequence I64)) (local values) (index (i64 0)))
(sequence-replace (type (owned-sequence I64)) (index (i64 0)) (i64 23) (local values))
(sequence-pop (type (owned-sequence I64)) (local values))
```

Bind every returned sequence to an owning local before its next use. Push consumes
that local and returns the sequence with the new value appended. Length borrows
it. Get evaluates its index once, then reads under a short loan and returns an
ordinary immutable value. That returned value remains valid after replacement,
pop, disposal or transfer of the source sequence. Nested immutable backing may
remain shared; the result retains no source loan.

Replacement evaluates index, new value and source in that order. It consumes the
sequence and returns this owned product:

```text
(owned-product
  (field rest (owned-sequence I64))
  (field value I64))
```

Use `unpack-owned` to bind `rest` as the next sequence owner and `value` as the
ordinary displaced value. The storage preserves its allocation and capacity.
Replacing 17 with 23 returns 17 while a later get returns 23. Both get and replace
trap on a negative or out-of-range index; neither grows the sequence implicitly.

Pop removes the last value and returns an exhaustive owned choice. Its `empty`
case contains the reusable empty sequence; its `item` case contains an owned
product with `rest` and `value`. Use `match-owned`, then `unpack-owned`, to recover
their bindings. Even the empty outcome owns storage and must be retained, returned
or cleaned up.

## Generic standard wrappers

The standard package exports these ordinary pure generic functions with
`T: Transferable`. Parameter order is part of the public contract:

| Function | Arguments | Result |
| --- | --- | --- |
| `data-sequence-empty<T>` | none | `(owned-sequence T)` |
| `data-sequence-length<T>` | borrowed sequence | I64 |
| `data-sequence-push<T>` | ordinary value, consumed sequence | `(owned-sequence T)` |
| `data-sequence-pop<T>` | consumed sequence | owned choice containing empty sequence or rest/value product |
| `data-sequence-get<T>` | I64 index, borrowed sequence | ordinary T |
| `data-sequence-replace<T>` | I64 index, ordinary value, consumed sequence | owned product containing rest/value |
| `data-sequence-discard<T>` | consumed sequence | Unit |

For a project with the standard dependency under the `std` alias, a read is:

```text
(call std::data-sequence-get (types I64) (i64 0) (local values))
```

Author and export the generic library before constructing concrete consumers.
Stage its exact transport, add the exact dependency through native declarations,
then check, build and run through the public executable. Use canonical `change
draft` for unchanged re-entry and reviewed literal edits. The accepted meaning
graph remains the sole program authority; request files and projections are
proposals and derived views. The [library guide](native-library.md) describes the
ordinary package operations.

Owned elements continue to use `sequence-*` wrappers with `T: Owned` and
`borrow-owned-item` for lexical inspection. Their new `sequence-replace` consumes
an exact owning replacement local and returns the displaced owner alongside the
sequence. Ordinary get cannot produce an owner or a source-tied borrowed view.

## Working storage and limits

The native planner holds remaining dependency counts and levels in I64 sequences,
visited flags in a Bool sequence, and ready component indices in an append-only
I64 sequence. A scalar head consumes queue entries logically; each component is
appended at most once. Signed endpoints are checked before indexing. The planner
materializes the output map once and preserves its established complete/refusal
results. This is maintained native-tool adoption; compiler integration remains a
separate boundary with its own capacity and authority requirements.

Complete element admission includes unused substitutions, phantom arguments and
inactive syntax. Hidden resources, secrets and callables reject before even an
empty owner can be created. The container cannot be copied into ordinary data or
captured merely because its elements are ordinary. Transfer and sharing require
complete proofs for the sequence and its elements.

Reservation failure, bounds traps and cancellation join owned cleanup, expose no
successful partial result and allow a healthy subsequent invocation. They do not
undo completed effects or authorize an implicit retry. Existing finite type-depth,
collection, storage and preparation limits still apply. Retain preparation,
execution, growth and admission-work measurements separately; mutable storage
alone does not establish a speed or API-cost improvement.
