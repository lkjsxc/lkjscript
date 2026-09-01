# Repository Agent Instructions

## Scope and precedence

This file applies to the repository root and all descendants unless a nearer `AGENTS.md` or
`AGENTS.override.md` is more specific.

Apply instructions in this order:

1. active user instructions;
2. a campaign explicitly named by the user;
3. one implementation mandate reconciled as active against its complete file, the active checkout,
   and relevant external state;
4. the nearest applicable agent instruction file;
5. executable public contracts, black-box tests, and first-party verification policy;
6. normative specifications under `docs/spec/`;
7. current proved facts in `docs/status.md` and `docs/architecture.md`;
8. current implementation and executable-generated documentation;
9. completed or terminated campaigns, decisions, historical prompts, comments, and older commits.

Do not select a campaign by filename recency. Read its top-level status and terminal record, then
reconcile commits, worktree, consumers, releases, workflows, and user instructions. A baseline SHA
is orientation. A remote branch, local `HEAD`, tag, workflow candidate, public release, installed
binary, and running service are distinct states.

## Mission and enduring authority

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole editable authority for program
  meaning.
- Source text, compact requests, projections, indexes, caches, artifacts, runtime handles,
  deployment data, receipts, release archives, and checksums are derived, operational, or
  evidentiary; none is a second editable program authority.
- Operational application data, durable queues, and object bytes are deployment-selected effect
  authorities distinct from program meaning and from one another; live effects must never select
  or advance semantic `HEAD`.
- Ordinary application development must work through the distributed `lkjscript` executable.
- Application policy belongs in graph meaning. Rust owns generic semantic, compiler, runtime,
  adapter, verification, and distribution mechanisms.
- Maintained applications, protocol integrations, and recipes are evidence-bearing consumers of the
  platform, not product-priority authorities. Use them to prove reusable graph and runtime semantics;
  do not let enthusiasm for a protocol or demonstration justify application-private host semantics,
  roadmap priority, or a parallel authoring path.
- Choose the strongest graph-native semantic model that materially contracts authority or makes
  maintained-application invariants machine-checkable. Human familiarity, conventional syntax, and
  unaided manual authoring convenience are not design constraints.
- Unfamiliar abstractions are welcome when a maintained workload, bounded public authoring and
  failure behavior, dependency-closed migration and deletion, and independent proof justify them.
  Ambition does not relax determinism, resource, security, or evidence obligations.
- Accepted writes validate an exact-base complete candidate, make immutable canonical data durable,
  and expose one atomic visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance
  accepted authority.
- Backward compatibility is not the default. Move every maintained consumer, reject predecessor
  inputs, and delete predecessor paths in one dependency-closed cutover.
- AI-first public surfaces are deterministic, bounded, discoverable, actionable, and independently
  verifiable. AI-first does not justify ambiguous contracts, hidden mutable state, excessive
  vocabulary, or reduced readability.

Do not elevate current Rust types, module layout, schema, artifact encoding, target, packaging
format, CI provider, release service, or storage provider into language semantics.

## Version and identity authority

- The root `lkjscript` package version is the human-facing product release snapshot and owns its
  matching annotated tag. It is not a universal language, graph, CLI, repository, artifact,
  deployment, runtime, package, or tooling version.
- Each public or stored contract has one canonical owner and advances only when its own encoding or
  behavior requires it. Do not synchronize unrelated contract numbers.
- Semantic revisions, package revisions, content identities, target triples, commit SHAs, and
  digests remain separate identity domains.
- Release manifests and receipts bind exact distribution inputs; they do not select meaning.
- Do not add a duplicate `VERSION` file, edition ladder, workspace-wide version table, or
  handwritten current contract catalog. Use the executable owner or generated projection.

## Repository and information ownership

- `src/platform/`: semantic authority, publication, compiler, runtime, adapters, and public control.
- `src/bin/lkjscript.rs`: distributed process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, release, service, scale, and evidence
  tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package authority and generated consumer assets.
