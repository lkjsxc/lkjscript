# lkjscript

`lkjscript` is a meaning-oriented programming language and application platform.
The accepted typed meaning graph is the sole editable authority for a program.
Native declaration units and canonical drafts let people and agents propose changes;
reviewed plan/apply validates and publishes their meaning. Stable typed identities
preserve continuity through edits, while names remain mutable locators.

The current public release is [v0.1.44](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.44),
source `9ea93419f1c5e76126548ac5460752fe4cc4511b`. It provides complete native
declaration authoring, canonical `change draft`, exact offline libraries, graph-owned
tests, deterministic artifact bundles, and pure/task command execution through one
copied binary. HTTP applications, interactive WebSocket sessions, deployment-bound
outbound HTTPS, first-party local data and durable workers use the same typed
contracts and explicit deployment grants. Application development needs no Cargo,
compiler checkout or external semantic generator.

Libraries compose explicit type/effect/requirement parameters, finite recursive
nominal data, named callables and retained callbacks. Ordinary standard functions
provide folds, maps, binary64 operations, typed persistence and transaction-completion
reporting. Native user authoring is distinct from native contributor tooling or a
self-hosted compiler; Rust remains the supported kernel and platform boundary.

Public v0.1.41 also provides composable typed-cell updates in caller-owned
transactions, with an explicit guard and a separate standalone completion wrapper.
Public v0.1.43 adds bounded JSON argument/result files for project and standalone
commands, plus measured reductions in repeated catalog reads. Public v0.1.44 adds
the ordinary standard `list-window` function.
See [current status](docs/status.md) for that boundary and [release procedure](docs/release.md)
for verified distributed bytes, retained failures and recovery. Historical campaign
records do not redefine the behavior of the current public binary.

The supported binary target is `x86_64-unknown-linux-musl`, with exact static-binary
admission in pinned Alpine and Debian userlands. Native installation retains
immutable version slots and explicit local selection. Runtime-dependent `.lkja`
bundles need a compatible executable and grants; runtime selection performs no
application-data migration. The plaintext inbound listener and trusted local data
root are not a hostile-code sandbox or encrypted transport/storage boundary.

Start with the binary-only examples below. [Generated guides](docs/generated/operations.md)
and `lkjscript capabilities` expose the complete public surface;
[specifications](docs/spec/) own semantics and [performance evidence](docs/performance.md)
records measured workloads and their limits.

## Download and install

The supported runtime is Linux x86-64, statically linked for `x86_64-unknown-linux-musl`.
Public v0.1.41 includes immutable version slots and an explicit default selection. Anonymous
exact/latest downloads and installed application acceptance are complete; see
[release state](docs/release.md) for the frozen source and delivery evidence.
The latest bootstrap is acquired completely before execution with this single compound invocation:

```sh
(umask 077; installer=$(mktemp) || exit; trap 'rm -f "$installer"' 0; trap 'exit 129' HUP; trap 'exit 130' INT; trap 'exit 143' TERM; curl -q --fail --location --silent --show-error --proto '=https' --proto-redir '=https' --connect-timeout 15 --max-time 180 --max-filesize 16384 --output "$installer" https://github.com/lkjsxc/lkjscript/releases/latest/download/install.sh && sh "$installer")
```

The script needs `sh`, `curl`, `sha256sum`, `tar`, `mktemp`, `chmod`, `wc`, `rm`, and
`uname`. It does not install tools or use sudo. The installed native manager and applications need
none of those acquisition tools. The script installs into absolute `$HOME/.local` by default;
append `--prefix /absolute/owned/path` to its `sh` invocation to choose another prefix.
No shell profile changes. If needed, explicitly add the selected prefix's bin directory to this shell:

```sh
export PATH="$HOME/.local/bin:$PATH"
lkjscript runtime list
```

For download/inspect/run, use the exact immutable URL and review the complete script first:

