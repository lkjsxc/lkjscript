# Repository guidance

## Mission and authority

Build lkjscript as a powerful general-purpose, agent-first language.
Prefer orthogonal language and library mechanisms over application-specific host behavior.
The accepted typed semantic graph is the canonical editable program authority.
Requests, plans, views, indexes, compiled products and caches derive from accepted meaning.
Operational data, credentials, deployment configuration and live resources have separate owners.
Ordinary application development should work with the distributed executable without this checkout.
Prefer simple static distribution where justified; prove each supported platform separately.

Follow current user direction and applicable global, ancestor, root, scoped and override instructions.
Replacing this file does not reload or override instructions already governing the running session.
Historical campaigns are evidence, not permanent user policy or new external authorization.
An explicit mandate may revise engineering choices with reasons, preserved guarantees and transition.
Use explicitly supplied work and reconciled lifecycle records; never choose work by newest filename.
Separate observed source, normative contracts, reported evidence and unverified assumptions.

## Safe startup and changes

Inspect actual HEAD/branch, index, worktree, relevant untracked work, stashes, remotes and divergence.
Read relevant guidance, lifecycle endings/resumptions, specifications, callers, tests and tooling owners.
Check toolchain/resources and active local or hosted jobs before duplicating expensive work.
An architect's inspected revision is an observation, not permission to rewind the actual checkout.
Reconcile intervening work; verify completed behavior instead of implementing it again.
Continue, close, explicitly supersede, or retain a precise blocker for inherited obligations.
A pending external run does not automatically displace independently valid engineering.

Preserve unrelated work, stashes, secrets, operational state and immutable published identities.
Stage and review explicit intended paths; never use broad staging to capture unrelated changes.
Do not reset, clean, broadly restore or rewrite history to manufacture compliance.
Run destructive tests only against explicitly owned disposable resources.
Clean up owned processes and temporary state without touching unrelated services or files.
Keep large logs, binaries, temporary checkouts and `.artifacts/` outputs out of tracked narrative.

## Navigation and ownership

`src/bin/lkjscript.rs` and `src/platform/` own the executable and platform.
Start public workflows at `cli.rs`, `control/`, `change/` and `normalized_query.rs`.
`kernel/` owns canonical typed records, identity, relations and semantic validation.
`publication/` and `GraphRepository` own accepted transitions, distinct from storage and witnesses.
`package_interface.rs`, `package_transport/` and `compiler/` own exact package/build boundaries.
`execution/normalized/` owns prepared execution, values/types, codecs, resources and reference logic.
`builtin_standard.rs` and the maintained accepted graph writer own built-in package integration.
`packages/standard/` and `applications/lkjournal/` are maintained consumers; `tests/` covers public use.
`contributor.rs` provides read-only observations; do not assume it is a graph generator.
`tools/lkjscript-dev/` owns contributor checks, independent oracles, target admission and release tools.
`docs/spec/` is normative; status, roadmap, decisions, performance, evidence and release have distinct uses.
Regenerate `docs/generated/` through its owner; generated views are not competing editable authority.
`docs/campaigns/` retains initial mandates and appended execution, reconciliation and resumption records.

Follow actual callers and contracts; directory names are navigation aids, not architectural proof.
Keep volatile identifiers, campaign matrices, current runs and temporary subsets in their owners.

## Meaning, effects and resources

Plan/apply must converge on normalization, identity allocation, complete validation and logical review.
Recheck the accepted base under the publication lock; persist content before atomic exposure.
Invalid input, cancellation, exhaustion and failure must not partially publish canonical meaning.
A derived operation failing after acceptance does not retroactively undo the accepted semantic write.

Preserve exact types, substitutions, effects, capability identities, provenance and evaluation order.
Names locate meaning; matching names or interfaces alone never establish equivalent authority.
Callable identity, declared effects and actual deployment grants are distinct facts.
A callable descriptor must not acquire authority merely by being created, passed, bound or returned.
Effectful invocation requires the declared calling context and actual checked component bindings.
Preserve canonical grant identity for operation admission, quotas, transactions and resource provenance.
Do not clone authority or reset accounting when control passes through a helper or imported library.
Pure execution cannot perform task effects; an empty task requirement set does not establish purity.

Keep containment, capture safety, equality, encoding, session retention and affine ownership distinct.
A safe callable signature does not certify its actual retained prefix or prepared origin.
A visited type edge never licenses skipping unsafe members or admission of separate raw values.
Raw invocation, decoder, artifact, host and retained-value boundaries require actual admission.
Valid immutable internal values may share checked proof without repeated descendant validation.
Preserve lexical transaction ownership and language-order borrow, consume, branch join and cleanup.
Earlier effects retain possible visibility after failure unless the applicable transaction rolls them back.
Never replay live effects merely to compare evaluators or silently retry a callback.

Admit bounded work/storage before growth and use checked arithmetic at resource-sensitive boundaries.
Check cancellation independently of profiling and before installation/publication where required.
Release invocation-owned resources on success, failure, cancellation, exhaustion and shutdown.
Keep profiling, cancellation, operational quotas, defensive limits, sandboxing and execution fuel distinct.
Ordinary trusted execution should not permanently require selecting fuel just to run a program.
Changing current defaults needs an explicit contract and proof, not incidental limit removal.
Static linkage, validation and resource accounting do not prove hostile-code isolation.

