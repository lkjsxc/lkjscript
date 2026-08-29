# Repository Agent Instructions

## Scope and instruction precedence

This file applies to the repository root and all descendants unless a nearer `AGENTS.md` or
`AGENTS.override.md` is more specific for the files being changed.

Apply instructions in this order:

1. active user instructions;
2. a campaign explicitly named by the user;
3. exactly one implementation mandate reconciled as active against the complete campaign file, the
   active checkout, and relevant external state;
4. the nearest applicable agent instruction file;
5. executable public contracts, black-box tests, and verification policy;
6. normative specifications under `docs/spec/`;
7. current proved facts in `docs/status.md` and `docs/architecture.md`;
8. current implementation and executable-generated documentation;
9. completed or terminated campaigns, durable decisions, historical prompts, comments, and older
   commits.

Do not select a campaign because it has the newest filename or because its opening mandate still
contains words such as `incomplete`. Read the whole file. Its authoritative top-level status and
latest terminal completion or termination record decide lifecycle. A campaign is active only when
its authoritative status remains incomplete, its objective still matches current checkout and
external state, and it has not been superseded by later proved work.

When several mandates appear active, statuses conflict, or no mandate explains current in-flight
work, reconcile commits, working tree, current consumers, terminal records, tags, releases,
workflows, and user instructions before editing. Do not stack a new independent objective or choose
one arbitrarily. Files under `prompts/` are historical unless the user names one.

A baseline SHA is orientation. Audit later commits. A remote default branch, local `HEAD`, tag,
release, workflow artifact, and deployed service are distinct states. Historical text is evidence,
not current requirement; revalidate terminology, versions, limits, and assumptions.

## Mission and enduring invariants

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole editable authority for program
  meaning.
- Human-authored source text, compact requests, projections, indexes, artifacts, runtime handles,
  deployment data, caches, plans, receipts, release archives, and checksums are derived,
  operational, or evidentiary; none is a second editable program authority.
- Ordinary application development must work through the distributed `lkjscript` executable.
- Application policy belongs in graph meaning. Rust owns generic semantic, compiler, runtime,
  adapter, verification, and distribution mechanisms.
- Mutable names are locators. Stable typed identities express continuity. Exact semantic
  references, dense runtime indexes, content identities, revisions, and physical locations are
  separate domains.
- Accepted writes validate an exact-base complete candidate, make immutable canonical data durable,
  and expose one atomic visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance
  accepted authority.
- Backward compatibility is not a default. Move every maintained consumer, reject predecessor
  inputs, and delete predecessor paths in one dependency-closed cutover.
- Stored and hot representations remain replaceable without changing language meaning.
- AI-first design requires deterministic discovery, bounded outputs, actionable diagnostics,
  recoverability, and independent verification. It does not justify ambiguous contracts, hidden
  mutable state, excessive terminology, or reduced readability.

Do not elevate the current Rust representation, module layout, schema, artifact encoding, target,
packaging format, CI provider, release service, or storage provider into language semantics.

## Version and identity authority

Keep version domains explicit.

- The root `lkjscript` package version is the human-facing product release snapshot and owns its
  annotated release tag. It is not a universal version for the language, meaning graph, CLI,
  repository, artifact, deployment, runtime, standard package, or contributor tooling.
- Each public or stored contract has one canonical owner and advances independently when its own
  representation or behavior requires it. Do not synchronize unrelated contract numbers or bump a
  contract to match a product release.
- Registry content and its digest may change without changing the registry encoding contract.
- Semantic revisions, package revisions, artifact identities, target triples, commit SHAs, and file
  digests are identities, not aliases for the product version.
- Unpublished workspace tools do not inherit the product version merely because they share the
  workspace.
- The release manifest binds the exact release version, source, target, candidate bytes, and public
  contract identities for one product snapshot. It does not select semantic authority.
- Do not add a duplicate `VERSION` file, workspace-wide version table, edition ladder, generation
  alias, or handwritten catalog of current contract identities. Use the existing owner or an
  executable-generated projection.

