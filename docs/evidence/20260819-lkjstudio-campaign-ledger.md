# Semantic workbench and interactive software campaign ledger

This ledger records reproduced checkout facts, frozen workflows, decisions, and verification for the
active `lkjstudio` campaign. It is evidence and handoff state, not semantic authority. Measurements
are observations only and are updated after the named command completes.

## Starting checkout and effective instructions

- Audit date: 2026-08-19 UTC.
- Branch: `main`.
- Starting commit: `47cb2efbf3aeb6ff3e76bd7519fc4770070dfa81` (`Complete semantic
  development repository campaign`).
- Starting commit relation: exact audited campaign baseline; parent
  `d9f2993d7335c9e177b5f0ed34247bf6a49595ea`.
- Initial worktree changes: modified `AGENTS.md`; untracked `prompts/202608192013.md`.
- Root-policy SHA-256: `5024e819ba82bb7702ba45dc482b58e7e552e1926dee4377644bcc036d3a4a3d`.
- Root-policy size: 46,181 UTF-8 bytes across 1,159 lines.
- Campaign-prompt SHA-256: `77165302a567d5e62a7dfff34798c785cdd39b15329d76aa6135e7a531082171`.
- Campaign-prompt size: 212,678 UTF-8 bytes across 5,082 lines.
- No deeper `AGENTS.md` exists in this repository.
- The root policy and campaign prompt are user-delivered inputs. They are preserved and are not
  attributed to campaign implementation.
- Cargo metadata reports one stable Rust 2024 package with the library, `lkjscript`, `lkjwork`, and
  seven integration-test targets. Locked runtime dependencies are `base64`, `blake3`, `fs2`,
  `getrandom`, `serde`, and `serde_json`; `tempfile` is development-only.

## Reproduced baseline authority

The checked `applications/lkjwork` semantic project is healthy at workspace
`6894b57786a7e1ef14370d2da3a3cf33`, revision 7, snapshot
`346b634f111f2e44f78d00d92869b442f2178f57bb27dc59a65e43d9d1487e01`, and revision record
`fca34a0b75c75be765e3844fdf9a0fa3bb4f304ff173f8fa0df2dd58ef69b5d9`. It contains 3,339 graph
nodes, 546 durable identities, 2,793 function-local references, three targets, seven passing target
cases, and no completeness blockers.

| Boundary | Active identity | Direct predecessor rule |
|---|---|---|
| workspace protocol / machine schema | 12 / `lkjscript-machine-schema-v12` | 11 and older reject |
| semantic project / marker / change / session | 1 / `LKJPROJ1` / 1 / 1 | every other version rejects |
| revision record | 1 / `LKJREC01` | every other version rejects |
| workbench / context / edit document | 2 / 2 / 1 | context 1 and `plan` reject |
| workspace artifact / HEAD | 8 / `LKJTSM\0\x08` / `LKJHDA10` | format 7 / `LKJHEAD9` and older reject |
| build target | 1 | every other version or kind rejects |
| reusable release | 2 / `LKJREL\0\x02` | format 1 and older reject |
| application | 5 / `LKJAPP\0\x05` | format 4 and older reject |
| durable instance | 3 / `LKJINS\0\x03` | format 2 and older reject |
| runtime session | 2 | every other version rejects |

The three current targets are release `:529`, application `:530`, and product `:531`. Public target
build reproduces the checked 167,848-byte application exactly. Its file SHA-256 is
`f9b335db22fbecdacdf7047f8a8e8aa7711d030eccaf3ed42d3eb2783b3cc184`; semantic application digest
is `4eb891dc2f400e070d8feaf3ff8aa14e35420010d2ded3ace1a107cec8e45092`.

## Executable ownership map

- `src/project.rs`: project discovery, selected authority, project reads, project changes, backup,
  response preflight, and target operations.
- `src/transaction.rs`, `src/validate.rs`, and `src/schema.rs`: the one proposal normalizer, complete
  candidate validator, and closed semantic vocabulary.
- `src/history.rs`, `src/persistence.rs`, and `src/diff.rs`: canonical immutable revision history,
  publication/recovery, and exact semantic diff.
