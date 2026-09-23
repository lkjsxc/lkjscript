# Rank frequency counts with an ordinary generic library

This example returns the most frequent Text keys in descending frequency order.
An ordinary generic `select-by` function accepts the application's comparison
function and keeps the requested prefix during merging. Equal frequencies retain
the Map's ascending key order. The library also offers full stable merge sort;
both functions accept a separate application's own nominal records.
The same library also offers `select-buffered-by` for workloads that benefit from
collecting candidate replacements and merging them in batches.

The commands below use the built-in standard from development v0.1.44 or later,
which provides `list-window`. The [summary guide](native-summary.md) covers a single most frequent
key; the [paging guide](native-pagination.md) returns a complete table in key order.

## Author the ranking command

Copy the executable to `./lkjscript` in a fresh directory and create a project:

```sh
./lkjscript new ./ranking --template command --name ranking
./lkjscript package builtin query owners --name list-window
./lkjscript --project ./ranking status
```

Save [the complete native request](examples/ranked-counts.lkjc) as `ranked.lkjc`.
Replace `BASE` in its first line with the project's current `rev_...`.
Review the plan, then apply its exact returned `plan_...` token:

```sh
./lkjscript --project ./ranking change plan --input-file ./ranked.lkjc --output ./ranked.logical-plan
./lkjscript --project ./ranking change apply --input-file ./ranked.lkjc --plan RANKING_PLAN
./lkjscript --project ./ranking check
./lkjscript --project ./ranking run top --arguments '[["pear","apple","pear"],2]'
./lkjscript --project ./ranking run numbers --arguments '[[9,-5,2,9,0,-1]]'
./lkjscript --project ./ranking run select-numbers --arguments '[[9,-5,2,9,0,-1],3]'
./lkjscript --project ./ranking build --output ./ranked.lkja
```

The results are `[{"key":"pear","value":2},{"key":"apple","value":1}]`
and `[-5,-1,0,2,9,9]`; `select-numbers` returns `[-5,-1,0]`.
Native tests also cover empty input, equal frequencies, extreme signed counts,
bounded selection, and comparator non-invocation on empty/singleton input.

You can reconstruct editable declarations from accepted meaning after the original
input is unavailable. Find the ranking module with `query find module ranking`,
then use its `mod_...` identity:

```sh
./lkjscript --project ./ranking change draft --owner RANKING_MODULE --output ./ranking-edit.lkjc
./lkjscript --project ./ranking change plan --input-file ./ranking-edit.lkjc
```

The unchanged draft reports `outcome=unchanged` and creates no publication token.
It preserves the generic contracts, parameter order and private helper references.
Make a deliberate body edit in this draft, then review and apply the resulting plan.

`sort-by<Item>` accepts a `Function(Item,Item)->Bool`. Supply a strict ordering:
the comparator says whether its first argument belongs before its second. Recursive
halves are processed left to right. During merging, the right value is selected
only when it precedes the left value; otherwise the left is retained first. Thus
equivalent values preserve their input order. Empty and singleton lists invoke
no comparator. Callback failures propagate through ordinary pure calls.

`select-by<Item>(items, count, precedes)` returns the first `count` values in the
same stable order for a consistent, non-failing strict weak ordering. Count is clamped to
zero through the input length, including at both signed I64 extremes. It shares
the input through index ranges and keeps at most `count` values from each recursive
result. Its merge stops at that bound. A nonpositive count returns an empty list
without invoking the comparator. Callback calls differ from full sorting; a
callback failure propagates only when that call is reached.

The counting command still builds its complete Map and entry list, even for zero
count. Selection avoids sorting all entries but is not a streaming frequency
counter. Full `sort-by` still constructs immutable halves using standard
`list-window`. Normal preparation, execution and result limits apply. Both
algorithms are native library bodies in the request, using ordinary public calls.

`select-buffered-by` has the same arguments, count clamping and stable-result
contract as `select-by`. It first sorts the requested number of items, then scans
the remainder against the worst retained item. Better candidates accumulate in
a buffer bounded by the requested count. Each full buffer is sorted and merged
with the retained prefix; a final partial buffer is merged at the end. Earlier
retained items win equivalent comparisons against later buffered items. A complete
selection uses the original range sort directly. The full frequency table is
still built, and comparison calls/failures can differ between algorithms.

The `top-buffered` command uses this alternative:

```sh
./lkjscript --project ./ranking run top-buffered --arguments '[["pear","apple","pear"],2]'
```

It returns the same two entries as `top`. For standalone execution, set the
descriptor's `target` to `top-buffered`. The [matched prototype measurements](../performance.md#native-buffered-selection)
show improvements for small prefixes over varied/permuted inputs, but a slowdown
for descending numeric input. Choose using the actual workload; neither function
promises a universal speedup.

## Run the bundle with file input

Save `ranked.deployment.json` beside the bundle:

```json
{
  "artifact": "ranked.lkja",
  "target": "top",
  "listen": null,
  "http": null,
  "session": null,
  "worker": null,
  "streams": {
    "maximum_chunk_bytes": 65536,
    "maximum_buffered_chunks": 8,
    "maximum_total_bytes": 67108864,
    "maximum_live_streams": 1024
  },
  "configuration": {},
  "secrets": [],
  "grants": []
}
```