Normative compatibility belongs to the owning contract and predecessor policy. Current release
facts belong in `README.md` and `docs/status.md` only after publication is proved.

## Repository map and information ownership

- `src/platform/`: semantic authority, repository, compiler, runtime, adapters, and public control.
- `src/bin/lkjscript.rs`: distributed process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, release, service, scale, and evidence
  tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package authority and generated consumer assets.
- `applications/lkjournal/`: maintained application authority and deployment material.
- `.github/workflows/`: hosted orchestration; reusable validation belongs in first-party tooling.
- `docs/campaigns/`: implementation mandates and concise terminal evidence.
- `docs/spec/`: normative behavior.
- `docs/status.md`: current proved facts and limitations.
- `docs/architecture.md`: current dependency direction and boundaries.
- `docs/roadmap.md`: deferred evidence-gated work.
- `docs/performance.md` and `docs/evidence/`: measurements and structured evidence.
- `docs/generated/`: executable-generated contracts and catalogs; never hand-edit.
- `docs/decisions/`: durable decisions and reversal conditions.
- `docs/release.md`: release preparation, publication, verification, and recovery procedure.
- `prompts/`: historical inputs.

Do not turn `AGENTS.md` into a campaign plan, architecture specification, current-status report,
measurement log, contract catalog, release ledger, or roadmap.

## Start of work

Before editing, capture:

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -20 --oneline --decorate
git remote -v
find .. \( -name AGENTS.md -o -name AGENTS.override.md \) -print
rustc --version
cargo --version
```

Then identify:

- the uniquely reconciled active campaign or implicit in-flight work;
- pre-existing and later work;
- current semantic, operational, distribution, and external authorities;
- maintained consumers and unsupported examples;
- independent oracles;
- migration and deletion targets;
- irreversible effects and recovery branches; and
- the smallest authoritative path and symbol set needed to begin.

Prefer exact symbol, contract-owner, consumer, and history searches over broad rescans. Inspect
external GitHub state only when the objective depends on it, and recheck volatile state immediately
before acting.

## Git, working-tree, and history safety

Do not rewrite history or destroy unfamiliar work. Unless the active user explicitly requires and
understands the exact action, do not use:

```text
git reset --hard
git clean -fd
git checkout -- .
git restore .
git rebase
git push --force
git push --force-with-lease
```

- Preserve unrelated and pre-existing work.
- Stage explicit paths; do not use `git add .` or `git add -A`.
- Inspect `git diff --cached` before every commit.
- Keep commits coherent and pair behavior with its proving tests.
- Avoid unrelated formatting, renames, dependency updates, or cleanup.
- Do not commit `.artifacts/`, transient migration/release output, downloaded tools, private
  databases, or large logs.
- Fetch and inspect remote state before a normal push; push only by fast-forward when safe and when
  the active campaign authorizes it.
- Never move, replace, force-update, or delete a published tag or immutable release.
- End clean unless preserved pre-existing work is identified explicitly.

A normal push, workflow dispatch, tag, release, migration, deployment, or destructive data action is
not implied by permission to edit the checkout. Perform an external mutation only when the active
campaign explicitly authorizes it and all named preconditions hold.

## Build and verification

The workspace uses stable Rust 2024 and the locked dependency graph. When present,
`rust-toolchain.toml` owns the exact repository toolchain and added targets.

Use narrow checks while iterating:

```sh
cargo fmt --all -- --check
cargo test --locked --lib FILTER
cargo test --locked --test public_cli FILTER
cargo test --locked --test general_service FILTER
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
```

Repository verification entry points are:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

Run the narrowest relevant test during implementation, product verification after maintained
consumer/generated-asset changes, service verification after runtime/deployment changes, and a fresh
full profile only after final content is assembled. A release or target-admission campaign may
require a separate exact-candidate receipt in addition to source full verification. Do not treat a
host build as proof for a different distributed target.

The harness owns gate dependencies, fingerprints, runtime identity, bounded logs, required outputs,
evidence reuse, classifications, and receipts. Reuse only exact-input evidence when the profile
permits it. Never report reused, skipped, unavailable, stale, failed, or unrun evidence as fresh
passed evidence. Keep large output under `.artifacts/` and return paths, identities, digests, and
bounded failure summaries.

Report genuine environment unavailability; do not weaken or omit a required gate. A gate required
for a supported release target cannot become optional because one local machine lacks a dependency.

## Rust and first-party tooling

- Rust is the default implementation and repository-tooling language.
- Do not add Python or replace typed validation with shell-only logic. Shell and workflow steps
  orchestrate explicit first-party commands and standard tools; they do not own product semantics or
  duplicate acceptance logic.
- Keep `Cargo.lock` authoritative. First-party Rust remains safe; production lint policy forbids
  `unsafe`, `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.
