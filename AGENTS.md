# Repository guidance

## Mission and authority

Build a powerful general-purpose, agent-first language. Prefer reusable abstractions and precise
public discovery over application-specific host behavior. Maintained applications are consumers and
witnesses, not owners of language priorities. A newly authored workload can establish a foundation's
value; distinguish it from existing adoption. Ordinary application development should use the
distributed executable without a contributor checkout.

One accepted revision of the typed semantic graph owns editable program meaning. Requests, textual
views, indexes, caches, compiler representations and artifacts derive from it; none is a competing
editable program. Operational data, deployment configuration, credentials and running services have
separate authorities. Executing effects does not silently advance semantic HEAD.

Follow the applicable instruction hierarchy and current explicit user direction. A supplied mandate
may revise engineering guidance with a rationale, preserved guarantees and affected-owner cutover;
it cannot override higher-priority instructions. Distinguish instructions, normative contracts,
observations, hypotheses and implementation decisions. Investigate contradictions. Historical
limitations, assistant recommendations and repeated wording are not automatically permanent policy.

## Navigation and ownership

- `src/platform/` owns semantic authority, public control, compiler/runtime and host adapters;
  `src/bin/lkjscript.rs` owns process entry. `tests/` owns public black-box acceptance.
- `packages/standard/` owns maintained standard graph meaning and derived assets;
  `applications/lkjournal/` owns the maintained application, not language policy.
- `tools/lkjscript-dev/` owns contributor checks, independent oracles, scale, target admission,
  release preparation and evidence. Workflows orchestrate these owners rather than duplicate policy.
- `docs/spec/` owns normative contracts; `docs/architecture.md` maps dependencies; `docs/status.md`
  records proved state; `docs/roadmap.md` holds contingent opportunities; `docs/decisions/` records
  rationale. `docs/performance.md` and `docs/evidence/` retain bounded evidence. `docs/release.md`
  owns distribution procedure. Generate `docs/generated/` through its executable owners.
  `docs/campaigns/` holds initial mandates and appended lifecycle records, not a second task system.

## Startup and work safety

Read the supplied mandate, this guide and applicable ancestor, global, scoped and override guidance.
Check actual instruction discovery and size limits when relevant. Replacing a file does not reload
instructions into a running session; reconcile consequential changes explicitly without changing
unrelated global configuration.

Establish actual branch, commit/tree, index/worktree, relevant untracked files, stashes, remotes,
divergence, toolchain, resources and active external actions. Read complete terminal/resumption
records for potentially active work. Use the explicitly supplied mandate and lifecycle records,
not the newest filename. A historical baseline is not permission to rewind. Missing historical
scratch logs or a source/public version difference alone does not reopen completed work.

Preserve unrelated work and operational data. Stage explicit paths and inspect the staged diff;
avoid broad `git add .` or `git add -A`. Do not reset, clean, broadly restore, rewrite history or
remove stashes to manufacture cleanliness. Use disposable owned resources for destructive tests;
account for and clean up only your own processes, ports, containers and files. Keep secrets, large
logs and `.artifacts/` contents out of tracked narrative. Diagnose failures before retrying.

## Semantic and engineering guarantees

Use the pinned toolchain, manifests and lockfile. Preserve safe-Rust and panic/unwrap/expect/todo/
unimplemented lint boundaries and narrow test allowances. Contributor semantic tooling belongs in
the first-party Rust owner, not a parallel Python or shell implementation. Admit external inputs
through the applicable version/digest owner; avoid unrelated dependency or formatting churn.

Typed changes converge on `GraphRepository`. Plan and apply share normalization, allocation,
validation, selected tests and logical review meaning. Validate the complete candidate, recheck
its base under publication ownership, durably prepare canonical content, then expose accepted
meaning atomically. Stale, malformed, interrupted, cancelled or exhausted preparation cannot
partially publish. A post-acceptance derived-work failure is not a failed semantic write.

Use checked arithmetic and appropriate bounds before allocation, traversal and output. Canonical
encoding and identity must be deterministic, not dependent on enum order, incidental layout, host
paths, clock time or hash iteration. Regenerate maintained assets through canonical producers;
never hand-edit generated meaning or contracts.

Bind derived reuse to exact relevant revisions, dependencies, contracts, options, targets and
policies. Clean recomputation must detect invalid reuse. Do not repair canonical corruption by
trusting a cache. Strictly admit transported and loaded artifacts before use. Keep one maintained
production path after cutover; independent test/reference implementations are not alternate
editable authorities.

