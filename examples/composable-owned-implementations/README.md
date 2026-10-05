# Implementations with explicit prerequisites

These literal proposals author three independent packages. The library exports
`Maximum<T,W>` and `DelegatingReader<T,W>` before any concrete item package exists.
The item package provides cells, buffers and a nested owned product. The consumer
chooses explicit implementations and observes the full prerequisite identities
through source-tied selections and first-tie behavior.

This is a designed language witness. [Status](../../docs/status.md) owns accepted
source and executable availability; the public CLI tests own its actual validation.

## Author through the public product

Copy a compatible executable and the literal proposals outside the compiler
checkout. Create a minimal library project, export and stage the exact builtin
standard transport, and add its exact dependency. Author these proposals separately
in this order:

1. [Worklist algorithms](../owned-worklists/library.lkjc), with the standard dependency.
2. [IndexRead selector](../owned-read-results/library.lkjc).
3. [Generic storage and readers](../generic-owned-implementations/storage.lkjc).
4. [Composable adapters](adapters.lkjc).

Each proposal starts with `request base=BASE` from current `status`. Use
`change plan --input-file FILE`, apply the same input with its exact plan token,
then `check`. Draft accepted modules and require unchanged re-entry. Export the
completed library before creating the item package.

Create a minimal item project. Stage the exact standard and library transports,
add both dependencies, and import `owned-worklists` from the exact library package
and package revision. Author the existing independent
[element proposal](../generic-owned-implementations/elements.lkjc), draft and export
it. The proposal supplies no reader or selector specialization. Its nested product
stores the input in a cell with tag 5 and stamp 7, so observation and finishing
return the input plus 12.

Create a command consumer project, stage both completed transports, and add both
exact dependencies. Import `owned-worklists`, `owned-read-results`, `generic-storage`
and `composable-adapters` from the library locator and `generic-elements` from the
item locator. Author [application.lkjc](application.lkjc). The
[native library guide](../../docs/guides/native-library.md) gives the exact
dependency, staging and export commands.

## Explicit composition

`Selection<Item>::best(Self borrow) -> Item` declares its result borrowed from
argument 0. `Maximum<T,W>` requires `Element<T>` followed by `IndexRead<W,T>` and
maps `best` directly to the existing generic `select-max`. Its mapping explicitly
passes both prerequisites in that order. No adapter rewrites the algorithm.

`DelegatingReader<T,W>` requires an `IndexRead<W,T>` and maps both methods to generic
forwarding functions with that exact prerequisite. The consumer nests this scheme
twice over `FlatReader<OwnedI64Cell>` and over `ReverseReader<OwnedI64Cell>`.
The signatures and enclosing declarations are equal; the selected leaf identity
changes the first tie. The generic scenario constructs `Maximum` from its lexical
element and reader parameters, preserving complete symbolic applications.

The `Drain<T,W>` scheme requires `Element<T>` and `Worklist<W,T>`. Its consuming
method maps a named task with an empty effect row to a wrapper around the existing
generic drain. Joined parallel children receive exact cell and buffer applications.
Use an explicit deployment descriptor for this task even when sources remain
present.

## Observable behavior

Build with `build --output composable-owned-implementations.lkja` beside the supplied
[descriptor](composable-owned-implementations.deployment.json). The command accepts
one I64 list; its shared cell/buffer/product domain is `0..255`.

For `[[2,7,11,3]]`, every scenario has length 4 and empty-length 0:

| Scenario | selected | tied-selected | drained | reused |
| --- | --- | --- | --- | --- |
| cell-flat, buffer-chunked, cell-nested | [11] | [7] | [3,11,7,2] | [17] |
| product-flat, product-chunked | [23] | [19] | [15,23,19,14] | [29] |
| cell-reverse, cell-nested-reverse | [11] | [3] | [3,11,7,2] | [17] |

Selection returns a view of the original collection. Both view scopes end before
draining that same owner. The empty owner is observed, reused and drained again.
The modulo-four observer distinguishes ties through the ordinary identity observer;
reverse traversal selects the last original tie. Empty input has empty selected,
tied-selected and drained lists, with the same empty-owner reuse result.

Individual `composable-cell-nested`, `composable-cell-nested-reverse` and
`composable-product-chunked` targets accept the same argument. The
`composable-parallel-drain` task returns `{"left":[3,11,7,2],"right":[3,11,7,2]}`
for the example input, after consuming both collections and joining cleanup.

## Acceptance obligations

Copied-product tests retain literal proposals, exact exports, canonical drafts,
and reviewed edits preserving declaration identities. They observe a reuse edit
from 17 to 19, then restore it. Independent expected outputs cover empty,
singleton, first ties, octet limits and 31/32/33-item boundaries. Each individual
target receives 513 items. The same built artifact runs after all authoring
projects and transports are removed, requiring zero residual handles, zero
remaining tasks and no cleanup failures.

Missing, extra, reordered, foreign-scope and incorrectly typed prerequisites,
invalid unused method mappings, altered result provenance and wrong task kind
must reject without changing accepted HEAD. Successful execution after rejection
checks that the accepted project remains usable. These are public behavior
obligations; lower-level forged transport/artifact, resource and recursion checks
remain with their independent verification owners.
