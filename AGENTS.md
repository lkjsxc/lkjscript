# Repository guidance

## Mission and authority

Build lkjscript as an ambitious general-purpose language for agents and humans.
Prefer composable language/library mechanisms over application-specific compiler or host behavior.
The accepted typed semantic graph is the canonical editable program authority.
Names, requests, plans, projections, indexes, compiled products and caches derive from accepted meaning.
Operational data, secrets, deployment configuration and live resources have separate owners.
Ordinary application development and execution should work with the distributed executable.
Distinguish an installed executable, a runtime-dependent application bundle and an executable embedding a runtime.
Prefer simple static distribution where justified; prove each supported platform separately.

Follow current user direction and applicable global, ancestor, root, scoped and override instructions.
An explicit mandate may revise engineering choices with reasons, preserved guarantees and a credible transition.
Historical exclusions and assistant conventions are evidence, not permanent user requirements.
Replacing this file does not reload or override instructions governing the running session.
External authority comes from the current task, not a file, tool availability or historical prose.
Use supplied work and reconciled lifecycle records; never select work merely by the newest filename.
Separate observed implementation, normative commitments, reported evidence and unverified assumptions.

## Safe startup and edits

Inspect actual HEAD/branch, index, worktree, relevant untracked work, stashes, remotes and divergence.
Read applicable guidance, active lifecycle endings/resumptions, specifications, callers, tests and tooling owners.
Check available toolchain/resources and active local or hosted jobs before duplicating expensive work.
An architect's observed revision is not an instruction to rewind the checkout.
Preserve intervening changes; verify already completed behavior instead of reimplementing it.
Continue, close, explicitly supersede, or retain a precise blocker for inherited obligations.
A pending external run does not automatically displace independently valid engineering.

Preserve unrelated work, stashes, secrets, operational state and immutable published identities.
Stage explicit intended paths, inspect the staged diff and avoid broad staging.
Do not reset, clean, broadly restore or rewrite history to manufacture a clean result.
Run destructive tests only against explicitly owned disposable resources.
Clean up owned processes, containers and temporary state without touching unrelated services or files.
Keep logs, binaries, temporary checkouts and `.artifacts/` products out of tracked narrative.
Archive the initial mandate unchanged and append concise execution/reconciliation/resumption records.

## Navigation and ownership

- `src/bin/lkjscript.rs` owns process dispatch, signals, output and process-level failure reporting.
- `src/platform/cli.rs`, `control/`, `change/` and `normalized_query.rs` own public graph workflows.
- `kernel/` owns typed records, identity, relations, substitutions and semantic validation.
- `publication/` and `GraphRepository` own accepted transitions; storage and witnesses have separate roles.
- `package_interface.rs`, `package_transport/` and `compiler/` own exact package/build/transport boundaries.
- `execution/normalized/` owns prepared execution, codecs, values/types, resources and reference logic.
- `execution/control.rs`, `runtime.rs` and `deployment.rs` own execution policy, lifecycle and operational binding.
- `project_creation/` owns recipe lowering and atomic auxiliary output; `contract/` owns public discovery.
- `builtin_standard.rs` and the maintained accepted graph writer own built-in package integration.
- `packages/standard/` and `applications/lkjournal/` are maintained consumers; `tests/` covers public use.
- `contributor.rs` provides read-only observations; do not assume it is a graph generator.
- `tools/lkjscript-dev/` owns contributor checks, independent oracles, target admission and release tools.
- `docs/spec/` is normative. Status, roadmap, decisions, performance, evidence and release have distinct owners.
- Regenerate `docs/generated/` through its owner. `docs/campaigns/` retains mandates and lifecycle records.

Follow actual callers and contracts; names and directory placement are navigation aids, not proof.
Keep temporary subsets, campaign matrices, commits, release identities and workflow IDs in their existing owners.
Discover actual command grammar before use; a mandate's proposed command is unavailable until implemented.

## Meaning, evaluation and authority

Plan/apply must converge on normalization, identity allocation, complete validation and logical review.
Recheck the accepted base under the publication lock; persist content before atomic exposure.
Invalid input, cancellation, exhaustion and failure must not partially publish canonical meaning.
A derived operation failing after acceptance does not retroactively undo the accepted semantic write.

Preserve exact types, substitutions, effects, capability identities, provenance and evaluation order.
Names locate meaning; matching names or interfaces alone do not establish equivalent authority.
Callable identity, declared effects and actual deployment grants are distinct.
Creating, binding, transporting or returning a callable must not acquire authority.
Effectful invocation requires both the calling context's allowance and checked component bindings.
Preserve canonical grant identity and accounting through helpers, imports and terminal calls.
Pure execution cannot perform task effects; an empty task requirement set does not establish purity.
A terminal transfer must preserve the original continuation and applicable transaction/resource owners.

Keep containment, capture safety, equality, encoding, session retention and affine ownership distinct.
A safe callable signature does not certify its retained prefix or prepared origin.
A visited type edge never licenses skipping unsafe members or admitting unrelated raw values.
Raw invocation, decoder, artifact, host and retained-value boundaries require actual admission.
Valid immutable internal values may share checked proof without repeated descendant validation.
Preserve language-order borrow, consume, branch joins, argument evaluation and cleanup.
Earlier effects may remain visible after failure except where an applicable transaction rolls them back.
Never replay live effects for differential evidence or silently retry an invocation.

## Runtime, policy and cleanup

Keep deployment preparation, application invocation, result encoding/delivery and cleanup distinguishable.
Resolve strict artifact/target/type/grant admission before loading secrets or constructing live adapters.
Where an entry has a typed external boundary, reject invalid arguments and intrinsically unencodable results before effects.
A late result/output/cleanup failure cannot undo committed effects or establish safe retry.
Preserve primary failures together with possible-visibility and cleanup evidence.

