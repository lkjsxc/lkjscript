# Repository guidance

## Mission and authority

Build an agent-first programming language and application platform whose capabilities compose.
Agents must be able to discover, author, inspect, change, validate, execute, and operate meaningful
programs through the distributed product. Human-familiar syntax and manual editing are not primary
constraints; precise semantics, bounded context, useful diagnostics, and predictable behavior are.
Applications, including `lkjournal`, are consumers and proving workloads, not automatic owners of
platform priorities. Prefer reusable language, library, runtime, and operational mechanisms over
host-implemented application policy. A representative public-path witness may justify a foundation
that existing limitations prevent consumers from expressing. Do not impose an arbitrary count of
pre-existing applications. Prove usefulness and composition, and distinguish a new witness from
existing adoption.

One accepted revision of the typed semantic graph is the sole editable authority for program
meaning. Text, requests, projections, indexes, caches, compiler structures, package transports,
artifacts, receipts, and host objects are not parallel editable programs. Names locate; semantic
identities preserve continuity. Generic runtime mechanisms and necessary host adapters are
legitimate, but metadata around application-specific Rust behavior is not graph-owned behavior.
Operational data, queues, object bytes, deployment policy, credentials, and running services have
separate explicit authorities. Live effects never silently advance semantic `HEAD`.

Follow the applicable instruction hierarchy and current explicit user decisions. This file applies
repository-wide; inspect more specific guidance before working in its scope. A supplied campaign
may explicitly revise an engineering rule with rationale and an affected-owner cutover; neither an
old campaign nor the newest filename automatically establishes current authority.

Keep instructions, normative contracts, observations, decisions, and proof distinct. Specifications
state intended behavior; code and execution establish implemented behavior; tests can expose a
bug or a deliberately changed contract. Explain discrepancies rather than ranking all these as
interchangeable instructions. Historical preferences, limitations, roadmaps, and measured results
are not permanent user policy.

## Repository owners

- `src/platform/` owns semantic authority, public control, publication, compiler/runtime, and host
  adapters. `src/bin/lkjscript.rs` owns process entry. `tests/` owns public black-box acceptance.
- `packages/standard/` owns maintained standard graph meaning and its generated consumer assets.
  `applications/lkjournal/` owns the maintained application, not language policy.
- `tools/lkjscript-dev/` owns contributor verification, independent application oracles, scale,
  target admission, release preparation, and evidence. Workflows orchestrate those owners rather
  than duplicating their policy.
- `docs/spec/` owns normative behavior; `docs/architecture.md` maps authority and dependencies;
  `docs/status.md` records current proved state; `docs/roadmap.md` holds contingent hypotheses;
  `docs/decisions/` holds rationale and reversal conditions. `docs/performance.md` and
  `docs/evidence/` retain bounded evidence. `docs/release.md` owns distribution procedure.
  `docs/generated/` is generated through its executable owners. `docs/campaigns/` archives initial
  mandates and appended lifecycle records; do not introduce a competing status ledger.

## Startup and work safety

Read the complete supplied mandate and applicable global, root, scoped, and override guidance.
Check actual instruction discovery and configured size limits when relevant; do not assume all
nested files were automatically loaded. Editing guidance does not reload a running agent session.
Apply consequential reconciled instructions explicitly, without changing global configuration.

Establish actual branch, commit/tree, index/worktree, relevant untracked files, remotes/divergence,
toolchain, preserved work, and in-flight external actions. A historical baseline is orientation,
not permission to rewind. Read complete lifecycle records of potentially active campaigns and
reconcile real unfinished work before selecting an unrelated objective. Source, tag, workflow,
downloaded binary, and running service are different states. A source/public version difference,
an optional unperformed push, or unavailable ignored historical logs alone does not create an
active release obligation. Give actual irreversible external work an explicit safe disposition.

