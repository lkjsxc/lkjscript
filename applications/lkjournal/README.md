# lkjournal

`lkjournal` is an actor-aware resource journal whose current editable application authority is the
typed meaning graph repository in this directory. It stores named Markdown-like text without
parsing it, retains an immutable snapshot for every accepted update, publishes named objects, and
completes one durable indexing job for each created resource.

Its stable modules are:

- `domain` (`mod_86e34c967b6c9ebc5b3db7da53012a48`) for domain records, exact-base
  transitions, and ownership meaning;
- `service` (`mod_50e2d3318b93f572dad082bd4f42c526`) for routes, strict JSON schemas,
  authorization, data spaces/indexes/schema transitions, rendering, object reconciliation, and
  enqueue policy; and
- `worker` (`mod_0510586a801c429b7a4a49a217de7fab`) for queue claim and exact-attempt
  completion meaning.

Current normalized identity:

- repository: `repo_95f988c5423fe3eb823c329ef0832d51`;
- package: `pkg_20000000000000000000000000000001`;
- semantic revision: `rev_8c9af517a19991e1e71c69dfa427fdddf0e0f9f69161522a7cf6889db88f938f`;
- semantic state: `semantic_state_6ab133945cc984ab98305897c1ce9daaec7f6ce089ea937f60023f326aa5dc9f`;
- artifact manifest: `artifact_manifest_2087386a0c1ff708b04db2221a5a20ac3d3f66ac3443e4dc2a25f032f38398d1`;
- artifact bundle: `artifact_bundle_b5a2f01d952416c5f9b61b36d1335ded8698acc653ae677974b43dc57c8acb6c`;
- 1,559 live root semantic owners and one exact built-in standard dependency.

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

Check compiles and links the exact two-package closure and runs 5 application tests plus 11 standard
tests. All 16 must agree between normalized bytecode and the canonical reference interpreter.
`generated/lkjournal.lkja` is the deterministic maintained artifact bundle output. Check and build do
not change accepted `HEAD`.

The normalized artifact contains target `serve`
(`target_e6f0a45c5f938ba39a19de585e8fc0d7`) and target `work`
(`target_3608e4377fe1adb3ef15e610a9b5e0e5`), their component/port meaning, and declared requirements.
It contains no grants, credentials, listener address, host paths, or deployment secrets.

## Current service and worker boundary

Both maintained deployment descriptors name `generated/lkjournal.lkja`, the 813,625-byte
artifact bundle above (SHA-256
`9bc15d247ff571df09acc3c1002b87015846f46f74f9c57523147ecec1db5d28`). `serve` resolves target
`serve`; `worker` resolves target `work`. Preparation strictly loads the standalone bundle,
validates the runner, exact component requirement closure, grants, secrets, and adapters, and emits
readiness only after first-party data/queue/object preflight. It does not discover this typed
meaning graph repository or read accepted `HEAD`.

To exercise the service, initialize the configured first-party root, create the local object host
directory, and bind the bootstrap secret without committing its value:

```sh
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
mkdir -p state state/objects
../../target/release/lkjscript data initialize --root state/data
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The service descriptor listens on `127.0.0.1:8080`, admits at most 16 active plus 64 queued
requests, bounds request bodies to 8 MiB and response bodies to 4 MiB, and assigns a 30-second
operational deadline. The worker runs at most two tasks. Deployment JSON is operational authority,
not program meaning.

The service `data` and worker `durable_queue_data` grants share `state/data` under strict distinct
namespaces. Actor, session, resource, immutable-snapshot, object-metadata, lookup, and job facts use
explicit graph-owned spaces/indexes with canonical typed values. Object bytes remain under the
local/S3 object capability. Live effects run once through the production VM and never through
differential replay.

## Routes

| Method and path | Meaning-owned behavior |
|---|---|
| `GET /health` | readiness after complete adapter/data preflight |
| `GET /` | escaped server-rendered service page |
| `POST /initialize?actor=…` | bootstrap-token check, schema transition, actor/password creation |
| `POST /login?actor=…` | password verification and random expiring bearer session |
| `POST /resources` | strict typed JSON create, resource/snapshot transaction, enqueue |
| `GET /resources` | actor-owned resource summaries |
| `GET /resource?id=…` | authenticated owner read |
| `GET /resource/history?id=…` | immutable ordered snapshots |
| `POST /resource/update?id=…` | strict typed JSON exact-base update and snapshot transaction |
| `POST /objects?name=…` | streaming no-replace object publication and data metadata reference |
| `POST /objects/reconcile?name=…` | reconcile a possibly visible object publication |

Unknown routes return 404. Missing/invalid sessions return 401, cross-actor checks return 403,
stale updates return 409, and malformed typed JSON returns 400. Those outcomes are application
values rather than adapter diagnostics.

The bootstrap token is checked by the generic secret-verifier adapter and never becomes a
serializable language value. Password hashes use bounded Argon2 deployment parameters. Session
expiry, actor ownership, data spaces/indexes, object keys, and job payloads remain graph policy.

The HTTP listener is plaintext, and the local data/object roots are not encrypted. Do not expose
them across an untrusted boundary without appropriate external transport/storage protection. The
runtime is not a hostile-code or multi-tenant sandbox.

## Service acceptance

```sh
cargo run --locked -p lkjscript-dev -- service --binary target/release/lkjscript
```

The acceptance tool first builds through the public command and requires byte equality with the
checked-in bundle. It stages only the copied binary, artifact, descriptors, configuration/secrets,
one initialized shared data root, and a local object directory; it snapshots canonical typed
meaning authority before and after. It exercises initialization, login, actor isolation,
resource/history/object reconciliation, durable claim/completion/stale attempts, restart,
failed-startup/no-readiness, logical backup, absent-root restore, post-restore equality, shutdown,
and cleanup. Bounded evidence is retained under `.artifacts/lkjscript-dev/service/`. This product
and service workflow has no container, database server, connection secret, or host database-library
prerequisite.

Contributor-only `lkjscript-dev data-oracle` separately uses an exact PostgreSQL 16.15 image for
neutral migration and differential/resource evidence. That tool is not a deployment provider,
application helper, public import path, or permanent dual reader/writer.
