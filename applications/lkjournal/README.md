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
- semantic revision: `rev_0c800bcaf3fb598035b3d29d6bf886dc42f62537569e0aa5124f5fa8c95745a9`;
- semantic state: `semantic_state_11cccd6ab1f48dbcc637737aab3e4740a29a1b3678e94b9e0899b4cc755d4c0c`;
- package revision: `package_revision_02f6b51664a610d0a653aa901cef11e39a77bd713c8750defa1aeea5c841faa9`;
- artifact manifest: `artifact_manifest_561a57c404ee1429bbc6f4bf326b8089c4acf86b033c98ac5cf5535f7abbd463`;
- artifact bundle: `artifact_bundle_fedd83ac62aeeed15a772c40cc075d85b76dfda95e37cf66f6892f0d4edd356a`;
- 2,040 live root semantic owners and one exact built-in standard dependency.

## Inspect and verify current authority

From the repository root:

```sh
cargo build --workspace --release --locked
target/release/lkjscript status --project applications/lkjournal
target/release/lkjscript query --project applications/lkjournal owners --limit 20
target/release/lkjscript inspect --project applications/lkjournal owner module \
  mod_50e2d3318b93f572dad082bd4f42c526
target/release/lkjscript check --project applications/lkjournal
target/release/lkjscript build --project applications/lkjournal \
  --output /tmp/lkjournal-current.lkja
```

Check compiles and links the exact two-package closure with 90 application and 196 total compiler
units, then runs 7 application tests plus 20 standard tests. All 27 must agree between normalized
bytecode and the canonical reference interpreter.
`generated/lkjournal.lkja` is the deterministic maintained artifact bundle output. Check and build do
not change accepted `HEAD`.

Package-visible task function `decl_0693166bd7c29bee83d2ead289148f65` (`update-resource`) retains
its stable identity and delegates the exact data-only commit subtree to private same-module helper
`decl_53936ef7d46ee491d41aef8c37cdffef` (`commit-resource-update`). Their definition bodies contain
93 and 101 records respectively; the moved root remains
`expr_22692186086bc39d6caf2cfe244879c8`.

The normalized artifact contains HTTP target `serve`
(`target_e6f0a45c5f938ba39a19de585e8fc0d7`) with route-set
`http_routes_5343767ca1ac4bfc0c59fa610ddbd011f135ad87043921426a20ad3191e0aea2`,
worker target `work`
(`target_3608e4377fe1adb3ef15e610a9b5e0e5`), and interactive target `lkjournal-live-1`
(`target_4370908b66ee6a998ac707bbe43f351b`), their component/port meaning, and declared requirements.
It contains no grants, credentials, listener address, host paths, or deployment secrets.

The `serve` target has no universal or fallback port. Its eight exact and three pattern routes
are `GET /`, `GET /health`, `GET /resource/{id}`, `GET /resource/{id}/history`, `GET /resources`,
`POST /initialize`, `POST /login`, `POST /objects`, `POST /objects/reconcile`,
`POST /resource/{id}/update`, and `POST /resources`. Each pattern capture indexes the handler's
trailing unrestricted `id: Text` parameter; each route selects its own function-backed component
port. Authentication, authorization, body validation, domain transitions, data/object/queue policy,
and responses remain in those handlers; an unmatched valid method/path pair is the platform-owned
empty 404 and invokes none of them.

## Current service, interactive, and worker boundary

All three maintained deployment descriptors name `generated/lkjournal.lkja`, the 1,062,638-byte
artifact bundle above (SHA-256
`1a1cf9b5fd7c920e3f6f5a788fc21fa16c35e19238b3f33ea5ccd771fb4311a8`). The service descriptor
resolves `serve`, the worker descriptor resolves `work`, and `live.deployment.json` resolves
`lkjournal-live-1`. Preparation strictly loads the standalone bundle,
validates the runner, route-indexed handler and component requirement closure, grants, secrets, and adapters, and emits
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
| `GET /resource/{id}` | authenticated owner read through a typed path capture |
| `GET /resource/{id}/history` | immutable ordered snapshots through a typed path capture |
| `POST /resource/{id}/update` | strict typed JSON exact-base update and snapshot transaction through a typed path capture |
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
