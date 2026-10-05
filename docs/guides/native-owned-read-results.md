# Return read-only views from native libraries

`borrow-from` lets a pure library return a view into caller-owned storage. The
caller observes that view inside `borrow-call`, then regains permission to consume
its original owner. Types and exact witnesses carry the relationship across
packages; the [specification](../spec/owned-read-results.md) defines admission and
cleanup.

A function declares the parameter by name:

```lisp
(parameter create storage (type (owned-sequence OwnedI64Cell)) (use borrow))
(returns OwnedI64Cell (borrow-from storage))
```

Its body may return that parameter or a child selected through an existing owned
read scope. The returned view must come from the selected input on every path.
An owned-contract method uses a zero-based parameter position. `IndexRead<Item>`
has `length(Self borrow) -> I64` and `at(I64, Self borrow) -> Item`, so `at` declares
`(returns Item (borrow-from 1))`. Concrete method targets retain that exact
relationship after type substitution.

Use the returned view through a lexical call:

```lisp
(borrow-call
  (implementation-call owned-read-results::select-max
    (types OwnedI64Cell (owned-sequence OwnedI64Cell))
    (implementations concrete@worklist-carriers::Scalar
      concrete@read-carriers::FlatCellReader)
    (local storage))
  (binding selected (type OwnedI64Cell))
  (in (call worklist-carriers::cell-read (local selected))))
```

The source argument is an exact live local. `call`, `implementation-call` and
`method-call` keep their normal invocation operands and evaluation order. Within
the body, selected has read rights and its source and ancestors are protected.
Returning the observed I64 ends the scope. A later consuming drain can then use
storage. A pure forwarding helper can return the view itself when its own
`borrow-from` names that same source.

The [maintained example](../../examples/owned-read-results/README.md) reuses native
worklist builders and consuming methods. Its new generic selector and readers
are additive modules in the same two library packages; the consumer is a third
package. Each storage representation supports cells and one-octet buffers.
The consumer checks empty storage before selecting, observes two selected views,
drains all original elements in LIFO order, and reuses the resulting empty owner.

For `[[2,7,11,3]]`, ordinary value comparison selects 11. An alternate observer
compares each value modulo four. Values 7, 11 and 3 share the greatest key, and
the selector keeps the first, 7. An identity observer on the returned view makes
this tie choice visible. Both views end before draining `[3,11,7,2]`.

The example README gives exact authoring order, dependency imports and standalone
execution. Canonical drafts retain result relationships and view binding
identities. Invalid local escape, wrong-input borrowing, ordinary call exposure
and protected-owner consumption reject before accepted HEAD changes.
Use [status](../status.md) for the selected executable's availability and
`capabilities --section change` for its current native authoring surface.
