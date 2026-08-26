# lkjournal

`lkjournal` is an actor-aware resource journal whose current editable application authority is the
Graph 5 repository in this directory. It stores named Markdown-like text without parsing it,
retains an immutable snapshot for every accepted update, publishes named objects, and completes one
durable indexing job for each created resource.

Its stable modules are:

- `domain` (`mod_86e34c967b6c9ebc5b3db7da53012a48`) for domain records, exact-base
  transitions, and ownership meaning;
- `service` (`mod_50e2d3318b93f572dad082bd4f42c526`) for routes, strict JSON schemas,
  authorization, SQL, migrations, rendering, object reconciliation, and enqueue policy; and
- `worker` (`mod_0510586a801c429b7a4a49a217de7fab`) for queue claim and exact-attempt
  completion meaning.

Current normalized identity:

- repository: `repo_95f988c5423fe3eb823c329ef0832d51`;
- package: `pkg_20000000000000000000000000000001`;
- semantic revision: `rev_0f660831701b710fc7cd6e5f2c87cd754a944adc4ce77e1aca4649711946b4db`;
- semantic state: `semantic_state_067e2ba593a62c71757d24aaf717ddf28027454bf11b623e292d939120520cd4`;
- artifact manifest: `artifact_manifest_97447a36407a29bb2b979ac42191d774334e661d799f65399b6eba904d593834`;
- 1,313 root semantic owners and one exact built-in standard dependency.

## Inspect and verify current authority

From the repository root:

```sh
cargo build --workspace --release --locked
target/release/lkjscript --project applications/lkjournal status
target/release/lkjscript --project applications/lkjournal query owners --limit 20
target/release/lkjscript --project applications/lkjournal inspect owner module \
  mod_50e2d3318b93f572dad082bd4f42c526
target/release/lkjscript --project applications/lkjournal check
target/release/lkjscript --project applications/lkjournal build \
  --output /tmp/lkjournal-current.lkja
```

Check compiles and links the exact two-package closure and runs 5 application tests plus 7 standard
tests. All 12 must agree between normalized bytecode and the canonical reference interpreter.
`generated/lkjournal.lkja` is the deterministic maintained artifact-10 output. Check and build do
not change accepted `HEAD`.

The normalized artifact contains target `serve`
(`target_e6f0a45c5f938ba39a19de585e8fc0d7`) and target `work`
(`target_3608e4377fe1adb3ef15e610a9b5e0e5`), their component/port meaning, and declared requirements.
It contains no grants, credentials, listener address, host paths, or deployment secrets.

## Current service and worker boundary

Released `serve` and `worker` have not yet cut over to artifact 10. They deliberately use the
separate read-only file `frozen-service/lkjournal-artifact-v4.lkja`, SHA-256
`d0a57a74161903a302472cbd8997762434b64cc58bd8ae36577b9ba31d2f96a3`. The deployment
descriptors refer to that exact path. The service harness verifies its digest and copies only the
descriptor/artifact runtime inputs to an isolated run; it never opens this Graph 5 repository.

To exercise that retained service behavior, create an empty PostgreSQL database and bind the two
named secrets without committing their values:

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The service descriptor listens on `127.0.0.1:8080`, admits at most 16 active plus 64 queued
requests, bounds request bodies to 8 MiB and response bodies to 4 MiB, and assigns a 30-second
operational deadline. The worker runs at most two tasks. Deployment JSON is operational authority,
not program meaning.

This frozen artifact proves only retained service behavior. It is not the current `lkjournal`
build, cannot regenerate the Graph 5 repository, and is not evidence of normalized service/worker
completion.

## Routes

| Method and path | Graph-owned behavior |
|---|---|
| `GET /health` | readiness independent of database work |
| `GET /` | escaped server-rendered service page |
| `POST /initialize?actor=…` | bootstrap-token check, migration, actor/password creation |
| `POST /login?actor=…` | password verification and random expiring bearer session |
| `POST /resources` | strict typed JSON create, resource/snapshot transaction, enqueue |
| `GET /resources` | actor-owned resource summaries |
| `GET /resource?id=…` | authenticated owner read |
| `GET /resource/history?id=…` | immutable ordered snapshots |
| `POST /resource/update?id=…` | strict typed JSON exact-base update and snapshot transaction |
| `POST /objects?name=…` | streaming no-replace object publication and database reference |
| `POST /objects/reconcile?name=…` | reconcile a possibly visible object publication |

Unknown routes return 404. Missing/invalid sessions return 401, cross-actor checks return 403,
stale updates return 409, and malformed typed JSON returns 400. Those outcomes are application
values rather than adapter diagnostics.

The bootstrap token is checked by the generic secret-verifier adapter and never becomes a
serializable language value. Password hashes use bounded Argon2 deployment parameters. Session
expiry, actor ownership, SQL, object keys, and job payloads remain graph policy.

The HTTP listener is plaintext and PostgreSQL uses `NoTls`. Do not expose either across an
untrusted network without an appropriate external trusted transport boundary. The runtime is not a
hostile-code or multi-tenant sandbox.

## Service acceptance

```sh
cargo run --locked -p lkjscript-dev -- service --binary target/release/lkjscript
```

The acceptance tool requires a cached `postgres:16-alpine` image. It starts an isolated database,
exercises live HTTP and worker paths, performs `pg_dump`/`pg_restore`, restarts, and retains bounded
evidence under `.artifacts/lkjscript-dev/service/`. Missing container/database prerequisites are
reported as unavailable, never as pass.
