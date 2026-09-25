# A durable browser editor in ordinary lkjscript

The [editor](examples/editor.lkjc) combines the ordinary [UI library](examples/ui.lkjc)
and [strict form codec](examples/form-codec.lkjc) with existing configuration,
secret verification, HTTP and transactional data capabilities. It implements one
shared note with explicit saves, visible conflicts and restart persistence. The
application contains no HTML, CSS or JavaScript source. The UI library renders
HTML/CSS; the browser runs no application script. There is no Node.js server,
external database, app-specific intrinsic or compiler rewrite.

This milestone selects **v0.1.46**; the [release record](../release.md) owns its
actual publication state. The immutable v0.1.45 executable lacks the general byte
operations used by the form codec and correctly rejects this closure. Use a
compatible executable. The development observations below do not certify an
older public binary or an unaccepted release candidate.

## Create the exact libraries

Use a fresh directory on supported Linux x86-64. Put a compatible executable and
these literal files in it: `ui.lkjc`, `ui-tests.lkjc`, `form-codec.lkjc`,
`editor.lkjc`, `editor-tests.lkjc`, and [editor.deployment.json](examples/editor.deployment.json).
Names below are local examples. Shell substitutions bind public metadata only;
they do not generate application meaning. Review each original proposal and
complete logical plan before applying its token.

```sh
work="$(pwd)"
LKJ="$work/lkjscript"
"$LKJ" new ui --template command --name ui
"$LKJ" --project ui status
```

Set `LIBRARY_BASE` to this project's reported `revision id`:

```sh
sed "s/LIBRARY_BASE/$LIBRARY_BASE/g" ui.lkjc > ui-bound.lkjc
"$LKJ" --project ui change plan --input-file ui-bound.lkjc --output ui.plan
# Review; set PLAN to this plan's own token.
"$LKJ" --project ui change apply --input-file ui-bound.lkjc --plan "$PLAN"
"$LKJ" --project ui status
# Set LIBRARY_BASE to the new revision, not its previous value.
sed "s/LIBRARY_BASE/$LIBRARY_BASE/g" ui-tests.lkjc > ui-tests-bound.lkjc
"$LKJ" --project ui change plan --input-file ui-tests-bound.lkjc --output ui-tests.plan
# Review; replace PLAN with this plan's token.
"$LKJ" --project ui change apply --input-file ui-tests-bound.lkjc --plan "$PLAN"
"$LKJ" --project ui check
"$LKJ" --project ui package current export --kind transport --output ui.lkjp
```

Keep the export's `id`, `revision`, `package-revision` and `transport` as
`UI_PACKAGE`, `UI_REVISION`, `UI_PACKAGE_REVISION` and `UI_TRANSPORT`.
Create the independent forms library:

```sh
"$LKJ" new forms --template command --name forms
"$LKJ" --project forms status
# Set LIBRARY_BASE to the forms project's revision id.
sed "s/LIBRARY_BASE/$LIBRARY_BASE/g" form-codec.lkjc > forms-bound.lkjc
"$LKJ" --project forms change plan --input-file forms-bound.lkjc --output forms.plan
# Review; replace PLAN with this plan's token.
"$LKJ" --project forms change apply --input-file forms-bound.lkjc --plan "$PLAN"
"$LKJ" --project forms check
"$LKJ" --project forms package current export --kind transport --output forms.lkjp
```

Keep its corresponding four values as `FORMS_PACKAGE`, `FORMS_REVISION`,
`FORMS_PACKAGE_REVISION` and `FORMS_TRANSPORT`. Fresh projects create fresh
identities. Do not copy someone else's plan token or infer a package revision.

## Author, check and build the editor

```sh
"$LKJ" new editor --template http --name editor
"$LKJ" --project editor package dependency stage \
  --transport "$UI_TRANSPORT" --input-file ui.lkjp
"$LKJ" --project editor package dependency stage \
  --transport "$FORMS_TRANSPORT" --input-file forms.lkjp
"$LKJ" --project editor status
```

Set `EDITOR_BASE` to the editor's current revision. Staging is not selection: the
literal proposal explicitly selects both exact supplier revisions. Bind longer
placeholders first:

