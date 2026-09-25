# Repository guidance

## Purpose, authority and implementation choices

Build a useful general-purpose language and ecosystem. Substantial programs should be authored, understood, composed, changed, tested, executed and distributed through the public product. Make operations discoverable for agents and understandable for people. Do not couple the language to a particular model. Prefer ordinary libraries and reusable mechanisms over application-specific compiler behavior.

Develop toward practical self-sufficiency: ordinary application and library development should not require an external semantic generator, a second programming language, or access to compiler internals. Prefer direct native authoring and ordinary lkjscript programs for first-party behavior and tooling when the public facilities support the actual workload. A wrapper around a Python/Rust generator, or a string emitter that preserves the same external dependency, does not establish this outcome.

The Rust kernel, bootstrap, platform adapters and existing contributor/release tools remain supported implementation boundaries, not permanent language requirements. Migrate a boundary when a concrete native replacement can assume its responsibilities with equivalent correctness, recovery and maintainability; retire the replaced path. Do not undertake a wholesale rewrite or weaken verification merely to increase the proportion of lkjscript. Distinguish native user workflows, native tool implementations and a self-hosted compiler in all claims.

Architecture is revisable with evidence and an explicit compatibility transition. The accepted typed meaning graph is the current editable program authority. Authored requests and editable projections are proposals, not a second synchronized authority. Names, indexes, compiled products, deployment configuration, operational data and secrets have different owners. Historical campaigns and incidental implementation limits are not permanent user requirements.

Follow current user direction and applicable higher-priority and narrower instructions. Implementation mandates may delegate normal mainline integration and configured CI/release operations within actual permissions. They do not authorize production deployment, operational-data destruction, purchases, credential or access-control changes, protection bypasses, force-pushing or rewriting immutable publication history. A replacement instruction file does not automatically reload an active session.

## Startup, preservation and lineage

Before editing, reconcile actual HEAD and branch, index/worktree, relevant untracked files, stashes, worktrees, remotes, divergence, applicable guidance and relevant active jobs. Inspect the selected mandate and material predecessor endings. Classify obligations as complete, actionable, externally blocked or explicitly superseded. An architect's observed SHA is not a reset target. Check the pinned toolchain and available memory/disk before expensive work.

Preserve unrelated work, secrets, operational state and evidence. Stage intended paths only and review the staged diff. Do not use broad staging, destructive cleanup or credential-bearing output to simplify a report. Destructive tests belong in owned disposable resources. Join owned processes and clean up their resources without disturbing unrelated services.

Prefer main when collaboration and checkout ownership permit it; otherwise use justified isolation and one responsible integrator. Reuse the appropriate outstanding lineage for continuation. A frozen release source does not freeze main. Do not accumulate replacement branches or parallel reporting systems.

Read existing guidance before reconciling a supplied replacement. Install the reconciled guidance in the mutable checkout before substantive work, without modifying frozen release inputs. Archive the initial mandate unchanged in `docs/campaigns/`; append clearly separated reconciliation, execution and resumption findings. On a real filename collision, preserve both originals using the existing collision convention. Keep binaries, giant logs and `.artifacts` outputs out of tracked narrative.

## Maintained owners

| Responsibility | Starting points |
| --- | --- |
| Public operations, authoring and publication | `src/bin/lkjscript.rs`, `src/platform/cli.rs`, `src/platform/control/`, `src/platform/change/`, `src/platform/publication/` |
| Meaning, storage and inspection | `src/platform/kernel/`, `src/platform/storage/`, `src/platform/normalized_query.rs` |
| Libraries, transport and compilation | `src/platform/package_interface.rs`, `src/platform/package_transport/`, `src/platform/compiler/` |
| Execution and application data | `src/platform/execution/normalized/`, `src/platform/execution/control.rs`, `src/platform/data.rs` |
| Runtime and distribution | `src/platform/runtime.rs`, `src/platform/deployment.rs`, `src/platform/installation/`, `src/release_container/` |
| Discovery and maintained consumers | `src/platform/project_creation/`, `src/platform/contract/`, `src/platform/builtin_standard.rs`, `packages/standard/`, `applications/lkjournal/` |
| Verification and release tooling | `tests/`, `tools/lkjscript-dev/src/check/`, `tools/lkjscript-dev/src/release/`, `.github/workflows/release.yml` |

