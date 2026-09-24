# Compose text with an ordinary standard function

The development standard exports the pure graph-owned function
`text-join(items: List<Text>, separator: Text) -> Text`. This is development source
after the frozen public v0.1.44 release; that published executable does not export
this function. Use a compatible executable embedding the new standard, or explicitly
stage and select its exact transport with a compatible runtime.

Joining preserves the items' order and exact text. Empty input returns empty text;
a singleton returns its item without a separator. Empty items still occupy positions:
joining `["", "cat", ""]` with `":"` returns `":cat:"`. No separator is added before
the first item or after the last. An empty separator concatenates directly. There
is no escaping, quoting, whitespace trimming or Unicode normalization. Formatting
and trust policy belong to the caller, not this general-purpose function.

The body and its private balanced-range helper are ordinary native-authored meaning,
not a Rust join implementation or new intrinsic. The helper prefixes the separator
at each nonfirst leaf, then combines balanced halves. For an empty separator it
retains the leaf text directly. This avoids a growing-prefix left fold and an extra
right-subtree copy at every internal boundary. It still allocates concatenated text;
it is not a rope, zero-copy builder or streaming output API. Indexing has its existing
persistent-list cost; recursive control depth grows logarithmically with item count.
Normal admission, output and execution limits remain in force.

## Author and run the example

The [literal example](examples/text-join.lkjc) provides three ordinary Command targets:
`join`, `batch`, and `escaped`. The last explicitly maps `html-escape-text` over its
input before joining with an application-owned `<br>` separator. It is not an HTML
parser, attribute/URL sanitizer or general template engine.

```sh
lkjscript new text-app --template command --name text-app
lkjscript --project text-app status
```

Copy the literal example to a working file and replace `BASE` with the reported
current semantic revision. The command recipe already selects the executable's
exact standard supplier. Plan, review and apply the authored declarations:

```sh
lkjscript --project text-app change plan --input-file text-join.lkjc --output text.logical-plan
lkjscript --project text-app change apply --input-file text-join.lkjc --plan TOKEN
lkjscript --project text-app check
lkjscript --project text-app run join --arguments '[["red","green","blue"]," / "]'
lkjscript --project text-app run escaped --arguments '[["<tag>","&"]]'
lkjscript --project text-app run batch --arguments '[[{"items":["","猫",""],"separator":":"},{"items":[],"separator":"unused"}]]'
```

Replace `TOKEN` with the actual plan token. The three results are respectively
`"red / green / blue"`, `"&lt;tag&gt;<br>&amp;"`, and `[":猫:", ""]`.
The example adds two tests to the command recipe and exact standard tests.

Larger JSON input uses `--arguments-file INPUT.json`, mutually exclusive with
`--arguments`. Use `--result-file OUTPUT.json` for complete bounded results larger
than the compact display record. The output destination must not exist; input and
output limits still apply. Wrong element types, a non-Text separator or an incorrect
argument count reject rather than coercing values.

## Distribute without the authoring project

```sh
lkjscript --project text-app build --output text.lkja
```

Place this ordinary deployment descriptor alongside `text.lkja`:

```json
{
  "artifact": "text.lkja",
  "target": "join",
  "listen": null,
  "http": null,
  "session": null,
  "worker": null,
  "streams": {
    "maximum_chunk_bytes": 65536,
    "maximum_buffered_chunks": 8,
    "maximum_total_bytes": 1048576,
    "maximum_live_streams": 1024
  },
  "grants": [],
  "secrets": [],
  "configuration": {}
}
```

Then run:

```sh
lkjscript run --deployment text.deployment.json \
  --arguments-file INPUT.json --result-file OUTPUT.json
```

Only the compatible executable, artifact, descriptor and
ordinary input remain necessary; the authoring project and native request are not
runtime dependencies. Existing applications keep their old exact package selections
unless explicitly upgraded. No application-data or artifact-format migration is added.