```sh
sed -e "s/UI_PACKAGE_REVISION/$UI_PACKAGE_REVISION/g" \
  -e "s/UI_REVISION/$UI_REVISION/g" -e "s/UI_PACKAGE/$UI_PACKAGE/g" \
  -e "s/FORMS_PACKAGE_REVISION/$FORMS_PACKAGE_REVISION/g" \
  -e "s/FORMS_REVISION/$FORMS_REVISION/g" -e "s/FORMS_PACKAGE/$FORMS_PACKAGE/g" \
  -e "s/EDITOR_BASE/$EDITOR_BASE/g" editor.lkjc > editor-bound.lkjc
"$LKJ" --project editor change plan --input-file editor-bound.lkjc --output editor.plan
# Review; replace PLAN with this plan's token.
"$LKJ" --project editor change apply --input-file editor-bound.lkjc --plan "$PLAN"
"$LKJ" --project editor status
# Set EDITOR_BASE to the newly accepted revision.
sed "s/EDITOR_BASE/$EDITOR_BASE/g" editor-tests.lkjc > editor-tests-bound.lkjc
"$LKJ" --project editor change plan --input-file editor-tests-bound.lkjc --output editor-tests.plan
# Review; replace PLAN with this plan's token.
"$LKJ" --project editor change apply --input-file editor-tests-bound.lkjc --plan "$PLAN"
"$LKJ" --project editor check
mkdir runtime
"$LKJ" --project editor build --output "$work/runtime/editor.lkja"
cp "$LKJ" "$work/runtime/lkjscript"
cp editor.deployment.json "$work/runtime/editor.deployment.json"
```

The recorded development checks pass 95 UI, 106 forms and 165 combined editor
tests with production/reference agreement. The editor contributes **38 new pure
tests**; shared standard, starter and supplier tests are not new tests. The UI's
new `action-form`, `hidden-input` and `text-block` are ordinary package definitions.
Exact old imports remain unchanged. A consumer explicitly updating the closed
`ui::node` variant must account for the new cases in any exhaustive matches.

## Configure private local access and run

The supplied descriptor binds loopback `127.0.0.1:8080` and trusts only
`http://127.0.0.1:8080`. Choose an unused port and update **both** settings together.
The origin has no trailing slash, path, query or fragment. `localhost` and
`127.0.0.1` are not interchangeable under this exact policy. Keep the process on a
trusted host; plaintext loopback is the tested starting point, not a public listener.

Initialize a new, owned data directory once:

```sh
cd "$work/runtime"
./lkjscript data initialize --root data
```

The secret is the full expected HTTP Basic Authorization value, not just a password.
Choose a nonempty ASCII username without a colon and a strong private password.
For an interactive Bash session, without putting the literal password in history:

```bash
read -r -p 'Editor username: ' username
read -r -s -p 'Editor password: ' password
printf '\n'
export LKJSCRIPT_EDITOR_AUTHORIZATION="Basic $(printf '%s:%s' "$username" "$password" | base64 | tr -d '\r\n')"
unset password
./lkjscript serve --deployment editor.deployment.json
# After the server stops:
unset LKJSCRIPT_EDITOR_AUTHORIZATION username
```

Do not print or commit that value, put it in a URL, include it in a bundle or send it
to a diagnostic service. For managed operation, supply the variable through the
operator's private process/secret configuration. The descriptor carries the variable
name only. Its example authority revisions are not passwords or authentication.
Missing secret configuration fails startup before readiness.