- Tests may use narrow, explicit allowances with a reason.
- Prefer typed objects, checked conversions, deterministic collections, explicit ownership, bounded
  decoding, and canonical serialization.
- Reject malformed tags, foreign identities, duplicates, noncanonical order, overflow, trailing
  input, path traversal, and exhausted limits at the owning boundary.
- Do not couple contract bytes or digest identity to Rust enum order, serde implementation shape,
  memory layout, filesystem paths, wall clock, or hash iteration.
- Regenerate owned artifacts through their executable or typed owner.
- Prefer existing dependencies or checked platform tools over unnecessary additions. Pin downloaded
  tools and test images by exact version and cryptographic digest before execution.
- Delete temporary migration, feasibility, packaging, and predecessor tooling after its last
  consumer.

When a native dependency or cross-target build fails, identify the exact build script, compiler,
linker, feature, and maintained behavior before changing dependencies. Do not remove public behavior
merely to make a target compile.

## Semantic authority, compiler, and runtime

- Lower public mutations to typed semantic intent before validation or publication.
- The repository publication API is the sole normal accepted-authority writer.
- Plan and apply share normalization, allocation, impact analysis, validation, and logical-result
  construction. Review evidence binds every claimed semantic effect.
- Keep witnesses, compiler scheduling, physical layout, caches, volatile observations, and
  distribution metadata outside semantic review identity unless a normative contract says
  otherwise.
- Reprepare repository-dependent facts and recheck the exact base under the publication lock.
  Durable canonical data precedes the single visibility change.
- Derived witnesses, indexes, compiler state, artifacts, plans, deployment data, receipts, and
  release files may rebuild but cannot select or silently alter accepted meaning.
- Missing disposable state may rebuild. Inconsistent canonical authority is corruption.
- Keep implementation-disjoint complete oracles until retained evidence justifies removal.
- A derived failure after accepted publication must not be reported as a failed semantic write.

Compiler caches, manifests, artifacts, dense indexes, and prepared programs derive from exact
accepted authority. Reuse only when repository, revision, semantic state, contracts, options, target,
and dependency closure match. Clean and incremental compilation must agree. Strictly validate
artifacts before execution or publication. Preserve separate production and canonical-reference
oracles for pure behavior; never duplicate live effects. Release resources on cancellation,
exhaustion, failure, and shutdown.

Do not add JIT, AOT, SIMD, memory mapping, custom allocators, resident sessions, specialization, or
broad caching without a measured maintained workload, independent oracle, and reversal condition.

## Language abstraction discipline

- Treat a typed semantic form implemented by the graph, validator, compiler, and runtime but
  unavailable through the distributed authoring surface as incomplete public capability, not as a
  finished language feature.
- Prefer completing an existing typed form and expressing reusable pure behavior in
  `packages/standard/` before adding a host intrinsic, special opcode, source generator, macro, or
  second authoring representation.
- A reusable provider-independent operation belongs in standard Graph meaning when existing
  language forms can express it correctly. Rust owns a new primitive only when maintained workloads
  and independent evidence show that ordinary meaning cannot own the required semantics or resource
  boundary.
