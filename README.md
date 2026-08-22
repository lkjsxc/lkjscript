# lkjscript

`lkjscript` is a meaning-oriented programming language and capability-oriented application
platform. The canonical authority for every accepted program is one revisioned typed semantic
graph. Names, review text, indexes, artifacts, bytecode, runtime handles, and deployment bindings
are projections or consumers; none is a second editable program truth.

Ordinary development uses the public semantic CLI. It supports bounded orientation and query,
stable-ID mutation and refactoring, drafts, exact-base validation and publication, semantic diff
and three-way merge, package tests, deterministic builds, review projection, deep reconstruction,
backup, and restore. Maintained `.lkj` source, package descriptors, and the predecessor source store
were removed by incompatible direct cutover.

The verified bootstrap is stable Rust 2024 on Linux x86-64.

## Build and orient

```sh
cargo build --workspace --release --locked
target/release/lkjscript semantic help
target/release/lkjscript --project packages/standard semantic status
target/release/lkjscript --project applications/lkjournal semantic orient --limit 20
target/release/lkjscript --project applications/lkjournal semantic find handle --exact
```

Every ordinary command emits one strict JSON value. Growing reads accept explicit item, byte,
work, depth, fanout, revision, and continuation controls. Exact lookups use revision-bound local
index shards; broad traversal uses a disposable full relation index. Missing or corrupt indexes
rebuild from canonical graph objects.

## Change meaning

Allocate IDs in their exact domain, construct one closed transaction request, then plan, validate,
and apply the identical request:

```sh
target/release/lkjscript semantic id-allocate module --count 1
target/release/lkjscript --project applications/lkjournal \
  semantic plan --request-file /tmp/change.json
target/release/lkjscript --project applications/lkjournal \
  semantic validate --request-file /tmp/change.json
target/release/lkjscript --project applications/lkjournal \
  semantic apply --request-file /tmp/change.json
```

A request names the graph contract, repository, exact base revision, ordered high-level operations,
preconditions, and work budgets. Acceptance publishes one revision atomically. Stale base,
precondition failure, invalid meaning, foreign identity, exhaustion, no-change, and corruption are
distinct and publish nothing. Broad results inline at most 64 affected owners and point to exact
revision expansion.

Use drafts for non-executable multi-step work:

```sh
target/release/lkjscript --project applications/lkjournal semantic draft-create
target/release/lkjscript --project applications/lkjournal semantic draft-status DRAFT_ID
```

## Review, test, build, and recover

```sh
target/release/lkjscript --project applications/lkjournal semantic diff \
  --base BASE_REVISION --result RESULT_REVISION
target/release/lkjscript --project applications/lkjournal semantic test
target/release/lkjscript --project applications/lkjournal semantic build \
  --output /tmp/lkjournal.lkja
target/release/lkjscript --project applications/lkjournal semantic text-project \
  --output /tmp/lkjournal.review.json
target/release/lkjscript --project applications/lkjournal semantic backup \
  --output /tmp/lkjournal.lkjb
target/release/lkjscript --project applications/lkjournal semantic doctor --deep
```

The deterministic review projection is span-free and explicitly non-authoritative; it cannot be
applied or imported. A graph-native artifact lowers directly from graph objects without reparsing
text. Package tests compare the production bytecode VM with an implementation-disjoint semantic
reference interpreter. Backup captures one exact accepted revision DAG and restore verifies every
canonical object before atomic visibility.

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

See [applications/lkjournal/README.md](applications/lkjournal/README.md) for application behavior
and operator constraints.

## Verify

```sh
tools/check focused
tools/check changed
tools/check product
tools/check service
tools/check full
```

Successful gates print one aggregate result and a receipt path while retaining bounded child logs
under `.artifacts/check/`. `changed` is selection convenience and widens uncertainty to `full`;
`full` is fresh.

Normative contracts live under [docs/spec](docs/spec). Current implementation and limits are in
[docs/status.md](docs/status.md), the layer map is [docs/architecture.md](docs/architecture.md),
and reproduced observations are [docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, or production portability beyond the
verified Linux x86-64 environment.
