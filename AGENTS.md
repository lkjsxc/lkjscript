# Repository guidance

## Mission and authority

Build a powerful general-purpose language and ecosystem for authoring, composing, evolving and distributing substantial programs through the product executable. Optimize for precise, discoverable operations that agents can use reliably and people can inspect and direct. Do not couple the language to a particular model or confuse a shorter notation with a better authoring workflow. Prefer general mechanisms and ordinary libraries over application-specific compiler behavior.

Architecture is revisable with evidence, preserved guarantees and a deliberate transition. Historical campaigns, assistant prescriptions and incidental implementation limits are not permanent user requirements. The accepted typed meaning graph is the current canonical editable program authority; proposals, projections, names, indexes and compiled products have distinct roles. Operational data, secrets, installations and deployment settings have separate owners.

Follow current user direction, applicable guidance and higher-priority instructions. Delegated implementation campaigns authorize normal non-forced pushes to main and ordinary configured CI/release use within actual permissions and protections. Accepted work should reach remote main, not stop at a local commit, topic push or open PR. This does not authorize production deployment, operational-data destruction, purchases, billing or credential changes, access-control changes, protection bypasses or rewriting published history. A replacement of this file does not reload an active session's instructions.

## Safe startup and one working lineage

Before editing, inspect actual HEAD/branch, index/worktree, relevant untracked files, stashes, worktrees, remotes, divergence, applicable guidance and active local/remote jobs. Read the current mandate and relevant predecessor endings; reconcile material obligations as completed, actionable, blocked or explicitly superseded. An architect's inspected revision is an observation, never a reset target. Check the pinned toolchain and available memory/disk before expensive work.

Preserve unrelated work, operational state, immutable publication identities and secrets. Stage only intended paths and review the staged diff. Do not use broad staging or destructive cleanup to manufacture a clean result. Do not expose credential-bearing URLs, environment values or secrets in reports. Destructive tests operate only on owned disposable resources; join owned processes and clean up their resources without touching unrelated services.

Prefer main when checkout ownership, collaboration and integration requirements permit it. Use isolation for a concrete reason, normally one mutable campaign lineage, and reuse the appropriate existing lineage for a continuation. A frozen release source does not freeze main. An immutable event SHA or tag may supply the required source identity without a permanent reporting branch.

Read existing instructions before reconciling a supplied root candidate. Install the reconciled candidate in the mutable checkout before substantive implementation; never change frozen release inputs merely to add guidance. Archive the initial mandate unchanged under `docs/campaigns/`, followed by clearly separated concise reconciliation/execution/resumption additions. Preserve both originals on an actual filename collision. Keep binaries, logs and `.artifacts` output out of tracked narrative. Delegate bounded independent work with clear ownership and one responsible integrator.

## Navigation and maintained owners

Follow actual callers and narrower guidance before editing. Paths are relative to the repository root.

| Area | Entry points |
| --- | --- |
| Public operations and authoring | `src/bin/lkjscript.rs`, `src/platform/cli.rs`, `src/platform/control/`, `src/platform/change/`, `src/platform/publication/` |
| Meaning, storage and inspection | `src/platform/kernel/`, `src/platform/storage/`, `src/platform/normalized_query.rs` |
| Libraries and compilation | `src/platform/package_interface.rs`, `src/platform/package_transport/`, `src/platform/compiler/` |
| Execution and application data | `src/platform/execution/normalized/`, `src/platform/execution/control.rs`, `src/platform/data.rs` |
| Runtime and distribution | `src/platform/runtime.rs`, `src/platform/deployment.rs`, `src/platform/installation/`, `src/release_container/` |
| Discovery and maintained consumers | `src/platform/project_creation/`, `src/platform/contract/`, `src/platform/builtin_standard.rs`, `packages/standard/`, `applications/lkjournal/` |
| Contributor proof and release | `tests/`, `tools/lkjscript-dev/src/check/`, `tools/lkjscript-dev/src/release/`, `.github/workflows/release.yml` |

`docs/spec/` owns normative semantics; `docs/spec/verification.md` owns verification obligations and `docs/release.md` owns publication. Use `docs/status.md` for concise checkout/public findings and `docs/roadmap.md` for revisable direction. Link history to its existing owner rather than copying a growing chronology into these summaries or creating parallel ledgers.

Generate accepted assets through their maintained owner. `src/platform/contributor.rs` is read-only observation, not an accepted graph writer. Generate `docs/generated/` through matching product discovery, not a second hand-maintained grammar.