## Evolution and derived state

Separate logical identity, semantic revision, canonical encoding, artifacts and runtime representation.
Keep encoding independent of physical layout, addresses, hash order, paths and wall time.
Change the owner that needs to change; do not couple every improvement to a global version increment.
Preserve unchanged identities and bytes where their meaning is unchanged, with concrete evidence.
A necessary incompatibility needs migration or clear rejection, recovery and obsolete-path retirement.
Never silently discard live data or rewrite releases to make a transition appear seamless.
Prefer one maintained production path after cutover; temporary bridges need removal conditions.

Bind caches and proofs to actual revisions, dependencies, contracts, options, target and policy.
Clean recomputation remains an oracle; a cache cannot excuse canonical corruption.
Strict transported loading must independently admit content even when its producer reported success.
Reference implementations are proof tools, not competing editable program authorities.
Do not turn finite witnesses, provisional subsets or past limits into permanent language semantics.

## Contributor entry points and proof

Use the pinned toolchain, manifests and lockfiles; preserve safe Rust and production lint guarantees.
First-party contributor semantics belong in existing Rust tooling, not parallel Python/shell owners.
Preserve no-Python and product-surface gates unless a mandate explicitly justifies an equivalent revision.

Source-verified contributor entry points:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check focused --machine
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

Use `docs/spec/verification.md`, the actual check registry and `docs/release.md` for required gates.
Choose focused feedback during iteration and dependency-closed acceptance after inputs stabilize.
These profiles are choices by claim and impact, not a ritual to run all of them sequentially.
Widen proof when impact is uncertain; do not silently substitute a cheap check for a required gate.
Regenerate maintained semantic assets and public guides through their owners and verify equality.
Reuse compiled outputs safely; a build cache is not a correctness certificate.
Freeze copied candidates; concurrent builds must not overwrite the executable under verification.

New meaning requires freshly authored behavior through the real executable outside the checkout.
Exercise relevant discovery/edit/check/build/transport/execution/recovery boundaries.
Privileged fixtures, raw storage edits and hidden host behavior cannot substitute for a public feature.
Keep bounded public projections complete through explicit revision-bound pagination.
Label maintained adoption separately from newly constructed public witnesses.

Use independent expectations, disjoint reference logic, negative inputs and fault sensitivity as needed.
Shared production machinery cannot independently certify its own correctness.
Performance claims require equivalent semantics, baseline/workload identity and environment details.
Separate preparation, execution, allocation, cleanup and disabled instrumentation cost when relevant.
Retain unfavorable results; wall time alone does not establish complexity or provider quota charges.

Distinguish fresh passes, valid bound/reused evidence, failures, cancellations, skips, unavailable and unrun work.
Reuse requires the owning contract's complete input/verifier/environment/workload/trust bindings.
Shared evidence names its source and scope; never invent fresh receipts for unexecuted work.
Acquisition, integrity, admission, behavior and cleanup remain distinct proof boundaries.
Run expensive acceptance after meaningful inputs stabilize and rerun affected proof after changes.
Distinguish tested/release identities from later report-only containing commits.
Avoid self-referential evidence hashes and endless report/retest loops without weakening input binding.
Use existing concise evidence owners instead of parallel ledgers.

## Remote work, integration and delivery

Before dispatch/rerun, inspect matching runs and exact source/inputs; do not duplicate healthy work.
No idle polling, indefinite watches, sleep/status loops or repeated unchanged log reads.
Normally make one initial status read and one later read at a natural work boundary.
Additional reads need changed state, a specific diagnostic question or explicit user direction.
Bounded network retries are distinct from agent monitoring; neither permits indefinite waiting.
While a required hosted job runs, finish useful independent work without invalidating its inputs.
When only external waiting remains, end with commit, run/attempt, observed state and exact resumption action.
Do not promise background monitoring or merge around pending branch protection.

Complete the mandate's implementation, public behavior, consumer cutover, documentation and required proof.
Preserve its initial archive and append lifecycle records through the established convention.
A blocker must not erase safe progress or silently replace the assignment with an unrelated smaller task.
External authority comes from the user task, not this file, historical prose or tool availability.
Inspect current permissions, protection and automatic effects before authorized pushes and integration.
Repository integration, publication and production deployment are separate actions.
Deployment, destructive live-data changes, spending, credentials and broad administration need explicit authority.
Use only applicable scoped authorization for release control; do not change permissions/settings as a workaround.

Publish at selected meaningful milestones, not after every campaign and not indefinitely later.
When selected and ready, carry publication through the established release owner.
Keep exact-source preparation, target admission, immutable publication and public-download acceptance distinct.
A local build, upload, tag, green subjob or dispatch is not verified public delivery.
Preserve independent public route admission and truthful within-pair evidence binding.
Do not replace public tags/assets, fabricate fresh latest-route proof, or cancel publishing work blindly.
Report engineering, integration, publication, public acceptance and deployment separately.
Include actual commits, changed behavior, evidence scope, blockers, preserved work and owned cleanup.
