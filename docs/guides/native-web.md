# An editable web app from one binary

**Creation boundary:** public v0.1.47. `lkjscript capabilities new` must
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
lkjscript build --deployment service.deployment.json
```

Review the plan and use its returned token. The deployment-build selector requires
development v0.1.48 or later; discover it with `lkjscript capabilities build`.
It removes the manual artifact-name and copied-descriptor edits from repeated builds.
The `output.path` record names the complete content-addressed bundle in `generated/`;
`deployment.path` names a complete descriptor beside `service.deployment.json`.
Copy that returned deployment path into the next command:

```sh
lkjscript serve --deployment DEPLOYMENT_PATH
```

New deployment descriptors are owner-only. Sharing one with another runtime account
requires an explicit access decision; build never broadens its permissions or changes
an existing file's owner or mode.

Building never replaces the original descriptor, deletes a working bundle or changes
a running server. An unchanged rebuild verifies and reuses the same exact pair;
changed code or configuration produces the required new immutable outputs. Stop an
earlier server first when the configured fixed listener cannot be shared. This
starter's port-zero listener also permits independently observed old/new processes.

The descriptor stays beside its template, preserving the meaning of relative
persistent-data paths in stateful applications. Building neither opens nor migrates
that data. The retained original pair can be selected again with its original serve
command, but choosing an old program never rolls back application data. This stateless
example needs no migration; other applications need their own recovery policy.
Older runtimes without this selector still support `build --output` to an unused name
and an explicitly copied/repointed descriptor. That route remains create-new and
rejects even identical existing bytes. Do not delete retained working files simply
to make a build succeed. See the [build contract](../spec/semantic-cli.md#build)
for bounds, exact reuse, failure states and explicit retention.

A changed title preserves the declaration identity and its callers. A body with the wrong return type is rejected without advancing the
accepted head.
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

For `build --deployment`, retain the returned `build-….deployment.json` and its
referenced `generated/build-….lkja` instead of renaming them to the example names.
Move the containing directory as a unit, preserving that relative layout and the
owner-only descriptor permissions. Invocation from a different working directory
does not rebase its artifact or local-data paths. An unchanged rebuild at the new
location reuses the same filenames and bytes; the location is not program identity.

A stateful application's declared data directory, such as `state.lkjdata/`, is a
separate required resource. Moving only configuration and code does not migrate,
restore or initialize it. Stop its writers and follow that application's storage
transfer/recovery procedure before moving data. A same-filesystem, stopped command
workflow tests complete-directory relocation, retained store identity and subsequent
transactions; it does not establish a live cross-filesystem backup or migration
protocol. This web starter remains stateless and creates no data directory.

A bounded independent observation created this template with development v0.1.47,
removed the authoring graph and served its unchanged bundle with the anonymously
acquired public v0.1.46 musl executable. Ordinary GET, Unicode text, escaping and
dark-theme responses passed, followed by joined shutdown. That older executable
still rejects `new --template web`: runtime compatibility does not add a creation
recipe to an already published binary. The [campaign](../campaigns/202609251450.md)
retains the exact binary relationship and does not generalize it to arbitrary
future bundles.

The deployment grants only the HTTP request-stream interface. Its listener is
plaintext and loopback-only; it does not acquire host filesystem, outbound network,
secret-verifier or application-data authority. Operational request deadlines and
live resource limits remain independent of optional cumulative execution quotas.
Do not treat this scaffold as a public multi-tenant deployment or a hostile-code
sandbox.

The [durable editor](native-editor.md) separately demonstrates explicit POST,
shared authentication, Origin admission, transactional saves and visible conflicts.
Those policies are not silently inherited by this stateless starter.
