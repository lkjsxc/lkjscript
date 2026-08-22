# lkjscript

`lkjscript` is a meaning-oriented programming language and capability-oriented application
platform. The canonical authority for every accepted program is one revisioned typed meaning
graph under `lkjscript-meaning-graph-4`. Names, review text, indexes, artifacts, bytecode, runtime
handles, and deployment bindings are projections or consumers; none is a second editable program
truth. The physical root is a fixed manifest over six immutable path-compressed Merkle maps, while
the complete logical graph remains the reconstruction and validation oracle.

One released executable is enough for ordinary offline application development. In an empty
working directory, an agent can discover the platform, create a project, inspect and change its
graph, run graph-owned tests, build an artifact, run a target, back up the authority, and restore
it. This path needs no repository checkout, Cargo, Rust toolchain, network registry, or external
bootstrap artifact.

The verified bootstrap is stable Rust 2024 on Linux x86-64.

## Start from one binary

Discover the exact CLI v4 contract and create a command application:

```sh
lkjscript capabilities
lkjscript capabilities new
lkjscript new ./hello --template command --name hello
lkjscript --project ./hello inspect project --limit 20
lkjscript --project ./hello check
lkjscript --project ./hello run main
lkjscript --project ./hello build --output ./hello.lkja
```

`new` accepts an absent or empty ordinary directory. It rejects nonempty destinations and symlink
components, constructs the complete repository in a private sibling stage, and makes it visible
with one rename after durable publication. The `minimal` template creates one empty package; the
`command` template binds the exact embedded standard package and creates an ordinary graph-owned
function, component, port, test, and command target. It prints the new repository, package,
revision, optional built-in dependency, and allocated stable identities.

The embedded standard package is inspectable and exportable:

```sh
lkjscript package builtin inspect
lkjscript package builtin export --output ./standard.lkja
```

Its bytes are integrity checked as an ordinary graph-native artifact and reproduced from the
maintained standard package during repository verification.

## Inspect and change meaning

Global `--project PATH` selects a project explicitly; from inside a project, discovery also walks
ordinary parent directories. Bounded direct commands replace the former universal namespace:

```sh
lkjscript --project ./hello inspect status
lkjscript --project ./hello inspect targets --limit 20
lkjscript --project ./hello query find main --exact --limit 10
lkjscript --project ./hello query context --seed DECLARATION_ID --depth 4
```

Every finite command emits one strict CLI v4 JSON value. Growing reads accept item, byte, work,
depth, fanout, revision, and continuation controls. Project-bound results identify the exact
observed revision where applicable, and every response stays below the 4 MiB hard response bound.

One `change` request may create connected meaning with request-local symbols. The symbols are
defined before use, checked in their identity domain, and returned as stable IDs in the result;
there is no separate preallocation step. For example:

```sh
lkjscript --project ./hello change --dry-run --request \
  '{"contract_version":3,"changes":[{"change":"create_module","as":"$notes","name":"notes"},{"change":"create_record","as":"$note","module":"$notes","name":"Note","fields":[{"name":"text","type":{"type":"text"}}],"exported":true}]}'
```

Use `--commit` on the same normalized request to publish one accepted revision. Omitting
`base_revision` binds the request to the observed current revision once; idempotent replay requires
an explicit base. Stale base, precondition failure, invalid meaning, foreign identity, exhaustion,
no-change, and corruption remain distinct and publish nothing. Drafts provide separate
non-executable authority for multi-step work.

Pure functions support explicit rank-1 type parameters. Calls and named function values provide
their type arguments explicitly; `invoke` applies a named function value. The graph stores stable
type-parameter identities, and validation, bytecode, and the semantic reference interpreter agree
on substitution. There is no type-class constraint system, type-argument inference, generic task
function, lexical closure, or captured environment.

Graph contract 4 uses exact package/module identities in canonical imports, exact
module/component/port identities in targets, and exact package/module/declaration identities in
types, calls, named function values, constants, requirements, and exports. Lexical variables and
constant references are distinct forms. Module and declaration rename therefore update only the
stable owner and persistent name/summary paths; importer objects are unchanged. Exact references
may use a request-local symbol, a local `decl_` identity, or the discoverable
`exact:PACKAGE/mod_ID/decl_ID` selector.

