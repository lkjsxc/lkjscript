# Repository guidance

## Mission and authority

Build a general-purpose language in which humans and agents can author, compose, maintain and distribute substantial programs through the product executable. Prefer coherent semantics and ordinary libraries over application-specific compiler behavior.

The accepted typed meaning graph is the current canonical editable program authority. Input notations, names, projections, indexes and compiled products have distinct roles. Operational data, secrets, installations and deployment settings have separate owners. Architecture is revisable with evidence, preserved guarantees and an explicit transition; historical campaign exclusions and incidental limits are not permanent user requirements.

Follow current user direction and higher-priority instructions. This file and available tools do not grant authority. Replacing the file does not reload an active session's instructions.

## Startup and safe changes

Inspect actual branch/HEAD, index/worktree, relevant untracked files, stashes, remotes and divergence. Read applicable guidance, the mandate and relevant campaign endings. Reconcile material obligations as complete, actionable, blocked or superseded. An architect's revision is an observation, never a reset target. Check toolchain, memory/disk capacity and active jobs before expensive work.

Preserve unrelated work, secrets, operational state and published identities. Use isolated work when needed. Stage intended paths and review the staged diff; do not use broad staging, destructive cleanup or history rewriting to manufacture a clean result. Use owned disposable resources for destructive tests. Join owned processes and clean up owned temporary resources without touching unrelated services. Keep binaries, logs and `.artifacts` output out of tracked narrative.

Archive the initial mandate unchanged under `docs/campaigns`; append concise reconciliation, execution and resumption records. Preserve both originals on a filename collision. Reconcile a supplied root candidate with legitimate intervening changes and install it in the mutable development checkout before substantive work. Never add guidance or reporting to frozen release inputs merely to make them current.

Delegate independent bounded work with clear ownership and one responsible integrator.

## Navigation

Paths are relative to the repository root. Follow actual callers and narrower owners before editing.

| Area | Maintained entry points |
| --- | --- |
| Public operations | `src/bin/lkjscript.rs`; `src/platform/cli.rs`; `src/platform/control/`; `src/platform/normalized_query.rs` |
| Typed authoring and publication | `src/platform/control/change/`; `src/platform/change/`; `src/platform/publication/` |
| Meaning and physical graph storage | `src/platform/kernel/`; `GraphRepository`; `src/platform/storage/` |
| Packages and compilation | `src/platform/package_interface.rs`; `src/platform/package_transport/`; `src/platform/compiler/` |
| Execution and application data | `src/platform/execution/normalized/`; `src/platform/execution/control.rs`; `src/platform/data.rs` |
| Runtime and installation | `src/platform/runtime.rs`; `src/platform/deployment.rs`; `src/platform/installation/` |
| Discovery and built-ins | `src/platform/project_creation/`; `src/platform/contract/`; `src/platform/builtin_standard.rs` |
| Consumers and contributor proof | `packages/standard/`; `applications/lkjournal/`; `tests/`; `tools/lkjscript-dev/src/` |

`docs/spec` owns normative semantics. Start with the affected language, semantic-authority, semantic-storage, semantic-cli, effects-capabilities, data-capabilities or packages-components document. `docs/spec/verification.md` and `tools/lkjscript-dev/src/check/` own verification; `docs/release.md` and `tools/lkjscript-dev/src/release/` own publication.

`docs/status.md` holds current findings; `docs/roadmap.md` holds revisable direction. Link history to its existing campaign owner instead of copying chronology into current summaries.

Generate accepted assets through their maintained owner. `src/platform/contributor.rs` is a read-only observation surface, not an accepted graph writer. Generate `docs/generated` through matching product discovery, not another hand-maintained grammar.

## Cross-cutting guarantees

Public authoring passes through typed intent, complete validation, review and publication. Input notations propose changes; inspection projections describe meaning. Preserve lexical/generic scope, visibility, typed references, authored order, identity continuity and review bindings. Exact name resolution grants neither mutation nor execution authority.

Recheck under the publication lock, persist content before atomic exposure, and preserve accepted inputs and immutable results on idempotent retry. Invalid, stale, cancelled or exhausted requests cannot partially publish meaning. Failure of a derived operation does not undo an accepted write.

Preserve types, substitutions, provenance, effects and evaluation order through transport, compilation and execution. Admit the complete relevant closure, including unused arguments and unreachable syntax. Separate semantic validity from finite preparation capacity. Loaders independently admit producer content; caches bind actual context.

Callable identity, declared effects and deployment grants are distinct. Invocation requires current allowance and exact grants through imports, binding and tail calls. Pure execution cannot invoke a task callable even with an empty effect row. Preserve affine ownership, consumption, borrowing, capture safety and raw/retained-value admission.

