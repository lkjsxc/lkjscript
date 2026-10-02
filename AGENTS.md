# Repository guidance

## Direction

Build an ambitious agent-native general-purpose language under [project direction](docs/direction.md). Language design comes first: composable types, generics, traits, ownership, lifetimes, effects, scalable parallel execution and a genuinely shared runtime. Reliable and economical AI development is an important secondary objective. Personal applications, immediate benchmark superiority and immediate API-cost savings are not prerequisites for a coherent language advance. Memory safety and avoiding unnecessary copies are baseline goals.

The accepted typed meaning graph is the sole editable program-meaning authority. Authored requests and projections are proposals, never a synchronized second source. Type/ownership/lifetime/effect contracts are semantic; layout and recomputable analyses are derived. Preserve machine-checkable contracts and discovery without requiring human comprehension, manual review or one particular model. Names, artifacts, deployment, operational data and secrets have separate owners.

Ordinary development must work through the product, without external semantic generators. Prefer native first-party behavior when it can own the actual semantics and failures. Eventual complete self-hosting, including removing Rust, remains the goal. Current safe Rust boundaries and Arc/Box implementations are replaceable mechanisms. Ownership, local regions and optional tracing may coexist without a mandatory global heap. The Rust-only/hosting request for `lkjsxc/lkjstr` does not apply here.

lkjsxc retains official-project authority; Apache-2.0 remains the license. Current user instructions override historical selections. Breaking changes and resets of owner-authorized experimental lkjscript data are permitted without mandatory migration; unrelated/third-party resources remain protected. Implementation requests delegate normal mainline integration and configured release operations, not purchases, production deployment, credentials/access changes, protection bypasses, force-pushing or rewriting immutable publication history.

## Read current state, not the archive

Start with [status](docs/status.md), then the relevant owner below. Read [roadmap](docs/roadmap.md) when selecting work and [direction](docs/direction.md) when revising architecture. Reconcile actual HEAD/branch, index/worktree, relevant untracked files, stashes, worktrees, remotes/divergence and active jobs before editing. Check toolchain and memory/disk before expensive builds. An old observed SHA is not a reset target.

`docs/campaigns/` is a historical archive. Do not create a campaign or transcribe the conversation for each task. Do not recursively read old prompts or predecessor endings at startup. Follow an exact historical link only to resolve a concrete uncertainty, recover original evidence or diagnose a regression. Historical prompts are not additional active requirements. Preserve existing originals and links.

`docs/status.md` owns current availability and unfinished handoff: selected source/worktree, exact receipt or run/attempt, concrete remaining gate, retained resources and next action. Replace stale state instead of appending a diary. Link original evidence rather than mirroring every live transition; do not edit tracked status during source-stability checks. Semantics belong in `docs/spec/`, durable rationale in `docs/decisions/`, delivery in `docs/release.md`, and completed changes in Git/release notes. Keep logs/binaries in `.artifacts/`, not tracked narrative.

Reduce recurring context and rework: search owners before reading whole files, retain compact actionable findings, delegate distinct responsibilities and reuse valid observations. Correctness and required acceptance take precedence. File bytes, elapsed time and output length are not measured API tokens or money; claim monetary savings only from actual usage/billing evidence.

## Maintained owners

| Responsibility | Starting points |
| --- | --- |
| Public operations/publication | `src/platform/cli.rs`, `control/`, `change/`, `publication/` |
| Meaning/storage | `src/platform/kernel/`, `storage/`, `normalized_query.rs` |
| Libraries/compilation | `src/platform/package_interface.rs`, `package_transport/`, `compiler/` |
| Execution/data | `src/platform/execution/normalized/`, `execution/control.rs`, `data.rs` |
| Runtime/distribution | `src/platform/runtime.rs`, `deployment.rs`, `installation/`, `src/release_container/` |
| Discovery/native consumers | `src/platform/project_creation/`, `contract/`, `builtin_standard.rs`, `packages/standard/`, `applications/lkjournal/` |
| Verification/release | `tests/`, `tools/lkjscript-dev/src/check/`, `tools/lkjscript-dev/src/release/`, `.github/workflows/release.yml` |

Short names such as `control/`, `kernel/` and `contributor.rs` refer to `src/platform/`; `tests/`, `tools/`, `packages/`, `applications/`, `.github/` and `src/` start at the repository root. Follow callers and narrower guidance before editing. Generate maintained assets through their supported owners and `docs/generated/` through product discovery. `contributor.rs` is a read-only observer, not a graph writer. Keep safe Rust, no-Python and product-surface gates.

## Guarantees

Preserve scope, visibility, typed references, authored order, intended identity continuity and review bindings. Validate complete candidates, recheck under publication lock and persist before atomic exposure. Invalid/stale/cancelled/exhausted proposals must not partially publish. Accepted idempotent retries retain their immutable result; a derived failure does not undo acceptance. Lookup grants no mutation/execution authority.

