# General-platform campaign ledger

Status: completed implementation evidence ledger, started and finalized 2026-08-21 UTC. This file
records reproduced facts and decision gates for the campaign in `prompts/202608211949.md`; it is not
semantic authority.

## Starting authority

| Fact | Reproduced value |
|---|---|
| Commit | `cc9b465227237a1600e9f2cb4e8e7f85ae59093a` (`Record final lkjedit reproduction evidence`) |
| Branch and upstream | `main`, `origin/main` |
| Push remote | `origin`, `https://github.com/lkjsxc/lkjscript.git` |
| Root policy | `AGENTS.md`, SHA-256 `38b679686f077fdaf0ca56081933af084285bcc5fdd52ec2ece578c04fcd421d`, 225,454 bytes, 4,141 lines |
| Campaign artifact | `prompts/202608211949.md`, SHA-256 `d9ef90f3e2966a6b86ddd40abc4da14b95b8b4c58a70a6bb2057e232b33030af`, 361,578 bytes, 6,819 lines |
| Initial worktree | modified delivered `AGENTS.md`; untracked delivered campaign prompt; unrelated untracked `package-lock.json` |
| Unrelated exclusion | `package-lock.json`, SHA-256 `e7cb3a5cd2e52bbeaf3a696f4cf141e63f0d7aeb0e523df51b2458406636e86b`, 88 bytes; do not edit or stage |
| Applicable instructions | root `AGENTS.md` only |

The audited campaign baseline equals the checked-out commit. The modified root policy has the
delivered digest. No baseline claim was inferred from the prompt when a public workflow could be
reproduced.

## Environment and tools

- Linux `7.0.0-29-generic`, x86-64, 20 logical CPUs.
- `rustc 1.96.0`, `cargo 1.96.0`, Rust edition 2024; Python 3.12.3; Git 2.43.0; ripgrep 15.2.0;
  jq 1.7.
- `/usr/bin/time` is unavailable. Bash `time -p` and Python `time.perf_counter_ns` are the retained
  wall/process timing mechanisms. Process RSS and provider token/price telemetry are unavailable.
- Cargo metadata reports one workspace package, `lkjscript 0.1.0`. Direct dependencies are
  `base64`, `blake3`, `crossterm`, `fs2`, `getrandom`, `rustix`, `serde`, `serde_json`,
  `signal-hook`, `unicode-segmentation`, `unicode-width`, and the test-only `tempfile`.
- The repository contains no local `unsafe` block. The patched `crossterm` dependency is vendored.

## Baseline verification

All commands ran from the starting checkout with locked dependencies.

| Profile | Result | Elapsed | Receipt |
|---|---:|---:|---|
| `./tools/check quick --machine` | 2/2 passed | 5.715 s | `.artifacts/check/20260821T122604.976746Z-4127817-quick/receipt.json` |
| `./tools/check product --machine` | 7/7 passed | 187.029 s | `.artifacts/check/20260821T122614.135438Z-4128174-product/receipt.json` |
| `./tools/check full --machine` | 15/15 passed | 730.608 s | `.artifacts/check/20260821T122930.983038Z-4129073-full/receipt.json` |

The full profile's dominant gates were workspace tests (537.914 s), complete `lkjedit` doctor
(108.278 s), and `lkjedit` acceptance (75.424 s). `lkjwork` complete doctor and acceptance took
2.167 s and 3.108 s. The original checker retained complete child logs when run, but an initial
contract-2 retention defect later removed the predecessor receipt directories; those raw logs are
unavailable. The aggregate observations above were captured before cutover. The predecessor tool
exposed only `quick`, `product`, and `full`, printed one line per gate in human mode, had no accepted
affected-gate model, and did not reuse a complete pass receipt.

## Current authority and formats

| Domain | Current exact owner or identity | Observation |
|---|---|---|
| Authored program | one full typed-meaning graph snapshot per accepted project revision | no maintained source; one package and one module in each first-party project |
| Semantic schema | `lkjscript-tsm008`; snapshot magic `LKJTSM\0\x08` | closed hostile decoder and canonical encoding |
| Project head | `LKJHDA10`; project protocol 14 | exact current revision, snapshot, and revision-record owner |
| Project session | contract 3 | caller-owned acceleration, not authority |
| Semantic document/context | contract 2 | proposal and task projection, not maintained authority |
| Reusable release | contract/format 2 | immutable exact dependency closure |
| Application | contract 8, internal format 9 | exact release graph, one entry, one invocation profile, policy, tests |
| Stateful instance | contract/format 3 | immutable records plus current cache and checkpoints every 64 revisions |
| Runtime | contract/session 2 | synchronous, one active transition and host operation; no queue or compiled cache |
| Interactive profile | profile contract 3 | initialize/update/resume/render plus product-shaped action routes |
| Selected filesystem | interface contract 1 | separately strong confined root mechanics |
| Terminal | interface contract 1 | caller-owned live terminal and capacity-one action worker |