Apply the supplied mandate's bounded adaptation contract. Distinguish fixed outcomes and safety
invariants from adjustable implementation, generation, filename, and pre-publication version details.
Incorporate legitimate progress with an appended reconciliation and renewed affected proof; a
semantic addition is not harmless merely because it is a superset. Verify and close an already
satisfied outcome rather than reimplementing it. A real conflict stops the dependent unsafe action,
not independent safe obligations. Adaptation cannot lower acceptance or silently change the objective.

Preserve unrelated changes, stashes, unknown projects, and user data. Stage explicit paths and
inspect the staged diff; do not use broad `git add .` or `git add -A`. Make coherent scoped commits.
Avoid unrelated formatting, dependency changes, and generated-file churn. Do not commit secrets,
private deployments, large logs, temporary resources, or `.artifacts/` contents.

Local edits, local commits, normal pushes, workflow dispatch, publication, deployment, settings,
credentials, and operational-data mutation require distinct authorization. Permissions and older
campaigns do not supply it. Before an authorized normal push, refresh the remote and check safe
fast-forward state and automatic effects. Never rewrite shared history, force-push, retag published
identities, delete unfamiliar work, or bypass execution safeguards. Do not reset, clean, restore,
rebase, or remove stashes to manufacture a clean checkout.

Use disposable isolated resources for destructive tests. Bound and clean up owned services, files,
processes, ports, and containers. Resource shortages do not authorize host, production, credential,
or paid-infrastructure changes. Diagnose an unchanged failure rather than repeatedly rerunning it;
one failed build does not establish a universal minimum resource requirement.

## Engineering invariants

Use the pinned toolchain, workspace manifests, and lockfile. Production Rust is safe; preserve the
repository's unsafe-code and panic/unwrap/expect/todo/unimplemented lint boundaries and narrowly
scoped test allowances. Keep contributor tooling in the first-party Rust owner; do not add Python
or a shell-based semantic implementation. Pin and verify external inputs where the owner requires
version and digest admission.

Use checked arithmetic and explicit limits before allocation, traversal, growth, and output.
Keep canonical serialization and identity deterministic. Do not derive stable identity from enum
order, incidental Serde/layout choices, host paths, wall time, or hash iteration. Regenerate through
the canonical producer, never by hand-editing generated contracts or application assets.

Typed requests converge on `GraphRepository`, the normal accepted-authority writer. Plan and apply
share normalization, allocation, impact, validation, selected tests, and logical review meaning.
Validate the exact complete candidate, recheck its base under publication ownership, durably prepare
canonical content, then expose accepted meaning atomically. Stale, malformed, cancelled, exhausted,
corrupt, or interrupted preparation never partially publishes. Failure of post-acceptance derived
work must not be reported as a failed semantic write.

Caches, summaries, and compiled outputs are disposable derivations with explicit input witnesses.
Bind reuse to the relevant repository/revision, contracts, options, targets, dependencies, and
policies. Clean recomputation must detect invalid reuse; never repair canonical corruption by
trusting a cache. Strictly validate transported and loaded artifacts before use. Keep normal
production paths singular; references and test-only oracles may be implementation-disjoint without
becoming alternate editable authority.

Preserve exact type, effect, capability, lifetime, and resource provenance boundaries through
optimization. Retain cancellation, accounting, deterministic evaluation where promised, and cleanup.
Distinguish ordinary expected results, traps, exhaustion, corruption, infrastructure failure, and
possible external visibility. A committed external effect is not undone by a later language error.
Transactions only promise rollback within their actual authority. Never replay live effects through
the reference evaluator to obtain comparison evidence.

Backward compatibility is not the default goal, but breaking changes require a complete affected-
consumer cutover, appropriate rejection and recovery, and deletion of replaced production paths.
Preserve unchanged supported behavior, formats, and identities; do not create gratuitous identity
churn. Transitional mechanisms need a removal condition. Separate semantic revisions, package
identities, encodings, source versions, executable versions, targets, and artifact digests. Version
movement does not authorize tagging, publication, or deployment.

## Public product and operational boundaries

