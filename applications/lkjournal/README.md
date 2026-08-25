# lkjournal

`lkjournal` is an actor-aware resource journal whose canonical application meaning is the typed
graph under `.lkjscript/meaning`. It stores named Markdown-like text without parsing it, retains an
immutable snapshot for every accepted update, publishes named objects, and completes one durable
indexing job for each created resource.

The current graph has three stable modules:

- `domain` (`mod_86e34c967b6c9ebc5b3db7da53012a48`) owns domain records, exact-base
  transitions, and ownership meaning;
- `service` (`mod_50e2d3318b93f572dad082bd4f42c526`) owns routes, strict JSON schemas,
  authorization, SQL, migration, rendering, object reconciliation, and enqueue policy; and
- `worker` (`mod_0510586a801c429b7a4a49a217de7fab`) owns queue claim and exact-attempt
  completion meaning.

The maintained graph-4 revision is
`rev_eb60847c2ebc2098c65a3e425398fb63ae74e08f47cdda3067069acacea7fa90`. Its root package
artifact is `artifact_55c3b229f8cbdd53fb153e0859375404df5e31f66f6128736f5d8f95f71dfe98`, and
`lkjournal.lkja` has bundle digest
`artifact_fd1b07fbf5caafc92499eead7077f2ffe638bbf1a8c48f154eb9a09fcc3bf78d`. The bundle contains
the exact standard package artifact
`artifact_6ea73654d153ac4410ff4aaad329373dce27a58bb0d8c61eaa31cd6d66bcb3f6`. It contains
requirements and no grants, credentials, listener address, host paths, or deployment secrets.

## Inspect and verify

From the repository root:

```sh
cargo build --workspace --release --locked
target/release/lkjscript --project applications/lkjournal inspect project --limit 20
target/release/lkjscript --project applications/lkjournal inspect owner \
  mod_50e2d3318b93f572dad082bd4f42c526 --body
target/release/lkjscript --project applications/lkjournal check
target/release/lkjscript --project applications/lkjournal build \
  --output /tmp/lkjournal.lkja
target/release/lkjscript --project applications/lkjournal doctor --deep
```

The check command runs the exact two-package closure: 5 `lkjournal` tests plus 7 standard tests,
for 12 passing checks, and requires equality between prepared bytecode and the semantic reference
interpreter.

## Run

Create an empty PostgreSQL database and bind the two named secrets. Do not commit their values.

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The service descriptor listens on `127.0.0.1:8080`, admits at most 16 active requests plus 64
queued requests, limits request bodies to 8 MiB and response bodies to 4 MiB, and assigns a
30-second operational deadline. The worker runs at most two tasks. Deployment JSON binds runtime
grants; it is not program authority.

The HTTP listener is plaintext, and PostgreSQL connections use `NoTls`. The loopback listener and
trusted local database path are deliberate deployment assumptions. Do not expose either adapter
across an untrusted network without an appropriate external trusted transport boundary. lkjscript
does not plan to add HTTP TLS termination, PostgreSQL TLS, certificate management, or ACME; the
external boundary does not turn the runtime into a hostile-code or multi-tenant sandbox.

## Routes

| Method and path | Graph-owned behavior |
|---|---|
| `GET /health` | readiness response independent of database work |
| `GET /` | escaped server-rendered service page |
| `POST /initialize?actor=…` | bootstrap-token check, migration, actor/password creation |
| `POST /login?actor=…` | password verification and random expiring bearer session |
| `POST /resources` | strict typed JSON create, atomic resource/snapshot transaction, enqueue |
| `GET /resources` | actor-owned resource summaries |
| `GET /resource?id=…` | authenticated owner read |
| `GET /resource/history?id=…` | immutable ordered snapshots |
| `POST /resource/update?id=…` | strict typed JSON exact-base update and snapshot transaction |
| `POST /objects?name=…` | streaming no-replace object publication and database reference |
| `POST /objects/reconcile?name=…` | reconcile a possibly visible object publication |

Unknown routes return 404. Missing or invalid sessions return 401, cross-actor checks return 403,
stale updates return 409, and malformed typed JSON returns 400. These domain outcomes are graph
values, not adapter diagnostics.

The bootstrap token is checked by the generic secret-verifier adapter and never becomes a
serializable language value. Password hashes use bounded Argon2 deployment parameters. Session
expiry, actor ownership, SQL, object keys, and job payloads remain application meaning rather than
generic Rust policy.

## Acceptance and recovery

```sh
cargo run --locked -p lkjscript-dev -- service --binary target/release/lkjscript
```

The acceptance tool requires a cached `postgres:16-alpine` image. It starts an isolated database,
exercises live HTTP and worker paths, performs `pg_dump`/`pg_restore`, restarts, and retains bounded
evidence under `.artifacts/lkjscript-dev/service/`.

Canonical program backup is separate:

```sh
target/release/lkjscript --project applications/lkjournal backup \
  --output /tmp/lkjournal-meaning.lkjb
```

The output is a segmented backup directory, not a monolithic file. Database, object, and
canonical-meaning backups are distinct operational authorities and must be coordinated by
deployment policy.
