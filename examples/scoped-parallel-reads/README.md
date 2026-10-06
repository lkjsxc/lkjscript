# Recursive scoped parallel reads

These literal proposals author a generic library before independent cell, buffer
and nested-product items exist. Named tasks borrow the same owned storage across
joined recursive children. The consumer then drains and reuses that original
owner. [Status](../../docs/status.md) owns tested source and public availability;
these files are a language witness, without a runtime speedup claim.

## Three independently authored packages

Create a minimal library project and add its exact builtin standard dependency.
Author these proposals separately, using the current `request base=BASE`, then
`change plan`, apply the unchanged input with its exact token, and `check`:

1. [Worklist algorithms](../owned-worklists/library.lkjc).
2. [Borrowed indexing](../owned-read-results/library.lkjc).
3. [Generic storage](../generic-owned-implementations/storage.lkjc).
4. [Recursive read tasks](library.lkjc).

Export the completed library before creating the item project. Stage the exact
standard and library transports in that minimal project, add both dependencies,
import `owned-worklists` from the library locator, and author the existing
[independent items](../generic-owned-implementations/elements.lkjc). Export those
items after their native check and unchanged canonical draft re-entry.

Create a command consumer and stage both completed transports. Add their exact
dependencies and import `owned-worklists`, `owned-read-results`, `generic-storage`
and `scoped-parallel-reads` from the library locator, plus `generic-elements` from
the item locator. Author [application.lkjc](application.lkjc).
The [native guide](../../docs/guides/native-scoped-parallel-reads.md) explains
the authoring and lifetime contract; the [library guide](../../docs/guides/native-library.md)
gives exact staging and export commands.

## What executes

`range-sum<T,W>` borrows `W: Owned + Shareable`. Intervals of at most 32 items use
the same sequential indexed helper as `serial-sum`; larger intervals split into
two joined task calls. Both children may read the same root. Each indexed read
returns a source-tied item view whose lexical scope ends after its exact `Element`
observer returns ordinary I64 data. `sum` always supplies a valid range;
direct `range-sum` callers must supply `0 <= start <= end <= length`.

`parallel-sums` launches recursive sums with ordinary and modulo-four `Element`
witnesses over the same storage. Their equal types retain distinct implementation
identities. `RecursiveSum<T,W>` maps the borrowed `Sum` task method to the same
generic sum with exact element and reader prerequisites. The method does not
duplicate the algorithm. The app exercises both Shareable-only library constraints
and the combined `Owned + Transferable + Shareable` consumer constraints.

Build `scoped-parallel-reads.lkja` beside the supplied
[grant-free descriptor](scoped-parallel-reads.deployment.json), then run with
arguments `[[2,7,11,3]]`. Each cell/buffer flat/chunked scenario returns:

```json
{"length":4,"sum-left":23,"sum-right":11,"method-sum":23,"drained":[3,11,7,2],"empty-length":0,"reused":[17]}
```

Each nested-product scenario returns:

```json
{"length":4,"sum-left":71,"sum-right":11,"method-sum":71,"drained":[15,23,19,14],"empty-length":0,"reused":[29]}
```

The product adds its retained tag 5 and stamp 7 to each input. Empty input has zero
sums, an empty drain, and the same owner-reuse result. The combined target returns
these under `cell-flat`, `cell-chunked`, `buffer-flat`, `buffer-chunked`,
`product-flat` and `product-chunked`. Each supplied individual descriptor selects
`scoped-read-` followed by that scenario name. The shared input domain is `0..255`.

The [serial cell/flat descriptor](scoped-read-cell-flat-serial.deployment.json)
selects the same complete result using sequential sums throughout. Compare it with
`scoped-read-cell-flat` on matched inputs, exact witnesses, storage construction,
drain and reuse. Tests retain fresh-process preparation, invocation and encoding
times plus modeled observations. They check zero serial parallel scopes, the
32-item cutoff and nested groups on 513 inputs. These observations do not establish
a speedup or quantify API costs.

## Acceptance obligations

Copied-product tests author and export the library before items, retain literal
proposals, check unchanged drafts and task-use inspection, edit the reuse value
while preserving declaration identities, and compare complete independent results.
Empty, singleton, octet limits, 31/32/33-item chunk boundaries and every individual
513-item target exercise recursive access and subsequent original-owner reuse.
The same artifacts run after authoring projects and transports are removed.

Move-only generic evidence cannot authorize shared reads. Task result loans,
borrowed task consumption, mismatched borrowed task methods and either orientation
of read/consume aliasing must reject without advancing accepted HEAD. A later valid
invocation must remain healthy. Every successful run requires zero residual owned
handles, joined task cleanup and no cleanup failures. Runtime owners separately
prove actual overlap, nested custody, allocation identity and cleanup on failure.
