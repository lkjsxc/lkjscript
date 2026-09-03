# lkjscript

`lkjscript` is a meaning-oriented programming language and application platform. One accepted
revision of a typed meaning graph repository is the sole editable authority for a program. Names are
mutable locators; stable typed identities preserve continuity. Source text, compact requests,
indexes, compiler caches, artifacts, deployment descriptors, and runtime handles are projections
or consumers rather than alternate program truths.

The supported v0.1.16 executable provides offline command, editable inbound HTTP,
deployment-bound outbound HTTPS, and reviewed stateful HTTP application lifecycles through one
copied binary. They create typed meaning graph projects, inspect and change accepted meaning, run
graph-owned tests, build deterministic artifact bundles, and execute through the standalone
deployment boundary without Cargo, a checkout, or an application helper. Its stateful workflow
uses a deployment-selected first-party local data root and durable queue; no product or public
verification path provisions PostgreSQL.

Current source is unreleased product snapshot `0.1.19`; the immutable supported release remains
`v0.1.16`. Both include
public exact built-in dependency, component, requirement, function-backed
port, and command/HTTP target authoring. All four built-in recipes lower through that same typed
authored-operation engine without changing their resulting meaning or atomic project-creation
contract. The release also includes one deployment-bound outbound `HttpClient.get` capability and
a closed `nostr-relay-info` recipe proved against deterministic loopback raw HTTP/TLS/DNS fixtures.
No deployment, live relay, WebSocket, or NIP-01 event flow is claimed here.

Unreleased product 0.1.19 replaces the disposable monolithic object locator with one atomic
manifest over bounded immutable sorted segments. Healthy repository open and accepted sealing no
longer scan every old pack footer or rebuild and rewrite the complete catalog; exact pack entries
and accepted `HEAD` remain canonical, and missing, predecessor, or inconsistent catalog state is
reconstructed under the publication lock. This adds no public operation or authoring path and does
not change graph meaning, pack/object bytes, maintained semantic revisions, deployment, or the
immutable release.

Product 0.1.16 adds deterministic bounded `inspect owner ... --detail definition` pages for one
live local function. They expose its complete accepted contract, structural body, exact reference
cutoff, and revision-bound validation facts without exposing storage or creating a second authoring
format. Immutable v0.1.16 publishes this projection through the same copied-binary workflow.

Product 0.1.17 introduced one canonical exact-requirement binding for a final consume-only
resource parameter on a private same-package acyclic task helper. Direct named calls move one live
resource after ordinary arguments finish; compiler, Artifact 12, preparation, VM, and the
independent reference path recheck the exact requirement/interface and prevent restoration after
failure. Public compact `add.parameter requirement=...`, plan/apply, and definition inspection
expose the complete workflow. The maintained `lkjournal` worker now keeps claim/dispatch in its
stable entry and transfers a live lease once into a graph-authored lifecycle helper. This source
change has not been tagged or released and changes no deployment or durable queue data.

Unreleased product 0.1.18 adds the sole new compact operation `extract.function`. It derives one
private same-module helper from an exact proper expression subtree, preserves every movable owner
identity, infers ordered captures and the least task-requirement closure, and replaces the selected
occurrence with one direct call through ordinary reviewed plan/apply. The maintained `lkjournal`
`update-resource` definition retains its identity while its data-only commit subtree is now owned by
private helper `commit-resource-update`; both resulting definitions are independently inspectable.
The extraction changes no Graph 7, compiler, Artifact 12, runtime, deployment, or operational-data
contract and remains unreleased.

Product 0.1.15 introduced exact-interface affine capability resources. Public compact records expose
`type.capability-resource` and operation-parameter `use=borrow|consume`; validation rejects
fabrication, aliases, foreign authority, branch disagreement, escape, and use after consume before
publication. The maintained standard queue now returns an absent/live resource variant, exposes
metadata only through `lease-info` borrow, and consumes leases through renewal, completion, or
failure. Raw attempt/worker transition tokens are no longer graph or adapter inputs. The queue data
and backup formats remain unchanged. Immutable `v0.1.15` publishes these semantics through the
same copied-binary authoring, build, service, and worker boundaries.

The current source and immutable v0.1.16 binary include public explicit type-parameter,
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
userlands, and its distributed, first-party-data, and outbound HTTPS workflows passed independently
from both exact-tag and latest downloads. These observations do not claim a minimum kernel, every
x86-64 environment, or broader Linux portability.

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
[`v0.1.16` release page](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.16) owns the immutable
version-specific
[archive](https://github.com/lkjsxc/lkjscript/releases/download/v0.1.16/lkjscript-x86_64-unknown-linux-musl.tar.gz)
and [checksum](https://github.com/lkjsxc/lkjscript/releases/download/v0.1.16/SHA256SUMS). See the
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

The immutable v0.1.16 download above exposes this complete workflow from the same copied executable:

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

### Nostr relay information from the public binary

The immutable v0.1.16 download can create the complete closed NIP-11 information slice:

```sh
mkdir -p /tmp/lkjscript-relay-info-demo
cp /tmp/lkjscript-download/lkjscript/lkjscript /tmp/lkjscript-relay-info-demo/lkjscript
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

The immutable v0.1.16 download exposes the complete first-party boundary and topology authoring
through one copied candidate's application-facing discovery:

```sh
./lkjscript capabilities data
./lkjscript capabilities change
./lkjscript capabilities --section deployment
./lkjscript package builtin inspect
./lkjscript package builtin query owners --name DataStore
./lkjscript package builtin inspect owner interface decl_...
```

The exact public identity query `./lkjscript --version` prints only `lkjscript 0.1.16`.

Public compact change records can add an exact staged built-in dependency, components,
requirements, function-backed ports, command/HTTP targets, interfaces, operations and externals,
create task functions, rebind requirements/dependencies, and compose structural records, lexical
bindings, fields, lists, variants, matches, exact built-in calls, requirement-scoped capability
calls, and lexical transactions. The topology records are:

```text
add.dependency package=PKG semantic-revision=REV package-revision=PACKAGE_REVISION
create.component as=$COMPONENT module=MODULE name=NAME visibility=private|package|public
add.port as=$PORT component=COMPONENT name=NAME type=TYPE function=DECLARATION
create.target as=$TARGET name=NAME component=DECLARATION port=PORT runner=command|http
```

The public
vocabulary also includes exactly
`add.type-parameter`, `expression.function-value`, and `expression.invoke`; there is no lambda,
capture, partial application, or inference alias. The generated
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

The public v0.1.16 executable can obtain one complete admitted local neighborhood
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

The public v0.1.16 executable can project one complete accepted local function definition through
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

Since v0.1.15 the worker has used affine `QueueLeaseState`. In current 0.1.19 source its stable
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
