# Pure generic consumer folds

This example keeps the existing language and runtime unchanged. It exports ordinary
`fold-four<Item, State>` and `fold-eight<Item, State>` functions over immutable
lists, with a caller-supplied pure `(State, Item) -> State` function. The private
loops process four or eight elements per tail transfer, then a scalar remainder.
It is a measured library prototype, not a replacement for `standard::list-fold-left`,
a new intrinsic, a borrowed iterator, or a globally selected optimization.

## Meaning and boundaries

Both public wrappers obtain the length from the exact list and start at index zero.
The caller cannot supply a traversal bound or private cursor. Each step takes the
previous step's state and the next item in ascending index order. No associative,
commutative, duplicable-effect or parallel-reduction law is assumed. Empty input
returns the initial state without invoking the callback. A callback failure aborts
the fold; no partial successful result is returned.

The full-block condition uses `block <= limit - index`. For an admitted call,
`0 <= index <= limit` initially and after every full block or scalar step. Thus a
full block proves each projected index and the next index fit within `limit`
before any `index + offset` is evaluated. The immutable list is retained throughout;
there is no unsafe indexing or unchecked child projection. These are ordinary
unrestricted values and pure callbacks, not owned/affine carriers, borrowed results,
effectful folds, cancellation masking, or resource-budget equivalence.

Unrolling reduces repeated loop/call setup, not the asymptotic cost of indexed
persistent-list access. It makes a larger function and may retain more intermediate
state or require larger live-local bounds. It is not a zero-allocation or general
linear-iterator claim. Budget refusal thresholds may differ while the declared
success, failure and cleanup contracts remain mandatory.

## Public authoring and independent import

Start two fresh `command` projects with one compatible installed product. Apply
[library.lkjc](library.lkjc) to the supplier through `change plan` and `change apply`,
replacing only `BASE` with its observed current revision. Run `check`, then export
its complete source closure:

```sh
lkjscript --project supplier package current export --kind transport --output folds.lkjp
```

Stage that observed transport in the independent consumer. Staging stores bytes;
it does not select a dependency or change accepted meaning. In
[consumer.lkjc](consumer.lkjc), replace `BASE`, `FOLD_PACKAGE`, `FOLD_SEMANTIC`, and
`FOLD_REVISION` with the exact observed identities. Its explicit `add.dependency`
selects the closure; the `use` clause alone does not. Plan, apply, check and build
through the same public operations. The consumer owns its positional weighted
callback and a distinct record state, rather than sharing the supplier's test oracle.

The maintained `public_cli` tests author both projects from these literal files,
export/import the exact supplier, compare complete results to an independent
positional dot product and the standard fold, then physically remove both disposable
authoring projects, proposals and supplier transport. The retained artifact runs
again through the copied product. The tests cover both block boundaries, remainders,
empty and nonzero initial states, negative values, typed callback rejection,
trapping callbacks, repeated execution and unchanged executable/artifact identities.
Timing thresholds are deliberately absent from this correctness test.

```sh
cargo test --locked --test public_cli native_blocked_folds -- --test-threads=1
```

The example changes neither `packages/standard` nor product identity. Source-test
acceptance, final distributed bytes and running applications remain separate.

## What the cost experiment found

The [retained experiment](study.md) compares existing consumer strategies and this
blocked prototype under the same 0.1.89 product. The third matrix completed 1,500
invocations with exact scalar results, unchanged bound inputs and checked cleanup.
At 16,384 entries and eight repeats, reusing the same entry list with the standard
fold took a median 522.617ms of measured invocation time; the eight-item generic
fold took 342.668ms. Re-enumerating before each standard fold took 543.680ms.
These measurements include construction of the common Map and are not a new
runtime's speed-up or a claim about all programs.

An unfavorable result is retained: on an empty Map with eight repeats, the
blocked-eight route took 0.175ms versus 0.133ms for the reused-list standard fold.
Small/cold workloads, large callback states and other payload shapes require their
own decisions. The measured executable is a selected source-built product, not an
assertion that these library functions are present in the latest public release.

## Nested persistent histories

[history.lkjc](history.lkjc) is a second independently imported consumer. Its Item
is a record containing text and a list of integers. Its History state holds a
current list of items and a list of every retained prefix. Each callback appends
one item, then retains that new prefix without changing any earlier value.

The maintained history matrix compares complete results against an independent
prefix-slicing oracle, not the callback's recurrence. Fifteen lengths cover both
sides of four/eight-item blocks and the 32-way persistent-list boundary, with two
initial states, three implementations and attached/source-deleted execution:
180 complete result runs. The seeded initial history is deliberately independent
of the current list. Empty input preserves both exactly; the API does not impose
an unstated consistency law. Separate cases reject a malformed nested final item
and malformed unused initial history, including for an empty input, then require
successful recovery. Standard-fold results and all retained prefixes remain checked.

Run the complete seven-case family, including both independent oracle tests:

```sh
cargo test --locked --test public_cli native_fold -- --test-threads=1
```

These correctness cases set no timing threshold and do not remeasure the old
scalar study. The nested state may retain substantially more data; no speed or
memory benefit follows merely from block traversal. No unchecked cursor, owned
carrier or borrowed-result interface is added.

The final-byte public harness requires the exact scalar and history behavior
witnesses and every enumerated case in `native_declarations::native_fold::` under
candidate acceptance/workload generation 6, retaining the fold obligations
introduced in generation 5. It uses the extracted candidate, not
a development-binary fallback, and requires unchanged identities and joined
cleanup. Availability of a library example in source, full source acceptance,
exact host-product proof and finalized distribution acceptance remain separate;
[status](../../../docs/status.md) owns the current selection.