- `src/workbench/context.rs`, `document.rs`, and `view.rs`: current context capsule, editable
  proposal, and derived views.
- `src/target.rs`, `src/release/`, and `src/application.rs`: target meaning and deterministic release
  and application derivation.
- `src/compile.rs`, `src/core_ir.rs`, and `src/interpret.rs`: lowering, independent verification, and
  the explicit-frame execution oracle.
- `src/instance.rs`: durable application state, queries, grants, host attempts/outcomes, and replay.
- `src/runtime.rs` and `src/runtime_protocol.rs`: topology-neutral synchronous execution and the
  caller-owned foreground session.
- `src/bin/lkjscript/project.rs`: public one-shot and foreground project CLI projection.
- `src/bin/lkjwork/`: product boundary conversion, locator/grant adaptation, and safe rendering;
  task policy remains semantic application meaning.

No current owner exists for an interactive application profile, terminal events/frames/lifecycle,
editor buffers/cursors/selections/undo/search, selected filesystem grants/read/save/reconciliation,
or a semantic-project host grant. These are campaign boundaries, not implicit extensions of the
current blob interface.

## M0 reproduced verification

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo test --workspace --all-targets --all-features --locked`: passed; 250 tests passed and three
  explicitly manual tests were ignored.
- `cargo build --workspace --release --locked`: passed using the already warm target directory.
- `git diff --check`: passed.
- `lkjscript doctor --project applications/lkjwork --deep`: passed at revision 7.
- `lkjscript target test lkjwork --project applications/lkjwork`: seven cases passed.
- Two-path target derivation check: a fresh no-overwrite target output compared byte-identically to
  `applications/lkjwork/lkjwork.lkja`.
- `applications/lkjwork/acceptance.py`: passed; final product revision 30, final state digest
  `5b90b58b24938161480a72c6b0e334d6cba527a23064af5a2cae43cf88d20749`, with pure-query no-write,
  unknown-blob reconciliation, backup/restore, corruption rejection, and installed-binary checks.
- Functional frozen corpus: passed; final product revision 85, final state digest
  `2bb4b8069beb70ef3da69d5efc3229a458158e3b90adbf08f62458fdb85f4602`, 86 records, and 586,630
  retained bytes.
- Provider/model/token/cache/price telemetry is unavailable from repository workflows. No token or
  monetary claim will be inferred from bytes.

## Reproduced authoring baseline

The revision-7 orientation is 1,228 response bytes and its exact known-digest reply is 252 bytes.
The historical targeted refactor command remains:

```text
lkjscript context --purpose refactor \
  --target 6894b57786a7e1ef14370d2da3a3cf33:544 \
  --project applications/lkjwork