Admit target/type/grant contracts and intrinsically unencodable external results before live adapters or secrets; reject invalid external arguments before effects. A transaction's atomicity covers only its actual authority. Earlier effects may survive failure. Cleanup failure, missing output or cancellation does not prove rollback or safe retry. Do not replay live effects for differential verification. Join owned resources on all exits; dropping a future alone is not cleanup.

Ordinary trusted execution needs no invented instruction budget. Optional metering, cumulative quotas, live limits, input limits, cancellation, deadlines and profiling have different purposes. Use checked accounting and admission before growth. Observational overflow is not quota exhaustion. Safe Rust, static linkage and quotas do not establish hostile-code isolation.

Separate logical identity, semantic revision, encoding, artifacts, runtime layout and application data. Canonical bytes cannot depend on addresses, physical placement, hash iteration, paths or wall time. Compatibility changes need detection, consumer analysis, migration or clear rejection, and viable recovery. Never silently discard live data or reinterpret settings. Temporary bridges need identified consumers and retirement conditions.

Authenticate historical acceptance separately from current validity without rewriting history. Current proof binds exact meaning, dependencies and validator. Old receipts do not authorize current execution. Revalidation preserves HEAD/identity; repair requires a completely valid post-change candidate.

Distinguish an installed runtime, runtime-dependent application artifacts and executables embedding a runtime. Prove static distribution for selected targets. Sharing installation bytes implies neither a daemon nor shared process fate. Installation and selection do not migrate application data or authorize execution.

## Development and verification

Use the pinned toolchain, manifests and lockfiles, safe Rust and production lint guarantees. First-party contributor semantics belong in existing Rust tooling. Preserve no-Python and product-surface gates unless the task deliberately revises their policy with equivalent protection.

Source-verified contributor entry points:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check focused --machine
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

These are choices, not a mandatory sequence. The last command prints target policy; discover build/admit arguments from maintained dispatchers and `docs/release.md`. `check changed` reads current Git-status paths, not a commit range; source/test changes select full. A clean-tree changed check does not establish campaign acceptance. Full requires fresh evidence.

Discover application operations through `lkjscript capabilities` and maintained guides. Proposed interfaces do not exist until implemented and advertised. New public capabilities require fresh authoring and use through a copied candidate executable outside the language checkout. Retain literal inputs; hidden generators or privileged storage edits cannot substitute for authoring. Keep generated guides, built-in assets and maintained consumers consistent.

Use focused feedback while iterating, then dependency-complete acceptance after relevant inputs stabilize. Honor maintained gates and widen when impact is uncertain. Avoid overlapping profiles solely for reassurance. Concurrent Cargo builds can replace release outputs through feature unification; respect `release_command_lifecycle` dependencies and immutable executable copies.

Map claims to independent expected results and meaningful failure cases. Changed machinery cannot be its own oracle. Use deterministic reference adapters disjoint from live effects. Performance claims require equivalent behavior, baseline, workload/environment, relevant stages and limitations; retain unfavorable results. Elapsed time and output bytes do not establish model-token or billing savings.

Reuse evidence only with valid source, verifier, environment, workload and trust bindings. A cache is not a correctness certificate. Rerun affected proof after relevant changes or failures. Separate tested source, frozen release source, distributed bytes and later reporting commits; avoid circular report/hash/retest obligations. Label fresh, valid reused, failed, cancelled, skipped, unavailable and unrun results accurately. Extend existing proof owners before introducing permanent machinery.

## Integration, publication and return

Complete selected behavior, consumer transition, documentation, retirement and required proof. Preserve useful work if a separate dependency blocks completion. If the selected outcome already exists with required proof and delivery, reconcile it and report satisfaction; do not invent another campaign.

Refresh permissions, protections, divergence and consequential automatic effects before authorized integration. Complete ordinary integration when authorized and ready. Do not bypass gates, force-push, alter permissions or overwrite immutable publication identities. Repository publication and production deployment are separate actions.

Follow the maintained release protocol. Batch useful milestones and reconcile inherited delivery commitments; a deferral needs an observable trigger. Preserve accepted frozen sources while authorized development proceeds independently. Serialize publishers, inspect matching attempts and inputs before dispatch or rerun, and reuse healthy work. Public delivery requires prescribed anonymous acquisition, distributed behavior and original-reader admission; an upload or green subjob is insufficient.

Do useful independent work while remote jobs run. Avoid indefinite watches, sleep/status loops, repeated unchanged logs and hidden polling. Inspect at meaningful work boundaries or for concrete diagnosis. If only external completion remains, report source, run/attempt, observed state, missing gate, retained evidence and the next resumption action. Do not promise unattended monitoring.

Return actual revisions and actions, useful outcomes, verification and limitations, compatibility impact, preserved work and cleanup. Report engineering, verification, integration, publication and public acceptance separately. Keep campaign objectives, matrices, identities, measurements and historical incidents outside this file.
