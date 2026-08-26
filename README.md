# lkjscript

`lkjscript` is a meaning-oriented programming language and capability-oriented application
platform. The canonical authority for every accepted program is one revisioned typed meaning
graph under `lkjscript-meaning-graph-4`. Names, review text, indexes, artifacts, bytecode, runtime
handles, and deployment bindings are projections or consumers; none is a second editable program
truth. The physical root is a fixed manifest over six immutable path-compressed Merkle maps, while
the complete logical graph remains the reconstruction and validation oracle.

The released executable currently provides a binary-only normalized vertical slice for discovery,
minimal project creation, status, exact owner inspection, bounded owner/name/relation query, and
semantic change planning and publication. Check, build, run, service, worker, package, history,
draft, review, backup, restore, and doctor still use predecessor authority and are not yet available
for a newly created normalized project. The remaining cutovers are tracked in the roadmap.

The verified bootstrap is stable Rust 2024 on Linux x86-64.

## Start from one binary

Discover the exact current CLI contract and create a minimal accepted project:

```sh
mkdir -p /tmp/lkjscript-demo
cp /path/to/released/lkjscript /tmp/lkjscript-demo/lkjscript
cd /tmp/lkjscript-demo
export PATH="$PWD:$PATH"
lkjscript capabilities
lkjscript capabilities new
lkjscript new ./hello --template minimal --name hello
lkjscript --project ./hello status
lkjscript capabilities change
```

`new` accepts an absent or empty ordinary directory. It rejects nonempty destinations and symlink
components, constructs the complete repository in a private sibling stage, and makes it visible
with one rename after durable publication. The current normalized command accepts only the
`minimal` template, which creates an empty package. It prints compact records naming the project,
repository, package, semantic root, accepted revision, and durable publication receipt.
After the copy, this normalized workflow needs no source checkout, Cargo invocation, network, or
external data file other than an authored change record.

The embedded standard package is inspectable and exportable:

```sh
lkjscript package builtin inspect
lkjscript package builtin export --output ./standard.lkja
```

Its bytes are integrity checked as an ordinary graph-native artifact and reproduced from the
maintained standard package during repository verification.

## Inspect and change meaning

Global `--project PATH` selects a project explicitly; from inside a normalized project, discovery
also walks ordinary parent directories without following symbolic links:

```sh
lkjscript --project ./hello status
lkjscript --project ./hello inspect owner module mod_...
lkjscript capabilities query
lkjscript --project ./hello query owners --limit 20
```

`capabilities`, normalized `new`, `status`, exact `inspect`, `query`, and `change` emit
deterministic compact line records. Finite predecessor commands still emit JSON until their direct
cutover. Compact responses have independent record and byte bounds and identify the exact observed
revision where applicable.

One compact `change` file may create connected meaning with request-local symbols. Definitions and
flat expression/type fragments are checked in their identity domains, and allocated identities are
returned in both plan and apply results; there is no separate preallocation step. For example,
replace `rev_...` with the exact revision returned by `status`:

```sh
cat >change.lkjc <<'EOF'
request base=rev_...
create.module as=$notes name=notes
create.record as=$note module=$notes name=Note visibility=public
add.field as=$text record=$note name=text type=text
EOF
lkjscript --project ./hello change plan --input-file change.lkjc
lkjscript --project ./hello change apply --input-file change.lkjc --plan plan_...
```

After publication, the executable can rediscover the allocated identities without the change
receipt. Replace each placeholder with the exact ID or continuation returned by the preceding
query:

```sh
lkjscript --project ./hello query find module notes
lkjscript --project ./hello query find declaration Note --parent mod_...
lkjscript --project ./hello query find field text --parent decl_...
lkjscript --project ./hello query owners --kind declaration --limit 2
lkjscript --project ./hello query owners --kind declaration --limit 5 \
  --continuation qcont_...
lkjscript --project ./hello query relations decl_... \
  --direction outgoing --kind declaration_module
lkjscript --project ./hello query relations mod_... \
  --direction incoming --kind declaration_module
```

Query reads only the current accepted revision. It emits canonical compact records, creates no
index or continuation file, and never changes HEAD. A continuation is bound to the repository,
package, exact revision, direction, filter, and logical resume key; restart the query after an
accepted change. Fuzzy search, historical query, context traversal, generic impact, JSON query
requests, and the former callers/callees aliases are intentionally absent.

For a common single-owner edit, the direct adapter constructs the same typed request without a
record file. Replace the placeholders with the exact accepted revision and exact typed owner ID:

```sh
lkjscript --project ./hello change plan rename.owner \
  --base rev_... --owner mod_... --name renamed
lkjscript --project ./hello change apply rename.owner \
  --base rev_... --owner mod_... --name renamed --plan plan_...
```

Direct `--owner` is an exact `OwnerKey`; it does not accept a name or request-local symbol. Add
equal `--idempotency KEY` and `--intent TEXT` values to both commands when those controls are used.

`change plan` parses, normalizes, allocates, analyzes, and validates without publication.
`change apply` reparses and reprepares the same typed request, requires the exact returned `plan_`
digest, and publishes only after rechecking its explicit base. Raw JSON change requests and the
former `--request`, `--dry-run`, and `--commit` grammar are rejected. The currently exposed compact
operation/type/expression subset is discoverable from `capabilities --section change`, `type`, and
`expression`; change discovery reports all 13 operations, all 49 operation fields and their forms,
and the sole direct operation's exact usage. The broader typed engine remains private until each
form has a complete public workflow.

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

The executable registry owns the current semantic-summary, semantic-fact, and validator identities;
see [the generated contract table](docs/generated/contracts.md). The summary contract defines
integrity-bound, rebuildable module summaries for public signatures, implementations, types, calls,
effects, capabilities, deployment, and tests. The semantic-fact contract stores exact summary
bindings, graph-owned test owners, and typed reverse
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

Normalized public query reads canonical owner bindings and committed namespace/relation witness
maps through one revision-pinned `GraphRepository` view; it has no correctness dependency on a
query index and never invokes complete graph reconstruction. The predecessor local and broad query
indexes remain private only for exact out-of-scope workspace, diff, legacy inspect, change,
transaction, and repository consumers pending their own cutovers.

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
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

Successful gates print one aggregate result and a receipt path while retaining bounded child logs
under `.artifacts/lkjscript-dev/check/`. Reusable gates identify fresh versus reused evidence by exact inputs;
the authoritative `full` profile requires fresh execution.

Normative contracts live under [docs/spec](docs/spec). Current implementation and limits are in
[docs/status.md](docs/status.md), the layer map is [docs/architecture.md](docs/architecture.md),
and reproduced observations are [docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, or production portability beyond the
verified Linux x86-64 environment.
