# Durable web app from one executable

`new --template web-editor` creates an editable browser note application from the
executable alone. It uses the same ordinary [editor](examples/editor.lkjc), UI,
form codec and tests as the [independent-package composition](native-editor.md).
No application-authored HTML/CSS/JavaScript, Node.js, database server, downloaded
template or host-language application generator is required. The shared native UI
library owns the rendered HTML/CSS; the browser runs no application script.

This starter was introduced in the **v0.1.49 candidate**, whose publication was
withheld for a shared resident-runtime correction. The additive successor is
**v0.1.50**. Check [release availability](../status.md) and
`lkjscript capabilities new`: the selected executable must list `web-editor`.
Frozen public v0.1.48 does not include this template. The older manual editor
composition remains a separate supported path.

## Create, inspect and build

Copy a compatible executable to a fresh working directory on supported Linux
x86-64. In Bash, with an absent `notes` destination:

```sh
LKJ="$(pwd)/lkjscript"
"$LKJ" new notes --template web-editor --name notes
"$LKJ" --project notes status
"$LKJ" --project notes check
"$LKJ" --project notes build --output notes/generated/application.lkja
```

Creation reports ordered next actions, including explicit data initialization and
operator-supplied secret configuration before serving. It publishes one accepted
initial graph, `service.deployment.json` and an empty `generated/` directory. It
does not build an artifact, initialize operational data, read credentials or open a
listener. `check` compares production and reference evaluation of the graph tests.

The `ui`, `ui-tests`, `forms`, `editor` and `editor-tests` modules are locally
editable. The built-in standard is the only selected external package. This is
creation-time vendoring: installing another executable does not silently update
the accepted application or replace exact library imports in other projects.
The shared editor declaration body has one maintained source; an exact, checked
linkage preamble selects local modules instead of independent package imports.

Review `notes/service.deployment.json`. The initial listener is
`127.0.0.1:8080`, with exact origin `http://127.0.0.1:8080`. Select an unused
port and update both values together when necessary. The target is `editor`.

## Initialize private data and supply a credential

Initialize the new, owned store explicitly, once:

```sh
"$LKJ" data initialize --root notes/notes.lkjdata
```

`notes.lkjdata` is a **directory**, not one serialized file or a new storage
format. Its suffix identifies application data in this starter; old roots keep
their names and formats. The descriptor resolves it relative to its own location.
Do not put it in version control, bundle it as program source, or reinitialize it
to handle an error. Keep the entire directory for restart and backup; no automatic
substitute store is created when it is missing.

The descriptor names `LKJSCRIPT_EDITOR_AUTHORIZATION`, not a credential value.
Supply the complete HTTP Basic Authorization value privately to the server
process. For a trusted local interactive Bash session, choose a nonempty ASCII
username without a colon and a strong private password:

```bash
read -r -p 'Editor username: ' username
read -r -s -p 'Editor password: ' password
printf '\n'
export LKJSCRIPT_EDITOR_AUTHORIZATION="Basic $(printf '%s:%s' "$username" "$password" | base64 | tr -d '\r\n')"
unset password
"$LKJ" serve --deployment notes/service.deployment.json
# After stopping the server:
unset LKJSCRIPT_EDITOR_AUTHORIZATION username
```

Open the configured address on that trusted host and supply the same username
and password at the browser prompt. The encoding command provisions operator
configuration; it does not generate application code and is not a runtime
dependency. Managed services should use private process/secret configuration.
Never put the value in a URL, committed file, artifact or diagnostic output.

The default is loopback, not a public deployment. Remote use requires a reviewed
forwarding arrangement or an operator-owned HTTPS reverse proxy preserving the
configured Host, Origin and Authorization. Basic authentication needs TLS on an
untrusted network. Do not admit arbitrary or null origins to bypass a proxy
configuration error. See the [full operator policy](native-editor.md#configure-private-local-access-and-run).

## Edit the accepted native application

Find the exact module, then its `page` declaration:

```sh
"$LKJ" --project notes query find module editor
# Set MODULE to the returned exact owner id.
"$LKJ" --project notes query find declaration page --parent "$MODULE"
# Set PAGE to this declaration's returned exact owner id.
"$LKJ" --project notes change draft --owner "$PAGE" --output page.lkjc
```

Change a native text literal such as the page title in `page.lkjc`, preserving its
base and owner references. Plan, inspect and apply that proposal's own review:

```sh
"$LKJ" --project notes change plan --input-file page.lkjc --output page.logical-plan
# Review the complete logical plan; set PLAN to its own returned token.
"$LKJ" --project notes change apply --input-file page.lkjc --plan "$PLAN"
"$LKJ" --project notes check
"$LKJ" --project notes build --deployment notes/service.deployment.json
```

This last build publishes an unselected immutable artifact and sibling deployment
descriptor. Use the exact returned deployment path when explicitly starting the
new version; the previous descriptor, artifact and running process remain
unchanged. Do not guess the content-addressed filename. Same-structure scalar
edits preserve existing expression identities; structural changes use ordinary
replacement and review.

Neither build mode saves notes or migrates data. Old and new application snapshots
can use the same configured store and retain its transaction/conflict rules.
Retain the selected executable, descriptor, referenced artifact and whole data
directory with their relative layout for source-free operation. The `notes/`
directory is not source-only after initialization: do not delete it wholesale
when moving or removing authoring files.

## Save, conflict and restart

GET reads without writing. A valid authenticated same-origin POST saves within a
completed transaction and redirects to GET. A stale revision or competing save
returns 409 with the submitted draft and current saved text. Compare them and
explicitly save the reviewed draft; there is no automatic overwrite or merge.
Missing secret configuration or data prevents normal startup. Malformed stored
data and exhausted revisions are not silently replaced.

On the corrected Linux runtime, stop with Ctrl+C (SIGINT), or have the process
owner send SIGTERM to its exact child. Wait for successful process exit and the
`stopped` receipt with no remaining tasks or cleanup failures before replacing
that process. Restart the explicitly selected deployment and the same store
without another initialization. Do not select processes by a broad name match.
The [resident termination contract](../spec/runtime-http-streams.md#process-owned-termination)
and [current availability](../status.md) distinguish corrected source from older
published executables. SIGKILL and a vanished PID do not establish graceful cleanup.

A missing response does not prove that a submitted save rolled back; copy the draft
and read the saved state before deciding to submit again.
The [editor contract and limits](native-editor.md#save-conflict-and-failure-semantics)
cover UTF-8 bounds, line endings, authentication, Origin, failure and corruption.

This is one bounded shared note, not per-user accounts, sessions, multiple
documents, rich text, autosave, a Wasm backend or a complete web framework.
The existing stateless `web` starter remains independent and needs neither a
credential nor a data store. The [campaign](../campaigns/202609261600.md) records
the exact starter tests, failed trials and source-acceptance boundary.
