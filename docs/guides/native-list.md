# Serve a paged list with ordinary native code

The [complete declaration](examples/list-site.lkjc) composes the existing typed
HTML and response libraries with standard `list-window`, `text-join` and
`bytes-get`. This example requires the v0.1.45 development product; public
v0.1.44 does not provide the byte/text operations it uses. See [release status](../status.md)
for the actual distribution state. No runtime or compiler change is introduced
by this example, and its native numeric parser is not a new standard intrinsic.

## Request and output contract

`GET /` receives repeated `item` query values in their decoded order. An omitted
item list is empty; explicitly empty entries remain real list items. `title` uses
its first decoded value, or `Native list` when absent. All displayed title/item
text passes through the existing typed HTML library, never a raw-markup escape hatch.
Unknown query keys are ignored. There is no server-side persistent list or login.

`start` defaults to 0 and `count` to 10. An explicitly supplied value must occur
exactly once and contain 1–19 ASCII decimal bytes. Leading zeroes are accepted
within that byte bound. Empty strings, signs, spaces, exponents, Unicode digits,
more than 19 bytes and values beyond 9,223,372,036,854,775,807 are rejected.
A supplied count must additionally be at most 100; zero is valid. Invalid pagination
returns HTTP 400 with a fixed plain-text explanation, rather than a resource trap.

These are application choices, not universal language or HTTP constraints. The
native parser reads UTF-8 bytes and checks the next decimal multiplication against
`(I64_MAX - digit) / 10` before arithmetic. The query adapter already bounds input;
this is not a hostile-input memory sandbox or arbitrary-precision parser.

The result shows the selected count, total input items, starting index and escaped
list. A start beyond the end gives an empty window, including I64_MAX. There are
no automatic pagination links, form controls, CSS/JavaScript assets or search index.
The example tests server-side composition, not a finished browser UI.

## Author the exact libraries and site

Use the same compatible executable for all three new projects. Follow
[HTML library creation](native-html.md#create-and-check-the-library) and
[response library creation](native-http.md#create-the-response-library). The commands
remain valid with the newer executable; select its observed exact standard rather
than copying package identities from an older run. Retain both exported transports
and their actual package/revision/transport records.

Follow the [two-library staging and binding steps](native-html-http.md#prepare-exact-library-exports),
using `list-site.lkjc` instead of `html-site.lkjc`. The placeholders are identical:
`SITE_BASE` and each library's `*_PACKAGE`, `*_REVISION`, `*_PACKAGE_REVISION`.
Stage both transports, replace longer placeholder names before their prefixes,
review the native plan and apply its exact token. Then:

```sh
"$LKJ" --project "$work/site" check
mkdir "$work/runtime"
"$LKJ" --project "$work/site" build --output "$work/runtime/application.lkja"
cp "$LKJ" "$work/runtime/lkjscript"
cp "$work/site/service.deployment.json" "$work/runtime/service.deployment.json"
```

In the copied deployment change `artifact` to `application.lkja` and `target` to
`paged-web`. Keep the actual generated stream grant, listener and policy objects.
The handler declares the required stream owner but makes no body-read call. The
new generated execution object has four null cumulative quotas; operational
cancellation, deadlines, live limits, codecs and exact grants remain separate.
See [resident policy](resident-policy.md), not old v0.1.44 defaults.

## Run outside the authoring project

```sh
cd "$work/runtime"
./lkjscript serve --deployment service.deployment.json
```

Read the actual loopback port from the `ready` event. From another terminal:

```sh
curl -i --get 'http://127.0.0.1:ACTUAL_PORT/' \
  --data-urlencode 'title=<Reading list>' \
  --data-urlencode 'item=First item' \
  --data-urlencode 'item=日本語 & <script>text</script>' \
  --data-urlencode 'item=' \
  --data-urlencode 'start=1' --data-urlencode 'count=2'
```

This returns two list items, including the empty one. Markup-looking input is
escaped text. POST, HEAD and unknown paths return 404 in this exact route set;
no automatic HEAD-to-GET, 405 or redirect policy is implied. Stop with Ctrl+C.
The observed SIGINT path joins successfully; equivalent SIGTERM behavior is not claimed.

Only the executable, artifact and descriptor remain in the runtime directory.
Authoring roots, transports and literal proposals can be moved away without
changing these runtime paths. Retain them separately for future edits; the bundle
is not an authoring backup or a self-contained machine executable.

## Re-enter one function instead of the whole module

For a small edit, discover the current module and its declaration separately:

```sh
"$LKJ" --project "$work/site" query find module catalog
"$LKJ" --project "$work/site" query find declaration page --parent MODULE
"$LKJ" --project "$work/site" change draft --owner PAGE_FUNCTION --output page-draft.lkjc
```

Replace MODULE and PAGE_FUNCTION with those observed identities. The namespace
class is `declaration`, not the returned owner's `pure_function` kind. Inspect the
current draft and change only the count fallback from 10 to 25, then plan/review/apply
and rebuild through the same public operations. Do not replace every literal 10:
the numeric parser's radix is a different responsibility. This guide's canonical
initial example still defaults to 10; the edited program is a separate observation.

The actual reviewed module edit retained the catalog identity and passed all 98
tests. Old and new detached servers then passed six additional HTTP checks: their
different defaults, identical explicit count=2 and identical rejection of count=101.
Both joined shutdown with no remaining tasks; building the new bundle did not
rewrite the old executable/artifact/descriptor trio.

A matched comparison at the same edited revision prepared the same 25-to-10 change
from a whole-module draft and a function-only draft. The inputs were respectively
18,732 bytes/283 lines and 3,066 bytes/41 lines. Both logical plans were 155,246
bytes/513 records, with the same structural/semantic validation counts and 34 created,
34 retired and one updated owner. Neither comparison plan was applied. Targeted
drafting reduces proposal context here, not the replacement subtree or complete
review artifact. These byte counts are not model-token, timing or monetary estimates.

## Independent observations

The same native declarations pass 98 graph tests with production/reference
agreement: 63 exact-standard, 16 HTML, seven response, two starter and ten new
application tests. This is not 98 new tests. The libraries are freshly authored
through the public executable rather than imported from compiler storage.

A detached Command target `numbers` exercises the same parser for 1,860 inputs,
compared against independent arbitrary-precision integer expectations. Cases include
I64_MAX neighbors, leading zeroes, all single-byte-code-point substitutions at four
positions, invalid syntax and deterministic wider integers. The `-1` command result
marks invalid nonnegative input; it is not a general signed-integer parser.

The loopback observation passes 72 HTTP requests: 61 pagination/content cases,
three expected route/method rejections and eight concurrent clients. Exact status,
UTF-8 body, content length, content type and successful-response nosniff are checked.
After source paths are moved away, SIGINT exits zero with zero active, queued or
remaining tasks. This is bounded composition/recovery evidence, not a throughput,
accessibility, browser-compatibility or security certification.

Two valid but deliberately wrong programs, with a weakened count bound and an
off-by-one I64 limit, each trigger normalized_test_failed. Check stops at the
first failing test; this is not a full failing-test inventory.

The [campaign](../campaigns/202609250650.md) retains the initial syntax rejection,
its correction, exact product source, independent inputs and release boundaries.