```sh
curl -q --fail --location --proto '=https' --proto-redir '=https' --connect-timeout 15 --max-time 180 --max-filesize 16384 --output install-v0.1.41.sh https://github.com/lkjsxc/lkjscript/releases/download/v0.1.41/install.sh
cat install-v0.1.41.sh
sh install-v0.1.41.sh --prefix "$HOME/.local"
```

Trust the initial script as executable code from the selected GitHub HTTPS source. Its embedded
digests bind the subsequently downloaded exact archive and executable; they are not an independent
signing authority. Once downloaded, that script always selects its embedded version, even if latest
moves. A missing pinned asset fails without fallback. Reacquiring a newer bootstrap is an explicit
additive update. The archive's `SHA256SUMS` remains a one-line archive checksum; authenticated release
metadata and attestations are separate from anonymous download.

The public native offline boundary takes a local archive and its expected lowercase SHA-256:

```sh
manager="$HOME/.local/lib/lkjscript/versions/v0.1.41/x86_64-unknown-linux-musl/lkjscript"
"$manager" runtime install --archive "$PWD/lkjscript-x86_64-unknown-linux-musl.tar.gz" --sha256 "$archive_sha256" --prefix "$HOME/.local"
"$manager" runtime list --prefix "$HOME/.local"
"$manager" runtime select v0.1.41 --prefix "$HOME/.local"
```

Set `archive_sha256` to the exact archive checksum from the chosen trusted release. Historical
dry-run archives retain their declared, unverified publication status. v0.1.41 supports
publication-neutral content; older managers such as v0.1.38 may reject that encoding,
so use the exact new bootstrap when upgrading. Different archives cannot replace one immutable
tag slot. Installation without `--activate` does not change the default. Inventory
reports full payload integrity as unchecked; selection and exact reinstall fully validate retained
payloads. A corrupted slot is preserved: use a new owned prefix for recovery.

Pinned applications use the emitted versioned absolute executable path. Selecting another default
changes future invocations through `PREFIX/bin/lkjscript`; already running processes and pinned paths
keep their original bytes. After installing and selecting v0.1.32, which has no `runtime` command,
recover through the retained newer manager:

```sh
"$manager" runtime select v0.1.32 --prefix "$HOME/.local"
"$HOME/.local/bin/lkjscript" --version
"$manager" runtime select v0.1.41 --prefix "$HOME/.local"
"$manager" run --deployment /absolute/application/command.deployment.json
```