Follow callers and narrower guidance before changing an owner. `docs/spec/` owns semantics; `docs/spec/verification.md` owns proof obligations; `docs/release.md` owns delivery. Keep `docs/status.md` concise and current, and `docs/roadmap.md` revisable. Link detailed history to its existing owner instead of copying chronology into summaries.

Generate maintained assets through their supported owner. `src/platform/contributor.rs` is a read-only observer, not an accepted-graph writer. Generate `docs/generated/` through product discovery, not a separately maintained grammar.

## Guarantees across layers

Authoring proceeds through typed intent, complete validation, review and publication. Preserve lexical and generic scope, visibility, typed references, authored order, intentional identity continuity and review bindings. Name lookup grants neither mutation nor execution authority. Recheck under the publication lock and persist content before atomic exposure. Invalid, stale, cancelled or exhausted changes must not partially publish meaning. Accepted idempotent retries retain their original immutable result; a failed derived action does not undo acceptance.

Preserve types, substitutions, provenance, effects and evaluation order through transport, compilation and execution. Admit the whole relevant closure, including unused arguments and unreachable syntax. Separate semantic validity from finite preparation capacity. Loaders independently admit producer content, and caches bind their actual context. Historical acceptance is not current execution permission; preserve history while enabling reviewed repair.

Callable identity, declared effects and deployment grants are distinct. Require exact current allowances and grants through imports, binding and tail calls. Pure execution cannot invoke a task merely because its effect row is empty. Preserve affine consumption, borrowing, capture safety and admission of raw or retained values.

Reject invalid external contracts and intrinsically unsupported boundary types before opening live adapters or reading secrets; admit input values before their effects. Runtime result encoding can still fail after effects and does not imply rollback. Transactions are atomic only within their actual authority; independent effects can survive failure. Cancellation, missing output and cleanup failure do not prove rollback or safe retry. Do not replay live effects for differential verification. Join owned resources on every exit; dropping a future is not joined cleanup.

Ordinary trusted execution has no invented cumulative instruction budget. Optional metering, quotas, live limits, input bounds, cancellation, deadlines and profiling serve different purposes. Check accounting and reserve before growth; observational overflow is not quota exhaustion. Safe Rust, static linkage and resource limits do not establish a hostile-code sandbox.

Keep logical identity, semantic revision, encoding, artifacts, installation and application data separate. Canonical bytes cannot depend on addresses, physical placement, hash iteration, host paths or wall time. Compatibility changes need consumer analysis, detection, migration or explicit rejection, and viable recovery. Never silently discard data or reinterpret settings. A bridge needs a real consumer and retirement condition. Installation or runtime selection neither migrates data nor grants execution authority; distinguish runtime-dependent artifacts from self-contained executables. State the actual tested portability boundary.

## Development and evidence

Use `rust-toolchain.toml`, manifests and lockfiles. Preserve safe Rust and production lint requirements. Keep the maintained no-Python and product-surface gates; migrating appropriate tooling to lkjscript does not justify removing their protections. External launch/assertion harnesses are not evidence that the product itself can author a program.

Maintained entry points include:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

Select the relevant entry points rather than running this as a compulsory sequence. Discover profiles, selectors and release arguments at their maintained owners. `check changed` uses current Git-status paths, not a commit range; a clean-tree changed check cannot prove a committed implementation. Full requires fresh evidence. Release acceptance follows the dependency-complete source/candidate mapping in `docs/release.md`, not an invented duplicate suite.

Use focused feedback while developing and dependency-complete acceptance after relevant inputs stabilize. Honor required gates and protections. On memory-constrained hosts, set checker `--jobs` and `CARGO_BUILD_JOBS` independently: the Cargo limit does not serialize separate verification gates. Prefer lower concurrency over overlapping debug/release compilers or reduced coverage. Do not repeat overlapping broad suites for reassurance. Concurrent Cargo configurations can replace shared outputs through feature unification: honor producer dependencies and preserve immutable executable copies. Select test executables from Cargo's reported outputs, not guessed hashed filenames.

