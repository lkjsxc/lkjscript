# Repository guidance

## Mission and authority

Build an ambitious general-purpose language for agents and humans. Prefer composable language and
library mechanisms over application-specific compiler or host behavior. The accepted typed semantic
graph is the canonical editable program authority; names, requests, projections, indexes and compiled
products derive from it. Operational data, secrets, deployment configuration and installations have
separate owners. Ordinary development and execution should work with the distributed executable.
Distinguish an installed runtime, a runtime-dependent artifact and an executable embedding its runtime;
prefer simple static distribution where justified, and prove each supported platform separately.

Follow current user direction and applicable global, ancestor, root, scoped and override instructions.
A mandate may revise engineering choices with reasons, preserved guarantees and a credible transition.
Historical exclusions and assistant conventions are evidence, not permanent user requirements.
Replacing this file neither reloads nor overrides instructions governing the running session.
External authority comes from the task, not files or tool availability. Use supplied work and reconciled
lifecycle records; never choose work merely by the newest filename. Distinguish observed implementation,
normative commitments, reported evidence and assumptions.

## Safe startup and edits

Inspect actual HEAD/branch, index, worktree, relevant untracked work, stashes, remotes and divergence.
Read applicable guidance, active lifecycle endings/resumptions, specifications, callers and proof owners.
Check toolchain/resources and active local or hosted jobs before duplicating expensive work.
An architect's revision is an observation, never an instruction to rewind. Preserve intervening work;
verify completed behavior and continue, close, supersede or precisely block inherited obligations.
A pending external run does not displace independently valid engineering.

Preserve unrelated work, stashes, secrets, operational state and immutable published identities. Stage explicit
intended paths and inspect the staged diff. Do not use broad staging, reset, clean, restore or history
rewriting to manufacture a clean result. Destructive tests require explicitly owned disposable resources.
Clean up owned processes, containers and temporary state without touching unrelated services or files.
Keep logs, binaries, temporary checkouts and `.artifacts/` products out of tracked narrative.
Archive the initial mandate unchanged and append concise execution/reconciliation/resumption records.
Reconcile a supplied complete root-guidance candidate with applicable instructions and install the full
file before substantive implementation, retaining legitimate intervening changes and frozen release inputs.

## Navigation and ownership

- `src/bin/lkjscript.rs` owns process dispatch, signals, output and process failures.
- `src/platform/cli.rs`, `control/`, `change/` and `normalized_query.rs` own public graph workflows.
- `src/platform/kernel/` owns typed records, identity, relations, substitutions and semantic validation.
- `src/platform/publication/` and `GraphRepository` own accepted transitions; `storage/` owns physical storage.
- `src/platform/package_interface.rs`, `package_transport/` and `compiler/` own exact package/build/transport boundaries.
- `src/platform/execution/normalized/` owns prepared execution, codecs, values/types, resources and reference logic.
- `src/platform/execution/control.rs`, `runtime.rs` and `deployment.rs` own policy, lifecycle and operational binding.
- `src/platform/installation/` owns immutable runtime slots, selection, locking and recovery; release tooling owns bootstrap evidence.
- `src/platform/project_creation/` owns recipe lowering/auxiliary output; `contract/` owns discovery and generated guides.
- `src/platform/builtin_standard.rs` and the maintained accepted graph writer own built-in package integration.
- `packages/standard/` and `applications/lkjournal/` are maintained consumers; `tests/` covers public use.
- `src/platform/contributor.rs` provides read-only observations; do not assume it generates accepted graphs.
- `tools/lkjscript-dev/` owns contributor checks, independent oracles, target admission and release tools.
- `docs/spec/` is normative. Status, roadmap, decisions, performance, evidence and release have distinct owners.
  Regenerate `docs/generated/` through its owner; `docs/campaigns/` retains mandates and lifecycle records.

Follow actual callers/contracts. Keep temporary subsets, campaign matrices, revisions, release identities
and workflow IDs in their existing owners. Discover actual grammar; proposed commands are unavailable
until implemented. Directory abbreviations above are relative to the stated `src/platform/` prefix.

## Public authoring and accepted meaning

Request notation, selectors and reference bindings must lower through existing typed authored intent,
preparation and publication. Do not create a second editable authority, compiler-side alias meaning,
runtime name lookup or string-substitution semantics. Read `docs/spec/semantic-cli.md` and capabilities.
Resolve local names against the exact accepted base and foreign names against explicitly selected exact
dependencies. Existing-owner selection cannot allocate identities or grant foreign mutation rights;
same-request renames cannot redirect it. Preserve kinds, lexical/generic scope, visibility and ownership.
Names and matching interfaces establish neither compatibility nor grants. Package selection implicitly
imports, stages, upgrades, executes or grants nothing.