```

It returns exactly 53,811 bytes. Within the context payload, 64 aliases encode to 9,946 bytes and 64
expanded nodes encode to 40,949 bytes. The node set is truncated with 44 discovered frontier nodes
omitted. Transaction-operation and expression-form catalogues together encode to 870 bytes. This
establishes the baseline question: a selected function task pays mainly for a broad owner/nominal
closure and duplicated alias records, not for legal-edit vocabulary or the outer envelope.

The prior `why TASK` dogfood used three accepted change inputs totalling 13,796 bytes, three apply
processes/engine opens, one target test, one target build, and zero rejected correction proposals.
Exact files-opened and provider telemetry were not retained.

## Frozen workflows and independent oracles

The campaign freezes workflows A–M from the active prompt. The primary early measurements are:

- A/B/H/I: exact project read, local edit, proposal validation/application, continuation, and stale
  draft; whole-function replacement and full candidate validation are the oracles.
- C/L: one cross-cutting `lkjwork` feature and its exact target closure; full eager target preparation
  is the oracle.
- D/G: 100 local edits and semantic navigation; full context and one-shot commands are the oracles.
- E/F/M: `lkjstudio` bootstrap, deterministic event replay, and one self-change; a simple flat editor
  reference model and sequential full-frame replay are the oracles.
- J/K: selected-root file open/save/conflict/unknown reconciliation; direct independently observed
  filesystem bytes are the oracle.

Every retained measurement records request/response bytes, calls, process/engine opens, validation
attempts, correction depth, elapsed time, retained bytes, and exact authority before/after when
observable. Byte observations remain bytes.

## Initial architecture candidates and stop rules

- Evolve the existing project/transaction owners. A duplicate privileged workbench API is rejected
  unless a distinct authority is discovered.
- Prototype a closed revision-bound graph projection and apply continuation. Retain them only when an
  equal complete task reduces calls, observation bytes, correction depth, or elapsed time without
  weakening preflight/idempotency.
- Retain exact-base local operation IDs and whole-function replacement unless a measured structural
  transform improves a complete edit. No durable function-local identity is planned.
- Start with pure foreground interactive state, a sequential event loop, full frames, one pending
  host action, flat canonical text, Unicode-scalar editor indices, and all symlinks rejected by a
  selected-root adapter. Each may be replaced only by complete-workflow evidence.
- Application update/render meaning must remain in the checked `lkjstudio` semantic project. Native
  code may own only terminal mechanics, selected host adaptation, artifact discovery, deployment,
  and independent assertions.
- Do not add a daemon, general async runtime, worker pool, persistent cache/index, broad filesystem,
  raw-ANSI application value, general source language, or compatibility path without crossing the
  active campaign gate.

## M2 semantic query, proposal, and continuation slice

The first retained CLI-economy slice adds one closed derived semantic-query owner. Query plans bind
workspace, exact revision and snapshot, projection, exact roots, page limit, opaque continuation,
and result digest. The retained projections are `summary`, `exact`, `children`, `function`,
`owner_chain`, `dependencies`, `incoming_uses`, `callers`, `callees`, `targets`, and `blockers`.
There is no query language, persistent index, cache, or new meaning authority. Continuations are
base64url-encoded canonical payloads authenticated by a domain-separated BLAKE3 digest and reject
projection, root, revision, snapshot, limit, or plan changes.

Observed revision-7 `lkjwork` response bytes from release/debug binaries built from this worktree:

| Observation | Response bytes | Returned facts |
|---|---:|---:|
| prior refactor context for `query_entry` | 53,811 | 64 expanded nodes plus 64 aliases; truncated |
| exact `function` query for `query_entry` | 33,133 | 53 nodes; complete |
| `callees` query for `query_entry` | 1,613 | 8 call facts |
| `targets` query | 1,016 | 3 target summaries |
| combined function/callees/targets observation | 35,762 | complete selected facts |
| ten-item first function page | 7,762 | 10 nodes plus a 487-byte continuation |
| ten-item second function page | 7,154 | next 10 nodes |
| unchanged exact function query | 391 | exact domain and result digest |
| generated `query_entry` proposal envelope | 12,550 | exact metadata and document |
| generated proposal document alone | 11,846 | packet-free whole-function replacement |

The selected function/callees/targets observation is 18,049 bytes (33.5 percent) smaller than the
old targeted context. The directly editable document is 41,965 bytes (78.0 percent) smaller. These
are byte observations, not token or cost estimates. The new generated proposal carries exact
workspace, revision, schema, function scope, durable references, and base-local draft symbols; it
does not require the context capsule or aliases. Parsing and validation still normalize through the
existing transaction owner. An unedited proposal deterministically rejects as semantic no-change.

Every published project change now also returns a bounded fixed continuation containing the exact
new revision and snapshot, revision-record digest, accepted-change and semantic-diff digests,
requested created bindings, changed functions, affected targets, explicit local-alias invalidation,
and its own digest. Validate-only returns no continuation. The continuation is capped at 64 KiB and
is included in response preflight before publication. Because its shape is fixed, it introduces no
new response-projection input or idempotency ambiguity.

Focused verification:

- `cargo check --workspace --all-targets --all-features --locked`: passed.
- `cargo test --locked workbench::query_plan`: three focused contract tests passed.
- `project_cli::semantic_query_and_function_proposal_are_bounded_and_base_exact`: passed. It covers
  paging, plan-mismatch rejection, known-digest reuse, packet-free proposal round trip, and stale
  proposal rejection after a public-CLI backup advances independently.
- `project_cli::public_project_workflow_discovers_validates_applies_reviews_and_recovers`: passed
  with validate-only continuation absence and published continuation facts asserted.
- `project_cli::superseded_project_change_document_and_session_versions_reject`: passed for all
  three direct predecessors.
- Manual one-shot and foreground-session `callees` results both contained the same eight facts.

The whole-function document remains the edit oracle. Existing local operand/operation operations
remain available, but no new structural address is retained yet; equal-task evidence has not shown
that another edit semantic is needed.

After this retain decision, the affected public boundaries cut over directly: project change
requests are version 2, editable documents are version 2, and the one-shot project envelope and
foreground project session are version 2. Their version-1 predecessors reject in a focused public
CLI test. The unchanged project marker/storage contract remains version 1; the raw engine protocol
and machine schema remain version 12 because neither boundary changed. Semantic-query and
project-change-continuation contracts are newly introduced at version 1.

## Verification state

- M0 checkout/instruction audit: passed.
- M0 baseline full repository gates: passed.
- M0 public semantic-project and `lkjwork` product workflows: passed.
- M1 frozen workflows/oracles: recorded; first executable query/proposal fixture passed.
- M2 query/proposal/continuation prototype: retained; focused equal-task observations recorded.
- M3 selected semantic CLI cutover: passed. Raw protocol 13, machine schema v13, project
  change/machine/session 2, document 2, query 1, and continuation 1 are the sole current forms.
- M4 pure editor core: passed through the checked `lkjstudio` semantic target and independent flat
  Rust reference model.
- M5 interactive profile and deterministic headless route: passed.
- M6 terminal runner: focused lifecycle/fault tests and pseudo-terminal acceptance passed.
- M7 semantic project host: same-owner integration, stale rejection, project action acceptance, and
  target actions passed.
- M8 selected filesystem: confinement, pagination, read, create/replace, conflict, injected unknown
  visibility, and all reconciliation classes passed.
- M9 complete workbench: target cases plus headless/live acceptance passed.
- M10 `lkjwork` dogfood: revisions 8 and 9 add the blocked-task summary field and semantic case;
  target/product acceptance passed.
- M11 workbench self-change: revision 46 moved workbench actions from Ctrl to Alt, preserving Ctrl
  editor shortcuts; target and live acceptance passed.
- M12 performance/recovery: passed; focused rollback/reconciliation observations, the revision-48
  release workload, and fresh-copy observations are recorded below.
- M13 direct cutover: passed; full tests reject direct predecessor commands, change/document/session
  versions, workspace artifact 7, and application artifact 7.
- M14 fresh-checkout proof: passed; structured receipt and exact digests are recorded below.
- M15 final handoff: passed; final repeated gates, exact limitations, evidence, and unperformed
  remote actions are recorded here and in the user handoff.

## Retained workbench authority

The checked `lkjstudio` project is workspace `6ee361b40e2ce5041d64321d79c3db0d`, revision 48,
snapshot `12898095ee151d9d0c6f46fdbd17838ed88febd17533c6c6badb731b1f4cf83e`, and record
`79837c3208dadc34e941192c54c1fcb2252260fe9b3d657055e09ee9ed1a3961`. It has 3,301 graph nodes,
359 durable identities, 2,942 function-local references, 59 tombstones, three targets, 29 passing
target cases, and no blockers. Its semantic repository contains 49 immutable revisions including
genesis in 101 authority files and occupied 3,615,856 file-payload bytes when observed.

The current target produces a 161,562-byte application. Checked-file SHA-256 is
`d7c89b503a8b3ca882495919105812ebf38735dd3e019daf2127f6b6cdd9e039`; application digest is
`74597b6fb8fbc38dd2d6191f979fe8e380af32de6bb5a085812dac436fe4b683`; graph digest is
`39e1d780bd08cd76562fe37ab308cb47696c7b67851da6d1aca8ef67cba7fe85`; root release is
`9ba6554e46709b5148236c55f37ec410b3d3ede6449e749b84429d71bb3675cc`.

All maintained application logic and target definitions were authored through public
`lkjscript change validate|apply --document` operations. Temporary Python scripts existed only in
`/tmp` to emit bounded public semantic documents and were not retained. No Python, shell, Rust,
macro, fixture, checked JSON, or generated source reconstructs the graph. `lkjstudio.lkja` is a
checked derived distribution artifact reproduced by target build.

## Retained architecture decisions

| boundary | selected | rejected alternatives and reason |
|---|---|---|
| interactive state | pure foreground, ephemeral | durable-per-event created inappropriate history; hybrid checkpoint added a second cadence; host-owned state moved policy out of meaning |
| event loop | sequential, one pending action | worker/async/daemon had no measured complete-product blocker |
| terminal | generic native runner over interactive artifact | product-specific policy and language raw terminal primitives duplicated authority |
| rendering | bounded full frame | row/cell patches added derived correctness state before output became dominant; raw ANSI was unsafe |
| editor content | flat nominal scalar sequence with checked slice/concatenate | application-level copy loops exhausted the retained paste fuel; line/piece/rope/gap owners remain unjustified and flat allocate-new results remain the oracle |
| cursor | Unicode scalar index | bytes risk invalid boundaries; graphemes require a versioned segmentation dependency; line-column duplicated position state |
| semantic editing | generated function document plus strict JSON | structured-only UI was less usable; new source language risked second authority; narrow verbs did not cover atomic graph work |
| continuation | fixed exact apply receipt | fresh global context costs another call; arbitrary delta/lease/index added projection and lifetime complexity |
| project integration | in-process adapter over the same `Project` owner | private graph/store calls were forbidden; subprocess session added process/framing overhead without an isolation benefit |
| filesystem | one pinned selected-root adapter | semantic-only lost selected-file story; product-specific access duplicated policy; broad ambient grant was forbidden |
| target validation | eager complete oracle | selective validation/cache lacked a measured 20 percent complete-loop win |

Editor/application meaning owns buffer allocation/nonreuse, scalar content, cursor/selection,
movement, insert/delete/replace, exact literal search, bounded undo/redo, viewport, active buffer,
dirty/pending status, action choice, outcome transition, exit, and frame content. Native code owns
only artifact/deployment parsing, typed value adaptation, terminal mechanics, exact project grant,
and selected-root filesystem mechanics.

## Boundary contracts and bounds

| boundary | contract | exact retained bound / publication meaning |
|---|---|---|
| interactive profile | 2 | rows/columns 1..=1,000; frame 131,072 scalars; paste 65,536 scalars; status 4,096 bytes; events publish nothing |
| headless replay | 3 | 10,000 events/actions and 8 MiB request; deterministic sequential frames/actions; publishes nothing |
| terminal | 3 | 8 MiB encoded frame and 10,000 actions; raw/alternate/paste/cursor cleanup; terminal output never changes host authority |
| editor reference | application + oracle | 100 buffers, 130,752 scalars/buffer after fixed chrome, 32 undo entries, 2,097,152 retained undo scalars |
| project host | 2 | 4 MiB request, 32 MiB response, 32 selectors at 512 bytes, 64-byte action ID; reads publish nothing, apply delegates one exact publication |
| workbench host | 2 | 1 MiB and 130,048 Unicode-scalar content; 4,096-byte opaque file token; origin is never rendered meaning |
| selected filesystem | 1 | path 32/255/4,096; directory 4,096 total/256 page; file 8 MiB; save publishes once or returns explicit unknown token |

The terminal adapter retains only key press/repeat, paste, resize, signal, EOF, and full frame. It
ignores unsupported key codes, release, focus, and mouse because the application has no consumer.
Controls render as U+FFFD, tabs use four-cell stops, combining marks at column zero gain a dotted
circle, and right-edge wide characters become U+FFFD. Cleanup attempts every acquired stage in
reverse and reports the first cleanup error after all attempts.

The filesystem adapter pins the selected directory and resolves UTF-8 relative components with
Linux `openat2` beneath/no-magiclink/no-symlink/no-mount-crossing flags. Reads bind content digest,
size, device/inode, and modification/change times. Save uses create-no-replace or expected-digest
atomic exchange, file/directory synchronization, explicit unknown visibility after possible rename,
and present/absent/conflicting/indeterminate reconciliation. No unknown save is retried.

## Dependency decision

The product adds `crossterm`, `signal-hook`, `unicode-width`, and direct `rustix` use. Crossterm owns
portable terminal mode/event/control mechanics; signal-hook supplies process-safe signal flags;
unicode-width supplies cell width; rustix supplies Linux polling, `openat2`, descriptor-relative
filesystem operations, atomic rename, and synchronization. Their exact current consumer is the
complete workbench. No large TUI/async framework was added.

Published crossterm 0.29 can spin on Unix PTY EOF because its mio reader accepted zero-byte reads and
suppressed non-WouldBlock errors. The exact crate is vendored under `vendor/crossterm` with one
documented narrow patch: zero bytes become `UnexpectedEof`, and other non-interruption errors
propagate. A PTY master-close acceptance now exits promptly with typed output-unavailable behavior.
The dependency patch is operational, not application policy.

## Dogfood receipts

`lkjwork` revision 8 added one durable `Summary.blocked` field and rewrote every exact constructor,
export, application profile mapping, target case, and native self-described rendering consumer.
Revision 9 added the `summary_reports_blocked_tasks` application case. Current authority is revision
9, snapshot `4ae531e41c93b69583ee14c44837ce1d5f6748e427d3a02f60c7164b2174eec1`, record
`fb28346946daef184262e06781ee42c537913d63ba313ef50834744458f30561`, 3,353 nodes, 547 durable
identities, three targets, eight semantic cases, and no blockers. The checked artifact is 170,545
bytes with SHA-256 `dbd9e2e3ec63e6291978ba0409428d75a657decc701bc9959b23ce893b8d26eb`.

The workbench self-change was one public document apply at exact revision 45. Revision 46 changed
two function bodies and one target definition in one accepted mutation; its record contains 271
semantic diff facts and affected all three target levels. Validation published nothing; commit
published once. The semantic target's 29 cases and native integration then proved Alt actions and
Ctrl editor undo/redo simultaneously.

The post-self-change dependency closure used two further public semantic changes. Revision 47
replaced the `replace_range` and `render_workbench` scalar-copy loops with the new checked sequence
slice/concatenate operations; record
`cd6599b0f582c9bd1bb773e080d01fd68de65c20fb7e80d031570b3fd99800b2` binds those two function
bodies. Revision 48 changed only the `core-application` target policy from 5,000,000 to the existing
100,000,000 deterministic-fuel ceiling; record
`79837c3208dadc34e941192c54c1fcb2252260fe9b3d657055e09ee9ed1a3961` binds that target-only
change. Both candidates validated without publication and each apply published exactly one revision.

## Focused verification after self-change

- `lkjscript target test core-application --project applications/lkjstudio`: 29/29 passed at
  revision 48.
- `cargo test --test interactive_application --locked`: 9/9 passed, including failed-step and
  failed-resume rollback checks at the exact frame bound.
- `applications/lkjstudio/acceptance.py`: headless determinism, non-terminal rejection, normal PTY,
  project-action PTY, signal PTY, and EOF PTY passed.
- Application target regeneration produced the checked 161,562-byte artifact exactly.
- Provider/model token classes, cache classes, dated prices, and billing formula were unavailable.
  No token or monetary claim is made.

## Revision-48 release workload

Raw observations are retained in
[`20260819-lkjstudio-workload.json`](20260819-lkjstudio-workload.json), SHA-256
`11f54bc1b08a5f482989f81f31cc377ec3906892495aad3e15bee46385a6e78b`. They used the optimized
runner on a warm Linux x86-64 host with 20 logical CPUs visible, did not drop the page cache, and
include process startup, artifact validation, JSON framing, semantic execution, and response
encoding. Only artifact-version and project-orientation observations have five samples; headless
rows are single complete-workflow observations and do not support per-event percentile claims.

| complete observation | exact result |
|---|---:|
| artifact version/validation | 11.075 ms median / 11.563 ms p95 / 15,405 response bytes |
| project orientation | 361.340 ms median / 368.737 ms p95 / 1,239 response bytes |
| exact `render_workbench` function query | 366.422 ms / 69,761 response bytes |
| exact `render_workbench` proposal | 360.316 ms / 189,752 response bytes |
| 10,000 mixed events | 27,145.253 ms / 9,999 changed / 0 actions |
| 1,000 growing inserts plus close | 10,962.479 ms / 1,000 changed |
| 100-buffer corpus plus close | 287.601 ms / 99 changed |
| one 65,536-scalar paste plus close | 804.825 ms / accepted once / 262,249 request bytes |
| 1x1 resize plus close | 18.318 ms / accepted bounded minimal frame |

The mixed replay is approximately 2.71 ms per input when total elapsed time is divided by 10,000,
but the harness did not observe individual event latency and therefore reports no event p50/p95.
The growing-insert result is the retained flat-text/full-frame optimization gate. It does not justify
a piece table, rope, patch renderer, worker, or execution tier by itself; a replacement must improve
the same complete workflow while preserving the flat sequence and full-frame oracles.

## Isolated fresh-copy proof

The structured receipt is
[`20260819-lkjstudio-fresh-checkout.json`](20260819-lkjstudio-fresh-checkout.json), SHA-256
`9f5572b8fb789c35448d93449b218f34f0bdf5f10bfd4ae13c15df8dd9978f11`. The source was copied to
`/tmp/lkjscript-fresh.3Gbjqr` without `.git`, `target`, Python caches, or engine locks. Sorted
maintained-file manifests matched at
`ffd9903d238c223bc318efa7fb7bdfe37aba8922f7769b18fb9c65d1346a2971`. The receipt itself is
excluded from that pre-receipt comparison to avoid a self-referential digest.

An initially absent Cargo target directory built the locked release workspace in 204 seconds. The
installed Rust toolchain and external Cargo registry/source caches were already present; repository
target artifacts were not copied, and the build log showed no network download. Shallow and deep
doctor reported both semantic projects healthy. Top-level target tests passed all 29 `lkjstudio`
cases and all eight `lkjwork` cases. Two distinct no-overwrite workbench outputs were byte-identical
to each other and the checked artifact.

The copy passed deterministic headless replay, non-terminal rejection, normal/project/signal/EOF
pseudo-terminal scenarios, selected-filesystem temp-root tests, the complete public semantic-project
validate/apply/history/recovery workflow, and `lkjwork` product acceptance. Direct predecessor
commands, project change/document/session version 1, workspace artifact 7, and application artifact
7 rejected. Application trees contain no `build.py`, `generate.py`, `bindings.json`, or `build.rs`.

## Final gate audit

The first complete test attempt exposed two bounded catalogue-maintenance defects: the public
machine descriptor had crossed the existing byte-slice `length` field with the new sequence-slice
`end_exclusive` field, and independent schema/wire sample tables still described 34 operations.
The executable byte-slice semantics had not changed. The descriptors were corrected, sequence slice
and concatenate were added to the independent wire corpus, and exact schema projection measurements
were refreshed to 1,244 manifest bytes, 121,966 selected-agent-root bytes, 175,411 full bytes, and
105 unchanged bytes. Focused strict-Serde/schema tests passed before the complete rerun.

Final main-worktree gates:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo test --workspace --all-targets --all-features --locked`: 292 passed, three explicitly
  manual performance tests ignored, zero failed.
- `cargo build --workspace --release --locked`: passed.
- `git diff --check`: passed.
- Complete `lkjstudio` and `lkjwork` release acceptances: passed.

After all verification completed, the campaign-scoped tree, including the user-provided root
`AGENTS.md` and campaign prompt, was committed locally at the user's request so the worktree was
clear. The exact commit is the repository commit containing this evidence and is deliberately not
embedded here because doing so would be self-referential. No push, pull request, merge, release,
remote mutation, or deletion of those delivered artifacts was performed.
