# Generic owned implementations over independent items

These literal proposals author three ordinary packages. The first exports generic
Worklist and IndexRead algorithms plus flat/chunk32 implementation schemes before
the item package exists. The second supplies only element semantics. The third
applies the same storage and reader declarations to cells, buffers and a newly
authored nested owned product.

[Status](../../docs/status.md) owns actual source acceptance and public executable
availability. The [guide](../../docs/guides/native-generic-owned-implementations.md)
explains use; the [specification](../../docs/spec/generic-owned-implementations.md)
owns exact scheme application and independent admission. This is a designed
language witness, with no compiler-pass replacement or production application
adoption claim.

## Author and export before items exist

Copy a compatible executable and the literal inputs outside the compiler checkout.
Create a minimal library project. Export and stage the exact builtin standard
transport, then add its exact dependency through a reviewed proposal. Author
[the Worklist algorithms](../owned-worklists/library.lkjc),
[the IndexRead selector](../owned-read-results/library.lkjc), and
[storage.lkjc](storage.lkjc), in that order, as separate proposals. Their module
names are owned-worklists, owned-read-results and generic-storage. Each file imports
std once; do not concatenate repeated std imports into one proposal.

Every proposal starts with `request base=BASE` from current `status`. Use
`change plan --input-file FILE`, apply the unchanged input with its exact token,
then `check`. Draft each accepted module and require unchanged re-entry. Export
the completed generic package before creating the element project.

The Flat<T> scheme maps Worklist methods directly to the standard generic sequence
functions. Chunked<T> maps the same methods to the ordinary generic chunk functions.
FlatReader<T> and ChunkedReader<T> return source-tied item views. ReverseReader<T>
has the same contract and Self as FlatReader<T>, but visits indices in reverse.
SequenceTask<T> consumes a sequence in a named empty-row task. No scheme requires
an element observer or resolves one implicitly.

Create a minimal element project. Stage the exact standard and completed library
transports, add both dependencies, and import owned-worklists from the exact
library package and package revision before authoring [elements.lkjc](elements.lkjc):

```text
declarations.begin
(units (use owned-worklists LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

The element module defines independent cell and buffer witnesses and this third
item type:

```lisp
(owned-product
  (field payload (owned-product (field cell OwnedI64Cell) (field tag I64)))
  (field stamp I64))
```

Construction stores the input in its cell, tag 5 and stamp 7. Observation reads
the cell through both ancestor scopes and adds the metadata. Finishing consumes
both products and the original cell, returning the same input plus 12. The element
package supplies no specialized sequence, chunk or reader methods. Check, draft
and export it.

Create a command consumer project. Stage both completed transports, add their
exact dependencies, and import owned-worklists, owned-read-results and generic-storage
from the library locator, plus generic-elements from the element locator. Author
[application.lkjc](application.lkjc). The [native library guide](../../docs/guides/native-library.md)
gives exact staging/export commands and the complete observed dependency locators.

## Complete observable behavior

The combined target takes one ordinary I64 list. It exercises cells, one-octet
buffers and products, so its shared input domain is `0..255`. Build through
`build --output generic-owned-implementations.lkja` beside the supplied
[grant-free descriptor](generic-owned-implementations.deployment.json).

```sh
./lkjscript run --deployment generic-owned-implementations.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

For `[[2,7,11,3]]`, every result contains length 4 and empty-length 0. The complete
remaining fields are:

| Result fields | selected | tied-selected | drained | moved | reused |
| --- | --- | --- | --- | --- | --- |
| cell-flat, cell-chunked, buffer-flat, buffer-chunked | [11] | [7] | [3,11,7,2] | [2,7,11,3] | [17] |
| product-flat, product-chunked | [23] | [19] | [15,23,19,14] | [14,19,23,15] | [29] |
| cell-reverse | [11] | [3] | [3,11,7,2] | [2,7,11,3] | [17] |

Selection observes a borrowed result, ends its scope, and drains the unchanged
original owner in LIFO order. The empty owner is observed and reused. A separate
construction then moves every item into the other representation; destination
drain reverses the movement and restores authored order. Products reveal both
nested custody and ordinary metadata in every observed/finished value.

The modulo-four observer makes the first tie distinguishable through an identity
observer. ReverseReader selects the last original tie because its first index is
the last stored element. Equal contract applications do not erase exact scheme
identity. Empty input returns empty selected, tied-selected, drained and moved
lists, with the same empty-owner reuse result.

The bundle also exposes individual generic-cell-flat, generic-cell-chunked,
generic-buffer-flat, generic-buffer-chunked, generic-product-flat,
generic-product-chunked and generic-cell-reverse targets. generic-direct-length
returns 1 after a symbolic Flat<T> builder and a direct reader method call.
generic-parallel-lengths runs two joined generic children through SequenceTask<T>;
each returns the input length after consuming and cleaning up its cell/buffer
sequence. Run that named task through an explicit deployment descriptor with the
same bundle and empty grants, even when sources remain present; an empty effect
row does not make a task eligible for the pure source/reference runner. These
observations establish behavior, not worker fairness or speedup.

## Acceptance

The copied-product tests retain literal inputs, exact exports, canonical drafts
and reviewed identity-preserving edits. They observe an edited reuse result before
restoring it. Independent expected results cover empty, singleton, first ties,
octet limits and 31/32/33-item boundaries in the combined command. Each of the
seven representation/reader targets receives 513 elements independently, both
attached and after authoring projects and transports are removed. The combined
seven-scenario source/reference invocation reaches the existing allocation bound
at that large workload; the original failure is retained and limits are unchanged.
Direct method calls, symbolic applications,
closed task mappings and generic structured children share the same schemes.

Invalid unused maps, missing/extra arguments, incompatible applications, wrong
constraints/contracts, hidden owned ordinary types and missing transfer bounds
reject without changing accepted HEAD. A separate cross-package fixture rejects
an expanding mapped-target cycle in untaken syntax while admitting finite plain
and parameter-permuting recursion. Existing public execution helpers require
complete canonical outputs and joined cleanup with no residual owned handles.

Independent kernel, ownership, reference and artifact tests own additional forged
transport, result-provenance and failure-cleanup obligations. Exact tests and
release acceptance remain at their maintained owners; no derived artifacts are
authored or edited directly.