The public registry must exhaustively describe supported operations, records, types, errors, limits,
and recovery. Use one precise public mechanism rather than aliases, silent fallback, or checkout-
only generators. Adapters are transports to typed operations, not alternate authoring authorities.
Bound output explicitly; revision-pin queries and continuations and reject stale or invalid ones
rather than silently truncating or mixing revisions. Generated discovery must match executable
behavior, including negative cases.

Prove the complete public path. An internal builder or frozen fixture is not general authoring;
a host test is not copied-binary operation. Ordinary application development must not depend on a
private checkout. Prefer self-contained static distribution where exact-target inspection and
execution support it; linker flags do not establish portability. Additional targets need independent
admission. Do not infer sandboxing, multi-tenant isolation, inbound transport security, signing,
reproducibility, or general portability from unrelated tests.

Deployment grants and live-state ownership stay explicit. Validate configuration before readiness;
redact secrets and clean partial startup. Operational effects cannot rewrite graph meaning. Public
release identities are immutable; recovery must be additive and follow the distribution owner and
the current authorization.

## Verification and completion

Use `tools/lkjscript-dev/src/check/` and `docs/spec/verification.md` for gate selection, dependency
closure, fingerprints, reuse, and receipts. The verified workspace entry is
`cargo run --release --locked -p lkjscript-dev -- check`; its profiles include `focused`, `changed`, `product`,
`service`, `full`, and `self-test`. Discover their arguments from the owner. Iterate with the
narrowest useful checks. After relevant inputs stabilize, run the applicable authoritative full
profile freshly, plus independently required exact-candidate, target, or service admission. A host
full receipt does not substitute for target evidence. Do not rerun unchanged full checks without
an invalidation reason, lower gates to obtain a pass, or require every historical suite indiscriminately.

Acceptance must be able to disprove the claim. Use independent fixed expectations, disjoint
reference behavior, algebraic properties, invalid programs, and relevant failure/recovery cases.
Identify shared machinery and demonstrate fault sensitivity where agreement could be tautological.
Graph-owned behavior cannot be a hardcoded consumer identity; optimized execution cannot hide a
fallback, moved scan, changed semantics, or unaccounted work. Measurements retain units, workloads,
configuration, failures, and trade-offs; synthetic observations are not production measurements.

Classify work as `fresh passed`, `reused passed`, `skipped`, `unavailable`, `failed`, or `not run`
using the harness's equivalent owned states. Required unavailable or failed proof blocks completion.
Bind evidence to actual source/candidate inputs, toolchain, environment, workload, verifier, and
policy. Inspect receipts and outcomes, not only exit status. Later changes invalidate the checks
whose inputs they alter; calling a change documentation-only does not exempt it.

Keep large raw evidence outside tracked narrative. Retain bounded reproducible summaries and exact
input identities in the established evidence owner; ignored paths alone are insufficient for a fresh
session. Avoid self-referential evidence hashes. A tracked report may bind independently identifiable
implementation inputs, with final containing-commit and receipt identities reported out of band.
Do not label unexecuted final checks passed.

When the current mandate selects a public release, its completion includes publication and the
distribution owner's required independent exact-version and latest-download acceptance. Preparation,
a pushed tag, a dispatched workflow, and published-but-unverified bytes are distinct incomplete
states. Unavailable external authority blocks the affected action and claim, not useful safe local
work. Report implementation, distribution, and deployment outcomes separately; a release grant is
not a deployment grant. Release selection and concrete permissions belong in the current mandate.

Completion requires the fixed outcome, full public path, affected-consumer migration and predecessor
removal, accurate current/generated documentation, required passing proof, and accounted worktree
and external state. Preserve the archived initial mandate; update only allowed lifecycle metadata
and append terminal or resumption records. Report complete, blocked/incomplete, terminated, or
explicitly superseded with exact start/final identities, commits, acceptance/check dispositions,
durable evidence, deviations, limitations, preserved work, cleanup, and external actions actually
taken. A partial prototype is not completion. Preserve safe useful work when blocked and state the
smallest concrete condition for resumption, without silently selecting another objective.