Preserve exact type, effect, capability, lifetime and resource provenance through optimization.
Retain checked raw admission, cancellation, cleanup and promised evaluation order. Distinguish
expected results, traps, exhaustion, corruption, infrastructure failure and possible external
visibility. Transactions roll back only within their actual authority. Never replay live effects
through another evaluator merely to compare results.

## Evolution and performance

Backward compatibility is not the overriding objective. Separate logical identity, semantic
revision, durable schema, encoded format and runtime representation. Preserve meaningful continuity,
not every incidental byte forever. A breaking change needs identified consumers, regeneration or
migration, explicit rejection/recovery and retirement of replaced production paths. Live data is
not disposable derived state. Temporary bridges need removal conditions.

Optimize against a concrete opportunity, with profiling and meaningful baselines. Distinguish
profiling, cancellation, operational quotas, defensive limits and explicit execution fuel. Ordinary
trusted execution should not require choosing fuel as a permanent language concept. Existing limits
remain effective until an explicitly selected policy change supplies a complete contract and proof.
Do not remove protections incidentally, infer sandboxing, or freeze evaluator instruction counts as
universal semantics. Optional instrumentation needs honest disabled-mode cost when relevant.

## Public behavior and informative proof

Discovery must describe actual operations, types, errors, limits and recovery. Adapters carry typed
intent, not raw-store authoring. Bound output and revision-pin queries/continuations; reject stale
or invalid requests rather than mix revisions or silently truncate. Keep generated discovery and
negative behavior synchronized with executable owners.

Prove new product behavior through freshly authored public requests using a copied candidate outside
the source checkout. Internal constructors, frozen examples and host unit tests alone do not prove
ordinary authoring. Mark source-bound probes separately from product execution. Static linkage and
named target tests do not establish universal portability, hostile-code isolation or transport security.

Use `tools/lkjscript-dev/src/check/` and `docs/spec/verification.md` for authoritative gate selection,
dependency closure, fingerprints and reuse. Verified entry points include:

```sh
cargo run --release --locked -p lkjscript-dev -- check focused --machine
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
cargo build --release --locked -p lkjscript-dev
target/release/lkjscript-dev release target
```

Iterate narrowly; after relevant inputs stabilize, run required authoritative full closure plus
separately required candidate, target and distribution acceptance. Discover additional arguments
from their owners. Do not weaken gates, reuse invalidated evidence, or rerun unchanged expensive
checks without an invalidation reason. A host full receipt cannot replace exact-target evidence.

Use independent expected results, disjoint references, malformed inputs and relevant fault injection.
Identify shared machinery that makes agreement insufficient. Measurements retain preparation,
steady-state work, allocation, instrumentation, failures and trade-offs when relevant. Do not hide
fallback execution, move work outside measurement, drop checks or claim a universal speedup.

Classify proof as fresh passed, validly reused passed, failed, skipped, unavailable or not run using
owned receipt states. Required missing proof blocks the affected claim. Bind evidence to actual
source/candidate/verifier/workload/environment inputs. Retain concise reproducible records that
survive missing ignored logs. Avoid self-referential report hashes and endless report/retest loops;
report final containing identities separately. Never label an unexecuted final check passed.

## Integration and completion

External authority comes from the current mandate, not repository access or old campaigns. Local
commits, normal pushes, workflow actions, publication, settings changes, deployment and live-data
mutation are distinct actions. Before an authorized push, refresh remotes and inspect divergence,
review requirements and automatic effects. Never force-push, bypass protections or replace published
identities. Release-control exceptions must be explicit, narrow and campaign-bound.

A selected release completes only after its required immutable publication and independent public-
download acceptance. A build, tag, upload or dispatched workflow is not verified delivery. Preserve
older published assets; use additive recovery. Release authority does not authorize deployment,
spending, credential replacement or operational-data changes.

Complete the fixed outcome, affected-consumer cutover, predecessor removal, documentation, required
proof and authorized integration/release disposition. Preserve the initial mandate body; change only
permitted lifecycle metadata and append reconciliation, terminal or resumption records. Report actual
starting/final identities, commits, evidence, deviations, limits, preserved work, cleanup and external
actions. Adapt legitimate intervening progress without silently changing the objective or lowering
acceptance. Verify an already satisfied outcome instead of reimplementing it. When blocked, preserve
useful safe work, stop the dependent unsafe action and state the smallest real resumption condition.
