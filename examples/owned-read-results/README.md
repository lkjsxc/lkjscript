# Native borrowed-result selection

These literal inputs extend the existing native worklists with a generic
`IndexRead<Item>` contract and a source-tied `select-max` result. The library is
checked and exported before concrete readers exist. A third package observes
returned views, then consumes and reuses the original storage owner.

[Status](../../docs/status.md) owns tested source and public binary availability.
These inputs establish a designed language witness when their public tests pass;
they do not establish production application adoption or compiler self-hosting.
The [guide](../../docs/guides/native-owned-read-results.md) explains ordinary use,
and the [specification](../../docs/spec/owned-read-results.md) owns exact semantics.

The successor [generic implementation example](../generic-owned-implementations/README.md)
retains these selection algorithms while exporting reusable reader schemes before
the concrete items exist. A later nested owned product uses the same readers;
explicit same-Self reverse selection also makes scheme identity observable.

## Three packages without duplicated algorithms

Create a minimal `library` project. Stage the exact builtin standard transport,
then author [the existing worklist library](../owned-worklists/library.lkjc)
followed by [library.lkjc](library.lkjc) in that same project. Both modules use
`std`; the new module refers to `owned-worklists::Element` within its own package.
Export the completed library package only after both modules pass `check`.

Create a minimal `carriers` project. Stage the exact completed library and builtin
standard transports. Add the exact library dependency and both imports:

```text
add.dependency package=LIBRARY_ID semantic-revision=LIBRARY_SEMANTIC package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units (use owned-worklists LIBRARY_ID LIBRARY_PACKAGE_REVISION)
       (use owned-read-results LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

Author [the existing worklist carriers](../owned-worklists/carriers.lkjc) followed
by [carriers.lkjc](carriers.lkjc) in that same project. The worklist module provides
constructors, element observers, consuming stack methods and lengths. The new
module provides borrowed indexing for flat and chunk32 storage, plus two alternate
Element witnesses whose observers compare values modulo four. Export the completed
carriers package after `check`.

Create a `command` consumer project. Stage both completed transports and add their
exact dependencies. Import `owned-worklists` and `owned-read-results` from the
library locator, plus `worklist-carriers` and `read-carriers` from the carriers
locator. Author [application.lkjc](application.lkjc).

Every proposal begins with `request base=BASE`, using the current project `status`.
Plan through `change plan --input-file FILE`, apply the unchanged input with the
returned exact plan token, and `check`. Author each old/new module as its own
proposal using the newly observed base revision. Each file imports std once;
concatenating files that repeat the same supplier alias is invalid. Unchanged
`change draft --module MODULE --output FILE` re-entry must plan as `unchanged`.
The [native library guide](../../docs/guides/native-library.md) gives export and
staging commands, including the complete observed locator and transport digest.

## Complete independent output

Build with `build --output owned-read-results.lkja` beside the supplied
[grant-free descriptor](owned-read-results.deployment.json). Write `[[2,7,11,3]]`
to `arguments.json` and run with a fresh result path:

```sh
./lkjscript run --deployment owned-read-results.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

The `owned-read-results` target returns this scenario under each of `cell-flat`,
`cell-chunked`, `buffer-flat` and `buffer-chunked`:

```json
{"drained":[3,11,7,2],"empty-length":0,"length":4,"reused":[17],"selected":[11],"tied-selected":[7]}
```

`selected` observes the first maximal value; `tied-selected` compares modulo-four
keys, then observes the selected original value with the identity observer.
Values 7, 11 and 3 have equal greatest key 3, so returning 7 proves stable tie
selection. The complete original storage drains in LIFO order after both read
scopes end. Its empty owner is observed, reused for 17, and drained again.
The consumer maps empty input to `selected:[]` and `tied-selected:[]`; calling
`select-max` itself on empty storage traps through indexed read at zero.

The same artifact exposes individual `read-cell-flat`, `read-cell-chunked`,
`read-buffer-flat` and `read-buffer-chunked` targets. Each supplied descriptor
selects one scenario. Compare complete matched outputs before making performance
claims. The combined witness accepts octet-range values because each ByteBuffer
holds one octet; the cell implementations retain signed I64.

## Acceptance

The copied public executable authors all three packages outside the checkout,
exports generic declarations before readers exist, and retains literal inputs.
It checks canonical draft re-entry and edits the consumer's reuse constant while
preserving function identities, then restores the original value.
It compares complete independent outputs for empty, singleton, observable ties,
31/32/33-item chunk boundaries and 513 inputs across all four implementations.
It removes source projects and staged transports before repeating execution from
the compiled artifact, requiring joined cleanup and zero residual owned handles.

Separate public fixtures return a borrowed root, forward it through another helper,
perform sibling reads and consume the owner after scope exit. Invalid result source
positions, mismatched method contracts, wrong-input returns including aliased calls,
local-owner escape, ordinary call exposure and protected-owner consumption must
reject while preserving accepted HEAD. Kernel, runtime and artifact verification
add independent provenance and transfer-failure checks defined in
[verification](../../docs/spec/verification.md#source-tied-borrowed-result-obligations).