## Cross-cutting guarantees

Public authoring passes through typed intent, complete validation, review and publication. Preserve lexical/generic scope, visibility, typed references, authored order, identity continuity and review bindings. Name resolution grants neither mutation nor execution authority. Recheck under the publication lock and persist content before atomic exposure. Invalid, stale, cancelled or exhausted requests cannot partially publish meaning. Idempotent accepted retries preserve their original immutable results; a failed derived operation does not undo an accepted write.

Preserve types, substitutions, provenance, effects and evaluation order through transport, compilation and execution. Admit the complete relevant closure, including unused arguments and unreachable syntax. Separate semantic validity from finite preparation capacity. Loaders independently admit producer content; caches bind actual context. Historical acceptance and current validity are distinct: do not rewrite history or treat old receipts as present execution permission.

Callable identity, declared effects and deployment grants are distinct. Invocation requires current allowance and exact grants through imports, binding and tail calls. Pure execution cannot invoke a task callable merely because its effect row is empty. Preserve affine consumption, borrowing, capture safety, provenance and raw/retained-value admission.

Admit target/type/grant contracts and intrinsically unencodable external results before live adapters or secrets; reject invalid external arguments before effects. A transaction is atomic only within its actual authority. Independent effects may survive failure. Cleanup failure, cancellation or missing output does not establish rollback or safe retry. Do not replay live effects for differential verification. Join owned resources on every exit; dropping a future is not joined cleanup.

Ordinary trusted execution requires no invented instruction budget. Optional metering, cumulative quotas, live limits, input bounds, cancellation, deadlines and profiling have different purposes. Use checked accounting and admission before growth; observational overflow is not quota exhaustion. Safe Rust, static linkage and resource limits do not establish a hostile-code sandbox.

Separate logical identity, semantic revision, encoding, artifacts, runtime installation and application data. Canonical bytes cannot depend on addresses, physical placement, hash iteration, paths or wall time. Compatibility changes need detection, consumer analysis, migration or clear rejection, and viable recovery. Never silently discard live data or reinterpret settings. Bridges need identified consumers and retirement conditions. Shared installation implies neither a daemon nor shared process fate; distinguish runtime-dependent artifacts from self-contained executables. Installation/selection neither migrates application data nor grants execution authority. Prove portability only for the tested target boundary.

## Development and verification

Use `rust-toolchain.toml`, manifests and lockfiles; preserve safe Rust and production lint guarantees. First-party contributor semantics belong in the existing Rust tooling. Preserve the no-Python and product-surface gates unless an explicitly authorized policy revision supplies equivalent protection.

Maintained entry points include:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

These are alternatives for relevant work, not a compulsory sequence. Discover focused profiles and selection in `tools/lkjscript-dev/src/check/`; discover release arguments in its maintained dispatchers and `docs/release.md`. `check changed` selects current Git-status paths, not a commit range. A clean-tree changed check does not prove a committed implementation. Full requires fresh evidence.

Use focused feedback during iteration and dependency-complete acceptance after relevant inputs stabilize. Honor maintained gates and actual protections; widen coverage when impact is uncertain. Do not run overlapping full profiles solely for reassurance. Concurrent Cargo builds can replace outputs through feature unification: respect gate dependencies and use immutable executable copies. Select test executables from Cargo's reported build output, not a guessed hashed filename.

Discover public operations with `lkjscript capabilities` and maintained guides. New public capability needs fresh authoring and use through a copied candidate executable outside the language checkout, retaining literal inputs. A hidden host application, privileged storage edit or unreproducible generator cannot stand in for language capability. Keep discovery, generated guides, built-in assets and maintained consumers consistent.

Bind material claims to independent expected results and meaningful failures. Changed machinery cannot be its own oracle. Use controlled reference adapters, not live-effect replay. Performance claims need equivalent behavior, baseline, workload/environment, stage boundaries and limitations; retain unfavorable results. Elapsed time or output bytes alone do not establish model-token or billing savings.

Reuse evidence only with valid source, verifier, environment, workload, input and trust bindings. A cache is not a correctness certificate; relevant changes or failures invalidate affected proof. Retain authenticated originals needed for acceptance, diagnosis or resumption. When reconstructing evidence, test copies and preserve the required bytes, metadata and context; reconstruction is not a compulsory second acceptance stage. Never fabricate missing originals, rewrite failed receipts into passes or claim reconstructed historical context as a fresh live environment observation.

