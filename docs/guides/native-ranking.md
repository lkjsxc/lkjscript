# Rank frequency counts with an ordinary generic library

This example returns the most frequent Text keys in descending frequency order.
An ordinary generic merge-sort function accepts the application's comparison
function; standard `list-window` selects the requested result count. Equal
frequencies retain the Map's ascending key order. The same sorting library also
sorts a separate application's own nominal records.

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
./lkjscript --project ./ranking build --output ./ranked.lkja
```

The results are `[{"key":"pear","value":2},{"key":"apple","value":1}]`
and `[-5,-1,0,2,9,9]`. Native tests also cover empty input, equal frequencies,
bounded selection, and comparators that would trap if invoked on empty/singleton input.

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

The body constructs immutable lists for its halves and merged results. The complete
frequency table is sorted before selecting the requested count, including when
that count is zero. Normal preparation, execution and result limits still apply.
The generic function is authored in the request; it is not a standard sort intrinsic.

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
example sorts that many distinct keys and returns 17 entries in 460 bytes. Another
input contains 65,516 rows with 4,096 differently weighted keys; its complete top
17 also matches an independent result. These are bounded correctness observations,
not a latency or memory guarantee. The command still holds and sorts the full table.

## Transport the sorting library

Export the accepted project and stage it in another command project:

```sh
./lkjscript --project ./ranking package current export --kind transport --output ./rankings.lkjp
./lkjscript new ./batches --template command --name batches
./lkjscript --project ./batches package dependency stage --transport LIBRARY_TRANSPORT --input-file ./rankings.lkjp
./lkjscript --project ./batches package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION --name sort-by
./lkjscript --project ./batches status
```

Use the export's `id`, `revision`, `package-revision` and `transport` fields for
`LIBRARY_PACKAGE`, `LIBRARY_REVISION`, `LIBRARY_PACKAGE_REVISION` and
`LIBRARY_TRANSPORT`. [The complete consumer request](examples/ranked-consumer.lkjc)
defines its own private `Shipment` and passes a named `heavier` comparator to
`ranking::sort-by (types Shipment)`. Save it as `consumer.lkjc`, substitute those
identities and replace `CONSUMER_BASE` with the consumer's current revision.

```sh
./lkjscript --project ./batches change plan --input-file ./consumer.lkjc --output ./consumer.logical-plan
./lkjscript --project ./batches change apply --input-file ./consumer.lkjc --plan CONSUMER_PLAN
./lkjscript --project ./batches check
./lkjscript --project ./batches run batch --arguments '[[{"label":"pear","mass":9},{"label":"apple","mass":9},{"label":"猫","mass":11},{"label":"plum","mass":9}]]'
./lkjscript --project ./batches build --output ./batches.lkja
```

The result places 猫 first and preserves the original pear, apple, plum order
among equal masses. Its standalone descriptor uses artifact `batches.lkja`, target
`batch` and the same remaining fields as above. Both authoring projects can then
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

This exact repair also passes with the copied v0.1.43 executable: its kernel can
admit and execute these ordinary library bodies. Choosing newer library meaning
is an explicit graph change; upgrading an executable alone does not perform it.
The observation does not establish compatibility with every older executable.

The [continuation](../campaigns/202609222330.md) retains literal inputs and 39 public
commands: 38 succeed and one is the expected unchanged-HEAD dependency rejection.
Coverage includes detached ranking, all 4,096 numeric sort results, and the
transported nominal consumer on both selected executables. This is an ordinary
native library/workflow example; standard-library adoption and release acceptance
have their own evidence.