Direct predecessor rejection exists at current decoders, but every identity above is a candidate for
replacement rather than compatibility preservation.

## Maintained projects and public workflow

### `lkjedit`

- Workspace `6ee361b40e2ce5041d64321d79c3db0d`, revision 174, snapshot
  `7e037b9e97e2f04cd2243899a30a3721f31faf30c55aa9d7d97f050d22004aa4`, revision record
  `aa634314469c3712fd4fca1976d07ce20073c5162a4109abaec94c6dabfe107b`.
- 10,926 nodes: 1,217 durable identities, 9,709 local identities, 61 tombstones. Package node `:2`
  contains the single `editor` module at `:3` with 291 child declarations.
- Public `orient`, `status`, and `target list` take approximately 0.97--1.03 s and emit 1,247, 699,
  and 455 bytes. The application target description emits 15,520 bytes of explicit interactive
  shape mapping.
- Function query for `lkjedit::editor::update_editor_model_v4` emitted 37,897 bytes; its proposal
  response emitted 14,639 bytes (13,866-byte document); refactor context emitted 61,944 bytes.
  Each took approximately 1.01--1.03 s. Validating the unchanged public document returned typed
  `no_change` and published nothing.
- A public backup published 352 files and 51,704,083 bytes to a no-replace temporary destination.
- Checked artifact: 471,096 bytes, SHA-256
  `95cb525cea6440164e9eac58383fc194d79fc2b6df9baeadfc75309083e8338a`. Release binary:
  9,329,928 bytes, SHA-256 `b0e4ddd7242ef6cae10a5bf684470f170c99358a75d515c90208aad3d4c3767c`.

Equal-workload release measurements, one warm-cache sample:

| Workload | Elapsed | Request / response bytes |
|---|---:|---:|
| 10,000 mixed transitions | 27.768608630 s | 1,270,050 / 24,360 |
| 1,000 growing inserts (1,006 transitions) | 4.405541596 s | 130,812 / 24,477 |
| 100 tabs (795 transitions) | 2.815967447 s | 102,105 / 24,389 |
| 65,536-scalar paste | 83.456675 ms | 263,006 / 24,461 |
| 1-by-1 resize | 51.612490 ms | 504 / 744 |

The workload used `time.perf_counter_ns`, optimized release binaries, and warm host caches. A
single orientation took 998.527023 ms; version plus artifact validation took 36.663978 ms.

### `lkjwork`

- Workspace `6894b57786a7e1ef14370d2da3a3cf33`, revision 9, snapshot
  `4ae531e41c93b69583ee14c44837ce1d5f6748e427d3a02f60c7164b2174eec1`, revision record
  `fb28346946daef184262e06781ee42c537913d63ba313ef50834744458f30561`.
- 3,353 nodes: 547 durable identities and 2,806 local identities. Package node `:2` contains the
  single `main` module at `:3` with 150 child declarations.
- Public `orient`, `status`, and `target list` take approximately 0.24--0.25 s and emit 1,247, 692,
  and 457 bytes. Its application target description emits 50,866 bytes of stateful-profile mapping.
- Function query for `lkjwork::main::query_entry` emitted 33,133 bytes; its proposal response
  emitted 12,550 bytes (11,846-byte document); refactor context emitted 54,312 bytes. Each took
  approximately 0.25--0.26 s. Unchanged-document validation returned `no_change` and published
  nothing.
- A public backup published 22 files and 1,734,795 bytes to a no-replace temporary destination.
- Public product acceptance reproduced durable mutation, pure query, suspension, typed host
  success, unknown visibility plus reconciliation, restart, 31-record deep replay, corruption
  rejection, backup, and restore. Final revision was 30 with state digest
  `66e35d221306790e86e8a29812d04a2a2e5c9ff114d62398ee962cccd1b088b2`.
- Checked artifact: 170,545 bytes; inspected application digest
  `53c52e0a513c1b03b3c57f131aa372d64aea6cc04f07f0857fe24cd0fe45f985`.