Open the configured address in a browser on that host and use the chosen username
and password at the browser's authentication prompt. A remote workspace needs a
reviewed local forwarding arrangement, or an operator-owned HTTPS reverse proxy.
For a proxy, configure the exact public origin, preserve its Host and Authorization
and mount the application at `/`. The application does not trust `Forwarded` or
`X-Forwarded-*`, implement prefix rewriting, integrate an identity provider, or
terminate TLS. Do not weaken Origin checks to make an incorrectly configured proxy
work. Basic authentication needs TLS on an untrusted network; see
[RFC 7617, security considerations](https://www.rfc-editor.org/rfc/rfc7617#section-4).

Stop with Ctrl+C. Retain `data/`, the descriptor, bundle and selected executable,
and restart the same command. No reinitialization, authoring project, staged
transport, Cargo, Node.js or Python is needed. Data is not encrypted by this example;
backup, filesystem protection and secret rotation are operator responsibilities.
Closing a tab does not necessarily clear the browser's cached Basic credentials.
There are no per-user accounts, sessions or logout endpoint.

## Save, conflict and failure semantics

GET `/` reads the saved note and never writes, even with action-like query fields.
POST `/` requires authentication and exactly one configured Host and Origin before
reading the body or store. Missing, null, duplicate or foreign Origin rejects with
403. Missing/wrong credentials return 401; Host mismatch returns 400. Unknown
method/path returns 404. The accepted Content-Type spellings are the codec example's
two exact UTF-8 form spellings; missing, duplicate or unsupported values return 415.

A form must contain exactly one `base`, `text` and `intent=save` field. The encoded
body limit is 16384 bytes, enforced by HTTP and the codec. The original decoded note
limit is 4096 UTF-8 bytes, not characters. Tab and line breaks are permitted; other
ASCII controls and DEL reject. CRLF and bare CR normalize to LF. Empty notes and
leading line breaks remain valid. Bounds are checked before normalization.
The UI renderer inserts a separate leading LF after the textarea/pre start tag to
compensate for the [HTML first-newline rule](https://html.spec.whatwg.org/dev/syntax.html#element-restrictions),
preserving a real initial LF.

The client base is canonical decimal text, compared with the saved I64 version;
large 19-digit values do not undergo a potentially overflowing numeric conversion.
The read, comparison and conditional put share one transaction. A successful commit
advances the version once and returns 303 to `/`; refresh then performs a GET.
Only the transaction's committed outcome may report success.

A stale base or concurrent commit returns 409. The response retains the submitted
draft, separately shows the latest saved text, and binds its new hidden base to that
displayed version. The user must compare and explicitly press **Save reviewed draft**.
There is no automatic merge, overwrite, autosave or live-effect retry. If someone
saves again before that review is submitted, it conflicts again.

Malformed or incompatible stored data, invalid stored versions/noncanonical text,
and version exhaustion return 503 rather than silently creating an empty note or
wrapping the version. Operational store, cancellation and resource errors remain
platform errors. A response lost after commit does not prove rollback. Copy the draft
and read the saved version before deciding what to submit again. An invalid-form
error page does not promise to retain malformed or oversized input.

Responses use no-store, nosniff, frame denial and a CSP that admits no scripts and
allows forms only to the same origin. The **Referrer-Policy is `same-origin`**.
`no-referrer` is unsuitable here: the
[Fetch Origin algorithm](https://fetch.spec.whatwg.org/#append-a-request-origin-header)
turns a normal non-CORS form POST's Origin into `null` under that policy. The first
real browser trial reproduced this 403, and the corrected policy passed without
admitting null origins. Header presence alone is not a general security audit.

## Actual evidence and boundaries

The [campaign](../campaigns/202609251211.md) retains exact source, executable and
publication relationships, failed attempts and accepted originals. The maintained
[public CLI workload](../../tests/public_cli/native_editor.rs) authors literal
programs through copied public CLI operations, removes supplier/app projects before
execution, and checks admission, conditional writes, two-process conflicts, an
operational put failure, joined shutdown, source-free restart and fail-closed storage.
It also compiles, checks and builds the unchanged GET UI consumer against the
extended UI supplier, rather than assuming additive definitions preserve every
consumer. Its separate corruption fixture changes only an owned disposable data
store through another native command.

Independent raw HTTP observations complete 36 requests, including 8 concurrent
writers across 2 processes: exactly one save succeeds and seven drafts conflict.
Three lifecycles join with no active/queued/remaining work. Chromium 153.0.8010.12
with page JavaScript disabled separately performs actual authenticated navigation,
four form POSTs, redirects, two-tab conflict/review and refresh. It preserves leading
LF, Japanese text and emoji, makes only same-origin document requests, and has no
horizontal overflow at 360px. Desktop/mobile screenshots and request observations
are retained outside tracked source. Chromium is a test dependency, not an app
runtime. Other browsers and full accessibility are not certified by these results.

This is a small reusable composition example, not a complete framework or rich-text
editor. It has one shared note, bounded text, full-page navigation and shared access.
Multiple documents, user identity, sessions, incremental updates, synchronization,
Wasm and a more compact authoring surface remain separate product decisions.