Each input contains the complete key list and requested result count. For example,
save `[["pear","apple","pear"],1]` in `keys.json`, then run:

```sh
./lkjscript run --deployment ./ranked.deployment.json --arguments-file ./keys.json --result-file ./top.json
```

The new file contains `[{"key":"pear","value":2}]`. An empty list or a
nonpositive count produces `[]`; a larger count is clamped to the number of
distinct keys. File paths and typed JSON retain the
[usual result-file contract](native-library.md#save-a-command-result).
The executable, bundle, descriptor and input suffice after the authoring project
moves away. No graph project or comparison implementation from the compiler
checkout is needed at runtime.

Two argument members leave room for 99,998 Text elements under the current
100,000-item input bound; their encoded bytes must also fit one MiB. The measured
example selects from that many distinct keys and returns 17 entries in 460 bytes. Another
input contains 65,516 rows with 4,096 differently weighted keys; its complete top
17 also matches an independent result. These are bounded correctness observations,
not a latency or memory guarantee. The command still holds the full table.
[Matched measurements](../performance.md#native-bounded-selection-2026-09-23)
compare the earlier full-sort command with selection using identical input and
result bytes, including zero-count and complete-result cases.

## Transport the sorting library

Export the accepted project and stage it in another command project:

```sh
./lkjscript --project ./ranking package current export --kind transport --output ./rankings.lkjp
./lkjscript new ./batches --template command --name batches
./lkjscript --project ./batches package dependency stage --transport LIBRARY_TRANSPORT --input-file ./rankings.lkjp
./lkjscript --project ./batches package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION --name sort-by
./lkjscript --project ./batches package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION --name select-by
./lkjscript --project ./batches package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION --name select-buffered-by
./lkjscript --project ./batches status
```

Use the export's `id`, `revision`, `package-revision` and `transport` fields for
`LIBRARY_PACKAGE`, `LIBRARY_REVISION`, `LIBRARY_PACKAGE_REVISION` and
`LIBRARY_TRANSPORT`. [The complete consumer request](examples/ranked-consumer.lkjc)
defines its own private `Shipment` and passes a named `heavier` comparator to
`ranking::sort-by (types Shipment)` or `ranking::select-by (types Shipment)`.
Save it as `consumer.lkjc`, substitute those
identities and replace `CONSUMER_BASE` with the consumer's current revision.
To choose buffered selection, change its `ranking::select-by` call to
`ranking::select-buffered-by` before planning; its generic argument and parameters
are identical. Both versions accept the consumer's private `Shipment` type.

```sh
./lkjscript --project ./batches change plan --input-file ./consumer.lkjc --output ./consumer.logical-plan
./lkjscript --project ./batches change apply --input-file ./consumer.lkjc --plan CONSUMER_PLAN
./lkjscript --project ./batches check
./lkjscript --project ./batches run batch --arguments '[[{"label":"pear","mass":9},{"label":"apple","mass":9},{"label":"猫","mass":11},{"label":"plum","mass":9}]]'
./lkjscript --project ./batches run selected --arguments '[[{"label":"pear","mass":9},{"label":"apple","mass":9},{"label":"猫","mass":11},{"label":"plum","mass":9}],2]'
./lkjscript --project ./batches build --output ./batches.lkja
```

Full sorting places 猫 first and preserves the original pear, apple, plum order
among equal masses; selection returns 猫 and pear. Its standalone descriptor uses
artifact `batches.lkja`, target `batch` or `selected` and the same remaining fields
as above. Both authoring projects can then
move away without changing execution.

## A consumer with an earlier standard selection

One package closure must select one exact revision of each package. A consumer
created with v0.1.43 initially selects an earlier standard. Merely adding this
library then rejects with `package_revision_closure_package_conflict`, leaving
accepted HEAD unchanged. Staging a transport alone does not upgrade that selection.

The library producer can export its exact standard transport too:

```sh
./lkjscript package builtin export --kind transport --output ./standard.lkjp
```

Stage that transport at the consumer using its exact `STD_TRANSPORT` identity.
In the same proposed change that adds the library, include:

```text
replace.dependency package=STD_PACKAGE semantic-revision=STD_REVISION package-revision=STD_PACKAGE_REVISION
```

Resolve `std` against that explicitly selected package by replacing `(use std builtin)`
with `(use std STD_PACKAGE STD_PACKAGE_REVISION)` in the consumer request. The
export's `id`, `revision` and `package-revision` provide these values. Review the
complete dependency replacement together with the consumer's new declarations,
then apply its own returned plan token and check/build normally.

This repair was exercised with the earlier full-sort library and a copied v0.1.43
executable. Choosing newer library meaning
is an explicit graph change; upgrading an executable alone does not perform it.
The observation does not establish compatibility with every older executable.

The [continuation](../campaigns/202609222330.md) preserves the earlier 39-command
full-sort and dependency-repair observation separately from this selection
successor. Selection passes native tests, all 2,548 small numeric input/count cases,
and five requested counts over 511 consumer-owned nominal records after transport.
These are ordinary native library/workflow examples; standard-library adoption
and release acceptance have their own evidence.
