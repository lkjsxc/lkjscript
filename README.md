# lkjscript

`lkjscript` is a meaning-oriented programming language and application platform. One accepted
revision of a typed meaning graph repository is the sole editable authority for a program. Names are
mutable locators; stable typed identities preserve continuity. Source text, compact requests,
indexes, compiler caches, artifacts, deployment descriptors, and runtime handles are projections
or consumers rather than alternate program truths.

The supported v0.1.12 executable provides offline command, editable HTTP, and reviewed stateful HTTP
application lifecycles through one copied binary. They create typed meaning graph projects, inspect
and change accepted meaning, run graph-owned tests, build deterministic artifact bundles, and execute
through the standalone deployment boundary without Cargo, a checkout, or an application helper.
Its stateful workflow uses a deployment-selected first-party local data root and durable queue; no
product or public verification path provisions PostgreSQL.

The current source and immutable v0.1.12 binary include public explicit type-parameter,
named-function-value, and invocation records plus a graph-owned generic `list-fold-left`; the
maintained BBS uses that fold for header admission. Bounded revision-pinned `query context` and the
complete first-party ordered-data cutover are also public. The executable exposes data
initialize/verify/backup/restore, canonical typed application values, and
`data`/`durable_queue_data` deployment adapters while keeping semantic and operational authority
separate. Public product surfaces expose the root product version and opaque capabilities digest
without separate subsystem generation numbers.

The sole current public target is `x86_64-unknown-linux-musl`. Direct ELF inspection found no
runtime interpreter, `DT_NEEDED` library, or GLIBC symbol-version requirement. The exact binary
completed its command lifecycle in pinned Alpine 3.22.5/musl 1.2 and Debian 11/glibc 2.31
userlands, and its distributed and first-party-data HTTP workflows passed independently from both
exact-tag and latest downloads. These observations do not claim a minimum kernel, every x86-64
environment, or broader Linux portability.

## Download

Download the latest supported archive and its checksum without running a remote installer:

```sh
mkdir -p /tmp/lkjscript-download
cd /tmp/lkjscript-download
curl --fail --location --remote-name \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/lkjscript-x86_64-unknown-linux-musl.tar.gz
curl --fail --location --remote-name \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/SHA256SUMS
sha256sum --check SHA256SUMS
tar -xzf lkjscript-x86_64-unknown-linux-musl.tar.gz
./lkjscript/lkjscript capabilities
```

