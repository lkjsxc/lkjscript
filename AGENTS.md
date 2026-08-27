# Repository Agent Instructions

## Scope and precedence

This file applies to the repository root and all descendants unless a nearer `AGENTS.md` or `AGENTS.override.md` is more specific.

Apply instructions in this order:

1. active user instructions;
2. a campaign explicitly named by the user;
3. the newest incomplete implementation mandate under `docs/campaigns/YYYYMMDDHHMM.md`;
4. the nearest applicable agent instruction file;
5. executable public contracts, tests, and verification policy;
6. normative specifications under `docs/spec/`;
7. current facts in `docs/status.md` and `docs/architecture.md`;
8. current implementation and generated documentation;
9. completed campaigns, historical prompts, decisions, comments, and commits.

A campaign is active when it identifies itself as an implementation mandate and is not marked complete or terminated. `Status: incomplete` leaves it active. Files under `prompts/` are historical unless the user names one.

A baseline SHA is orientation. Audit later commits when `HEAD` has advanced. Historical text is evidence, not current requirement; revalidate terminology, versions, limits, and assumptions against the active checkout.

## Mission and invariants

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole authority for program meaning.
- Source text, compact requests, projections, indexes, artifacts, runtime handles, deployment data, caches, plans, receipts, release archives, and checksums are not second editable program authorities.
- Ordinary application development must work through the distributed `lkjscript` executable.
- Application policy belongs in graph meaning; Rust owns generic host mechanisms.
- Names are mutable locators. Stable typed identities express continuity.
- Exact semantic references, dense runtime indexes, and physical locations are separate.
- Accepted writes validate an exact-base complete candidate, make immutable data durable, and expose one atomic visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance accepted authority.
- Backward compatibility is not a default. Move maintained consumers, reject predecessor input, and delete predecessor paths in one dependency-closed cutover.
- Stored and hot representations remain replaceable without changing language meaning.
- AI-first does not justify ambiguous contracts, hidden state, excessive terminology, weak diagnostics, or undiscoverable behavior.

Do not elevate the current Rust representation, file layout, schema, artifact encoding, packaging format, CI provider, or release service into language semantics.

## Repository map

- `src/platform/`: semantic authority, repository, compiler, runtime, adapters, and public control.
- `src/bin/lkjscript.rs`: released process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, release, service, scale, and evidence tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package authority and consumer.
- `applications/lkjournal/`: maintained application authority and deployment material.
- `.github/workflows/`: hosted orchestration; keep reusable validation in first-party tooling.
- `docs/campaigns/`: implementation mandates and concise completion/termination evidence.
- `docs/spec/`: normative behavior.
- `docs/status.md`: current facts and limitations.
- `docs/architecture.md`: current layers and dependency direction.
- `docs/roadmap.md`: deferred evidence-gated work.
- `docs/performance.md` and `docs/evidence/`: measurements and structured evidence.
- `docs/generated/`: executable-generated contracts; never hand-edit.
- `docs/decisions/`: durable decisions and reversal conditions.
- `docs/release.md`: release preparation, publication, verification, and recovery.
- `prompts/`: historical mandates.

Do not turn `AGENTS.md` into a campaign plan, architecture specification, status report, measurement log, catalog, release ledger, or roadmap.

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

Then identify the active campaign, pre-existing work, later commits, current authorities, maintained consumers, independent oracles, deletion targets, and any external GitHub state the campaign depends on. Read the smallest authoritative path set and prefer exact symbol/consumer searches over broad rescans.

## Git and working-tree safety

Do not rewrite history or destroy unfamiliar work. Unless the active user explicitly requires and understands the exact action, do not use:

```text
git reset --hard
git clean -fd
git checkout -- .
git restore .
git rebase
git push --force
git push --force-with-lease
```

- Preserve unrelated work.
- Stage explicit paths; do not use `git add .` or `git add -A`.
- Inspect `git diff --cached` before every commit.
- Keep commits coherent and pair behavior with proving tests.
- Avoid unrelated formatting and cleanup.
- Do not commit `.artifacts/`, transient migration/release output, downloaded tools, or large logs.
- Fetch and inspect remote state before push; push only by normal fast-forward when safe.
- Never move, replace, force-update, or delete a published release tag.
- End clean unless preserved pre-existing work is identified explicitly.

## Build and verification

The workspace uses stable Rust 2024 and the locked dependency graph. When present, `rust-toolchain.toml` owns the exact repository toolchain.

Use narrow checks while iterating:

```sh
cargo fmt --all -- --check
cargo test --locked --lib FILTER
cargo test --locked --test public_cli FILTER
cargo test --locked --test general_service FILTER
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
```