Discover public operations through `lkjscript capabilities` and maintained guides. Prove new capability by fresh authoring and use through a copied executable outside the compiler checkout, retaining the literal inputs. A privileged storage edit, hidden host implementation or unreproducible generator cannot substitute for public behavior. Keep discovery, guides, assets and affected consumers consistent. Distinguish a designed witness from maintained adoption.

Use independent expected results and meaningful failures. Changed machinery cannot be its only oracle. Controlled reference adapters may complement live observations, but never duplicate live effects. Performance comparisons must hold behavior and workload constant, identify environment and stage costs, and retain unfavorable results. Output size and elapsed time do not establish model-token or monetary savings.

Use digests where an identity, integrity or compatibility contract needs them; they do not prove behavior or confer permission. Do not add duplicate hash inventories or new receipt systems without a distinct claim. Reuse evidence only when source, verifier, environment, workload, inputs and trust bindings remain valid. Explain invalidation. Preserve originals needed for diagnosis and resumption; never manufacture missing originals, relabel historical context as a fresh observation or rewrite failures as passes.

Separate verified source and distributed bytes from subsequent reporting/guidance commits. Validate consequential descendant changes separately, without stamping earlier proof with a later SHA or creating a report-hash-retest cycle. Report fresh, reused, failed, cancelled, skipped, unavailable and unrun evidence accurately.

## Mainline completion

Finish behavior, affected consumers, generated assets, documentation and required migration or retirement. Refresh remote main and protections before integration. Incorporate intervening work without overwriting it and validate consequential interactions. Use normal fast-forward or merge-based delivery; when a PR is required, finish the permitted merge path.

Verify that remote main contains the accepted result and record its relationship to the tested source. A local commit, topic push, open PR or enabled auto-merge is not mainline delivery. Squash/cherry-pick equivalence is not exact-source ancestry or an automatic transfer of evidence.

Keep mainline readiness distinct from release-only waiting, without treating a known product defect as a publication exception. Do not silently waive integration requirements; revise unnecessarily coupled ordering explicitly at its owner when authorized.

Retire campaign-owned branches/worktrees only after confirming integration or justified supersession, unchanged tips, preservation of unique work and ignored evidence, and absence of active jobs or required references. Preserve immutable tags and genuine failed history. Do not disturb unrelated stashes/worktrees or delete unmerged work to make branch counts look tidy.

## Publication and return

Select meaningful publication milestones, reconcile due delivery and give any deferral an observable trigger. Use `docs/release.md` and its configured owner. Inspect matching source/version/run/attempt and occupancy before dispatch; reuse healthy work and serialize shared publication identities. Use only explicitly authorized release-scoped selection operations. Do not change credentials, permissions or immutability. Correct defective published releases additively.

Authenticate producer repository, workflow, source, run/attempt and artifact identities independently of content. Never execute a candidate or transferred verifier with publishing credentials. Build only necessary configurations, finalize the distributable, accept its exact extracted executable and promote the same assets without rebuilding or rewriting their content.

Keep source and target proof at their respective owners. Transfer and anonymous acquisition normally need distinct identity, strict container/installation admission and a small lifecycle, not repeated broad semantic acceptance. The maintained CI terminal must distinguish candidate acceptance, publication and public verification. A green subjob or uploaded artifact alone establishes none of the later stages. Routine completion must not depend on an agent reconstructing every transient original afterward.

Resume failed boundaries using still-valid accepted inputs and original producer identities. Relevant changes, invalidating defects or unavailable trusted artifacts require appropriate renewed proof. Preserve evidence for the actual documented resumption window and report observed expiry. Do not restart healthy stages or mutate frozen inputs merely to append guidance. Historical gaps in superseded manual closure are not automatically current blockers.

Do useful independent work while jobs run; inspect at meaningful boundaries or for concrete diagnosis. Do not use indefinite watches, repeated unchanged logs, sleep/status loops or promises of unattended monitoring. When only external completion or unavailable authority remains, return the preserved lineage, exact source/run/attempt and gate, observed state, retained resources and next action.

The execution return states what became possible, actual changes and deviations, tested source and remote-main revision, evidence states, compatibility impact, publication status, retained resources, exact blockers and the most consequential next finding. Keep campaign identities, measurements, acceptance matrices and incident chronology out of this root file.
