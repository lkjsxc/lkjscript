# Serve typed HTML through two ordinary native libraries

The complete [site declaration](examples/html-site.lkjc) composes the separately
exported [typed HTML library](native-html.md) and [HTTP response library](native-http.md).
The public Linux x86-64 **v0.1.44** executable authors, checks, builds and serves it.
No Rust renderer, framework-specific intrinsic, JavaScript server or compiler checkout
is needed by the application. The declaration is a reproducible proposal; accepted
meaning remains the editable authority.

`GET /` selects the first decoded `title` and `body` query values. Missing values use
`Native page` and `Built from two ordinary libraries.`; explicitly empty values stay
empty. The ordinary `page` function constructs an HTML heading and paragraph from
library-owned variants, renders a document, encodes UTF-8 and constructs the existing
structural HTTP response. Its headers are `text/html; charset=utf-8` and
`x-content-type-options: nosniff`. Query text is never accepted as markup.

## Prepare exact library exports

Complete the library-authoring/export steps in [native HTML](native-html.md#create-and-check-the-library)
and [native HTTP](native-http.md#create-the-response-library), using the same executable.
Retain `html.lkjp` and `responses.lkjp` and each export's actual `package` record.
Do not borrow identities from an independently authored attempt.

| Export field | HTML placeholder | Response placeholder |
| --- | --- | --- |
| `id` | `HTML_PACKAGE` | `RESPONSES_PACKAGE` |
| `revision` | `HTML_REVISION` | `RESPONSES_REVISION` |
| `package-revision` | `HTML_PACKAGE_REVISION` | `RESPONSES_PACKAGE_REVISION` |
| `transport` | `HTML_TRANSPORT` | `RESPONSES_TRANSPORT` |

Place the executable, transports and literal `html-site.lkjc` in a fresh work directory.
Set the variables above to the observed export values. The shell below launches the
product and substitutes observed identities only; it does not generate program meaning.

```sh
work="$(pwd)"
LKJ="$work/lkjscript"
"$LKJ" new site --template http --name typed-html-site
cd "$work/site"
"$LKJ" package dependency stage --transport "$HTML_TRANSPORT" --input-file "$work/html.lkjp"
"$LKJ" package dependency stage --transport "$RESPONSES_TRANSPORT" --input-file "$work/responses.lkjp"
"$LKJ" status
```

Set `SITE_BASE` to the site's current `revision id`. Bind the complete literal request,
replacing longer placeholder names before their prefixes:

```sh
sed -e "s/HTML_PACKAGE_REVISION/$HTML_PACKAGE_REVISION/g" \
  -e "s/HTML_PACKAGE/$HTML_PACKAGE/g" \
  -e "s/HTML_REVISION/$HTML_REVISION/g" \
  -e "s/RESPONSES_PACKAGE_REVISION/$RESPONSES_PACKAGE_REVISION/g" \
  -e "s/RESPONSES_PACKAGE/$RESPONSES_PACKAGE/g" \
  -e "s/RESPONSES_REVISION/$RESPONSES_REVISION/g" \
  -e "s/SITE_BASE/$SITE_BASE/g" \
  "$work/html-site.lkjc" > "$work/site-bound.lkjc"
"$LKJ" change plan --input-file "$work/site-bound.lkjc" --output "$work/site.logical-plan"
```

Read the logical plan, then set `SITE_PLAN` to this plan's own `plan token`.
Both exact dependencies are explicit additions, not implied by transport staging.
Apply and check before selecting the resulting bundle:

```sh
"$LKJ" change apply --input-file "$work/site-bound.lkjc" --plan "$SITE_PLAN"
"$LKJ" check
mkdir "$work/runtime"
"$LKJ" build --output "$work/runtime/application.lkja"
cp "$LKJ" "$work/runtime/lkjscript"
cp "$work/site/service.deployment.json" "$work/runtime/service.deployment.json"
```

The recorded check passes **74** tests with production/reference agreement: 45 exact
standard tests, 16 HTML tests, seven response-library tests, one response starter test,
one site starter test and four new composition tests. The new tests cover escaped
response bytes, ordered headers, first/empty query selection and absent-value fallback.
Counts may differ when choosing another standard; these are not 74 newly authored tests.

## Select the deployment and run detached

In the copied `runtime/service.deployment.json`, change only:

| Field | Value |
| --- | --- |
| `artifact` | `"application.lkja"` |
| `target` | `"typed-web"` |

Keep the generated loopback listener `127.0.0.1:0`, runtime/execution/HTTP/stream
objects and the generated `streams` grant with its actual authority revision.
The artifact path is canonical and relative to the descriptor, not an absolute path.

**The stream owner is required even though this handler does not read the body.**
The current HTTP adapter creates request-scoped stream resources and requires one
exact byte-stream grant. The site's component and task signature explicitly declare
`streams` with the `ByteStream.read-all` operation and a maximum of one call. The
handler makes no such call. Removing the declaration and grant rejects before
readiness with `normalized_http_stream_grant`; grant-free resident HTTP is not a
supported mode demonstrated by this example.

```sh
cd "$work/runtime"
./lkjscript serve --deployment service.deployment.json
```

Read the actual `local_address` from the JSON `ready` event. In another terminal,
substitute its port for `ACTUAL_PORT`:

```sh
curl -i --get 'http://127.0.0.1:ACTUAL_PORT/' \
  --data-urlencode 'title=<Notes>' \
  --data-urlencode 'body=日本語 & <script>not markup</script>'
```

The response contains escaped text inside the fixed HTML structure, not a script
element. Stop the foreground service with Ctrl+C. The tested runner handles SIGINT
with a successful joined shutdown; no equivalent SIGTERM behavior is promised.

The runtime directory contains only `lkjscript`, `application.lkja` and the deployment
descriptor. The recorded test moves both fresh authoring roots away from their original
paths before serving. Exact library transports and the compiler checkout are not needed
at that runtime boundary; the bundle still requires its compatible executable.

## Observed acceptance and limits

The independent loopback test completes **22 HTTP requests**: 19 successful HTML
responses and three expected 404 responses, including eight simultaneous clients.
It checks default, repeated and explicitly empty query values, Unicode, entity text,
HTML-looking text, plus decoding, NUL/line endings and a wider query. Successful
responses match exact UTF-8 bytes, content length and headers. An independent parse5
8.0.0 traversal checks the fixed element/attribute structure and normalized text.
NUL and line endings follow parser normalization, not lossless browser round-tripping.

Both server lifecycles, including restart on the same immutable bundle, exit with code
zero after SIGINT. Both report admission stopped, zero queued/active/remaining tasks,
and no cleanup failures. The corrected descriptor's
stream grant is also checked against actual readiness, not inferred from a summary.
The original missing-stream-owner configuration separately rejects before readiness.

Wrong paths, POST and HEAD return 404 in this exact route set; there is no implicit
HEAD-to-GET or automatic 405/Allow policy. The generated resident instruction allowance
remains **10,000,000** and its request deadline remains **30 seconds**. Optional
foreground metering has not been extended to resident HTTP here.

This is native library composition and an executable guide, not a complete framework,
HTML sanitizer, throughput benchmark, public deployment, authentication system or
browser/Wasm client. The [campaign record](../campaigns/202609242248.md) separates
source acceptance, public-binary experiments and corrected harness failures.
