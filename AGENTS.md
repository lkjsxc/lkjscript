# Repository guidance

## Mission and authority

Build a general-purpose language in which humans and agents can author, compose, maintain and distribute substantial programs through the product executable. Prefer coherent semantics and ordinary libraries over application-specific compiler behavior. Architecture is revisable with evidence, preserved guarantees and an explicit transition; historical campaigns and incidental implementation limits are not permanent user requirements.

The accepted typed meaning graph is the current canonical editable program authority. Input notations propose changes; projections, names, indexes and compiled products have different roles. Operational data, secrets, installations and deployment settings have separate owners.

Follow current user direction and higher-priority instructions. For delegated implementation campaigns, normal non-forced pushes to main and ordinary configured CI/release use are authorized within actual permissions and repository protections. Accepted work should reach remote main, not stop on a topic branch or at PR creation. This does not authorize production deployment, destructive operational-data changes, purchases, billing or credential changes, access-control changes, protection bypasses or rewriting published history. Replacing this file does not reload an active session's instructions.

## Safe startup and working lineage

Inspect actual HEAD/branch, index/worktree, relevant untracked work, stashes, worktrees, remotes, divergence, applicable guidance and active local/remote jobs before editing. Read the mandate and relevant predecessor endings. An architect's revision is an observation, never a reset target. Reconcile material obligations as completed, actionable, blocked or explicitly superseded. Check the pinned toolchain and available memory/disk before expensive work.

Preserve unrelated changes, operational state, secrets and immutable publication identities. Stage intended paths and review the staged diff. Do not use destructive cleanup or broad staging to manufacture a clean result. Redact credentials from observations. Destructive tests use owned disposable resources; join owned processes and clean up their resources without touching unrelated services.

Prefer main and normal mainline delivery when the checkout and collaboration permit it. Use isolation for a concrete reason, normally one mutable campaign lineage, and reuse an appropriate existing lineage for a continuation. A frozen release commit does not freeze main. Keep required release inputs fixed, but do not create a permanent reporting branch for every observation.

Read existing applicable guidance, reconcile a supplied root candidate with legitimate intervening instructions, and install it in the mutable checkout before substantive work. Never update frozen release inputs merely to add guidance. Archive the initial mandate unchanged under `docs/campaigns`; append concise, clearly separated reconciliation/execution/resumption records. Preserve both originals on a filename collision. Keep binaries, logs and `.artifacts` output out of tracked narrative.

Delegate bounded independent work with clear ownership and one responsible integrator.

## Navigation and ownership

Paths are relative to the repository root. Follow actual callers and narrower owners before editing.

| Area | Entry points |
| --- | --- |
| Public operations and authoring | `src/bin/lkjscript.rs`; `src/platform/cli.rs`; `src/platform/control/`; `src/platform/change/`; `src/platform/publication/` |
| Meaning, storage and inspection | `src/platform/kernel/`; `src/platform/storage/`; `src/platform/normalized_query.rs` |
| Libraries and compilation | `src/platform/package_interface.rs`; `src/platform/package_transport/`; `src/platform/compiler/` |
| Execution and application data | `src/platform/execution/normalized/`; `src/platform/execution/control.rs`; `src/platform/data.rs` |
| Runtime and distribution | `src/platform/runtime.rs`; `src/platform/deployment.rs`; `src/platform/installation/` |
| Discovery and maintained consumers | `src/platform/project_creation/`; `src/platform/contract/`; `src/platform/builtin_standard.rs`; `packages/standard/`; `applications/lkjournal/` |
| Contributor proof and delivery | `tests/`; `tools/lkjscript-dev/src/check/`; `tools/lkjscript-dev/src/release/`; `.github/workflows/release.yml` |

`docs/spec/` owns normative semantics, including language, semantic authority/storage/CLI, effects, data and packages. `docs/spec/verification.md` owns proof obligations; `docs/release.md` owns publication. `docs/status.md` holds concise current findings and `docs/roadmap.md` holds revisable direction. Link historical evidence to its existing owner rather than copying chronology into these summaries.

Generate accepted assets through their maintained owner. `src/platform/contributor.rs` is a read-only observation surface, not an accepted graph writer. Generate `docs/generated` through matching product discovery, not another hand-maintained grammar.

## Essential guarantees

Public authoring passes through typed intent, complete validation, review and publication. Preserve lexical/generic scope, visibility, typed references, authored order, identity continuity and review bindings. Name resolution grants neither mutation nor execution authority. Recheck under the publication lock and persist content before atomic exposure. Invalid, stale, cancelled or exhausted requests cannot partially publish meaning. Preserve accepted inputs and immutable results on idempotent retry; a failed derived operation does not undo an accepted write.

Preserve types, substitutions, provenance, effects and evaluation order through transport, compilation and execution. Admit the complete relevant closure, including unused arguments and unreachable syntax. Separate semantic validity from finite preparation capacity. Loaders independently admit producer content; caches bind actual context. Historical acceptance and current validity are distinct: never rewrite history or treat old receipts as present execution permission.

Callable identity, declared effects and deployment grants are distinct. Invocation requires current allowance and exact grants through imports, binding and tail calls. Pure execution cannot invoke a task callable even with an empty effect row. Preserve affine ownership, consumption, borrowing, capture safety and raw/retained-value admission.

Admit target/type/grant contracts and intrinsically unencodable external results before live adapters or secrets; reject invalid external arguments before effects. A transaction is atomic only within its real authority. Earlier independent effects may survive failure. Cleanup failure, missing output or cancellation does not prove rollback or safe retry. Do not replay live effects for differential verification. Join owned resources on every exit; dropping a future alone is not cleanup.