Repository entry points are:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

Run the narrowest relevant test during implementation, product verification after maintained-consumer/generated-asset changes, and a fresh full profile only after final content is assembled. A release campaign may require separate fresh verification of the tagged source and the final completion commit.

The harness owns gate dependencies, fingerprints, runtime identity, bounded logs, required outputs, evidence reuse, classifications, and receipts. Reuse only exact-input evidence. Never report reused, skipped, unavailable, or failed evidence as fresh passed evidence. Keep large output under `.artifacts/` and return paths/digests.

Report genuine environment unavailability; do not weaken or omit a gate. A gate required for a supported release target cannot become optional because one local machine lacks a dependency.

## Rust and first-party tooling

- Rust is the default implementation and repository-tooling language.
- Do not add Python or replace typed validation with shell-only logic. Shell/workflow steps should orchestrate explicit first-party commands and standard tools.
- Keep `Cargo.lock` authoritative; use safe Rust. Repository lint policy forbids `unsafe` and production `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.
- Tests may use narrow allowances with an explicit reason.
- Prefer typed objects, checked conversions, deterministic collections, explicit ownership, bounded decoding, and canonical serialization.
- Reject malformed tags, foreign identities, duplicates, noncanonical order, overflow, trailing input, path traversal, and exhausted limits at the owning boundary.
- Do not couple contract bytes or digest identity to Rust enum order, serde shape, memory layout, filesystem paths, current time, or hash iteration.
- Regenerate owned artifacts through their executable or typed owner.
- Prefer existing dependencies or checked platform tools over unnecessary new dependencies.
- Pin downloaded tools by exact version and cryptographic digest before execution.
- Delete temporary migration, packaging, and predecessor tooling after its last consumer.

## Semantic authority, compiler, and runtime

- Lower public mutations to typed semantic intent before validation or publication.
- The repository publication API is the sole normal accepted-authority writer.
- Plan and apply share normalization, allocation, impact analysis, validation, and logical-result construction; review evidence binds every claimed semantic effect.
- Keep witnesses, compiler scheduling, physical layout, caches, volatile observations, and distribution metadata outside semantic review identity unless a normative contract says otherwise.
- Reprepare repository-dependent facts and recheck the exact base under the publication lock. Durable canonical data precedes the single visibility change.
- Derived witnesses, indexes, compiler state, artifacts, plans, deployment data, and release files may rebuild but cannot select or silently alter accepted meaning.
- Missing disposable state may rebuild; inconsistent canonical authority is corruption.
- Keep implementation-disjoint complete oracles until evidence justifies removal.
- A derived failure after accepted publication must not be reported as a failed semantic write.

Compiler caches, manifests, artifacts, dense indexes, and prepared programs derive from exact accepted authority. Reuse only when repository, revision, semantic state, contracts, options, and closure match. Clean and incremental compilation must agree. Strictly validate artifacts before execution/publication. Preserve separate production and canonical-reference oracles for pure behavior; never duplicate live effects. Release resources on cancellation, exhaustion, failure, and shutdown.

Do not add JIT, AOT, SIMD, memory mapping, custom allocators, resident sessions, or specialization without a measured maintained workload, independent oracle, and reversal condition.

## Public CLI and protocols

- The executable registry exhaustively owns finite operations, grammar, request/response models, limits, diagnostics, authority effects, and security nonclaims.
- One behavior has one public name. Reject unknown commands/options; do not add fallback routing or compatibility aliases.
- Public transports adapt into typed requests. Raw JSON, storage objects, generated schemas, source text, and review projections are not required authoring authorities.
- JSON may be a strict bounded runtime, deployment, evidence, or release-metadata adapter when its owner requires it.
- Finite output is deterministic, bounded, machine-readable, and names the observed revision when reading meaning. Classified finite outcomes use stdout and keep stderr empty unless the contract says otherwise.
- Growing results need explicit budgets and continuations or bounded files; never silently truncate.
- Keep large payloads/logs in files referenced by path and digest.
- A public operation is complete only when discovery, valid/failure behavior, bounds, copied-binary use, documentation, and predecessor rejection are proved.
- Do not advertise private operations or frozen artifact-runtime compatibility as current semantic/compiler completion.

## Release and hosted automation

Releases are derived distribution, not program meaning.

- The root package version owns release version unless a normative decision selects another owner.
- Published releases use strict annotated `vMAJOR.MINOR.PATCH` tags matching the package version and reachable from the maintained development branch.
- Build from the exact tag with locked dependencies and the exact repository toolchain.
- Admit a target only after its exact asset passes the copied-binary workflow and its runtime requirements are measured/documented.
- Use stable version-free asset names when `releases/latest/download/...` is a public path.
- Distribute the project license, required third-party notices, canonical release metadata, and cryptographic checksums with binaries.
- Make archive inventory, modes, ordering, timestamps, and metadata deterministic; reject links, traversal, duplicates, extras, malformed metadata, and output conflicts.
- Workflow artifacts are transient handoffs, not public releases.
- Pin every third-party action to a full commit SHA. Use explicit runners, timeouts, bounded output, scoped concurrency, and minimum permissions.
- Do not persist checkout credentials. Separate repository-controlled build/test execution from release-write authority; a publication job must not checkout or execute repository code.
- Use the ephemeral workflow token, not a long-lived personal token, for ordinary publication.
- Enable and verify immutable releases before publication. Create a draft, attach and verify all assets, then publish.
- Never clobber assets, force-push, retag, or destructively roll back a published release. Recovery is an additive patch release.
- Completion requires anonymous exact/latest downloads, checksums, asset digests, release-attestation verification, extraction, and public-binary behavior.
- Distinguish release integrity/identity from build provenance, code signing, sandboxing, and platform compatibility. Claim only proved properties.
- Keep procedure in `docs/release.md`, current facts in `docs/status.md`, and measurements in evidence files.

Treat broad CI, nightly channels, package registries, installers, mirrors, and auto-updaters as independent campaigns with named consumers and recovery policies.

## Errors, resources, security, and recovery

Diagnostics need a stable class/code, failed boundary, safe exact identity, and actionable correction. Preserve useful locations without leaking secrets or large payloads.

Keep resource dimensions separate: inputs, semantic work, affected owners/relations/objects/pages/units/tests, outputs, wall/CPU/RSS, filesystem/synchronization, runtime resources, build/archive/upload/download bytes, hosted jobs, retries, and publications. Every bound needs a unit, owner, classification, override policy, and rationale. Do not replace a multidimensional model with one `fuel` scalar or infer provider cost/compatibility without telemetry.

Treat paths, request files, artifacts, backups, continuations, deployment data, network input, tags, archives, manifests, checksums, and downloaded tools as hostile or mutable boundaries. Validate before allocation, execution, or external effect; reject symlink/non-regular surprises; use private staging and atomic visibility; redact secrets from all outputs and evidence.

Do not claim hostile-code sandboxing, multi-tenant isolation, encrypted graph storage, build provenance, binary signing, distributed consensus, or platform support without evidence. Do not add speculative TLS to the current plaintext HTTP/PostgreSQL boundary.

Cancellation and exhaustion are typed outcomes. Leave accepted authority unchanged and return recovery evidence. Validate completely before irreversible external publication; recover through new identities rather than mutation.

## Scope, documentation, and completion

Each substantial campaign has one primary objective and one dependency-closed final state. Before broad editing, identify authorities, consumers, migration/cutover order, deletion targets, oracles, observable acceptance, irreversible effects, and empirical questions. Prefer established vocabulary and one term per concept. Defer unrelated findings unless they invalidate the objective or present immediate authority, data-loss, security, or publication risk.

Documentation ownership:

- normative behavior: `docs/spec/`;
- current facts: `docs/status.md`;
- architecture: `docs/architecture.md`;
- deferred work: `docs/roadmap.md`;
- measurements/evidence: `docs/performance.md`, `docs/evidence/`;
- generated contracts: `docs/generated/`;
- durable decisions: `docs/decisions/`;
- mandates/completion: `docs/campaigns/`;
- release procedure: `docs/release.md`;
- public downloads/examples: `README.md`.

Do not duplicate catalogs, manifests, raw logs, full diffs, or binary inventories in handwritten Markdown.

Work is complete only when the selected public workflow succeeds at its real supported boundary, maintained consumers and deletion obligations are closed, independent oracles agree, generated/current documentation matches behavior, final verification is fresh or accurately unavailable, commits are coherent, push/external-action status is explicit, the working tree is clean or preserved work is named, and the campaign is marked complete or terminated.

For releases, additionally require exact tagged-asset verification, anonymous exact/latest integrity, immutable tag/release/assets, attestation verification, redistribution notices, measured runtime requirements, and public-binary behavior.

Before reporting completion, run:

```sh
git status --short
git diff --check
git log --oneline --decorate -20
```

The final report gives exact commands, classifications, commit/tag SHAs, receipt/log/artifact paths and digests, deviations, limitations, irreversible actions, push status, and the smallest next candidates. Do not claim public completion from a prototype, internal test, stale receipt, workflow artifact, draft release, frozen predecessor artifact, documentation, or an unavailable required gate.