- Every new abstraction must define its exact types, effect boundary, evaluation order, identity,
  equality and durability status, failure behavior, resource ownership, maintained consumers,
  independent oracle, migration, deletion, and reversal condition.
- Prefer named functions, explicit parameters, explicit type arguments, and explicit state before
  adding hidden capture or inference. Do not add lambdas, lexical capture, implicit coercion,
  constraints, dynamic dispatch, or textual metaprogramming merely to shorten compact requests.
- Function values and other abstractions must not hide capabilities, grants, secrets, live handles,
  external visibility, or mutable authority. Optimization representations remain derived and
  replaceable.
- A standard-library abstraction is complete only when at least one real maintained application
  uses it through the public executable and production/reference behavior agrees. A contrived
  fixture alone is not product evidence.

## Public CLI and protocols

- The executable registry exhaustively owns finite public operations, grammar, request/response
  models, limits, diagnostics, authority effects, and security nonclaims.
- One behavior has one public name. Reject unknown commands/options; do not add fallback routing or
  compatibility aliases.
- Public transports adapt into typed requests. Raw JSON, storage objects, generated schemas, source
  text, and review projections are not required authoring authorities.
- JSON may be a strict bounded runtime, deployment, evidence, or release-metadata adapter when its
  owner requires it.
- Finite output is deterministic, bounded, machine-readable, and names the observed revision when
  reading meaning. Classified finite outcomes use stdout and keep stderr empty unless the contract
  says otherwise.
- Growing results require explicit budgets and continuations or bounded files; never silently
  truncate.
- Keep large payloads and logs in files referenced by path and digest.
- A public operation is complete only when discovery, valid and failure behavior, bounds,
  copied-binary use, documentation, and predecessor rejection are proved.
- Do not advertise private operations, frozen artifacts, internal builders, or checkout-only paths
  as current public authoring.

## Release targets and hosted automation

Releases are derived distribution, not program meaning.

- The root product package version owns the release tag unless a durable decision selects another
  owner.
- Published releases use strict annotated `vMAJOR.MINOR.PATCH` tags matching that version and
  reachable from the maintained branch.
- Build from the exact tag with locked dependencies and the exact repository toolchain.
- Admit each public target independently through the exact candidate's copied-binary workflows,
  runtime-linkage inspection, resource observations, and documented limitations.
- Prefer one self-contained static public binary when it can pass the same maintained product and
  service oracles. A target triple or linker flag is not proof of staticness; inspect the final
  executable and execute it at the supported boundary.
- Do not create a multi-target matrix, compatibility asset, installer, mirror, or updater without a
  named maintained consumer and independent admission/recovery policy.
- Keep target selection, archive name, and linkage policy under one first-party owner. Do not copy
  mutable target catalogs or generated identities into workflow shell and prose.
- Use stable version-free asset names when `releases/latest/download/...` is a public path.
- Distribute the project license, required third-party notices, canonical release metadata, and
  cryptographic checksums with binaries.
- Make archive inventory, modes, ordering, timestamps, and metadata deterministic. Reject links,
  traversal, duplicates, extras, malformed metadata, and output conflicts.
- Workflow artifacts are transient handoffs, not public releases.
- Pin every third-party action to a full commit SHA. Use explicit runners, timeouts, bounded output,
  scoped non-cancelling concurrency, and minimum permissions.
- Do not persist checkout credentials. Separate repository-controlled build/test execution from
  release-write authority; a publication job must not checkout or execute repository code.
- Use the ephemeral workflow token, not a long-lived personal token, for ordinary publication.
- Enable and verify immutable releases before publication. Create a draft, attach and verify all
  assets, then publish.
- Never clobber assets, force-push, retag, or destructively roll back a published release. Recovery
  is an additive patch release.
