# Summarize a collection with ordinary lists and maps

This walkthrough builds a text-frequency command using ordinary standard functions
and a record defined by the application. It returns the number of input items,
number of distinct keys, most frequent key and that key's frequency. The internal
Map can be larger than a complete JSON Map result because the command returns
only the summary its caller requested.

Use a compatible lkjscript executable in a new directory. File arguments are
available in development v0.1.42 and are included in the corrected v0.1.43 delivery.
The earlier [native library walkthrough](native-library.md) explains exact package
transport and dependency selection; this example uses the recipe's standard library.

## Author the command

```sh
./lkjscript new ./counts --template command --name counts
./lkjscript --project ./counts status
./lkjscript package builtin query owners --name map-entries
```

Save the following complete request as `summary.lkjc`, replacing `SUMMARY_BASE`
with the `rev_...` from the project's status:

```text
request base=SUMMARY_BASE
declarations.begin
(units
  (use std builtin)
  (module create counts
    (record create Summary (visibility public)
      (field create items (type I64))
      (field create distinct (type I64))
      (field create mode (type Text))
      (field create frequency (type I64)))
    (function create count-step (visibility private)
      (parameter create totals (type (map Text I64)))
      (parameter create item (type Text))
      (returns (map Text I64)) (effect pure)
      (body (call std::map-insert (types Text I64) (local totals) (local item)
        (call std::add
          (call std::map-get-or (types Text I64) (local totals) (local item) (i64 0))
          (i64 1)))))
    (function create frequencies (visibility private)
      (parameter create items (type (list Text)))
      (returns (map Text I64)) (effect pure)
      (body (call std::list-fold-left (types Text (map Text I64))
        (local items) (map Text I64) (function-value count-step))))
    (function create summarize-step (visibility private)
      (parameter create state (type Summary))
      (parameter create entry (type (record (key Text) (value I64))))
      (returns Summary) (effect pure)
      (body
        (record Summary
          (field Summary::items
            (call std::add (field (local state) Summary::items) (field (local entry) (name value))))
          (field Summary::distinct
            (call std::add (field (local state) Summary::distinct) (i64 1)))
          (field Summary::mode
            (if (call std::less (field (local state) Summary::frequency) (field (local entry) (name value)))
              (field (local entry) (name key))
              (field (local state) Summary::mode)))
          (field Summary::frequency
            (if (call std::less (field (local state) Summary::frequency) (field (local entry) (name value)))
              (field (local entry) (name value))
              (field (local state) Summary::frequency))))))
    (function create summarize (visibility public)
      (parameter create items (type (list Text)))
      (returns Summary) (effect pure)
      (body
        (call std::list-fold-left (types (record (key Text) (value I64)) Summary)
          (call std::map-entries (types Text I64) (call frequencies (local items)))
          (record Summary (field Summary::items (i64 0)) (field Summary::distinct (i64 0))
            (field Summary::mode (text "")) (field Summary::frequency (i64 0)))
          (function-value summarize-step))))
    (test create empty (visibility private)
      (actual (call summarize (list Text)))
      (expected (record Summary (field Summary::items (i64 0)) (field Summary::distinct (i64 0))
        (field Summary::mode (text "")) (field Summary::frequency (i64 0)))))
    (test create repeated (visibility private)
      (actual (call summarize (list Text (text "pear") (text "apple") (text "pear"))))
      (expected (record Summary (field Summary::items (i64 3)) (field Summary::distinct (i64 2))
        (field Summary::mode (text "pear")) (field Summary::frequency (i64 2)))))
    (test create tied (visibility private)
      (actual (call summarize (list Text (text "pear") (text "apple"))))
      (expected (record Summary (field Summary::items (i64 2)) (field Summary::distinct (i64 2))
        (field Summary::mode (text "apple")) (field Summary::frequency (i64 1)))))
    (test create exact-text (visibility private)
      (actual (call summarize (list Text (text "") (text "猫") (text "é") (text "猫") (text "é"))))
      (expected (record Summary (field Summary::items (i64 5)) (field Summary::distinct (i64 4))
        (field Summary::mode (text "猫")) (field Summary::frequency (i64 2)))))
    (component create console (visibility private)
      (port create summary (type (function ((list Text)) Summary)) (function summarize))))
  (target create summary (component counts::console) (runner command) (port counts::console::summary)))
declarations.end
```

The first fold updates an immutable `Map<Text,I64>`; each repeated key increments
its existing count. `map-entries` then returns ordinary structural records with
`key` and `value` fields in ascending key order. The second fold constructs a
nominal `Summary`. It replaces the mode only for a strictly larger count, so ties
select the first key in the map's order. Exact Text identity distinguishes composed
and decomposed Unicode spellings. Integer additions retain their checked behavior.

Review and apply this request's exact `plan_...` token:

```sh
./lkjscript --project ./counts change plan --input-file ./summary.lkjc --output ./summary.logical-plan
./lkjscript --project ./counts change apply --input-file ./summary.lkjc --plan SUMMARY_PLAN
./lkjscript --project ./counts check
./lkjscript --project ./counts run summary --arguments '[["pear","apple","pear"]]'
./lkjscript --project ./counts run summary --arguments '[[]]'
```

The first result is `{"distinct":2,"frequency":2,"items":3,"mode":"pear"}`.
Empty input returns zero items, distinct keys and frequency, with an empty mode.
A real empty-string key can also be the mode; a positive frequency distinguishes
that result from empty input. The native tests cover repetition, ties and exact
Unicode keys. Project checking and execution compare the production and reference
evaluators.

## Run without the authoring project

```sh
./lkjscript --project ./counts build --output ./summary.lkja
```

Save `summary.deployment.json` beside the bundle:

```json
{
  "artifact": "summary.lkja",
  "target": "summary",
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

Save `[["pear","apple","pear"]]` as `items.json`, then run:

```sh
./lkjscript run --deployment ./summary.deployment.json --arguments-file ./items.json
```

Copy the executable, bundle, descriptor and input together to run elsewhere. The
bundle includes its standard dependency and needs no authoring project at runtime.
The descriptor grants no effects; the command is pure. Detached execution runs
production once and reports that differential verification was not performed.

The input remains bounded JSON: 1 MiB including whitespace and 100,000 array
members/object fields, including the outer argument array. For one list of Text,
99,999 strings fit the item count; their actual lengths must also fit the byte
bound. A 100,000-element list exceeds the item count before invocation. Intermediate
Maps stay subject to the runtime's actual structural and value limits. This
program does not stream an unbounded collection, and its summary does not return
the complete frequency table. To return bounded complete values, use the
[result-file interface](native-library.md#save-a-command-result).
For a complete frequency table larger than one result, the
[paging example](native-pagination.md) uses an ordinary generic window function
and shows its reuse with an application-owned record type.

The literal program and commands were exercised through a copied executable
outside the compiler checkout. The [continuation record](../campaigns/202609222330.md)
keeps exact source, inputs, outcomes and evidence scope. This is a public native
workflow example, separate from maintained library adoption and release acceptance.
