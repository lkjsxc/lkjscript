# lkjscript

`lkjscript` is a source-authored, meaning-oriented language and capability-oriented application
platform. Canonical UTF-8 modules are the maintained program authority. Validation derives one
typed semantic model; deterministic package artifacts carry exact dependency closures; component
ports declare typed capability requirements; deployment descriptors bind those requirements to
bounded native adapters.

The current release proves three materially different shapes through one prepared execution path:

- pure package functions and tests;
- a bounded resident HTTP component; and
- a durable PostgreSQL-backed worker.

`applications/lkjournal` is the maintained ordinary service. Its routes, SQL, migrations,
authentication and authorization policy, JSON schemas, HTML, object publication policy, and job
state machine are authored in lkjscript. Rust owns generic parsing, execution, protocol, database,
object-store, cryptographic, and deployment mechanics only.

## Build and inspect

The verified bootstrap is stable Rust 2024 on Linux x86-64.

```sh
cargo build --workspace --release --locked
target/release/lkjscript help
target/release/lkjscript --project packages/standard project orient
target/release/lkjscript --project applications/lkjournal module show service
target/release/lkjscript --project applications/lkjournal package test
target/release/lkjscript artifact inspect applications/lkjournal/lkjournal.lkja
```

Every CLI response is one strict JSON value. `project validate` and `project apply` require the
current revision and record digest. Validation, rejection, stale input, and authored no-change
publish nothing. An accepted formatting-only change publishes authored history with
`semantic_changed: false`.

```sh
target/release/lkjscript --project applications/lkjournal project status
target/release/lkjscript --project applications/lkjournal \
  project validate --revision REVISION --record RECORD_DIGEST
target/release/lkjscript --project applications/lkjournal \
  project apply --revision REVISION --record RECORD_DIGEST
target/release/lkjscript --project applications/lkjournal project history --limit 10
target/release/lkjscript --project applications/lkjournal project doctor --deep
```

The project store is content-addressed source contract 1 under `.lkjscript/source-v1`. Package
paths locate source; durable declaration identity is `(package_id, module, declaration)`. Exact
dependency identity contains package identity, semantic revision digest, and artifact digest, never
an ambient path or mutable tag.

## Run lkjournal

Start PostgreSQL, create an empty database, and supply deployment secrets through the named
environment bindings:

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
../../target/release/lkjscript serve --deployment service.deployment.json
```

The generic runner validates configuration, secrets, the artifact, target, requirements, grants,
and limits before listener admission. `POST /initialize?actor=operator` with the bootstrap bearer
token applies the exact application-authored migration and establishes the first actor. Start the
worker separately with the same database secret:

```sh
cd applications/lkjournal
../../target/release/lkjscript worker --deployment worker.deployment.json
```

See [applications/lkjournal/README.md](applications/lkjournal/README.md) for routes and operator
behavior. The deterministic tests use the same prepared component ports as the live listener and
worker. `tools/service-acceptance` exercises an isolated PostgreSQL container, live HTTP,
authentication, exact-base update, object publication, queue completion, graceful shutdown, and
database backup/restore.

## Verify

```sh
tools/check focused
tools/check changed
tools/check product
tools/check service
tools/check full
```

Successful checks print one aggregate line and a receipt path. Each gate retains separate bounded
stdout and stderr logs under `.artifacts/check/`. `changed` widens uncertain impact to `full`; it is
selection convenience, not authority. `full` never reuses a prior pass.

Normative current contracts are under [docs/spec](docs/spec). The concise layer map is
[docs/architecture.md](docs/architecture.md), implemented reality and limits are
[docs/status.md](docs/status.md), and reproduced measurements are
[docs/performance.md](docs/performance.md).

## Scope and trust

Authored applications are trusted program inputs, not hostile native-code tenants. The platform
does not claim a hostile-code sandbox, distributed transactions, cross-process stream handles,
TLS termination, PostgreSQL TLS in the current adapter, or cross-platform production support.
HTTP client, multipart, Markdown sanitization, terminal and selected-filesystem adapters, and
general scheduling remain unimplemented until a complete maintained consumer requires them.

The predecessor graph authority, profile-specific artifacts and runtimes, and active `lkjedit` and
`lkjwork` products were removed by direct cutover. Their reproduced baseline and deletion decision
remain in [the campaign ledger](docs/evidence/20260821-general-platform-campaign-ledger.md) and Git
history; no compatibility reader executes their current artifacts.
