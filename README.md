# lkjscript

Build, inspect, change, test and run typed programs through one executable.
Ordinary application and library development needs no Cargo, compiler checkout
or external semantic generator.

lkjscript is a meaning-oriented language and application platform. Native
declarations propose a program; reviewed changes publish its typed meaning graph.
The graph is the sole editable authority. Stable identities preserve declarations
through edits, while names remain useful, changeable locators. Pure functions,
tasks, exact libraries and standalone application bundles share this model.

**Public:** [v0.1.48](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.48),
with immutable deployment rebuilds, the editable native [web starter](docs/guides/native-web.md),
native authoring, offline libraries, general byte conversion and the facilities used by the ordinary
[form codec](docs/guides/native-forms.md) and [durable browser editor](docs/guides/native-editor.md).
Promotion [36214860067/1](https://github.com/lkjsxc/lkjscript/actions/runs/36214860067) completed
immutable publication and anonymous installed verification using the original accepted bytes.
Older v0.1.45 does not contain the byte operations required by these examples.
[Current status](docs/status.md) separates available releases, development source
and unproved properties.

## Download and install

The supported binary target is Linux x86-64, statically linked for
`x86_64-unknown-linux-musl`. Download and inspect the complete installer before
executing it. The exact URL below remains pinned even when a newer release appears:

```sh
curl -q --fail --location --proto '=https' --proto-redir '=https' \
  --connect-timeout 15 --max-time 180 --max-filesize 16384 \
  --output install-v0.1.48.sh \
  https://github.com/lkjsxc/lkjscript/releases/download/v0.1.48/install.sh
cat install-v0.1.48.sh
sh install-v0.1.48.sh --prefix "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
lkjscript --version
lkjscript runtime list
```

Use a new owned download filename. Proceed to execution only after the download
succeeds and the script is reviewed. The installer needs standard shell/acquisition
tools, including curl, sha256sum and tar; it neither installs tools nor uses sudo.
It does not change shell profiles. The first script is trusted executable code;
its checksums bind downloaded bytes, not an independent signing authority.

Installation retains immutable version slots. Explicit runtime selection changes
future invocations, not running processes or pinned executable paths. Keep each
application's compatible executable, bundle and descriptor. Installation never
migrates application data. See the [installation contract](docs/spec/product-surface.md#local-runtime-installation)
and [release procedure](docs/release.md) for integrity, offline installation,
selection and recovery. Windows/macOS binaries are not currently distributed.

## Start from one binary

In a fresh working directory, create and execute a command application:

```sh
lkjscript new hello --template command --name hello
lkjscript --project hello status
lkjscript --project hello check
lkjscript --project hello run main
lkjscript --project hello build --output hello/generated/application.lkja
lkjscript run --deployment hello/command.deployment.json
```

Both runs return `"hello"`. The project route checks pure execution against the
independent evaluator. The deployment route executes the bundle once, without
opening the authoring graph. Its descriptor resolves the artifact relative to the
descriptor, so keep that layout when moving the application.

The [first native command guide](docs/guides/native-command.md) replaces a sample
with your own square function, an expected-value test and an executable target.
It covers review, canonical re-entry and running after authoring sources are moved
away. No host-language code generator or storage edit is required.

### Foreground bundles from one installed runtime

Command deployments accept pure or task targets under exact grants. JSON argument
and result files support larger bounded values. The artifact is runtime-dependent,
not a self-contained native executable. [Command contracts](docs/spec/semantic-cli.md)
explain execution, typed boundaries, cancellation and post-effect failures.

### HTTP application from the public binary

Use the [HTTP library guide](docs/guides/native-http.md) for routes, query input,
request streams, standalone serving and reviewed edits. [Typed HTML](docs/guides/native-html.md)
and [HTML over HTTP](docs/guides/native-html-http.md) are ordinary composable
libraries and programs, not a compiler-owned web framework. These guides retain
their actual public-v0.1.44 experiments.

The [paged-list guide](docs/guides/native-list.md) adds native bounded
query-number parsing, list windows, exact text/HTML composition and reviewed
function-level editing. Its required standard is now available in v0.1.45.

### Native web starter

The [web starter](docs/guides/native-web.md) reduces initial setup to `new --template web`,
`check`, `build` and `serve`. It includes editable ordinary UI modules and a GET-form
application without manual library imports or application-authored HTML/CSS/JavaScript.
This template has been available since public v0.1.47, not in older frozen releases.
Check the selected executable's `capabilities new` and [current status](docs/status.md).

### Immutable rebuilds in public v0.1.48

After a reviewed edit and `check`, use `build --deployment service.deployment.json`.
It derives a content-addressed bundle and a sibling deployment descriptor, or verifies
and reuses their exact bytes. Run the returned `deployment.path` explicitly. No manual
artifact-name edits, original-file overwrite, process switch or data migration occurs.
The [web editing loop](docs/guides/native-web.md) and [build contract](docs/spec/semantic-cli.md#build)
cover retained old versions, independent data roots and failures. Check the selected
binary's `capabilities build`; this command is not retroactively added to older releases.

### Nostr relay information from the public binary

The [relay-information recipe](docs/generated/nostr-relay-info-authoring.md)
serves a closed information endpoint. It is not a full Nostr relay, event-signing
implementation or outbound WebSocket client.

### Stateful HTTP and first-party data

The [stateful HTTP guide](docs/generated/stateful-http-authoring.md) and maintained
[lkjournal application](applications/lkjournal/README.md) cover local transactions,
interactive subscriptions, durable work and explicit data policy. An independent
capability's side effects are not rolled back by an application-data transaction.

## Inspect and change meaning

```sh
lkjscript capabilities change
lkjscript capabilities query
lkjscript --project hello query find module application
lkjscript --project hello query owners --limit 20
```

`status` and revision-bound queries identify the current program. Author a native
proposal, run `change plan --input-file ...`, review its result, then apply that
same request with its exact plan token. A stale, invalid or cancelled change cannot
partially publish accepted meaning. Names and projections grant no authority.

`change draft` reconstructs editable native declarations from accepted owners;
the original input file is not a synchronized second source. [The command guide](docs/guides/native-command.md)
shows this workflow. [Generated grammar](docs/generated/change-grammar.md) and
[function inspection](docs/generated/function-definition.md) own the complete forms.

## Offline packages and the standard supplier

The [native library guide](docs/guides/native-library.md) creates a reusable library,
exports its complete implementation closure and imports it into a separate typed
consumer. Dependencies are exact; names do not select remote versions or grant
visibility. There is no mutable registry or network package resolver.

Standard libraries provide ordinary folds, maps, composition, binary64 operations
and typed persistence. [Counting](docs/guides/native-summary.md),
[pagination](docs/guides/native-pagination.md), [ranking](docs/guides/native-ranking.md)
and [text composition](docs/guides/native-text.md) demonstrate real programs and
measured tradeoffs. Read each guide's runtime boundary: text-join is not in v0.1.44.

## Maintained consumers

[Standard](packages/standard/README.md) and [lkjournal](applications/lkjournal/README.md)
are maintained meaning-graph packages with deterministic generated assets.
The development [native guide tool](tools/native-guides/README.md) renders all eight
reference pages, and the [native policy tool](tools/native-policy/README.md) owns
no-Python filename/shebang decisions. Rust retains metadata/filesystem observation,
orchestration, the kernel and platform adapters. Native tool adoption does not mean
that the compiler is self-hosted or all development already uses lkjscript.

## Public surface and compatibility

`lkjscript capabilities` is the current executable's exhaustive operation and
contract discovery surface. [Specifications](docs/spec/) own semantics;
[current limitations](docs/status.md#current-limits-and-unproved-properties) and
[the roadmap](docs/roadmap.md) distinguish implemented behavior from proposals.

Resident cumulative quotas, deadlines, cancellation, live limits and exact grants
are separate controls. [The policy guide](docs/guides/resident-policy.md) explains
new nullable quotas and preserved legacy settings. The listener remains plaintext;
safe Rust, static linkage and resource limits are not a hostile-code sandbox,
encrypted storage or multi-tenant isolation.

## Build and verify the repository

Contributors use the pinned Rust toolchain and locked dependencies. Application
users do not need this build. Select the relevant profile rather than running
several overlapping suites:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
```

On a memory-constrained contributor host, set `CARGO_BUILD_JOBS=1` and pass
`--jobs 1` to the checker. These independently bound compiler and gate concurrency;
all 26 gates remain required. Other hosts may select an appropriate higher concurrency.

`changed` reads Git-status paths; a clean-tree invocation does not test a committed
change. `full` requires all 26 gates fresh. [Release acceptance](docs/release.md#coverage-and-admission)
uses its separate source/finalized-target mapping, pinned userlands and installation
proof. Docker is used for the release's pinned userlands, not ordinary application
execution. Evidence is retained under `.artifacts/`; failed runs remain failed.

Read [contributor guidance](AGENTS.md), [architecture](docs/architecture.md),
[verification](docs/spec/verification.md) and [measured performance](docs/performance.md).
Detailed histories belong to their [campaign owners](docs/campaigns/), not this entry page.