## Reproduced editor product observations

The public `lkjedit headless` route and the checked application artifact were used.

- Line-end append is defective. Opening `one\ntwo\n`, pressing `$`, `a`, and `X` produced rows
  `one` and `Xtwo`; the insertion crossed the line boundary.
- Viewport following is defective. Opening 80 lines and issuing 60 `j` movements left the rendered
  body at lines 0--37 while the reported device cursor was clamped to row 39. The active semantic
  row was outside the rendered viewport.
- `j` and Down now change explorer selection and Enter routes to `filesystem_read`; therefore the
  older observation that number shortcuts are the sole route is not fully reproducible at this
  revision. The selected row is not rendered and the device cursor remains at row 1 after one or
  two downward moves, reproducing the comprehension/feedback defect.
- Existing acceptance covers explorer mouse selection, split geometry, tab drag/drop, large paste,
  and PTY lifecycle. It does not cover the three workflows above. Status completeness and visual
  boundary quality lack an independent semantic oracle; split and large-file latency have retained
  measurements but no interactive user-quality threshold.

## Language and application inventory

The pure language contains unit, bool, checked i64, immutable bytes/text, nominal products/sums,
and nominal homogeneous sequences. Operations cover constants, calls, conditionals, counted loops,
nominal construction/matching, bytes/text/sequence primitives, holes, return, and yield. The only
execution route is the explicit-frame interpreter.

The application profile set is `typed`, `bytes_stream`, `stateful`, and `interactive`. Stateful
execution hard-codes declined/unchanged/completed/suspended variant maps and at most one host
command. Interactive profile mapping includes 24 action kinds: `none`, 18 semantic-project
operations, and five selected-filesystem operations. The general application artifact exposes only
the `immutable_blob_v1` host interface with `put_blob` and `inspect_blob`; typed visibility and
reconciliation outcomes are strong and reusable.

Runtime default policy: 8 MiB request, 32 MiB response, 256 MiB application, one loaded application,
one active transition, one active host operation, one compilation, one instance store, zero queued
requests, zero compiled-unit bytes, and zero cache bytes. The reported topologies are `one_shot`
and `foreground_session`.

## Native-policy inventory

- `src/bin/lkjedit.rs` embeds the exact application and owns product help/launch grammar, but most
  editor transition/render meaning remains in the semantic project. `src/workbench_host.rs` and
  `src/application/interactive.rs` nevertheless encode semantic-project and selected-filesystem
  product action vocabularies in universal native contracts. `src/selected_filesystem.rs` contains
  an editor-named temporary prefix.
- `src/bin/lkjwork.rs` (64,553 bytes), `src/bin/lkjwork/project.rs` (34,195 bytes),
  `src/bin/lkjwork/render.rs` (17,936 bytes), and `src/bin/lkjwork/bindings.rs` (10,828 bytes) own
  product command policy, data-shape mirrors, response rendering, locator/backup policy, and many
  task-shaped decisions. `bindings.rs` embeds the checked artifact.
- Generic protocol/runtime candidates are `src/application.rs`, `src/instance.rs`, `src/runtime.rs`,
  `src/terminal.rs`, `src/selected_filesystem.rs`, the compiler/IR/interpreter, and project
  persistence. Their current contracts are still coupled to profiles or single-operation topology.
- Historical commit `47cb2efb` removed a 4,869-line private `lkjwork` graph builder and generated
  bindings. Commits `d52f7cda` and `7ffaf514` deliberately selected direct instance operation and a
  synchronous zero-queue kernel. Those choices were sound for their measured consumers, not proof
  for a resident service.

## Generality and gap matrix