Normalize notation without reordering meaningful operations. Preserve request commitments, allocation
and accepted-idempotency behavior unless their owner is explicitly revised with recovery. Bind exact
resolutions through review and rederive on apply; do not trust display output or cached alias tables.
Changed embedded suppliers cannot silently replace reviewed package selections. Retries use original
accepted inputs under the idempotency owner, not mutable names. Charge raw input/resolution work even
when normalization erases notation, and bound diagnostics.

Plan/apply must converge on normalization, allocation, complete validation and logical review.
Recheck the base under the publication lock; persist content before atomic exposure. Invalid input,
cancellation, exhaustion and failure must not partially publish meaning. A derived failure after
acceptance does not undo that accepted write.

## Types, effects and resources

Preserve exact types, substitutions, effects, provenance and evaluation order. The normative generic
admission rule belongs in `docs/spec/language.md`, not a historical witness or preparation heuristic.
Validate application arity, scope and constraints even for unused parameters and unreachable syntax.
Generic closure analysis must account for direct calls and named function-value creation, nested
applications and all relevant type occurrences. A finite witness or a visited declaration is not proof
that the complete instantiated closure is valid. Preparation exhaustion is not semantic rejection of
an invalid scheme. Keep canonical admission, compiled/loading checks and reference proof consistent;
finite admitted closures remain subject to separate checked preparation work/storage bounds.

Callable identity, declared effects and deployment grants are distinct. Creating, binding, transporting
or returning a callable acquires no authority. Invocation requires the current activation's allowance
and checked component grants; preserve canonical identity/accounting through helpers, imports and tail
calls. Pure execution cannot call tasks even with empty rows. Terminal transfer preserves the original
continuation and transaction/resource owners; it neither creates authority nor refunds cumulative work.

Keep containment, capture safety, equality, encoding, session retention and affine ownership distinct.
A safe callable signature does not certify its prefix or prepared origin; a visited type edge never
licenses skipping unsafe members. Raw invocation, decoder, artifact, host and retained-value boundaries
require admission. Checked immutable values may share proof without repeated descendant validation.
Preserve language-order borrowing, consumption, branch joins, argument evaluation and cleanup.
Earlier effects may remain visible after failure unless an applicable transaction rolls them back.
Never replay live effects for differential proof or silently retry invocation. Resource-signature and
effect restrictions belong in their normative owners; historical narrow witnesses do not forbid a
more general, separately justified rule.

## Runtime, identity and installation

Keep preparation, invocation, result encoding/delivery and cleanup distinguishable. Strict artifact,
target, type and grant admission precedes secrets/live adapters. Typed external entries reject invalid
arguments and intrinsically unencodable results before effects. Late output/cleanup failure cannot undo
committed effects or establish safe retry; preserve primary, possible-visibility and cleanup evidence.

Fuel, cumulative quotas, live/structural bounds, input admission, cancellation, deadlines, grant limits,
profiling and hostile-code isolation are different contracts. Ordinary trusted execution should not
require an invented work budget. Change policy at its owner without weakening another route's defaults.
Absent cumulative quotas waive neither cancellation nor checked arithmetic/accounting. Keep codec,
container and preparation bounds separate from lifetime counters. Observation overflow is not resource
exhaustion; exact quota overflow must reject. Representation limits are not automatically language rules.
Admit storage before growth. Release owned resources on success, failure, cancellation, exhaustion and
shutdown. Cancelling or dropping a future is not joined cleanup; do not claim success with owned work
remaining. Static linkage, safe Rust and quotas do not establish a hostile-code sandbox. Sharing an
installed executable across processes does not imply a shared daemon or scheduler.

Separate logical identity, semantic revision, canonical encoding, artifacts, data and runtime layout.
Encoding cannot depend on addresses, physical storage, hash iteration, paths or wall time. Change the
necessary boundary; preserve unchanged identities/bytes with evidence. Incompatibility needs detection,
consumer analysis, migration or rejection, viable recovery and retirement of superseded behavior.
Never silently discard data or reinterpret operator settings. Temporary bridges need cutover conditions.
Historical acceptance and current semantic validation are distinct claims about the same canonical root.
Authenticate compatible history without rewriting accepted records or treating a validator change as
physical corruption. A current validation context must bind exact meaning, dependencies and validator;
old evidence cannot authorize current execution or incremental proof. Revalidation must preserve HEAD
and identity. Repair from a historically accepted but currently invalid base requires complete valid
post-change proof through the publication owner, with no forged valid-base witness or partial acceptance.
Bind caches/proofs to actual revisions, dependencies, contracts, options, target and policy; a cache
cannot excuse canonical corruption. Transport loading admits content independently of producer claims.