Ordinary trusted execution requires no invented instruction budget. Optional metering, cumulative quotas, live limits, input bounds, cancellation, deadlines and profiling have different purposes. Use checked accounting and admission before growth. Observational overflow is not quota exhaustion. Safe Rust, static linkage and resource limits do not establish a hostile-code sandbox.

Separate logical identity, semantic revision, encoding, application artifacts, runtime installation and application data. Canonical bytes cannot depend on addresses, physical placement, hash iteration, paths or wall time. Compatibility changes require detection, consumer analysis, migration or clear rejection, and viable recovery. Never silently discard live data or reinterpret settings. Bridges need identified consumers and retirement conditions.

Distinguish an installed runtime, runtime-dependent application artifacts and self-contained executables. Sharing installation bytes implies neither a daemon nor shared process fate. Installation/selection does not migrate application data or grant execution authority. Prove static distribution for the selected target rather than claiming universal portability.

## Development and verification

Use the pinned toolchain, manifests and lockfiles, safe Rust and production lint guarantees. First-party contributor semantics belong in existing Rust tooling. Preserve no-Python and product-surface gates unless the current task deliberately revises their policy with equivalent protection.

Maintained contributor entry points:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check focused --machine
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

These are alternatives, not a mandatory sequence. Check parsing/selection lives in `tools/lkjscript-dev/src/check/`; release arguments live in its maintained dispatchers and `docs/release.md`. `check changed` selects current Git-status paths, not a commit range. A clean-tree changed check does not prove a committed campaign. Full requires fresh evidence.

Discover product operations with `lkjscript capabilities` and maintained guides. New public capability requires fresh authoring and use through a copied candidate executable outside the language checkout, retaining literal inputs. A hidden host application or privileged storage edit is not a language capability. Keep discovery, generated guides, built-in assets and maintained consumers consistent.

Use focused feedback while iterating and dependency-complete acceptance once relevant inputs stabilize. Honor maintained gates; widen coverage when impact is uncertain. Do not run overlapping full profiles solely for reassurance. Concurrent Cargo builds can replace outputs through feature unification; respect existing gate dependencies and immutable executable copies.

Map material claims to independent expected results and meaningful failure cases. Changed machinery cannot be its own oracle. Use controlled reference adapters, not live-effect replay. Performance claims require equivalent behavior, a baseline, workload/environment, stage boundaries and limitations. Retain unfavorable results; elapsed time or output bytes alone do not establish model-token or billing savings.

Reuse evidence only with valid source, verifier, environment, workload and trust bindings. A cache is not a correctness certificate. Relevant changes or failures invalidate affected proof. Distinguish tested source, frozen release source, distributed bytes and later reporting/guidance commits; do not fabricate proof at a descendant or require a report to certify its own hash. Review and validate a descendant's actual changes separately. Report fresh, valid reused, failed, cancelled, skipped, unavailable and unrun states accurately.

## Main integration and branch completion

Complete behavior, affected consumers, generated assets, documentation, migration/retirement and required integration proof. Refresh remote main and protections before integrating. Incorporate intervening work without overwriting it and revalidate consequential interactions. Use normal fast-forward or merge-based delivery; if a PR is actually required, complete the permitted merge path rather than treating an open PR as completion.

Verify remote main contains the accepted result and record its actual revision and relation to the tested source. Squash/cherry-pick equivalence does not transfer exact-source evidence or establish ancestry. Do not force-push, weaken protections or pretend an untested integration commit was the original candidate.

Separate integration readiness from public-release readiness. A release-only external wait need not keep accepted development off main. Preserve correctness and maintained integration gates; revise an unnecessarily coupled ordering explicitly at its owner rather than silently ignoring it. Known product defects are not release-only exceptions.

Reconcile inherited temporary branches/worktrees. Remove clearly campaign-owned references only after confirming integration or justified supersession, preservation of all unique work, unchanged tips, and no active job or required reference dependency. Keep frozen refs while needed; retire them when that responsibility ends. Never delete unmerged work merely to reduce branch count or disturb unrelated stashes/worktrees.

If integration is blocked, preserve and reuse the outstanding lineage. Name the exact gate, permission, conflict or dependency, its source/run identity and concrete resumption action. A local commit, pushed topic branch or PR is not remote-main completion.

## Publication, waiting and return

Use the maintained release owner. Publish meaningful milestones, reconcile inherited due delivery and attach any deferral to an observable trigger. Repository publication and production deployment are separate. Serialize shared release identities, inspect matching attempts before dispatch, and reuse healthy work. Preserve immutable tags/assets and genuine failed history; defective published releases normally require additive correction.

Public delivery requires the prescribed anonymous acquisition, distributed/installed behavior and matching original-reader admission. A green subjob or upload is insufficient. Keep originals long enough for their actual resumption need. Do not change frozen inputs or restart a healthy pipeline merely to append a report.

Do useful independent work while remote jobs run. No indefinite watches, sleep/status loops, repeated unchanged logs or hidden polling. Inspect at meaningful work boundaries or for concrete diagnosis. If only external completion remains, return source, run/attempt, observed state, missing gate, retained evidence and the next action. Do not promise unattended monitoring.

Return what became possible, actual revisions/actions, proof and limitations, compatibility impact, remote-main status, public-release status, cleanup/retention reasons and exact blockers. Keep campaign IDs, run identities, measurements, acceptance matrices and incident history out of this root file.