| Mechanism | Current classification | Interactive | Request service | Durable worker | Decision pressure |
|---|---|---:|---:|---:|---|
| Exact requirement/grant split | general invariant | yes | yes | yes | retain, broaden interfaces |
| Typed visibility/reconciliation | general invariant | save | DB/object/HTTP | queue/provider | retain in closed failure classes |
| Full typed graph as sole authored authority | semantic-development choice | verbose | unproven | verbose | prototype graph modules vs canonical source/AST |
| Full snapshot per revision | simple exact persistence | 51.7 MiB backup | growth risk | growth risk | compare content-addressed modules/manifests |
| Closed invocation profiles | product-shaped | yes | no | partial | replace with typed components/ports |
| One-command suspension | narrow state machine | workable | transactions awkward | retries awkward | compare typed tasks and effect plans |
| Product interactive actions | product-shaped native policy | editor | no | no | replace with interface operations |
| Pure explicit-frame interpreter | strong oracle | slow | throughput risk | CPU risk | retain oracle; prototype prepared bytecode |
| Synchronous zero-queue kernel | exact but insufficient topology | foreground | impossible concurrency | no workers | structured bounded resident runtime |
| Selected filesystem | standardized generic mechanics | yes | backup/import | batch | retain after capability cutover |
| Terminal adapter | standardized generic mechanics | yes | no | no | retain as runner capability |
| Immutable blob | generic narrow capability | attachments | content/cache | results | retain beneath object storage where natural |
| Modules/packages as semantic nodes only | incomplete reuse model | monolith | monolith | monolith | first-class exact packages and task context |

Universal contracts must not contain editor buffer/tab/mode/explorer vocabulary, work-ledger
task/priority/label vocabulary, or service route/table/user/note/media vocabulary. HTTP,
PostgreSQL, object storage, clocks, secure randomness, hashing, terminal I/O, and filesystem access
qualify through standardized external contracts plus a complete consumer. Task/effect/component
features must demonstrate at least interactive, request, and worker shapes.

## Preselection gates recorded at baseline

The following were the preselection gates. The resulting selections and implementation evidence are
recorded below rather than rewriting the original hypotheses.

1. **Authored authority.** Compare (a) graph authority with module-local documents, (b) canonical
   textual modules lowered to the semantic model, and (c) canonical structured AST modules. Reject
   dual editable truth. Measure eight equal authoring tasks, local bytes, parse/validate time, diff,
   rename/move, merge, and fresh reconstruction. Reverse a selected source/AST design if it cannot
   reconstruct every semantic owner independently or makes ordinary changes larger than the graph
   projection.
2. **Persistence.** Compare full snapshots, immutable module/object manifests, and journal plus
   checkpoints using the current editor and a representative service-size package. Retain exact
   one-publication behavior and deep reconstruction. Reverse added object complexity if current
   open/apply/backup does not materially improve or crash repair is less legible.
3. **Language abstractions.** Prototype only constructs required by routing, strict codecs,
   transactions, reusable collections, interactive state, and worker state machines. Remove any
   feature that does not reduce an equal complete program or lacks deterministic bounds.
4. **Effects.** Compare explicit commands, batched plans, resumable typed task IR, and capability
   calls from effectful functions. Ambient native calls are rejected. A transaction must remain one
   task-scoped live resource; durable continuations must contain only serializable facts.
5. **Application model.** Compare adding profiles with one component graph exposing typed ports,
   state owners, requirements, handlers, and runner descriptors. Select the latter unless a
   complete shape cannot preserve a stronger invariant. Old profile execution must be deleted on
   cutover.
6. **Resident runtime.** Prototype bounded structured task scopes over one semantic execution path.
   Stop if queues/resources lack exact owners or scheduler order enters pure results. Use mature
   native asynchronous mechanics only behind this contract.
7. **Execution tier.** Measure an optimized explicit-frame dispatch and compact bytecode against
   equal editor and service workloads. Keep the interpreter as oracle. Retain bytecode only if a
   dominant complete workflow improves materially without weaker fuel/resource accounting.
8. **Service vertical.** Choose the smallest independently useful actor-aware Markdown resource
   service that forces HTTP, strict JSON/safe HTML, persistence/transactions, configuration,
   secret-backed authentication, time/randomness, streaming, and one object/worker workflow. Keep
   all routes, schemas, authorization, rendering, and retry policy in lkjscript.

## Baseline absences and campaign blockers

There is no current source parser, exact dependency resolver, general map/set/iterator or closure,
effect/task IR, unified component artifact, resident service topology, stream/resource type, HTTP
server/client, strict application JSON/URL/form/multipart/HTML/Markdown package, relational
capability, PostgreSQL adapter, typed deployment configuration, secret capability, clock, secure
randomness interface, identifier/password API, object storage, durable generic queue, structured
logging/metrics, or production execution tier. These are implementation gaps rather than reported
blockers. External PostgreSQL/S3 availability has not yet been assumed; unavailable live services
will be reported and cannot be converted to integration success.

## Selected architecture and direct-cutover result

The bounded prototypes selected one canonical textual authority, one package/component artifact,
one pure/task language, and one bounded resident kernel. The old normal paths were removed rather
than retained as compatibility.