Semantic-summary contract 2 defines integrity-bound, rebuildable module summaries for public
signatures, implementations, types, calls, effects, capabilities, deployment, and tests.
Semantic-fact contract 3 stores exact summary bindings, graph-owned test owners, and typed reverse
dependency edges in three persistent Merkle maps. Content-addressed summaries, map pages, and one
revision/root-bound manifest are disposable acceleration. Every accepted revision commits to a
constant-size, revision-independent certificate over the exact fact roots; missing or malformed
cache bytes rebuild, and a rebuilt certificate mismatch is canonical corruption rather than an
alternate meaning.

Four precondition-free transaction classes have local preparation: eligible pure-function body
replacements validate their recursive import dependency slice, independent module creation
validates only the new empty modules, and module/declaration rename validates the renamed owners
plus their outgoing import dependencies. Structurally different pure bodies publish exact removed
identity tombstones in the same root delta. Every other change still reconstructs, canonicalizes,
and validates the complete logical candidate. Missing disposable indexes can also make an
otherwise local path broad. Either path prepares once; publication rechecks the exact base, root
delta, summary delta, and authenticated certificate without repeating semantic validation. The
complete validator and packed reconstruction remain full oracles.

Exact owner/name queries use local-index contract 3: a small revision/root-bound manifest selects
content-addressed shards, and the four local transaction profiles rewrite only touched buckets.
Body-only changes reuse every shard. Initial and complete-candidate publication seed the same
derived index from graph values already in memory. The broad relation index remains lazy and
rebuildable rather than delta-maintained.

## Review, history, and recovery

```sh
lkjscript --project ./hello history list --limit 20
lkjscript --project ./hello review --output ./hello.review.json
lkjscript --project ./hello backup --output ./hello.lkjb
mkdir ./restored
lkjscript restore --backup ./hello.lkjb --output ./restored
lkjscript --project ./restored doctor --deep
lkjscript --project ./restored doctor cleanup
```

The review projection is deterministic, span-free, explicitly non-authoritative, and has no apply
path. Backup contract 4 writes a segmented directory (the `.lkjb` suffix is only a locator),
copying canonical entries one at a time under integrity-bound manifest segments. Restore verifies
each entry and the exact retained closure in a private stage before making the restored repository
visible. Backup/restore still retain an O(object-count) sorted key set in memory; this is segmented
payload transfer, not a fully bounded-memory pack. Derived query and semantic indexes rebuild
rather than becoming backup authority.

`doctor cleanup` is a read-only retention preview rooted at HEAD's parent DAG plus every live
draft base DAG. It reports retained/reclaimable/derived counts, unknown entries, and an exact plan
digest, but always returns `destructive_ready: false` because revision pins, active-reader leases,
and registered backup roots are not represented. No garbage collector or canonical deletion
command exists.

## Run lkjournal

`applications/lkjournal` is the maintained service consumer. Its graph owns routes, SQL,
migrations, authentication and authorization, JSON schemas, rendering, object publication, and
queue transitions. Rust owns generic protocol, execution, database, object-store, cryptographic,
deployment, and runner mechanics only.

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The HTTP listener is plaintext and the PostgreSQL adapter uses `NoTls`. lkjscript does not plan to
implement TLS termination, PostgreSQL TLS, certificate management, or ACME. Deployments that need
encrypted transport must place an appropriate external trusted transport boundary around these
adapters; that boundary does not make the runtime a hostile-code or multi-tenant sandbox.

See [applications/lkjournal/README.md](applications/lkjournal/README.md) for application behavior
and operator constraints.

## Build and verify the repository

Application users need only the executable. Repository contributors can build and verify it with:

```sh
cargo build --workspace --release --locked
tools/check focused
tools/check changed
tools/check product
tools/check service
tools/check full
```

Successful gates print one aggregate result and a receipt path while retaining bounded child logs
under `.artifacts/check/`. Reusable gates identify fresh versus reused evidence by exact inputs;
the authoritative `full` profile requires fresh execution.

Normative contracts live under [docs/spec](docs/spec). Current implementation and limits are in
[docs/status.md](docs/status.md), the layer map is [docs/architecture.md](docs/architecture.md),
and reproduced observations are [docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, or production portability beyond the
verified Linux x86-64 environment.