- Completion requires anonymous exact/latest downloads, checksums, asset digests, release and asset
  attestation verification, strict extraction, exact candidate runtime-linkage inspection, and
  public-binary behavior.
- Distinguish static linkage from kernel compatibility, release integrity from build provenance,
  and tested platform evidence from universal portability. Claim only proved properties.

Keep detailed procedure in `docs/release.md`, current facts in `docs/status.md`, architecture in
`docs/architecture.md`, and measurements in evidence. Current-public documentation changes only
after the external object and all required postconditions are proved.

## Errors, resources, security, and recovery

Diagnostics need a stable class/code, failed boundary, safe exact identity, and actionable
correction. Preserve useful locations without leaking secrets or large payloads.

Keep resource dimensions separate: input records/bytes, semantic work, affected
owners/relations/objects/pages/units/tests, output bytes, wall/CPU/RSS, filesystem and
synchronization work, runtime resources, build/archive/upload/download bytes, hosted jobs, retries,
and publications. Every bound needs a unit, owner, classification, override policy, and rationale.
Do not replace a multidimensional model with one `fuel` scalar or infer provider cost,
compatibility, or SLOs without telemetry.

Treat paths, request files, artifacts, backups, continuations, deployment data, network input, tags,
archives, manifests, checksums, test images, and downloaded tools as hostile or mutable boundaries.
Validate before allocation, execution, or external effect; reject symlink/non-regular surprises;
use private staging and atomic visibility; redact secrets from all outputs and evidence.

Do not claim hostile-code sandboxing, multi-tenant isolation, encrypted graph storage, build
provenance, binary signing, distributed consensus, generic Linux portability, or untested platform
support. Do not add speculative TLS to the current plaintext HTTP/PostgreSQL boundary.

Cancellation and exhaustion are typed outcomes. Leave accepted authority unchanged and return
recovery evidence. Validate completely before irreversible external publication; recover through
new identities rather than mutation.

## Scope, documentation, and completion

Each substantial campaign has one primary objective and one dependency-closed final state. Before
broad editing, identify authorities, consumers, migration/cutover order, deletion targets, oracles,
observable acceptance, irreversible effects, and empirical questions. Prefer established vocabulary
and one term per concept. Defer unrelated findings unless they invalidate the objective or reveal an
immediate authority, data-loss, security, or publication hazard.

Documentation ownership:

- normative behavior: `docs/spec/`;
- current facts: `docs/status.md`;
- architecture: `docs/architecture.md`;
- deferred work: `docs/roadmap.md`;
- measurements/evidence: `docs/performance.md`, `docs/evidence/`;
- generated contracts: `docs/generated/`;
- durable decisions: `docs/decisions/`;
- mandates and terminal evidence: `docs/campaigns/`;
- release procedure: `docs/release.md`;
- public downloads and examples: `README.md`.

Do not duplicate catalogs, manifests, raw logs, full diffs, or binary inventories in handwritten
Markdown. Do not rewrite completed campaign history to describe current state; append a narrow
erratum only for a material historical error.

Work is complete only when the selected public workflow succeeds at its real supported boundary,
maintained consumers and deletion obligations are closed, independent oracles agree,
generated/current documentation matches behavior, final evidence is fresh or accurately classified,
commits are coherent, external-action status is explicit, the working tree is clean or preserved
work is named, and the campaign is marked complete or terminated.

For releases, additionally require exact tagged-source and tagged-asset verification, anonymous
exact/latest integrity and behavior, immutable tag/release/assets, attestation verification,
redistribution notices, measured runtime requirements, and additive recovery state.

Before reporting completion, run:

```sh
git status --short
git diff --check
git log --oneline --decorate -20
```

The final report gives exact commands, classifications, commit/tag/release identities,
receipt/log/artifact paths and digests, deviations, limitations, irreversible actions, push status,
and the smallest next candidates. Do not claim public completion from a prototype, internal test,
stale receipt, workflow artifact, draft release, frozen predecessor artifact, documentation, target
name, or unavailable required gate.
