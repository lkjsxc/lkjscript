# An editable web app from one binary

**Runtime boundary:** development v0.1.47. `lkjscript capabilities new` must
advertise `web`. This is not a retroactive addition to the immutable public
v0.1.46 executable. [Current status](../status.md) owns publication
state; the [campaign](../campaigns/202609251450.md) owns exact evidence.

The `web` starter is an ordinary native application plus editable local UI
modules. It does not require Node.js, a Rust toolchain, downloaded library sources,
manual dependency identities or an application-written HTML/CSS/JavaScript file.
The Rust runtime remains the reference implementation; this is not a Wasm backend.

## Create, check, build and serve

From an owned working directory with the selected binary on `PATH`:

```sh
lkjscript new my-app --template web --name my-app
cd my-app
lkjscript check
lkjscript build --output generated/application.lkja
lkjscript serve --deployment service.deployment.json
```

Use the actual loopback address reported by the `ready` event. The starter uses
`127.0.0.1:0`, allowing the operating system to choose an available port. Stop the
owned foreground process with Ctrl-C. No listener is opened by `new`, `check` or
`build`, and creating a project does not overwrite an existing destination.

The page contains a labeled name field and light/dark submit buttons. A browser
GET rebuilds the page from the URL; no page JavaScript runs and no server state is
saved. Empty names remain empty, repeated query keys select their first value,
and only the exact theme value `dark` selects the dark palette. Unicode and text
that resembles markup remain input data, escaped by the shared renderer.

**Do not enter secrets:** GET form values appear in URLs and may appear in browser
history and request logs. This starter has no accounts, authentication, persistent
saves, file upload, client-side event system or automatic live updates.

## What the project owns

Creation embeds these exact maintained literal sources and admits them through
the same declaration decoder, typed change preparation and publication owners
used by public native requests:

| Source | Created local module | Responsibility |
| --- | --- | --- |
| [ui.lkjc](examples/ui.lkjc) | `ui` | Typed controls, layout, themes, escaping and fixed HTML/CSS rendering. |
| [ui-tests.lkjc](examples/ui-tests.lkjc) | `ui-tests` | The same 19 shared UI tests used by the independent library example. |
| [web-starter.lkjc](examples/web-starter.lkjc) | `web` | Page structure, query interpretation, response headers, one HTTP route and 13 app tests. |

The only package dependency is the selected executable's exact built-in standard.
UI code is **vendored at creation**, not a hidden compiler intrinsic or a separately
pinned supplier. It belongs to this project and can be changed like any other
local declaration. Installing another runtime does not silently update an existing
project's UI. The [independent library workflow](native-library.md) remains available
when separately versioned, explicitly imported packages are appropriate.

Intermediate recipe revisions exist only in an owned private staging repository.
The destination becomes visible as one complete initial project. Rejected native
inputs and synchronous creation failures do not expose an intermediate graph or
partial deployment. The project retains the UI tests rather than weakening
validation to bootstrap the application.

## Edit native declarations, not generated browser code

The accepted graph is the program authority. An original recipe file or an
exported `.lkjc` draft is a proposal, not an automatically synchronized second
source. Inspect the accepted names and export a targeted draft:

```sh
lkjscript query find module web
lkjscript query find declaration page-title --parent MODULE_ID
lkjscript change draft --owner TITLE_ID --output title.lkjc
```

Copy the returned module and declaration identities into the corresponding
commands. In `title.lkjc`, change the text in the `page-title` function body, then:

```sh
lkjscript change plan --input-file title.lkjc
lkjscript change apply --input-file title.lkjc --plan PLAN_TOKEN
lkjscript check
lkjscript build --output generated/application.lkja
lkjscript serve --deployment service.deployment.json
```

Review the plan and use its returned token. Stop the earlier server before
restarting; rebuilding a file does not replace a running process's loaded program.
A changed title preserves the existing declaration identity and its callers. A
body with the wrong return type is rejected without advancing the accepted head.
An unchanged draft plans as unchanged.

To change the layout, draft `web::screen` using the same module/declaration lookup.
Its nodes are ordinary `ui::node` values. To change a reusable control or styling,
draft the relevant function in the local `ui` module. Applications compose typed
nodes; HTML/CSS ownership stays in that library. New controls or framework policy
are not silently injected by the compiler.

## Move a standalone application

Keep one compatible executable and this layout in a separate deployment directory:

```text
lkjscript
service.deployment.json
generated/
  application.lkja
```

The generated descriptor resolves the bundle relative to itself, so it needs no
path edits when the directory moves. Execute the copied binary's `serve` command
with that descriptor. The authoring graph, original native files, compiler checkout
and package transports are unnecessary at runtime. A bundle is not itself a native
executable; it still needs a compatible lkjscript runtime.

The deployment grants only the HTTP request-stream interface. Its listener is
plaintext and loopback-only; it does not acquire host filesystem, outbound network,
secret-verifier or application-data authority. Operational request deadlines and
live resource limits remain independent of optional cumulative execution quotas.
Do not treat this scaffold as a public multi-tenant deployment or a hostile-code
sandbox.

The [durable editor](native-editor.md) separately demonstrates explicit POST,
shared authentication, Origin admission, transactional saves and visible conflicts.
Those policies are not silently inherited by this stateless starter.
