# Compact recursive prerequisite proof

[The literal library](library.lkjc) extends the
[acyclic prerequisite workload](../compact-callable-proof/README.md) with a borrowed
`repeat` function. It recurses once with `again = false`, forwarding the same exact
reader, then invokes that reader. It never constructs a witness on its recursive
edge. The ordinary Boolean precedes the borrowed owned value in its parameter order.

The library still declares `read0` through `read24`. Each layer wraps the previous
reader twice in `Pair<T>`. These small declarations describe more than sixteen
million logical leaf occurrences at 24 layers, but their selection shapes form a
compact DAG. The recursive function need not expand all these paths to prove that
forwarding does not grow a type.

## Public use

Use the same public new/status/plan/apply/check, canonical draft and exact package
export procedure as the acyclic workload, substituting this literal library file.
The full generic library must be admitted and exported before a concrete item or
consumer exists. No built-in package or external semantic generator is needed.

The [shared literal consumer](../compact-callable-proof/consumer.lkjc) imports the
exported `compact-proof` module and executes four concrete layers. `Cell` and
`Alternate` have the same Self type and contract but distinct implementation
identities. Both borrow the original cell; only after the reads finish is that
cell consumed. The complete result for `[17]` is:

```json
{"alternate":99,"observed":17,"restored":17}
```

The source-selected `native_owned_demanded_callable_proof` public test also checks
the minimum/maximum I64 values, negative input and zero. It rejects a foreign
prerequisite in the unused deepest function and extra application operands without
changing accepted HEAD, re-enters canonical drafts unchanged, edits 99 to 100 while
preserving implementation identities, restores it, and runs the original artifact
after deleting both disposable authoring projects and their transport.

The copied public executable runs outside the compiler checkout with no grants,
secrets or inherited credentials. The new case belongs to the mandatory
`native_owned_` finalized-archive harness family; a local source run is not itself
finalized-archive acceptance.

## Scope

The source-matched v0.1.82 executable refuses this corrected literal generic
library with resource diagnostic `kernel_callable_flow_storage` and leaves the
accepted revision unchanged. This is separate from the earlier invalid fixture
iterations, whose original source refusals are retained with development evidence.

[Demanded provenance](../../docs/decisions/demanded-callable-provenance.md) retains
all expanding cycles by following incoming type flow from declaration parameters.
It does not merge same-shaped implementations or omit ordinary source admission.
The workload demonstrates large generic-source admission, not inexpensive concrete
execution of `read24`. Concrete witness storage, the number of actually demanded
paths and finite preparation limits remain independent concerns.

[Status](../../docs/status.md) records the exact accepted source and public release.