The archive also contains the Apache-2.0 project license, exact third-party notices, and canonical
release metadata. Its stable filename makes the latest URL durable; the
[`v0.1.12` release page](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.12) owns the immutable
version-specific
[archive](https://github.com/lkjsxc/lkjscript/releases/download/v0.1.12/lkjscript-x86_64-unknown-linux-musl.tar.gz)
and [checksum](https://github.com/lkjsxc/lkjscript/releases/download/v0.1.12/SHA256SUMS). See the
[maintainer release procedure](docs/release.md) for identity, packaging, verification, and
recovery details.

Installation is optional. Select a directory you own rather than piping a download into a shell:

```sh
install_dir="$PWD/bin"
mkdir -p "$install_dir"
install -Dm755 ./lkjscript/lkjscript "$install_dir/lkjscript"
"$install_dir/lkjscript" capabilities
```

## Start from one binary

Use the extracted or installed executable outside the checkout to create a useful command project:

```sh
mkdir -p /tmp/lkjscript-demo
cp /tmp/lkjscript-download/lkjscript/lkjscript /tmp/lkjscript-demo/lkjscript
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
and an exact dependency on the built-in standard package. It contains typed meaning authority only
and does not read the checkout, Cargo, the network, or an external template. Use `--template minimal`
for an empty dependency-free package.

### HTTP application from the public binary

The immutable v0.1.12 download above exposes this complete workflow from the same copied executable:

```sh
mkdir -p /tmp/lkjscript-http-demo
cp /tmp/lkjscript-download/lkjscript/lkjscript /tmp/lkjscript-http-demo/lkjscript
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

### Stateful HTTP and first-party data

The immutable v0.1.12 download exposes the completed first-party boundary through the same
application-facing discovery:

```sh
./lkjscript capabilities data
./lkjscript capabilities change
./lkjscript capabilities --section deployment
./lkjscript package builtin inspect
./lkjscript package builtin query owners --name DataStore
./lkjscript package builtin inspect owner interface decl_...
```

The exact standalone identity query `./lkjscript --version` prints only `lkjscript 0.1.12`.

Public compact change records can add exact component requirements, interfaces, operations and
externals, create task functions, rebind requirements/dependencies, update the starter handler
contract, and compose structural records, lexical bindings, fields, lists, variants, matches, exact
built-in calls, requirement-scoped capability calls, and lexical transactions. The public
vocabulary also includes exactly
`add.type-parameter`, `expression.function-value`, and `expression.invoke`; there is no lambda,
capture, partial application, or inference alias. The generated
[change grammar](docs/generated/change-grammar.md),
[built-in interface](docs/generated/builtin-standard.md),
[deployment schema](docs/generated/deployment.md), and
[stateful walkthrough](docs/generated/stateful-http-authoring.md) are the offline executable-owned
authoring references.

The maintained acceptance authors a bounded BBS from a fresh `http` project exclusively through
those public records. Its pure header reducer is passed as a named function value to the built-in
standard fold. Each post is stored once and one `(created-at, id)` index is maintained atomically.
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
file, directory, or symlink. `run` accepts a pure command target and the strict bounded JSON-array
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

The public v0.1.12 executable can obtain one complete admitted local neighborhood
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

Changes are typed semantic intent. For a direct rename, use the exact revision and owner returned
by `status` and `query`:

```sh
./lkjscript --project ./hello change plan rename.owner \
  --base rev_... --owner mod_... --name application-renamed
./lkjscript --project ./hello change apply rename.owner \
  --base rev_... --owner mod_... --name application-renamed --plan plan_...
```

Larger changes use strict compact records:

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

## Built-in standard package

The executable embeds one exact package transport and one exact artifact bundle generated from
`packages/standard`:

```sh
./lkjscript package builtin inspect
./lkjscript package builtin query owners --kind interface --name Database
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
../../target/release/lkjscript worker --deployment worker.deployment.json
```

Readiness binds the domain-tagged `artifact_bundle_...` identity after exact target, requirement,
grant, secret, adapter, and external-authority preflight. HTTP and worker effects execute once
through production; only pure deterministic behavior uses the reference interpreter.

Service and worker use separately validated `data` and `durable_queue_data` grants that share
`state/data`; object bytes remain beneath `state/objects`. The HTTP listener is plaintext and the
data root is unencrypted local trusted-host storage. Encrypted transport or storage requires an
external trusted boundary.

## Public surface and compatibility

The public capability projection is the exhaustive discovery surface for current operations,
request/response models, grammar, limits, diagnostics, authority effects, and security nonclaims.
It reports the product version and an opaque capabilities digest. See the generated
[operation table](docs/generated/operations.md) and focused capability guides.
Finite outcomes use deterministic bounded compact records and keep stderr empty.

Predecessor graph repositories are rejected before mutation or cache work. Removed project
operations such as `draft`, `history`, general package staging, `review`, `backup`, `restore`, and
`doctor` are absent and receive ordinary `cli_usage`; top-level `data backup|restore` are distinct
operational-data commands. There is no compatibility
flag, legacy mode, migration command, graph edition, fallback reader, or dual write.

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
service, and full. Product, service, full, target, transferred, and release-candidate verification
need no database server or container. The contributor-only PostgreSQL differential/resource oracle
is a separate required campaign receipt.

The harness records exact fingerprints, classifications, receipts, and bounded child logs under
`.artifacts/lkjscript-dev/check/`. The authoritative `full` profile requires fresh gates.

Normative contracts live under [docs/spec](docs/spec), current facts and limitations in
[docs/status.md](docs/status.md), the dependency map in
[docs/architecture.md](docs/architecture.md), and measurements in
[docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, or portability beyond its verified
environment.
