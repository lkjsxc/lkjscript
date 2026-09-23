# Return a complete collection in bounded pages

The [summary example](native-summary.md) returns a few requested statistics.
When callers need the complete frequency table, an ordinary generic function can
select a page from the sorted entries. This example authors that function, uses
it in a counting command and exports it for another application's record type.
Use a v0.1.43 or later executable for the result-file commands. The
[ranking guide](native-ranking.md) instead orders entries by their frequency
using an ordinary generic library and the v0.1.44 standard window.

## Author the page command

Copy your executable to `./lkjscript` in a new directory. Create a command project
and inspect the standard operations used by the example:

```sh
./lkjscript new ./counts --template command --name counts
./lkjscript --project ./counts status
./lkjscript package builtin query owners --name list-get
./lkjscript package builtin query owners --name list-append
```

Save [the complete native request](examples/paged-counts.lkjc) as `pages.lkjc`.
Replace `BASE` in its first line with the project's current `rev_...`. The request
contains three layers:

- `window<Item>` clamps the start and count to the available list and calls a
  tail-recursive helper. The helper copies the selected values in their original
  order using ordinary `list-get` and `list-append` operations.
- `count-step` folds Text keys into an immutable `Map<Text,I64>`.
- `page` obtains the map's sorted structural entries and selects a window using
  `(types (record (key Text) (value I64)))`.

The window policy is explicit application code. Negative start means zero;
negative count means an empty result; a start beyond the end also returns an empty
result. The requested count is clamped to the remaining length before addition,
so even the largest I64 start/count cannot overflow this index calculation.
No negative-index-from-the-end convention is implied.

v0.1.44 also provides this policy as the ordinary standard function
`std::list-window<Item>`. New programs can call it directly, for example
`(call std::list-window (types Text) (local items) (i64 0) (i64 10))`.
Discover it with `package builtin query owners --name list-window`.
The complete request here retains its own implementation so it also works with
v0.1.43 and demonstrates exporting an ordinary generic library. The standard
function and its maintained consumer have separate [acceptance obligations](../spec/verification.md).

Plan, review and apply with the exact returned `plan_...` token:

```sh
./lkjscript --project ./counts change plan --input-file ./pages.lkjc --output ./pages.logical-plan
./lkjscript --project ./counts change apply --input-file ./pages.lkjc --plan PAGES_PLAN
./lkjscript --project ./counts check
./lkjscript --project ./counts run page --arguments '[["pear","apple","pear"],1,1]'
./lkjscript --project ./counts build --output ./pages.lkja
```

The result is `[{"key":"pear","value":2}]`. The request's native tests cover
ordinary ranges, empty input, both signed I64 extremes and adjacent text-key pages.
Project checking and execution compare the production and reference evaluators.

## Save each page

Save this descriptor as `pages.deployment.json` beside the bundle:

```json
{
  "artifact": "pages.lkja",
  "target": "page",
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

Each input file contains three arguments: the same complete key list, a start
index and a requested count. For example, save
`[["pear","apple","pear"],0,1]` as `first.json` and
`[["pear","apple","pear"],1,1]` as `second.json`:

```sh
./lkjscript run --deployment ./pages.deployment.json --arguments-file ./first.json --result-file ./first-result.json
./lkjscript run --deployment ./pages.deployment.json --arguments-file ./second.json --result-file ./second-result.json
```

The files contain `[{"key":"apple","value":1}]` and
`[{"key":"pear","value":2}]`, respectively. Destinations must be absent in an
existing ordinary directory. The executable, bundle, descriptor and inputs suffice
after removing the authoring project from the runtime directory.

This command recomputes the pure frequency table for every page. Keep the input
list identical when assembling one logical table; changing it can shift page
boundaries. It is an application-level paging contract, not a stored snapshot or
an unbounded input stream.

The JSON limits still apply to every input and output. Three argument members
leave room for 99,997 Text items under the current 100,000-item input bound.
Their byte lengths must also fit within one MiB. Each output entry consumes three
JSON items: one list member and two object fields. Thus at most 33,333 of these
records fit the item count, and long keys may reach the byte limit earlier.
Reducing the requested count is the caller's choice; the result-file option does
not bypass either limit.

## Reuse the generic window

Export the accepted project through `package current export --kind transport`:

```sh
./lkjscript --project ./counts package current export --kind transport --output ./windows.lkjp
./lkjscript new ./batches --template command --name batches
```

The public `window<Item>` function retains its generic contract; its private
recursive helper travels in the code closure. Another project can stage the exact
transport and call `windows::window (types Shipment)` for its own nominal record.

[The complete consumer request](examples/window-consumer.lkjc) creates a `Shipment`
with Text label and I64 mass, imports the window function and supplies its own tests
and command. Save it as `consumer.lkjc`. Use these fields from the export's
`package` record:

| Export field | Placeholder |
| --- | --- |
| `id` | `LIBRARY_PACKAGE` |
| `revision` | `LIBRARY_REVISION` |
| `package-revision` | `LIBRARY_PACKAGE_REVISION` |
| `transport` | `LIBRARY_TRANSPORT` |

```sh
./lkjscript --project ./batches package dependency stage --transport LIBRARY_TRANSPORT --input-file ./windows.lkjp
./lkjscript --project ./batches package dependency query owners --package-revision LIBRARY_PACKAGE_REVISION --name window
./lkjscript --project ./batches status
```

Substitute the three `LIBRARY_...` identities in the request and the consumer's
current `rev_...` for `CONSUMER_BASE`. Review its own plan before applying:

```sh
./lkjscript --project ./batches change plan --input-file ./consumer.lkjc --output ./consumer.logical-plan
./lkjscript --project ./batches change apply --input-file ./consumer.lkjc --plan CONSUMER_PLAN
./lkjscript --project ./batches check
./lkjscript --project ./batches run batch --arguments '[[{"label":"apple","mass":7},{"label":"pear","mass":9},{"label":"猫","mass":11}],1,2]'
./lkjscript --project ./batches build --output ./batches.lkja
```

The result is `[{"label":"pear","mass":9},{"label":"猫","mass":11}]`. The library
does not need to know the consumer's field names or representation. To run this
bundle independently, use the descriptor above with artifact `batches.lkja` and
target `batch`, and supply the same three JSON arguments.

The native requests were exercised with a copied source-accepted executable outside
the compiler checkout. Three result files return all 99,997 independently checked
entries, and a 33,334-entry page rejects before file publication. The separately
authored Shipment consumer checks and runs after its authoring project moves away.
The [continuation](../campaigns/202609222330.md) retains literal inputs, original
outputs, source identities and scope. These are native workflow and library
composition examples, separate from maintained standard-library adoption and
release acceptance.