- `applications/lkjournal/`: maintained application authority and deployment material.
- `.github/workflows/`: hosted orchestration; reusable policy belongs in first-party tooling.
- `docs/spec/`: normative behavior.
- `docs/status.md`: current proved facts and limitations.
- `docs/architecture.md`: current authority, dependency direction, and boundaries.
- `docs/roadmap.md`: deferred evidence-gated work, not an automatic queue.
- `docs/decisions/`: durable decisions and reversal conditions.
- `docs/performance.md` and `docs/evidence/`: measurements and structured proof.
- `docs/generated/`: executable-generated public projections; never hand-edit.
- `docs/campaigns/`: timestamped implementation mandates and concise terminal history.
- `docs/release.md`: release preparation, publication, verification, and recovery procedure.

Project-local `generated/` directories contain replaceable artifacts, not generated documentation or
program authority. Do not turn `AGENTS.md` into a campaign, status report, roadmap, contract catalog,
command catalog, release ledger, or evidence log.

## Start-of-work reconciliation

Before editing, capture branch, `HEAD`, worktree, remotes/upstream, recent history, applicable agent
files, toolchain, and relevant external state. Identify:

- the uniquely active mandate or implicit in-flight work;
- pre-existing, later, and unrelated work;
- semantic, operational, distribution, and external authorities;
- maintained consumers and unsupported examples;
- migration and deletion targets;
- independent oracles;
- irreversible effects and recovery branches; and
- the smallest authoritative path and symbol set needed to begin.

The active checkout outranks an orientation baseline. Preserve supplied and unfamiliar work. Prefer
exact symbol, owner, consumer, and history searches over broad rescans. Inspect volatile external
state only when the objective depends on it, and refresh it immediately before mutation. If coherent
active or externally committed work remains incomplete, close, recover, or terminate it before
stacking an independent objective.

When a completed source campaign has advanced the root product snapshot and its release path is
ready, the source/public split is coherent incomplete work. Unless the user explicitly selects a
source-only train, close, block, or terminate publication before stacking another product delta; do
not advance the root product version merely to mark source progress.

## Campaign design and cutover discipline

Each substantial campaign has one primary objective and one dependency-closed final state. Before
broad editing, define authority, public value, consumers, ordering, failure behavior, deletion,
proof, external effects, and stopping conditions.

Prefer completing an existing typed/public workflow over adding a parallel representation. Preserve
unrelated data and behavior, implement the new authority, migrate every maintained consumer, switch
once, reject predecessor inputs, and delete predecessor readers, writers, adapters, fixtures,
generated assets, flags, aliases, and documentation after their last consumer. Compatibility
requires a named external need, owner, removal condition, and proof; it is not a default.

A typed semantic form unavailable through the distributed authoring surface is incomplete public
capability. Prefer the strongest reusable graph meaning and `packages/standard/` over host
intrinsics, special opcodes, generators, macros, or a second authoring format. New abstractions must
define types, effects, evaluation order, identity, failure, resources, consumers, oracle, migration,
and reversal. Treat affine or linear ownership, typestate and session protocols, richer effects,
refinement or dependent evidence, inference, capture, dynamic dispatch, specialization, JIT, AOT,
SIMD, allocation, and broad caching as evidence-gated options, not exclusions based on unfamiliarity.
Add them only from a measured maintained workload with independent proof.

## Git, worktree, and external safety

Do not rewrite history or destroy unfamiliar work. Unless the user explicitly requires and
understands the exact action, do not use destructive reset/clean/restore, rebase, force push, retag,
or published-object deletion.

- Preserve unrelated and pre-existing work.
- Stage explicit paths; do not use `git add .` or `git add -A`.
- Inspect the staged diff before every commit.
- Keep commits coherent and pair behavior with proving tests.
- Avoid unrelated formatting, renames, dependency updates, and cleanup.
- Do not commit `.artifacts/`, transient release/migration output, downloaded tools, private data,
  secrets, or large logs.