| Decision | Selected design | Losing alternative and reversal gate |
|---|---|---|
| Authored authority | canonical UTF-8 `.lkj` modules plus strict package metadata; derived typed semantic model | Graph authority required whole-graph edit/context and private structural operations; a maintained AST retained similar payload. Revisit only if source cannot independently reconstruct meaning or exact structural refactors require declared continuity. |
| Persistence | content-addressed source/dependency objects, immutable manifests/records, one atomic HEAD | Full snapshots repeated unchanged modules. Revisit after 10,000 modules or 250 ms current open/apply p50; retain exact reconstruction and one publication owner. |
| Language | small records/variants/lists/ordered maps/options/results/functions plus pure/task/transaction forms | Broad conventional feature import and low-level graph repetition were rejected. Add a feature only from two consumers or a standard boundary plus complete consumer. |
| Packages | explicit package/module IDs, exports/imports, exact digest-bound dependency DAG | Paths and mutable versions are locators only. No registry is needed for local deterministic composition. |
| Application | components with typed ports, declared requirements, and runner-kind targets | Adding service/worker profiles would preserve parallel semantic worlds. Component artifacts directly reject predecessor profiles. |
| Effects | pure functions plus statically closed task capability calls and lexical transaction scopes | Ambient native calls and opaque product commands were rejected. Durable continuations remain deferred because bounded job payloads suffice. |
| Runtime | one prepared bounded resident deployment shared by in-memory HTTP, live HTTP, and workers | Per-request process and unbounded detached async were rejected. Revisit scheduling only if a maintained workload needs application-visible ordering. |
| Execution | compact prepared bytecode production route plus independent AST oracle | Interpreter-only preparation/dispatch was rejected as production architecture; JIT was premature. Retain bytecode only under differential equality and no equal-workload regression. |
| Service proof | actor-aware resource journal with snapshots, objects, and durable jobs | Synthetic echo tests and a full `kjxlkj` port were rejected. Product concepts remain only in authored application modules. |

Current source authorities are:

- `standard` revision 5, record
  `9766c0b08788b8326757091526fddd50bceb4ec618874e4023d0fc972a3e8c49`, semantic
  `d0fe09ea464351240e77248f35b293653b198b238f0688b05c372903cd04630a`;
- `lkjournal` revision 7, record
  `e85d84f4bd23c1cb768b8387805710939fd1347a6604bd2ac7d3886b2aaf7beb`, semantic
  `cdbd18f4be55897dcd27b6601cf03e818d666e8ca90789ad02d86f4c4a4bce53`;
- checked component artifact digest
  `eec4c68b121bfa4bdf4af2b01e712040d9e40907e92bf490e345da75bb682af4`, 41,587 bytes;
  and exact standard dependency digest
  `a09712fe34ccc0315fdf6e55bbddf8e4ba433093a140075fa2d704d3876a8cab`, 9,602 bytes.

The old graph project/protocol/schema, source-free proposal path, full-snapshot format, release and
application profile artifacts, stateful instance/runtime protocol, product-specific binaries,
interactive action vocabulary, vendored terminal stack, active `lkjedit` and `lkjwork` trees, tests,
fixtures, generated invariants, evidence, and predecessor documentation are staged for deletion.
The public source authority explicitly rejects the old `.lkjscript/project` marker. Git retains
historical recovery; no active-tree archive or current execution reader remains.

`lkjedit` and `lkjwork` were deliberately deleted rather than incompletely migrated. Keeping them
would have retained the profile worlds and substantial application-specific Rust that the component
cutover removes. Their baseline user value and editor defects above remain real, but none is claimed
fixed. A future interactive consumer must start from the component/capability model and may recover
product policy from Git without restoring a compatibility path.

## Implemented capability and service evidence

The `standard` exact package defines general interfaces for configuration, secret verification,
wall clock, secure randomness, UUID generation, password hashing, byte streams, PostgreSQL, object
storage, and durable queues. Component validation closes every transitive effect and rejects unknown
or forged native intrinsic signatures before publication. Deployment binds exact interface,
operation set, component maxima, adapter kind, sharing domain, authority revision, descriptor
digest, and concrete limits before admission.

`lkjournal` owns 11 routes or fallbacks, its schema/migration/SQL, strict JSON shapes, actor/session
policy, ownership denial, HTML structure, exact-base snapshots, object pending/reconcile policy,
enqueue behavior, and worker completion in three `.lkj` modules. A native-source regression rejects
its product vocabulary in `src/platform`. Deterministic black-box tests invoke the same prepared
HTTP component and worker ports with disjoint scripted/memory adapters.

