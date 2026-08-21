# lkjournal

`lkjournal` is an actor-aware resource journal implemented as lkjscript application meaning above
generic adapters. It stores named Markdown-like text without parsing it, retains immutable
snapshots for every accepted update, publishes named objects, and completes one durable indexing
job per created resource.

The semantic owners are:

- `domain.lkj`: domain records, exact-base and ownership policy;
- `service.lkj`: routes, strict JSON schemas, authorization, SQL, migration, rendering, object
  reconciliation, and enqueue policy; and
- `worker.lkj`: queue claim and exact-attempt completion policy.

The checked `lkjournal.lkja` contains those modules plus the exact `standard` dependency. The
artifact contains requirements and no grants, credentials, paths, listener, or deployment secrets.

## Run

Build from the repository root:

```sh
cargo build --workspace --release --locked
target/release/lkjscript --project applications/lkjournal package test
```

Create a PostgreSQL database, then from this directory run:

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The service descriptor listens on `127.0.0.1:8080`. It admits at most 16 active requests plus 64
queued requests, limits request bodies to 8 MiB and response bodies to 4 MiB, and gives work a
30-second operational deadline. The worker runs at most two tasks. Change deployment values for a
real deployment; do not commit secrets.

## Routes

| Method and path | Application behavior |
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

Unknown routes return 404. Missing or invalid sessions return 401; cross-actor ownership checks
return 403; stale updates return 409; malformed typed JSON returns 400. Domain outcomes are HTTP
values authored in `service.lkj`, not adapter diagnostics.

The bootstrap token is compared by the generic secret-verifier adapter and is never passed into the
language as a serializable value. Password hashes use bounded Argon2 parameters from deployment.
Session expiry, actor ownership, object keys, SQL statements, and job payloads remain application
policy.

## Evidence and recovery

```sh
tools/service-acceptance --binary target/release/lkjscript
```

The acceptance command requires an already cached `postgres:16-alpine` Docker image, starts an
isolated database, exercises live handlers and the worker, performs `pg_dump`/`pg_restore`, restarts
against the restored database, and retains bounded evidence under `.artifacts/service/`.

The local object deployment writes beneath `state/objects`, which is excluded from source control.
The local adapter validates content type but the underlying local backend does not persist provider
attributes; S3 and memory adapters do. Database backup and object backup are separate operational
authorities and must be coordinated explicitly by a deployment.