Alternatively rerun that newer exact bootstrap. If the invoking manager was never installed,
management reports its actual path as external/unmanaged and explicitly reports no retained manager
slot. There is no automatic artifact/data migration, compatible-version search, daemon or updater.
Retain each application's matching executable, bundle and descriptor. See [release procedure](docs/release.md)
and [installation contract](docs/spec/product-surface.md#local-runtime-installation).

## Start from one binary

Use the extracted or installed executable outside the checkout to create a useful command project:

```sh
mkdir -p /tmp/lkjscript-demo
cp "$HOME/.local/bin/lkjscript" /tmp/lkjscript-demo/lkjscript
cd /tmp/lkjscript-demo
./lkjscript capabilities
./lkjscript new ./hello --template command --name hello
./lkjscript --project ./hello status
./lkjscript --project ./hello check
./lkjscript --project ./hello build --output ./hello.lkja
./lkjscript --project ./hello run main
```

The final command returns the typed text value `"hello"`. The command recipe owns one application
module, a private pure implementation, a component and port, target `main`, one graph-owned test,
and an exact dependency on the built-in standard package. The accepted graph owns program meaning;
the separate `command.deployment.json` auxiliary file supplies operator choices. Creation does not
read source assets from the checkout, Cargo, the network, or an external template. Use `--template minimal`
for an empty dependency-free package.

### Foreground bundles from one installed runtime

With a foreground-capable executable, this complete disposable example creates two bundles,
removes their authoring checkouts, and runs each from an unrelated directory:

```sh
lkj_runtime="/absolute/path/to/lkjscript"
foreground_demo=$(mktemp -d)
"$lkj_runtime" capabilities --section deployment
for app in alpha beta; do
  "$lkj_runtime" new "$foreground_demo/author-$app" --template command
  "$lkj_runtime" --project "$foreground_demo/author-$app" check
  mkdir -p "$foreground_demo/$app/generated" "$foreground_demo/cwd-$app"
  "$lkj_runtime" --project "$foreground_demo/author-$app" build \
    --output "$foreground_demo/$app/generated/application.lkja"
  cp "$foreground_demo/author-$app/command.deployment.json" "$foreground_demo/$app/"
  rm -r "$foreground_demo/author-$app"
  (cd "$foreground_demo/cwd-$app" && "$lkj_runtime" run \
    --deployment "$foreground_demo/$app/command.deployment.json")
done
```

Each returns typed `"hello"`, `execution-mode=production`, `verification=not-performed` and
completed cleanup. The starter needs no grants or budget editing. Authors can edit its ordinary
graph into a task with existing change operations. Its descriptor then grants the entire selected
component explicitly, with artifact-relative data roots; two descriptors may independently bind
`alpha/data` and `beta/data` while using the same installed executable. The package acceptance
witness does this with imported task callbacks and persistent execution counters.

The `.lkja` file is a dependency-complete runtime-dependent bundle; it does not embed an executable
runtime. Separate processes have independent invocation/deployment owners. Supplied complete
`execution` objects retain instruction fuel plus cumulative defaults of 256 MiB allocation,
1,000,000 collection items and 100,000 capability calls. Omission removes those lifetime quotas
only on foreground commands. Call depth/value stack, finite value and codec admission, list
representation, adapter limits and exact per-grant quotas remain. SIGINT/SIGTERM cancel and join;
a failure after invocation starts may leave earlier effects visible and is unsafe to retry blindly.
Keep the previous executable and immutable bundles for recovery; no operational data is migrated.

### HTTP application from the public binary

The supported download above exposes this complete workflow from the same copied executable:

```sh
mkdir -p /tmp/lkjscript-http-demo
cp "$HOME/.local/bin/lkjscript" /tmp/lkjscript-http-demo/lkjscript
cd /tmp/lkjscript-http-demo
./lkjscript capabilities new
./lkjscript new ./site --template http --name site
./lkjscript --project ./site status
```

Use the exact revision reported by `status` in the reviewed compact request:

```text
request base=rev_...
expression.static-text as=$response value="changed through the public CLI"
replace.body function=application/response-text body=$response
```

Save those records as `response-change.lkjc`, then use the `plan_...` token returned by the first
command in the second:

```sh
./lkjscript --project ./site change plan --input-file ./response-change.lkjc \
  --output ./response-change.logical-plan
./lkjscript --project ./site change apply --input-file ./response-change.lkjc --plan plan_...
./lkjscript --project ./site check
./lkjscript --project ./site build --output ./site/generated/application.lkja
./lkjscript serve --deployment ./site/service.deployment.json
```

The recipe creates one HTTP target, a graph-owned response function, handler, component, stream
requirement, port, and stable status-code test. It also creates a separate operator-editable
deployment descriptor and empty `generated/` directory before the destination becomes visible; it
does not create an artifact. The descriptor listens on `127.0.0.1:0`, and the ready event reports
the operating-system-selected loopback address. `SIGINT` performs bounded graceful shutdown.

### Nostr relay information from the public binary

The supported download can create the complete closed NIP-11 information slice:

```sh
mkdir -p /tmp/lkjscript-relay-info-demo
cp "$HOME/.local/bin/lkjscript" /tmp/lkjscript-relay-info-demo/lkjscript
cd /tmp/lkjscript-relay-info-demo
./lkjscript capabilities new
./lkjscript new ./relay-info \
  --template nostr-relay-info --name relay-info \
  --relay-url wss://relay.example/nip11
./lkjscript --project ./relay-info status
./lkjscript --project ./relay-info check
./lkjscript --project ./relay-info build \
  --output ./relay-info/generated/application.lkja
./lkjscript serve --deployment ./relay-info/service.deployment.json
```

The recipe normalizes `wss` to the exact `https` information endpoint and keeps that endpoint,
public-only address admission, TLS trust, and transport limits in the deployment descriptor. For
explicit local development it accepts `ws`/`http` only with a lexical loopback destination. Its
inbound `GET /relay-info` performs one HTTP/1.1 GET with
`Accept: application/nostr+json`; a bounded valid status-200 document is preserved byte-for-byte,
while remote status, media-type, and capability failures produce a local redacted 502. It does not
implement WebSocket, NIP-01, event signing, redirect following, retries, proxies, or arbitrary URLs.
See the generated [relay-information guide](docs/generated/nostr-relay-info-authoring.md) and the
normative [outbound client contract](docs/spec/outbound-http-client.md).

### Stateful HTTP and first-party data

The supported download exposes the complete first-party boundary and topology authoring
through one copied candidate's application-facing discovery:

```sh
./lkjscript capabilities data
./lkjscript capabilities change
./lkjscript capabilities --section deployment
./lkjscript package builtin inspect
./lkjscript package builtin query owners --name DataStore
./lkjscript package builtin inspect owner interface decl_...
```

The exact public identity query `./lkjscript --version` prints only `lkjscript 0.1.35`.

Public compact change records can add an exact staged built-in dependency,
components, requirements, function-backed ports, command/HTTP/interactive targets, interfaces,
operations and externals, create task functions, rebind requirements/dependencies, and compose structural records, lexical
bindings, fields, lists, variants, matches, exact built-in calls, requirement-scoped capability
calls, and lexical transactions. The topology records are:

```text
add.dependency package=PKG semantic-revision=REV package-revision=PACKAGE_REVISION
create.component as=$COMPONENT module=MODULE name=NAME visibility=private|package|public
add.port as=$PORT component=COMPONENT name=NAME type=TYPE function=DECLARATION
create.target as=$TARGET name=NAME component=DECLARATION [port=PORT] runner=command|http|interactive
add.http-route as=$ROUTE target=TARGET method=METHOD path=PATH port=PORT
set.http-route route=HTTP_ROUTE method=METHOD path=PATH port=PORT
```

The public binary also exposes the selector-indexed forms:

```text
add.http-route as=$ROUTE target=TARGET method=METHOD pattern="/literal/{capture}" port=PORT
set.http-route route=HTTP_ROUTE method=METHOD pattern="/literal/{capture}" port=PORT
```

Exactly one of `path` or `pattern` is required. A pattern uses 1 through 64 nonempty literal or
whole-capture segments and at most 32 unique captures. Each capture indexes a same-named ordered
unrestricted `Text` handler parameter after `HttpRequest`. Duplicate languages and incomparable
overlap reject; exact and strictly more-specific selectors win without authored priority. Matching
preserves raw segment spelling and performs no percent decoding, normalization, or query selection.

An HTTP target forbids a universal `port`; its graph-owned finite route set returns a fixed empty
404 for an unmatched valid pair.
Command and interactive targets still require one exact port. An interactive port must have the exact structural relation
`(Option<State>, SessionEvent) -> SessionDecision<State>` with one closed ordinary concrete
`State`. The same relation is independently reconstructed during accepted validation, package and
artifact construction/loading, and deployment preparation. `serve` selects either exact HTTP or
interactive topology; project `run TARGET` remains pure-command-only and artifact `run --deployment PATH` supports pure/task Command ports. See the normative
[structured-session contract](docs/spec/structured-sessions.md).

The public vocabulary also includes `add.type-parameter`, `expression.function-value`,
`expression.bind`, and `expression.invoke`. Binding supplies an explicitly ordered prefix;
anonymous bodies and automatic free-variable capture remain unavailable. The generated
[change grammar](docs/generated/change-grammar.md),
[function-definition projection](docs/generated/function-definition.md),
[built-in interface](docs/generated/builtin-standard.md),
[deployment schema](docs/generated/deployment.md), and
[stateful walkthrough](docs/generated/stateful-http-authoring.md), together with the public
[relay-information walkthrough](docs/generated/nostr-relay-info-authoring.md), are the offline
executable-owned authoring references.

The maintained acceptance creates a fresh dependency-free `minimal` project, exports and stages the
exact built-in transport without changing graph authority, then authors its dependency, complete
component/requirement/function-backed-port/target topology, and bounded BBS in one reviewed request
exclusively through those public records. Its pure header reducer is passed as a named function
value to the built-in standard fold. Each post is stored once and one `(created-at, id)` index is
maintained atomically.
The copied candidate builds equal clean/incremental artifacts and runs ordered create/list/update/
delete, stale and strict-input rollback, restart, failed startup, logical backup, absent-root
restore, and semantic-authority checks through one `lkjscript serve` process with no database
server or container:

```sh
cargo run --release --locked -p lkjscript-dev -- stateful-http \
  --binary target/release/lkjscript --machine
```

PostgreSQL 16.15 remains only in `lkjscript-dev data-oracle`. That contributor command uses an exact
immutable image to export bounded neutral BBS and `lkjournal` fixtures and compare facts, public
workflow receipts, and resource samples; it is not a public provider, import path, release
dependency, or application helper.

Operational data lifecycle is explicit and create-new:

```sh
./target/release/lkjscript data initialize --root /tmp/example-data
./target/release/lkjscript data verify --root /tmp/example-data
./target/release/lkjscript data backup --root /tmp/example-data \
  --output /tmp/example-data.lkjd
./target/release/lkjscript data restore --backup /tmp/example-data.lkjd \
  --root /tmp/example-data-restored
```

Restore creates a logically equivalent root with a new physical store identity. These commands do
not inspect or change a program repository, overwrite a destination, repair corruption, import SQL,
or switch deployment policy.

Project creation accepts an absent destination. It rejects invalid names, every existing
destination (including an empty directory), non-directory parents, and symlink path components
before visibility. The
repository is built and synchronized in a private sibling stage, then made visible by one rename.

`check`, `build`, and `run` share exact project discovery, dependency resolution, compilation,
artifact linking/loading, and dense runtime preparation. `check` runs every graph-owned test
through both execution tiers. `build` requires an explicit absent output path and never replaces a
file, directory, or symlink. Project `run TARGET` accepts a pure command target and the strict bounded JSON-array
argument adapter:

```sh
./lkjscript --project ./hello run main --arguments '[]'
```

All three operations identify the exact observed revision and leave accepted `HEAD` unchanged.

## Inspect and change meaning

Global `--project PATH` selects a repository. From within a repository, discovery also walks
ordinary parent directories without following symbolic links:

```sh
./lkjscript --project ./hello query find module application
./lkjscript --project ./hello query owners --limit 20
./lkjscript --project ./hello inspect owner module mod_...
```

Queries read canonical owner bindings and committed namespace/relation witnesses at one revision.
Growing results use bounded pages and revision-bound `qcont_` continuations; query never writes a
cursor, index, or semantic revision.

The supported executable can obtain one complete admitted local neighborhood
without coordinating repeated one-hop reads:

```sh
./lkjscript --project ./hello query context mod_... \
  --direction both --depth 2 --limit 20 --bytes 65536
```

Context owners carry minimum `depth` and precede canonical relation records. Traversal expands only
local owners, while retaining selected package and foreign endpoints as relation boundaries. The
complete neighborhood is admitted before paging; continuations are stateless and bind the exact
repository, package, revision, root, direction, depth, ordering, and resume section/key. Page item
and byte limits may change between requests. `./lkjscript capabilities query` reports the fixed
depth, owner, relation, witness, map, store, decode, continuation, and output bounds.

The supported executable can project one complete accepted local function definition through
stateless pages:

```sh
./lkjscript --project ./hello inspect owner pure_function decl_... \
  --detail definition --limit 20 --bytes 65536
```

Each page repeats the exact repository, package, revision, function, projection contract, complete
digest and counts, and page range. An `icont_` continuation resumes by exclusive logical record key
and permits different item and byte budgets; the executable reconstructs and validates the entire
definition on every request. Named declarations and types remain references rather than recursive
expansion. Dependency bodies, source/raw/JSON aliases, mutable cursors, and projection records used
as `change` input reject. The executable-owned
[definition guide](docs/generated/function-definition.md) reports every record, form, limit,
diagnostic, and containment nonclaim.

Changes are typed semantic intent. For a direct rename, use the exact revision and owner returned
by `status` and `query`:

```sh
./lkjscript --project ./hello change plan rename.owner \
  --base rev_... --owner mod_... --name application-renamed
./lkjscript --project ./hello change apply rename.owner \
  --base rev_... --owner mod_... --name application-renamed --plan plan_...
```

Complete native declarations keep a function's signature, body and children together.
For the `hello` project above, copy its current `revision id` from `status` into
`request base=rev_...`, then author an ordinary library function, test and command:

```sh
cat >squares.lkjc <<'EOF'
request base=rev_...
declarations.begin
(units
  (use std builtin)
  (module create math (as $math)
    (function create square (visibility public)
      (parameter create value (type I64))
      (returns I64) (effect pure)
      (body (call std::multiply (local value) (local value))))
    (function create answer (visibility private)
      (returns I64) (effect pure)
      (body (call square (i64 12))))
    (test create twelve-squared (visibility private)
      (actual (call square (i64 12))) (expected (i64 144)))
    (component create console (visibility private)
      (port create main (type (function () I64)) (function answer))))
  (target create squares (component math::console) (runner command) (port math::console::main)))
declarations.end
EOF
./lkjscript --project ./hello change plan --input-file squares.lkjc \
  --output squares.logical-plan
./lkjscript --project ./hello change apply --input-file squares.lkjc --plan plan_...
./lkjscript --project ./hello check
./lkjscript --project ./hello build --output squares.lkja
./lkjscript --project ./hello run squares
```

Review the plan and replace `plan_...` with its exact token before applying. The
command returns `144`; `check` includes the independent expected-value test. To
re-enter the accepted module later, take its `mod_...` identity from the `$math`
identity row returned by plan/apply and request a canonical editable proposal:

```sh
./lkjscript --project ./hello change draft --owner mod_... --output math-draft.lkjc
./lkjscript --project ./hello change plan --input-file math-draft.lkjc
```

The untouched draft reports `outcome=unchanged`; it requires neither the original
input nor a stored source-text copy. Edit the proposal, review a fresh plan and
apply its exact token to change the accepted program. The
[native authoring grammar](docs/generated/change-grammar.md) covers generic types,
effects, requirements and other declaration kinds.

Flat compact records remain supported for targeted changes. Refresh the base revision
after any accepted edit:

```sh
cat >change.lkjc <<'EOF'
request base=rev_...
create.module as=$notes name=notes
create.record as=$note module=$notes name=Note visibility=public
add.field as=$text record=$note name=text type=text
EOF
./lkjscript --project ./hello change plan --input-file change.lkjc \
  --output ./change.logical-plan
./lkjscript --project ./hello change apply --input-file change.lkjc --plan plan_...
```

Plan and apply share parsing, normalization, allocation, impact analysis, validation, selected
tests, and logical-result construction. The reviewed token binds both the request and its complete
logical semantic effects. The optional plan file is external evidence and is never imported as
authority. Apply reprepares against the exact base under the publication protocol.

After an accepted change, the executable may update an exact base compiler cache while the
prepared publication remains in memory. Cache status is reported separately. Cache failure cannot
roll back or misreport an accepted semantic revision; the next lifecycle command clean-builds and
replaces invalid derived state.

Run focused discovery for exhaustive current grammar, limits, and response fields:

```sh
./lkjscript capabilities change
./lkjscript capabilities query
./lkjscript capabilities check
./lkjscript capabilities build
./lkjscript capabilities run
```

## Offline packages and the standard supplier

Pure graph tail calls use constant control space in both execution tiers, including the standard
fold and named generic recursion across admitted packages. `run` reports each tier's peak call
frames and successful tail transfers under unchanged budgets; task frames retain their ordinary
resource ownership. Discovery and the [generated operation guide](docs/generated/operations.md)
describe the observations.

The public binary includes code-complete offline package composition. A copied executable can
export the current immutable graph and its exact transitive closure, including private bodies;
stage it without changing HEAD; inspect staged public signatures; and review/apply exact dependency
bindings. Check/build/run compile the admitted canonical code without producer directories. Public
release v0.1.30 includes this general package workflow.

```sh
./lkjscript --project ./library package current export --kind transport --output ./library.lkjp
./lkjscript --project ./consumer package dependency stage --transport package_transport_... --input-file ./library.lkjp
./lkjscript --project ./consumer package dependency query owners --package-revision package_revision_...
```

Use the exact exported identities in reviewed `add.dependency` or `replace.dependency` records.
Names do not resolve packages, staged transitive availability does not grant import visibility,
and private-body transport is not source confidentiality. There is no registry, mutable version,
network resolver, or package publication operation.

The [native library walkthrough](docs/guides/native-library.md) creates a generic
aggregation library, imports it into a separate typed shipping command, tests it,
and runs the resulting standalone bundle with literal native authoring inputs.
The [collection summary walkthrough](docs/guides/native-summary.md) composes List
and Map functions to count text keys and return an application-owned summary.
The [paging walkthrough](docs/guides/native-pagination.md) returns a complete
table in bounded files and transports its generic window function to another program.

The executable embeds one exact package transport and one exact artifact bundle generated from
`packages/standard`:

```sh
./lkjscript package builtin inspect
./lkjscript package builtin query owners --kind interface --name HttpClient
./lkjscript package builtin inspect owner interface decl_...
./lkjscript package builtin export --kind transport --output ./standard.lkjp
./lkjscript package builtin export --kind artifact --output ./standard.lkja
```

Both assets are strictly decoded and cross-checked at initialization. Product verification
regenerates their maintained owners and compares the bytes exactly. The built-in is not a general
package registry and never performs ambient path or network resolution.

The public standard interface includes
`list-fold-left<Item, State>(List<Item>, State, Function(State, Item) -> State) -> State`. The fold,
its recursion, and its tests are typed meaning; Rust contributes only the existing generic
compiler/runtime mechanisms.

## Maintained consumers

The standard package and `lkjournal` are typed meaning graph repositories and use the same lifecycle:

```sh
./target/release/lkjscript --project packages/standard check
./target/release/lkjscript --project packages/standard build \
  --output /tmp/standard-current.lkja
./target/release/lkjscript --project applications/lkjournal check
./target/release/lkjscript --project applications/lkjournal build \
  --output /tmp/lkjournal-current.lkja
```

Their checked-in files under `generated/` are deterministic current outputs. The standard artifact
and transport also own the executable's built-in bytes.

This development checkout adopts [a native guide tool](tools/native-guides/README.md).
Its accepted lkjscript program renders all eight reference pages and validates their
required exact references. Rust supplies observed metadata and the pure execution/
publication boundary; no Rust page renderer remains. This tool is not included in
the frozen v0.1.44 public release and is not a self-hosted compiler.
The [native repository-policy tool](tools/native-policy/README.md) also owns the
required no-Python gate's extension and bounded raw-shebang decisions. The two tools
share strict grant-free pure artifact embedding; Rust retains metadata/filesystem
observation and contributor orchestration. This introduces no application dependency.

`serve` and `worker` load the standalone artifact bundle named by their strict deployment
descriptors and prepare the selected target through the same normalized VM used by current graph
execution. The maintained descriptors name `generated/lkjournal.lkja`; a fresh public build must
be byte-equal to that file. Preparation reads the descriptor, its relative regular artifact,
configuration, named secrets, and host resources only. It does not discover or open editable
project authority:

```sh
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
mkdir -p state state/objects
../../target/release/lkjscript data initialize --root state/data
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript serve --deployment live.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

Readiness binds the domain-tagged `artifact_bundle_...` identity after exact target, relational
session shape where applicable, limits, requirement, grant, secret, adapter, and external-authority
preflight. HTTP, interactive, and worker effects execute once through production; only pure
deterministic behavior uses the reference interpreter.

Service and worker use separately validated `data` and `durable_queue_data` grants that share
`state/data`; object bytes remain beneath `state/objects`. The HTTP listener is plaintext and the
data root is unencrypted local trusted-host storage. Encrypted transport or storage requires an
external trusted boundary.

Since v0.1.15 the worker has used affine `QueueLeaseState`. In v0.1.21 its stable
entry claims and matches the live resource, then transfers that lease once into a private
requirement-bound task helper. The helper borrows `QueueLeaseInfo`, consumes through heartbeat,
matches the renewed lease, and consumes through complete or fail. A handle is bound to the exact
worker task scope, resource kind, `DurableQueue` interface, and `jobs` requirement. Dropping it
performs no implicit queue transition, and no application code threads attempt or worker transition
identity.

## Public surface and compatibility

The public capability projection is the exhaustive discovery surface for current operations,
request/response models, grammar, limits, diagnostics, authority effects, and security nonclaims.
It reports the product version and an opaque capabilities digest. See the generated
[operation table](docs/generated/operations.md) and focused capability guides.
Finite outcomes use deterministic bounded compact records and keep stderr empty.

Compatibility is explicit: supported predecessor encodings retain their independent
admission, while unsupported content rejects before mutation or execution. Historical
acceptance does not bypass current validation or grant execution authority. Preserve
old bundles and runtime slots for recovery; installation does not migrate application data.

`change draft` reconstructs editable native declarations, and `package stage` admits
exact code-complete offline closures. Removed top-level `draft`, `history`, `review`,
`backup`, `restore` and `doctor` commands remain absent; `data backup|restore` are
separate operational-data commands. Current spellings and compatibility boundaries
are discoverable from the selected executable.

## Build and verify the repository

Application users need only the executable. Contributors use the repository-owned verification
profiles:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

`stateful_http_application` is a non-cacheable required first-party-data gate in service and full
profiles; the separate stateless `distributed_http_application` gate remains required by product,
service, and full. The non-cacheable `outbound_http_application` gate is required by product,
service, and full and uses only implementation-disjoint local HTTP/TLS relay fixtures. Product,
service, full, target, transferred, and release-candidate verification
need no database server or container. The contributor-only PostgreSQL differential/resource oracle
is a separate required campaign receipt.

The harness records exact fingerprints, classifications, receipts, and bounded child logs under
`.artifacts/lkjscript-dev/check/`. The authoritative `full` profile requires fresh gates.

Normative contracts live under [docs/spec](docs/spec), current facts and limitations in
[docs/status.md](docs/status.md), the dependency map in
[docs/architecture.md](docs/architecture.md), and measurements in
[docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, inbound TLS, outbound privacy/DNSSEC, or
portability beyond its verified environment.