The current live acceptance receipt
`.artifacts/service/20260821T171340.084371Z-131501/receipt.json` passed in 3.160614082 s using an
isolated cached `postgres:16-alpine` container. It exercised health, denied bootstrap, exact
migration, Argon2 actor initialization, login, strict create/list/read/update/stale/history,
unauthenticated denial, 200,000-byte streamed local object publication, one PostgreSQL queue claim
and completion, graceful service/worker shutdown with zero cleanup failures, a 12,520-byte
`pg_dump`, restore into a second database, restart, and exact revision-1 read.

Repeated acceptance exposed a retryable first-use PostgreSQL connection race and then the sync
driver's nested-runtime panic when readiness was moved to deployment binding. The retained fix
establishes and configures each PostgreSQL grant on a contained blocking thread, retries only the
no-publication connection class within the declared wait bound, and publishes readiness only after
one reusable connection exists. Ten consecutive fresh database/service/worker/restore workflows
then passed, from `.artifacts/service/20260821T171057.675379Z-119920/receipt.json` through
`.artifacts/service/20260821T171137.972457Z-126746/receipt.json`.

The local object adapter conformance found that `object_store::local::LocalFileSystem` rejects
provider attributes. The retained contract validates content type but does not attempt to persist
provider attributes for local storage; memory and S3 adapters do. This is an explicit deployment
limitation, not application policy. Live S3 conformance remains unavailable and is not reported as
passing.

Thirty-one warm release package-test samples timed the same expressions in both execution routes.
`standard` bytecode p50 was 22,732 ns versus 28,895 ns for the AST oracle (21.3% lower);
`lkjournal` bytecode p50 was 88,997 ns versus 134,712 ns (33.9% lower). This retains bytecode under
the selected directional gate while keeping the independent oracle. The samples are too small for a
throughput or tail-latency claim; exact ranges and environment are in `docs/performance.md`.

Current direct dependencies are `argon2`, `axum`, `base64`, `blake3`, `bytes`,
`fallible-iterator`, `fs2`, `futures-util`, `getrandom`, `object_store`, `postgres`, `serde`,
`serde_json`, `tokio`, and `tower`, plus test-only `tempfile`. The predecessor direct terminal,
signal, Unicode-layout, and rustix dependencies and vendored crossterm patch were removed. Rustix
may remain transitive. First-party Rust still forbids `unsafe`.

## Current limitations after cutover

The implemented service foundation does not include outbound HTTP, response streaming, trailers,
multipart, complete URI/form/cookie codecs, Markdown parsing/sanitization, typed multi-context HTML,
compression, WebSocket, TLS termination, PostgreSQL TLS, live S3 conformance, general filesystem or
terminal capabilities, structured log/metric exporters, trace propagation, calendar scheduling,
or a hostile-code sandbox. HTTP bodies stream into tasks, while responses are bounded whole bytes.
The maintained application stores Markdown-like text but does not render Markdown. These surfaces
have no dormant public format and remain evidence-gated in `docs/roadmap.md`.

There is no equal-task provider request/token/correction-depth comparison between the predecessor
graph workflow and source workflow. Current orientation and module output bytes are measured, but
program size and task differ, so no token or cost saving is inferred.

## Facts differing from the campaign prompt

- The audited commit, root-policy digest, principal format/profile facts, and approximate
  performance values reproduced.
- Explorer keyboard movement exists at revision 174, but its selection is invisible; only the
  stronger historical claim that number shortcuts are the sole route did not reproduce.
- The measured 10,000-event run was 27.7686 s and the growing-insert run 4.4055 s in this
  environment, close to but not identical with the retained historical observations.
- `/usr/bin/time` is unavailable; process RSS remains unavailable.

## Publication result and evidence closure

The dependency-closed implementation commit
`349127e11668d7cd5974fd1be087197d2709c77b` was pushed as a fast-forward from
`cc9b465227237a1600e9f2cb4e8e7f85ae59093a` to `origin/main`; local, upstream, and remote refs were
verified equal. No force push, amend, rebase, merge, or release occurred. The unrelated
`package-lock.json` remained unchanged and unstaged. This retained correction records the later
discovery of the predecessor-log retention loss; its final commit and remote-ref verification are
reported in the campaign handoff.