Execution fuel, cumulative work quotas, live/structural bounds, input admission, cancellation,
operational deadlines, grant limits, profiling and hostile-code isolation are different contracts.
Ordinary trusted execution should not require users to invent a work budget.
Select policy explicitly at its owning public boundary; do not weaken another route's defaults incidentally.
An absent cumulative quota is not permission to skip cancellation, checked storage arithmetic or grant accounting.
Keep single-value, container, codec and preparation bounds independent of invocation-lifetime counters.
Observation overflow must not masquerade as real resource exhaustion; exact quotas must still reject overflow.
Finite witnesses and representation limits are not automatically permanent language semantics.

Admit bounded storage before growth and retain checked size/index arithmetic.
Release invocation/deployment-owned resources on success, failure, cancellation, exhaustion and shutdown.
Requesting cancellation or dropping a future is not proof of joined cleanup.
Do not report unqualified success with remaining owned work or failed required cleanup.
Static linkage, safe Rust, validation and quotas do not establish a hostile-code sandbox.
Separate processes sharing an installed executable do not imply a shared scheduler or daemon.

## Evolution and derived state

Separate logical identity, semantic revision, canonical encoding, artifacts, operational data and runtime representation.
Keep encoding independent of physical layout, addresses, hash iteration order, paths and wall time.
Change the boundary that needs changing; do not force every improvement into a global version increment.
Preserve unchanged identities and bytes where their meaning is unchanged, with concrete evidence.
A necessary incompatibility needs detection, affected-consumer analysis, migration or rejection,
a viable recovery path and retirement of superseded behavior.
Never silently discard live data, reinterpret operator settings or rewrite immutable releases.
Prefer one production owner after cutover; temporary bridges need explicit removal conditions.

Bind caches/proofs to actual revisions, dependencies, contracts, options, target and policy.
Clean recomputation remains an oracle; a cache cannot excuse canonical corruption.
Transported loading independently admits content even when its producer reported success.
Reference implementations are proof tools, not competing editable program authorities.

## Contributor commands and verification

Use the pinned toolchain, manifests and lockfiles; preserve safe Rust and production lint guarantees.
First-party contributor semantics belong in existing Rust tooling.
Preserve no-Python and product-surface gates unless a mandate explicitly justifies an equivalent revision.
Do not create a parallel Python/shell implementation, verification framework or evidence ledger.

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
These profiles are choices by claim/impact, not a ritual to run sequentially.
Widen proof when impact is uncertain; never silently replace a required gate with a cheaper check.
Regenerate maintained semantic assets and public guides through their owners and verify equality.
Freeze copied candidates; concurrent builds must not overwrite bytes under verification.

New capability needs fresh public authoring and use through the candidate outside the checkout.
Exercise relevant discovery/edit/check/build/transport/execution/recovery boundaries.
Privileged fixtures, raw storage edits and hidden host helpers cannot substitute for product support.
Keep bounded public projections complete through explicit revision-bound pagination.
Distinguish maintained adoption from newly designed composition witnesses.
Live effects execute once; use separate deterministic adapters for reference proof.

Use independent expectations, disjoint reference logic, negative cases and focused fault sensitivity.
Shared changed machinery cannot independently certify itself.
Performance claims need equivalent semantics, a baseline, workload identity and environment details.
Separate preparation, steady execution, allocation and cleanup where relevant; retain unfavorable results.
Wall time or output bytes alone do not establish provider billing or model token savings.

Report fresh, valid bound/reused, failed, cancelled, skipped, unavailable and unrun evidence accurately.
Reuse requires complete relevant input/verifier/environment/workload/trust bindings.
A build cache is not a correctness certificate; inherited receipts are not fresh executions.
Keep acquisition, integrity, admission, behavior and cleanup distinct.
Rerun affected proof after relevant changes; distinguish tested/release source from later reporting commits.
Avoid self-referential evidence hashes and endless report/retest cycles without weakening bindings.

## Remote work, integration and publication

Before dispatch/rerun, inspect matching runs and exact source/inputs; do not duplicate healthy work.
No idle polling, indefinite watches, sleep/status loops or repeated unchanged logs.
Normally read status initially and once at a useful later boundary.
Further reads need changed state, a concrete diagnostic question or explicit user direction.
Bounded network retries are distinct from agent monitoring; neither permits indefinite waiting.
While required hosted work runs, finish useful independent work without invalidating frozen inputs.
When only external waiting remains, end with exact commit, run/attempt, observed state,
outstanding gate and resumption action. Do not promise background monitoring.

Complete the mandate's public behavior, consumer cutover, documentation, retirement and required proof.
A blocker does not erase safe progress or authorize silently substituting an unrelated smaller task.
Refresh permissions, protections, divergence and automatic effects before authorized push/integration.
Never merge around pending required checks or force-push to make delivery succeed.
Repository integration, repository release and production deployment are separate actions.
Deployment, destructive live-data changes, spending, credentials and broad administration require task authority.
Use only the release owner's scoped control protocol; do not change permissions/settings as a workaround.

Publish at selected meaningful milestones, not automatically per campaign and not indefinitely later.
Respect or explicitly reconcile inherited publication triggers.
When selected and ready, complete the existing release owner through public-download acceptance.
Keep exact-source preparation, target admission, immutable publication and public behavior distinct.
A build, upload, tag, dispatch or green subjob is not verified public delivery.
Preserve independent route admission and truthful within-pair evidence binding.
Do not overwrite public tags/assets, fabricate fresh latest-route proof or blindly cancel publishers.
Report engineering, integration, publication, public acceptance and deployment separately,
including actual revisions/actions, limitations, preserved unrelated work and owned cleanup.