Distinguish tested source, frozen release source, distributed bytes and later reporting/guidance commits. Validate a descendant's actual changes separately rather than stamping old evidence with its hash or creating a circular report/hash/retest obligation. Report fresh, valid reused, failed, cancelled, skipped, unavailable and unrun states accurately.

## Main integration and branch completion

Complete behavior, affected consumers, generated assets, documentation, migration/retirement and required integration proof. Refresh remote main and protections before integrating; incorporate intervening work without overwriting it and revalidate consequential interactions. Use normal fast-forward or merge-based delivery. If a PR is required, complete the permitted merge path rather than treating its creation as completion.

Verify remote main contains the accepted result; record the actual remote revision and its relation to the verified source. Squash/cherry-pick equivalence does not transfer exact-source evidence or establish ancestry. Do not force-push, weaken protections or describe an untested integration commit as the original tested candidate.

Main readiness and public closure have separate maintained prerequisites. A release-only wait need not keep accepted work off main. Preserve correctness and required integration gates; revise unnecessarily coupled ordering explicitly at its owner. Known product defects are not release-only exceptions.

Reconcile inherited campaign-owned branches and worktrees. Remove temporary references only after confirming integration or justified supersession, preservation of unique work and required evidence, unchanged tips, and no active job or required reference dependency. Immutable tags and historical failures remain. A worktree's ignored evidence can still be valuable even when its commit is integrated; preserve that material before removing the worktree. Never delete unmerged work just to reduce branch count or disturb unrelated stashes/worktrees.

If integration is blocked, preserve and reuse the outstanding lineage. Record the exact gate, permission, conflict or dependency, its source/run identity and concrete resumption action. A local commit, topic push or open PR is not remote-main completion.

## Publication, waiting and return

Use the maintained release owner. Publish meaningful milestones, reconcile inherited due delivery, and attach deferral to an observable trigger rather than another attractive feature. Repository publication is not production deployment. Inspect matching sources, versions and attempts before dispatch; reuse healthy work and serialize shared publication identities. Preserve immutable tags/assets and genuine failed history. Defective published releases normally need additive correction.

Use only the release-scoped operations authorized by current instructions and `docs/release.md`. Do not change immutability, credentials or access controls. Artifact content identity, accepted behavior and permission to publish are separate claims. Authenticate the producing repository, workflow, source, run/attempt and artifact identities; matching hashes or a manifest assertion cannot grant publication authority. Never execute a candidate or transferred verifier with publishing credentials.

Build once per necessary configuration, finalize the distributable, and bind authoritative candidate acceptance to its exact extracted executable. Promote those same accepted assets without rebuilding or rewriting their content to change publication state. Preserve source-specific and target-specific proof at their appropriate owners. Transfer and anonymous public acquisition normally require identity, strict container/installer admission and a small meaningful lifecycle, not another broad semantic or application suite. Additional execution must protect a distinct claim.

The maintained CI owner must validate its required evidence and expose a terminal result distinguishing candidate acceptance, immutable publication and public verification. Routine success must not depend on a later agent manually reconstructing every transient original. A green subjob, uploaded asset, checksum or main push alone is insufficient. Record legacy evidence gaps honestly without automatically turning superseded manual closure into a new campaign obligation.

Resume a failed publication or public-check boundary using still-valid accepted inputs; preserve the producing run/attempt rather than relabeling its evidence. Changed relevant inputs, a real invalidating defect or unavailable trusted artifacts require the appropriate renewed proof. Retain candidate assets and evidence for the documented resumption window and report actual expiry. Preserve genuine failures and immutable releases. Do not restart valid completed stages or change frozen inputs merely to append guidance or a report. These requirements must be implemented at the release owner; guidance alone does not make an old pipeline satisfy them.

Do useful independent work while remote jobs run. No indefinite watches, sleep/status loops, repeated unchanged logs or hidden polling. Inspect at meaningful work boundaries or for concrete diagnosis. If only external completion remains, return the exact source, run/attempt, observed state, missing gate, retained evidence and next action; do not promise unattended monitoring.

The final report states what became possible, actual revisions/actions, verified and unverified claims, compatibility impact, remote-main and public-release state, cleanup or retention reasons, exact blockers and the most consequential next finding. Keep campaign IDs, source/release/run identities, measurements, acceptance matrices and incident chronology out of this root file.