- Fetch before a normal push; push only as a safe fast-forward when the active campaign authorizes
  it.
- Never move, replace, force-update, unpublish, or delete a published tag, release, or asset.
- End clean unless preserved work is named exactly.

Permission to edit or commit does not authorize a push, workflow dispatch, tag, release, package
publication, deployment, migration, settings change, credential change, or destructive data action.
Each external mutation must be explicitly authorized by the active campaign with named
preconditions. Recover published state additively.

## Rust, tooling, and generated output

Rust is the default implementation and repository-tooling language. Do not add Python or replace
typed validation with shell-only logic. Shell and workflows may orchestrate explicit first-party
commands and standard tools; they must not duplicate product semantics or acceptance logic.

Use stable Rust 2024, the locked dependency graph, and the exact toolchain/targets owned by
`rust-toolchain.toml` when present. Keep `Cargo.lock` authoritative. Production Rust remains safe;
repository lint policy forbids `unsafe`, `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.
Tests may use narrow documented allowances.

Prefer typed objects, checked conversions, deterministic collections, explicit ownership, bounded
decoding, and canonical serialization. Reject malformed, foreign, duplicate, noncanonical,
overflowing, trailing, exhausted, or path-traversing input at the owning boundary. Do not couple
contract bytes or digest identity to Rust enum order, serde shape, memory layout, filesystem paths,
wall clock, or hash iteration.

Regenerate owned assets only through their executable or typed owner. Prefer existing dependencies
or checked platform tools; pin downloaded tools and test images by exact version and cryptographic
digest. Delete temporary migration, feasibility, packaging, and predecessor tooling after its last
consumer.

## Semantic, compiler, and runtime boundaries

Lower public mutations to typed semantic intent before validation or publication.
`GraphRepository` publication is the sole normal accepted-authority writer. Plan and apply share
normalization, allocation, impact analysis, validation, selected tests, and logical-result
construction. Recheck repository-dependent facts and the exact base under the publication lock;
durable canonical data precedes the single visibility change.

Witnesses, compiler state, indexes, caches, artifacts, plans, deployment data, receipts, and release
files are derived. Missing disposable state may rebuild; inconsistent canonical authority is
corruption. Reuse derived state only when repository, revision, semantic state, contracts, options,
target, and dependency closure match. Clean and incremental compilation must agree. Strictly
validate artifacts before execution or publication.

Keep production and implementation-disjoint reference oracles for pure behavior. Live effects run
once through production and must not be replayed merely for comparison. A derived failure after
accepted publication must not be reported as a failed semantic write. Release resources on
cancellation, exhaustion, failure, and shutdown.

## Public CLI, deployment, and distribution

The executable registry exhaustively owns finite public operations, grammar, request/response
models, limits, diagnostics, authority effects, and security nonclaims. One behavior has one public
name. Reject unknown commands and options; do not add fallback routing or compatibility aliases.

Public transports adapt into typed requests. Raw JSON, storage bytes, generated schemas, source
text, and review projections are not required authoring authorities. JSON may be a strict bounded
runtime, deployment, evidence, or release-metadata adapter.

Finite output is deterministic, bounded, machine-readable where required, and identifies the
observed revision when reading meaning. Growing output requires explicit budgets and continuations
or bounded files; never silently truncate. Keep large payloads and logs in files referenced by path
and digest.

A public operation is complete only when discovery, valid and failure behavior, bounds,
copied-binary use, current/generated documentation, and predecessor rejection are proved. Do not
advertise checkout-only helpers, frozen artifacts, or internal builders as public capability.

Deployment descriptors are separate operator authority. They may select derived artifacts, grants,
secrets, adapters, coordinates, and resource limits, but never edit accepted meaning. Preparation
must validate completely before readiness or live effects and must clean up partial resources.

Releases are derived distribution. Follow `docs/release.md` and first-party release tooling. Admit
each target independently through exact candidate inspection and maintained copied-binary oracles.
Prefer one self-contained static public binary when it passes the same product and service
boundaries. A target triple or linker flag is not proof of staticness. Published releases use exact
annotated tags matching the root product version, immutable assets, isolated write authority, and
anonymous exact/latest verification. Recovery uses a new patch identity, never mutation. Claim only
the exact tested target, environments, integrity, and behavior.

## Verification and evidence truthfulness

Use the first-party `lkjscript-dev check` and release/service owners. Run the narrowest relevant
checks while iterating, product verification after maintained consumer or generated-asset changes,
service verification after resident/deployment changes, and one fresh full profile after final
content is assembled. Release or target work additionally requires exact-candidate admission and
the public boundaries owned by `docs/release.md`.

The harness owns gate dependencies, fingerprints, runtime identity, bounded logs, outputs, evidence
reuse, classifications, and receipts. Report every check as exactly one of `fresh passed`, `reused
passed`, `skipped`, `unavailable`, `failed`, or `not run`. Never present stale, reused, skipped,
unavailable, failed, or unrun evidence as fresh proof. A required unavailable gate prevents
completion; do not weaken it.

Distinguish source verification, target admission, packaging, workflow handoff, publication,
deployment, operator observation, protocol-client observation, and production observation. A host
build does not prove a distributed target. A workflow artifact or draft release is not a public
release. Large logs belong under `.artifacts/`; tracked evidence contains bounded summaries,
identities, digests, classifications, and pointers.

An evidence-only closure commit must not change inputs bound by final implementation or release
receipts. Rerun invalidated proof whenever source, workflow, target, policy, generated, normative, or
verification inputs change.

## Errors, resources, security, and recovery

Diagnostics need stable class/code, failed boundary, safe exact identity, and actionable
correction. Preserve useful locations without leaking secrets or large payloads.

Keep resource dimensions separate: input records/bytes, semantic work, affected owners/relations,
output bytes, execution stack/instructions, wall/CPU/RSS, filesystem/synchronization, runtime
resources, network/database work, archive/upload/download bytes, jobs, retries, and publications.
Each bound needs a unit, owner, classification, override policy, and rationale. Do not collapse the
model into one scalar or infer cost or service levels without telemetry.

Treat paths, request files, artifacts, continuations, deployment data, network input, tags, archives,
manifests, checksums, images, and downloaded tools as hostile or mutable boundaries. Validate before
allocation, execution, or external effect; reject symlink and non-regular surprises; use private
staging and atomic visibility; redact secrets from output and evidence.

Do not claim hostile-code sandboxing, multi-tenant isolation, encrypted graph storage, binary
signing, reproducible builds, general provenance, distributed consensus, generic Linux
portability, or untested platform support. Validate completely before irreversible publication and
recover through new identities.

## Documentation and completion

Normative behavior belongs in `docs/spec/`; current facts in `docs/status.md`; boundaries in
`docs/architecture.md`; deferred work in `docs/roadmap.md`; durable rationale in
`docs/decisions/`; proof in `docs/evidence/` and `docs/performance.md`; generated projections in
`docs/generated/`; release procedure in `docs/release.md`; and public downloads/examples in
`README.md`.

A timestamped campaign records its initial mandate. Do not silently rewrite that body to match the
outcome. Update only the minimal lifecycle field required by repository convention and append a
concise terminal record; current truth and large evidence belong in their owners.

Work is complete only when the selected public workflow succeeds at its real supported boundary,
maintained consumers and deletion obligations are closed, independent oracles agree, generated and
current documentation match behavior, evidence is fresh or accurately classified, commits are
coherent, external actions are explicit, the tree is clean or preserved work is named, and the
campaign is terminal.

The final report gives exact starting and final identities, commits, checks and classifications,
receipt/artifact paths and digests, deviations, limitations, irreversible actions, push/external
state, preserved work, and the smallest next candidates. Never claim completion from a prototype,
internal test, stale receipt, target name, documentation, or unavailable required gate.
