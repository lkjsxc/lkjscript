# Repository guidance

Build the agent-native language in [direction](docs/direction.md) and
[roadmap](docs/roadmap.md): composable types/contracts, ownership/regions/lifetimes,
effects, deterministic concurrency, shared execution and eventual total self-hosting.
Accepted typed meaning is the sole editable authority; text/projections/assets are
proposals or derived outputs. Use concise English. Support only current HEAD formats:
remove obsolete readers/envelopes/shims at changed owners and regenerate maintained
inputs together. No migration or legacy-data preservation is required. Preserve
unrelated work/data, secrets and immutable publication history. lkjsxc owns official
direction; Apache-2.0 remains. Product A.B.C components are opaque identifiers.

## Inspect and edit

Start with [status](docs/status.md) and relevant owners. Reconcile Git/remotes,
worktrees/live jobs and build resources. Old SHAs are not reset targets. Search
before large reads; campaigns resolve concrete history, never recurring diaries.

| Responsibility | Starting owners |
| --- | --- |
| Operations/publication | `src/platform/cli.rs`, `control/`, `change/`, `publication/` |
| Meaning/storage | `kernel/`, `storage/`, `normalized_query.rs` |
| Packages/compiler | `package_interface.rs`, `package_transport/`, `compiler/` |
| Execution/runtime/data | `execution/`, `runtime/`, `deployment.rs`, `data.rs` |
| Installation | `installation/`, `src/release_container/` |
| Discovery/consumers | `project_creation/`, `contract/`, `builtin_standard.rs`, `packages/standard/`, `applications/lkjournal/` |
| Verification/release | `tests/`, `tools/lkjscript-dev/src/check/`, `tools/lkjscript-dev/src/release/`, `.github/workflows/` |

Short platform paths start at src/platform; others at root. Follow callers/narrower
guidance. Public product operations own semantics, without external generators or
privileged writes; contributor.rs observes. Generate assets through supported owners
and docs/generated via discovery. Status stays below 100 lines: capabilities,
milestone/source/run/gate/next action. Replace stale state; no temporary/stash/failed
run histories. Specs/decisions/release/Git own semantics/rationale/delivery/completed
changes. Keep one checkout and one Cargo target at `$PWD/target`. Default checks
overwrite one compact receipt and delete joined scratch; retain originals only
with explicit `--retain-archive`. Never copy gate executables into .artifacts.

## Invariants

Preserve scope, visibility, types, order, identities and review bindings. Admit
complete closures including unused/untaken syntax. Revalidate under publication
lock, persist then expose atomically; rejected/stale/cancelled/exhausted candidates
cannot partly publish. Accepted keyed retries retain immutable results; derived
failure does not undo acceptance. Lookup grants no mutation or execution authority.

Preserve exact witnesses, substitutions, provenance, effects and evaluation order
through packages/compiler/runtime. Independently admit producer content and bind
cache context completely. Validity differs from finite preparation capacity and
historical acceptance from current permission. Empty task rows are not purity;
callable kind, allowances and deployment grants require separate admission.

Admit inputs before effects/secrets, unsupported boundaries before adapters.
Preserve affine custody/loans and join owned resources on every exit. Missing output,
cancel or cleanup/encoding failure proves neither rollback nor retry safety. Never
replay live effects for differential verification. Reserve modeled growth first;
quotas, live limits, input bounds, cancel, deadlines and observations differ.
Unmetered trusted execution has no invented instruction quota; saturation grants
no capacity. Safe Rust/static linkage is no hostile-code sandbox.

Canonical bytes cannot depend on addresses/hash order/host paths/time. Keep identity,
revision, encoding, artifacts, installation and data separate. Installation grants
no authority/migration; shared installation is no shared execution. Retain
forbid(unsafe_code), denied Clippy unwrap/expect/panic, no-Python and product-surface
gates. Never stub checks or relabel failures as passes.

## Verify and deliver

Bootstrap clean committed source at canonical root with
`verifier_root=$(bash tools/bootstrap-verifier.sh)`. Bootstrap and check share
`$PWD/target` under an exclusive lease. Hash the actual Cargo-selected outputs;
preserve Cargo's fresh/reused observation honestly. No binary copies or separate
build targets. Checker cwd cannot retarget its compiled checkout.

Use focused checks then fresh full acceptance: check changed --machine;
check full --fresh --machine; release target. Changed reads Git status, not committed
ranges. Set checker/Cargo jobs independently. [Verification](docs/spec/verification.md)
owns proof/gates/complete-context reuse; [release](docs/release.md) owns exact source/
candidate/authority. Preserve complete matrices and honest outcomes while sharing
immutable fixtures. Release handoffs explicitly retain their originals; default
checks retain only the bounded latest receipt. Public claims need independent expectations/public
authoring/detached copies. Digests prove identity, not behavior/permission; scoped
descendants cannot relabel proof and bytes/time do not measure API billing.

Stage intended paths, review diffs, integrate normally through main/required PR,
incorporate intervening work and independently verify remote ancestry. Normal
integration/configured release authority excludes purchases, production deployment,
credential/access/protection changes, force pushes and history rewrites. Accept final
archive bytes, then promote unchanged under current scoped authority. Never execute
candidates/verifiers with publishing credentials; require the owning terminal/public
result. Reuse healthy jobs, join owned work and preserve unrelated/unique resources.
At external waits report exact source/run/gate/evidence/next action. Finish with
capability, actual validation/limits, tested source versus remote main, compatibility
cut and publication state.