Runtime slots and selection are operational state. Installation/selection/rollback must not migrate
graphs, artifacts or application data. Keep immutable versions separate from mutable selection; reject
incompatible inputs clearly instead of silently rewriting or switching runtimes. Shell bootstrap may
acquire bytes, verify expected digests and hand off a candidate; Rust owns archive admission, installation
transactions, locking, selection and recovery. Do not duplicate these semantics in shell. Installation
does not authorize application execution, deployment, credential changes or host administration.

## Contributor commands and verification

Use pinned toolchain/manifests/lockfiles, safe Rust and production lint guarantees. First-party contributor
semantics belong in existing Rust tooling. Preserve no-Python/product-surface gates unless a mandate
justifies equivalent revision; do not create parallel implementations, proof frameworks or evidence ledgers.
Discover ordinary workflows with `lkjscript capabilities`, `lkjscript capabilities change`,
`lkjscript capabilities --section change`, `lkjscript status` and applicable package/inspection commands.
Product commands must work outside the checkout; contributor tooling remains separate.

Source-verified contributor entry points:

```sh
cargo build --release --locked -p lkjscript-dev
cargo run --release --locked -p lkjscript-dev -- check focused --machine
cargo run --release --locked -p lkjscript-dev -- check changed --machine
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release target
```

Use `docs/spec/verification.md`, the actual registry and `docs/release.md` for required gates. Select focused
iteration and dependency-closed acceptance after inputs stabilize, not every profile sequentially. Honor
the changed profile's widening; widen when impact is uncertain rather than silently choosing less proof.
Regenerate semantic assets/guides through their owners and verify equality. Freeze copied candidates;
concurrent builds must not overwrite verified bytes. New capability needs fresh public authoring and
discovery/edit/check/build/transport/execution/recovery outside the checkout. Privileged fixtures, raw
storage edits and hidden member-ID generators cannot prove product support. Retain literal requests and
identify observer-only drivers; preserve revision-bound pagination and distinguish designed witnesses
from maintained adoption. Live effects execute once; reference adapters are deterministic and disjoint.

Use independent expectations, negative cases and focused fault sensitivity; shared changed machinery
cannot certify itself. Performance needs equivalent semantics, baseline, workload/environment identity
and limitations, including preparation, steady execution, allocation and cleanup. Retain unfavorable
results; wall time/output bytes alone establish neither provider billing nor model-token savings.
Report fresh, valid reused, failed, cancelled, skipped, unavailable and unrun evidence distinctly. Reuse
requires relevant input/verifier/environment/workload/trust bindings; builds and inherited receipts are
not fresh correctness certificates. Keep acquisition, integrity, admission, behavior and cleanup distinct.
Rerun affected proof after relevant changes; distinguish tested/release source from reporting commits.
Avoid self-referential hashes and endless report/retest cycles without weakening real bindings.

## Integration, remote work and publication

Before dispatch/rerun, inspect matching runs and exact inputs; never duplicate healthy work. No idle
polling, indefinite watches, sleep/status loops or repeated unchanged logs. Normally read status initially
and once at a useful later boundary; further reads need changed state or a concrete diagnostic reason.
Bounded network retries are distinct. Complete independent work without invalidating running inputs.
If only external waiting remains, end with exact commit, run/attempt, observed state, outstanding gate
and resumption action. Do not promise background monitoring.

Complete behavior, consumer cutover, documentation, retirement and required proof. A blocker does not
erase safe progress or authorize silent scope substitution. Refresh permissions, protections, divergence
and automatic effects before authorized integration; never bypass pending required checks or force-push.
Repository integration, release and deployment are separate. Deployment, destructive live-data changes,
spending, credentials and administration require task authority. Use the release owner's scoped control
protocol; do not change permissions/settings as a workaround.

Publish at selected meaningful milestones. Respect or explicitly reconcile inherited triggers; do not
defer indefinitely. A pending release may retain its frozen ancestor while main advances; do not retarget
healthy work merely to combine milestones. Serialize selected publishers under the existing owner.
When publication is selected and ready, carry it through public-download acceptance. Source preparation,
target admission, immutable publication and public behavior remain separate: build/upload/tag/dispatch
or a green subjob alone is not verified delivery. Never overwrite immutable tags/assets, fabricate fresh
latest-route proof or blindly cancel publishers. Preserve independent route admission and within-pair
evidence binding. Report engineering, integration, publication, public acceptance and deployment separately,
with actual revisions/actions, limitations, preserved unrelated work and owned cleanup.