Admit complete relevant closures, including unused arguments and untaken syntax. Preserve types, substitutions, provenance, effects and evaluation order through transport/compilation/execution. Loaders independently admit producer content; caches bind their context. Separate semantic validity from finite preparation capacity and historical acceptance from current execution permission.

Callable identity, task kind, effects and deployment grants are distinct; an empty task effect row does not confer purity. Preserve exact allowances/grants, affine consumption, borrowing, capture safety and raw/retained-value admission. Reject unsupported boundaries before adapters/secrets, and admit values before effects. Transactions cover only their actual authority. Cancellation, missing output and cleanup/result-encoding failures prove neither rollback nor safe retry. Never replay live effects for differential verification. Join owned resources on every exit; dropping a future is not joined cleanup.

Ordinary trusted execution has no invented cumulative instruction budget. Metering, quotas, live limits, input bounds, cancellation, deadlines and profiling are separate policies. Reserve before modeled growth; observational overflow is not quota exhaustion. Safe Rust/static linkage/limits do not establish a hostile-code sandbox. Canonical bytes cannot depend on addresses, placement, hash iteration, host paths or wall time.

Keep logical identity, semantic revision, encoding, artifacts, installation and application data separate. All `A.B.C` components are opaque identifiers, without maturity/compatibility/change-size meaning. State intentional compatibility cuts; do not conceal corruption or unplanned reset. Installation neither migrates data nor grants authority. Shared installation or one process per app is not shared in-process execution. Dedicated-runtime and native-executable goals remain distinct.

## Verification

Use pinned toolchain/manifests/lockfiles. Maintained entry points: `cargo build --release --locked -p lkjscript-dev`, `lkjscript-dev check changed --machine`, `lkjscript-dev check full --fresh --machine`, `lkjscript-dev release target`. Discover selectors at their owner. The checker is bound to its compile-time checkout; shell cwd does not retarget it. `changed` selects current Git-status paths, not a commit range; a clean-tree run cannot prove committed implementation.

Use focused development checks, then dependency-complete fresh acceptance after inputs stabilize. [Verification](docs/spec/verification.md) owns proof obligations; [release](docs/release.md) owns source/candidate mapping. Honor required gates. Set checker `--jobs` and `CARGO_BUILD_JOBS` independently. Prefer less concurrency over reduced coverage or overlapping expensive builds. Cargo feature unification can replace outputs: honor producer dependencies, select test executables from Cargo output, and preserve immutable executable copies.

Prove public capability by fresh authoring and use through a copied executable outside the checkout, retaining literal inputs. Hidden host computation or privileged graph writes cannot substitute. Use independent expected results and meaningful failures; changed machinery cannot be its only oracle. Distinguish designed witnesses from maintained adoption. Performance comparisons hold behavior/workload constant, identify environment/stage costs and retain unfavorable results.

Reuse proof only while source, verifier, environment, workload, inputs and trust bindings remain valid. Digests establish specified identity/integrity, not behavior or permission. Preserve failures and unavailable originals honestly. Separate tested source/distributed bytes from reporting descendants; validate consequential descendant changes without relabeling old proof or creating a report-hash-retest cycle. Distinguish fresh, reused, failed, cancelled, ignored and unrun checks.

## Delivery

Preserve unrelated work, secrets and operational state. Stage intended paths only and review the staged diff. Use owned disposable resources for destructive tests and join owned processes without disturbing other services. Prefer main when checkout ownership permits; otherwise reuse justified isolation with one integrator.

Finish behavior, consumers, generated assets, docs and selected compatibility cut. Refresh remote main/protections, incorporate intervening work and validate interactions. Deliver through normal fast-forward/merge or required PR, then independently verify remote main contains the accepted result. A local commit, topic push, PR or auto-merge request is not delivery. Squash/cherry-pick equivalence is not exact-source ancestry. Keep mainline readiness separate from release-only waiting.

Select meaningful publication milestones; give deferrals an observable trigger. Inspect exact source/version/run/attempt and occupancy before dispatch, reuse healthy work and serialize publication identities. Authenticate producer repository/workflow/source/run/artifacts independently of contents. Never execute candidates/transferred verifiers with publishing credentials. Accept the finalized archive's exact executable, then promote unchanged assets without rebuilding. Require the maintained publication/public-verification terminal, not a green subjob. Resume with still-valid originals; changes, invalidation or expiry require renewed proof. Correct published defects additively.

Do useful independent work while jobs run. When only external completion or unavailable authority remains, report exact lineage/gate/state, retained resources and next action instead of indefinite polling. Retire owned branches/worktrees only after integration/supersession, unchanged tips, preservation of unique work/ignored evidence and absence of active jobs/references. Preserve unrelated stashes and immutable/failed history.

Return new capability, actual validation and limits, tested source versus remote main, compatibility impact, publication state and concrete remaining action. Keep the handoff concise and evidence at its original owner.
