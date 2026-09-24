# Build a page without authoring browser markup

The [UI library](examples/ui.lkjc), its [native tests](examples/ui-tests.lkjc),
and the separate [application](examples/ui-site.lkjc) run on the published Linux
x86-64 **v0.1.44** executable. The application describes headings, paragraphs,
cards, stacks, labeled inputs, submit buttons and a light/dark theme in ordinary
lkjscript. It contains no HTML, CSS or JavaScript source. The ordinary library
owns the fixed HTML/CSS rendering; the browser still receives HTML and CSS.
No JavaScript is generated or required by this example.

This is an executable library design, not a compiler intrinsic, a new built-in
package, a complete web framework or a browser/Wasm backend. The Rust kernel
is unchanged. Native declarations remain proposals; accepted meaning is the
editable authority. Existing exact package selections are not silently upgraded.

## What the library owns

`ui::theme` and `ui::spacing` are closed variants. `ui::node` composes text,
cards, stacks and query forms. A form contains only `ui::input` and `ui::button`
records, not child nodes, so nested forms are not representable through this API.
There is no raw-markup node, arbitrary attribute, script, URL or CSS-string input.
The private renderer helper cannot be imported by an application.

Every dynamic text/attribute position is escaped using standard
`html-escape-text`. Fixed classes select library-owned layout and palettes;
caller values never enter the stylesheet. Inputs use associated wrapping labels,
forms use fieldsets/legends, and buttons use native submission. Layout wraps at
small widths and exposes keyboard focus. These mechanisms are not a general HTML
sanitizer, validation of arbitrary field names, or an accessibility certification.

`ui::document(language, title, theme, nodes)` returns a complete document with
UTF-8 metadata, viewport, one page heading and the fixed stylesheet. The library's
small private balanced text join preserves public-v0.1.44 compatibility. A future
exact dependency update can adopt standard `text-join`; the pending v0.1.45
publication is not required to use this example.

## Create and export the library

Put `lkjscript`, `ui.lkjc`, `ui-tests.lkjc` and `ui-site.lkjc` in a fresh directory.
The executable must be the supported Linux x86-64 product. Use absolute paths for
the variables below. Shell substitutions bind observed identities only, not meaning.

```sh
work="$(pwd)"
LKJ="$work/lkjscript"
"$LKJ" new ui-library --template http --name native-ui-library
cd "$work/ui-library"
"$LKJ" status
```

Set `LIBRARY_BASE` to the reported `revision id`, then plan the complete literal:

```sh
sed "s/LIBRARY_BASE/$LIBRARY_BASE/g" "$work/ui.lkjc" > "$work/ui-bound.lkjc"
"$LKJ" change plan --input-file "$work/ui-bound.lkjc" --output "$work/ui.logical-plan"
```

Read the logical plan and set `UI_PLAN` to its own `plan token`. Apply, read the
new revision, and bind the tests to that revision, not the library's old base:

```sh
"$LKJ" change apply --input-file "$work/ui-bound.lkjc" --plan "$UI_PLAN"
"$LKJ" status
# Set LIBRARY_BASE to this new revision id.
sed "s/LIBRARY_BASE/$LIBRARY_BASE/g" "$work/ui-tests.lkjc" > "$work/ui-tests-bound.lkjc"
"$LKJ" change plan --input-file "$work/ui-tests-bound.lkjc" --output "$work/ui-tests.logical-plan"
# Review this plan and set UI_TESTS_PLAN to its own plan token.
"$LKJ" change apply --input-file "$work/ui-tests-bound.lkjc" --plan "$UI_TESTS_PLAN"
"$LKJ" check
"$LKJ" package current export --kind transport --output "$work/ui.lkjp"
```

The recorded library check passes **65** tests: 45 selected standard tests, one
HTTP starter test and 19 new UI tests. Keep the export's actual `id`, `revision`,
`package-revision` and `transport` as `UI_PACKAGE`, `UI_REVISION`,
`UI_PACKAGE_REVISION` and `UI_TRANSPORT`. A fresh run creates different identities.

