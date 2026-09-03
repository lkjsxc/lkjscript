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
  enqueue policy, plus the authenticated live-session protocol and subscription transitions; and
- `worker` (`mod_0510586a801c429b7a4a49a217de7fab`) for affine queue claim/dispatch and
  exact-requirement-bound helper processing, metadata borrow, renewal, completion, and retry/failure
  meaning.

Current normalized identity:

- repository: `repo_95f988c5423fe3eb823c329ef0832d51`;
- package: `pkg_20000000000000000000000000000001`;
- semantic revision: `rev_f71930edbba61200b2130aaad44e9b1e5e21982cef810185ab9259984f478580`;
- semantic state: `semantic_state_39fdb8b778a411a4cf40c0fda249982b08047ce175c3dad4cdd23fdfd127d0cf`;
- package revision: `package_revision_37904fd33a7db989bef78361a9138aa0b57d00e79f32a8dbb5dd90235bd483a6`;
- artifact manifest: `artifact_manifest_6141c850a10f6250412450f9c42be7196bd7eed545588cafe5db62f306aaa7db`;
- artifact bundle: `artifact_bundle_620b8802e9105addba8a36b828ac4c3afd046ef8f5d74a2e6c5d146f6dbf5189`;
- 2,125 live root semantic owners and one exact built-in standard dependency.

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

Check compiles and links the exact two-package closure with 91 application and 197 total compiler
units, then runs 7 application tests plus 20 standard tests. All 27 must agree between normalized
bytecode and the canonical reference interpreter.
`generated/lkjournal.lkja` is the deterministic maintained artifact bundle output. Check and build do
not change accepted `HEAD`.

Package-visible task function `decl_0693166bd7c29bee83d2ead289148f65` (`update-resource`) retains
its stable identity and delegates the exact data-only commit subtree to private same-module helper
`decl_53936ef7d46ee491d41aef8c37cdffef` (`commit-resource-update`). Their definition bodies contain
96 and 101 records respectively; the moved root remains
`expr_22692186086bc39d6caf2cfe244879c8`.

The normalized artifact contains HTTP target `serve`
(`target_e6f0a45c5f938ba39a19de585e8fc0d7`), worker target `work`
(`target_3608e4377fe1adb3ef15e610a9b5e0e5`), and interactive target `lkjournal-live-1`
(`target_4370908b66ee6a998ac707bbe43f351b`), their component/port meaning, and declared requirements.
It contains no grants, credentials, listener address, host paths, or deployment secrets.

## Current service, interactive, and worker boundary

All three maintained deployment descriptors name `generated/lkjournal.lkja`, the 1,083,675-byte
artifact bundle above (SHA-256
`2bdf2f1d2b4871b8aba7cf57932149685b9f8735334b71c21bab4f708d89b83d`). The service descriptor
resolves `serve`, the worker descriptor resolves `work`, and `live.deployment.json` resolves
`lkjournal-live-1`. Preparation strictly loads the standalone bundle,
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
../../target/release/lkjscript serve --deployment live.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The service descriptor listens on `127.0.0.1:8080`, admits at most 16 active plus 64 queued
requests, bounds request bodies to 8 MiB and response bodies to 4 MiB, and assigns a 30-second
operational deadline. The live descriptor listens on `127.0.0.1:8081`, admits at most 16 sessions
and 16 pending handshakes, bounds its item/byte mailboxes and per-transition output independently,
uses a 100-millisecond coalesced tick, and allows at most 24 hours total lifetime without changing
the one-hour callback ceiling. The worker runs at most two tasks. Deployment JSON is operational
authority, not program meaning.

The service `data` and worker `durable_queue_data` grants share `state/data` under strict distinct
namespaces. Actor, session, resource, immutable-snapshot, object-metadata, lookup, and job facts use
explicit graph-owned spaces/indexes with canonical typed values. Object bytes remain under the
local/S3 object capability. Live effects run once through the production VM and never through
differential replay.

The interactive graph accepts only `/live` with a valid `lkjournal` bearer session. Connection
state retains the actor and a bounded map from subscription IDs to last-observed resource
revisions. Strict text JSON arrays subscribe/replace or unsubscribe an ID. A subscription emits
the current actor-owned summaries in deterministic order and an end marker; later coalesced ticks
scan the same data authority and emit each new or advanced resource once before installing updated
ordinary state. Binary input and malformed or unknown operations select graph-owned errors or
close. Replacing and unsubscribing affect only that connection; reconnect starts from a new durable
snapshot rather than a restored runtime cursor. The protocol is application-specific and is not
Nostr.

The stable worker entry `decl_a914bb78de075ff44a857ac028d704f3` receives only
`QueueLeaseState`, claims, and matches the live case. It transfers that handle exactly once into
private task helper `decl_7f443401f4946c55fa239c5430e8ad93`, whose final consume parameter is
bound to exact `jobs` requirement `req_0cebded5cb056cda5484e39aa40594ad`. The helper borrows
`QueueLeaseInfo` for job/payload policy, consumes through heartbeat, matches the renewed state, and
consumes through complete or fail. The entry has no post-transfer use. Attempt and worker
transition identity never enter application values, and dropping a local handle does not
implicitly change durable queue state.

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
meaning authority before and after. Two worker processes exercise the entry/helper handoff and
claim/info/renew/complete,
retry/fail, expired-lease replacement, cancellation, and task cleanup. An independent bounded
queue-data observer records attempt advancement, terminal state, and cleared private transition
fields. The same run covers initialization, login, actor isolation, resource/history/object
reconciliation, restart, failed-startup/no-readiness, logical backup, absent-root restore,
post-restore equality, shutdown, and cleanup. Two independent raw RFC 6455 clients additionally
prove authenticated snapshots, create/update server push, replacement/unsubscribe and actor
isolation, slow-client containment, restart/resubscribe, valid fragmentation and ping/pong, and
connection-local malformed-frame failures without importing the production WebSocket codec.
Bounded evidence is retained under
`.artifacts/lkjscript-dev/service/`. This product and service workflow has no container, database
server, connection secret, or host database-library prerequisite.

Contributor-only `lkjscript-dev data-oracle` separately uses an exact PostgreSQL 16.15 image for
neutral migration and differential/resource evidence. That tool is not a deployment provider,
application helper, public import path, or permanent dual reader/writer.