Prepare the existing response library by following
[native HTTP](native-http.md#create-the-response-library), using the same executable
and its [unchanged literal](examples/http-responses.lkjc). Keep `responses.lkjp`
and the corresponding four `RESPONSES_*` values from that export. The UI library
does not duplicate HTTP response construction.

## Author the separate application

```sh
cd "$work"
"$LKJ" new site --template http --name native-ui-site
cd "$work/site"
"$LKJ" package dependency stage --transport "$UI_TRANSPORT" --input-file "$work/ui.lkjp"
"$LKJ" package dependency stage --transport "$RESPONSES_TRANSPORT" --input-file "$work/responses.lkjp"
"$LKJ" status
```

Set `SITE_BASE` to this site's current revision. Staging transports is not dependency
selection; the literal explicitly adds both exact dependencies. Bind longer names first:

```sh
sed -e "s/UI_PACKAGE_REVISION/$UI_PACKAGE_REVISION/g" \
  -e "s/UI_PACKAGE/$UI_PACKAGE/g" -e "s/UI_REVISION/$UI_REVISION/g" \
  -e "s/RESPONSES_PACKAGE_REVISION/$RESPONSES_PACKAGE_REVISION/g" \
  -e "s/RESPONSES_PACKAGE/$RESPONSES_PACKAGE/g" \
  -e "s/RESPONSES_REVISION/$RESPONSES_REVISION/g" \
  -e "s/SITE_BASE/$SITE_BASE/g" "$work/ui-site.lkjc" > "$work/ui-site-bound.lkjc"
"$LKJ" change plan --input-file "$work/ui-site-bound.lkjc" --output "$work/site.logical-plan"
# Review the plan and set SITE_PLAN to its own plan token.
"$LKJ" change apply --input-file "$work/ui-site-bound.lkjc" --plan "$SITE_PLAN"
"$LKJ" check
mkdir "$work/runtime"
"$LKJ" build --output "$work/runtime/application.lkja"
cp "$LKJ" "$work/runtime/lkjscript"
cp "$work/site/service.deployment.json" "$work/runtime/service.deployment.json"
```

The site check passes **83** graph-owned tests with production/reference agreement:
45 shared standard tests, seven response tests, three starter tests, 19 UI tests
and nine application tests. There are **28 newly authored tests**, not 83 new tests.

In the copied descriptor change only `artifact` to `"application.lkja"` and
`target` to `"native-ui"`. Preserve the generated loopback listener `127.0.0.1:0`
and exact `streams` grant. As in [HTML over HTTP](native-html-http.md), the HTTP
adapter requires that request-stream owner even though this handler never reads it.

```sh
cd "$work/runtime"
./lkjscript serve --deployment service.deployment.json
```

Read `local_address` from the `ready` event and open that address in a browser on
the same host. A remote host needs an appropriate local forwarding/access setup;
the example does not open a public listener. Stop with Ctrl+C. SIGINT shutdown and
restart of the same detached bundle are tested; equivalent SIGTERM behavior is not
asserted. The runtime needs only the executable, bundle and descriptor, not Cargo,
Node.js, Python, the library transports or the authoring projects.

## Query behavior and boundaries

The GET form submits to the current page and requests a full-page response. The
handler reads the first decoded `name`, `note` and `theme`; explicitly empty values
remain empty. Missing name/note use the visible defaults. Only the exact text
`dark` chooses the dark palette; unknown/empty/case-changed themes choose light.
Wrong paths, POST and HEAD return 404 in this explicit route set.

Inputs appear in the URL. **Do not enter secrets.** This is stateless personalization,
not saved server data, an account, a mutation action or a session. Reloading a URL
reconstructs that page. The response carries no-store, no-referrer, nosniff and a
restrictive CSP; inline style is allowed for the fixed library stylesheet, scripts
are not. These headers do not turn GET into a private or effectful action channel.

HTML parsing normalizes text, and single-line inputs remove line breaks. The
recorded Chromium observations drop NUL in body text and replace it in input
attributes; the renderer's raw UTF-8 output and browser form values are different
boundaries. Arbitrary control characters are not promised a lossless form round trip.

The v0.1.44 descriptor retains its generated 10,000,000 instruction allowance and
30-second request deadline. The newer nullable resident policy is not retroactively
applied. The inbound listener is plaintext; this is not a public production deployment.

## Observed verification

The [campaign](../campaigns/202609250807.md) owns the exact source relationship and
original evidence. Final literals were freshly authored, imported, checked and built
through copied public v0.1.44, without a compiler checkout. Six malformed proposals
reject with an unchanged revision; two deliberately unescaped implementations fail
the corresponding native tests.

Independent HTTP checks complete **52 requests** across two detached lifecycles,
including eight simultaneous clients, hostile strings, Unicode, repeated/empty
parameters, expected 404s and an exact-bundle restart. Both shutdown receipts show
zero active/queued/remaining work and no cleanup failures.

Chromium 144.0.7559.96 separately renders captured response bytes **offline**, with
page JavaScript disabled and zero browser network requests. At widths 320, 390 and
1280 it passes structure, associated labels, tab order/focus, native
`FormData(form, submitter)`, both palettes, control sizes and overflow checks.
Four captured hostile responses also pass independent parsing/normalization checks.
Measured normal-text contrast exceeds 4.5:1 for the tested palette pairs; this is
not a complete accessibility audit.

**Live browser navigation was denied by administrator policy.** HTTP behavior and
offline browser behavior are verified separately; click-to-network-to-render
end-to-end operation is not claimed. The denial was not bypassed. The repository's
Rust `check full --fresh` suite was not run for these example-only additions, and
its earlier source evidence is not relabeled as proof for this campaign.
