# Repository Agent Instructions

## Scope and precedence

This file applies to the repository root and every descendant unless a nearer `AGENTS.md` or `AGENTS.override.md` is more specific.

Apply instructions in this order:

1. active user instructions;
2. the active implementation mandate under `prompts/`;
3. the nearest applicable agent instruction file;
4. executable public contracts, tests, and verification policy;
5. normative specifications under `docs/spec/`;
6. current facts in `docs/status.md` and `docs/architecture.md`;
7. current implementation and generated documentation;
8. completed campaign evidence, historical prompts, decisions, comments, and commits.

The newest tracked `prompts/YYYYMMDDHHMM.md` is the active mandate unless the user names another prompt or a matching completion/termination record under `docs/campaigns/` explicitly closes it. A recorded baseline SHA is orientation. Audit later commits when `HEAD` has advanced.

Historical material is evidence, not current requirement. Do not inherit old terminology, contract numbers, limits, or implementation accidents without current verification.

## Mission and enduring invariants

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole authority for program meaning.
- Source text, compact records, projections, indexes, artifacts, bytecode, runtime handles, deployment descriptors, caches, logs, plans, and receipts are not second editable program authorities.
- Ordinary application development must be possible through the distributed `lkjscript` executable.
- Application policy belongs in graph meaning. Rust owns generic host mechanisms.
- Names are mutable locators. Stable typed identities express continuity.
- Exact semantic references, dense runtime indexes, and physical storage locations are separate.
- Accepted writes name an exact base, validate a complete candidate, make immutable data durable, and expose one atomic visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance accepted authority.
- Backward compatibility is not a default objective. Move maintained consumers, reject predecessor input, and delete predecessor paths as one dependency-closed cutover.
- Stored and hot representations must remain replaceable without changing language meaning.
- AI-first does not justify ambiguous contracts, hidden state, excessive terminology, weak diagnostics, or an undiscoverable CLI.

Do not elevate the current Rust representation, file layout, generated schema, or artifact encoding into language semantics.

## Repository map

Use each location according to its role:

- `src/platform/`: semantic authority, repository, compiler, runtime, adapters, and public control.
- `src/bin/lkjscript.rs`: released process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, service, scale, and evidence tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package consumer and authority.
- `applications/lkjournal/`: maintained application consumer and deployment material.
- `prompts/`: implementation mandates.
- `docs/campaigns/`: concise campaign completion or termination evidence.
- `docs/spec/`: normative behavior.
- `docs/status.md`: current implemented facts and limitations.
- `docs/architecture.md`: current layers and dependency direction.
- `docs/roadmap.md`: deferred evidence-gated work.
- `docs/performance.md` and `docs/evidence/`: measurements and retained structured evidence.
- `docs/generated/`: executable-generated contract documentation; never hand-edit.
- `docs/decisions/`: durable decisions and reversal conditions.

Do not turn `AGENTS.md` into a campaign plan, architecture specification, status report, measurement log, field catalog, or roadmap.

## Start of work

Before editing, capture:

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -20 --oneline
git remote -v
find .. \( -name AGENTS.md -o -name AGENTS.override.md \) -print
rustc --version
cargo --version
```

Then:

1. identify the active prompt and any matching completion record;
2. record pre-existing tracked and untracked work;
3. audit commits after the prompt baseline;
4. determine current authorities, maintained consumers, independent oracles, and deletion targets;
5. read the smallest authoritative path set needed by the campaign.

Prefer exact symbol and consumer searches over broad rescans.

## Git and working-tree safety

Do not rewrite history or destroy unfamiliar work. Unless the active user explicitly requires the exact action and its consequences are verified, do not use:

```text
git reset --hard
git clean -fd
git checkout -- .
git restore .
git rebase
git push --force
git push --force-with-lease
```

Additional rules:

- Never remove or overwrite unrelated user work.
- Do not use `git add .` or `git add -A`; stage explicit paths.
- Inspect `git diff --cached` before every commit.
- Keep commits coherent and pair behavior changes with proving tests.
- Do not mix unrelated formatting or cleanup into campaign commits.
- Do not commit `.artifacts/`, transient migration output, or large local logs.
- Retain only concise selected evidence in documented evidence locations.
- Fetch and inspect remote state before push.
- Push only by normal fast-forward when authorized and safe.
- End clean unless preserved pre-existing work is identified explicitly.

## Build and verification

The workspace uses stable Rust 2024 and the locked dependency graph.

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

Run the narrowest relevant test during implementation. Run product verification after maintained consumer and generated-asset changes. Run one fresh full profile only after final code, tests, generated files, evidence, and documentation are complete.

The verification harness owns:

- gate dependencies;
- exact fingerprints;
- bounded child logs;
- required outputs;
- fresh, reused, skipped, unavailable, and failed classification;
- receipt and artifact paths.

Reuse evidence only when the harness proves exact input identity. Never report stale, skipped, unavailable, or failed evidence as passed. Keep large logs under the harness artifact directory and return paths and digests instead of pasting them.

Environment-dependent checks may be unavailable. Report the exact reason and all independent evidence. Do not weaken a gate or fabricate success.

## Rust and first-party tooling

- Rust is the default implementation and repository-tooling language.
- Do not add Python or replace typed first-party tooling with shell orchestration.
- Keep `Cargo.lock` authoritative and use stable Rust.
- Safe Rust is required; repository lint policy forbids `unsafe`.
- Production code must satisfy workspace denial of `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.
- Tests may use narrow local allowances with an explicit reason.
- Prefer typed domain objects, checked conversions, deterministic collections, explicit ownership, and bounded decoding.
- Reject malformed tags, foreign identity domains, duplicates, noncanonical order, overflow, trailing input, and exhausted limits at the owning boundary.
- Do not couple contract bytes or digest identity to Rust enum order, serde shape, memory layout, filesystem paths, or hash iteration.
- Regenerate owned artifacts through their executable or first-party typed owner.
- Avoid new dependencies when the standard library or an existing dependency is sufficient.
- Delete temporary migration tooling and predecessor tooling after their last maintained consumer moves.

## Semantic authority and publication

- Lower public mutations to typed semantic intent before validation or publication.
- The repository publication API is the sole normal accepted-authority writer.
- Plan and apply for one operation share normalization, allocation, impact analysis, validation, and logical-result construction.
- A reviewed plan binds every semantic effect and validation or test claim offered for review.
- Keep witness maintenance, compiler scheduling, physical layout, cache work, and volatile observations outside review identity unless a normative contract explicitly includes them.
- Reject request components that can be checked before repository access as early as possible.
- Reprepare repository-dependent facts and recheck the exact accepted base under the publication lock.
- Make durable canonical data precede the single visibility change.
- After uncertain interruption, inspect current authority and retained receipts before retrying.
- Derived summaries, witnesses, indexes, compiler state, artifacts, review files, plans, and deployment data may rebuild but cannot silently alter accepted meaning.
- Missing disposable state may rebuild. Inconsistent canonical authority is corruption, not a cache miss.
- Keep implementation-disjoint complete oracles until differential evidence justifies removal.

A derived-cache failure after accepted publication must not be reported as a failed semantic write. Report accepted authority and derived-state status separately.

## Compiler, artifacts, and runtime

- Compiler caches, manifests, artifacts, dense indexes, and prepared programs are derived from exact accepted authority.
- Reuse a cache only when repository, revision, semantic state, compiler contract, options, and required object closure match exactly.
- A missing or invalid disposable cache may be deleted or rebuilt; it may never select semantics.
- Clean and incremental compilation must agree for equal accepted meaning.
- Artifact manifests bind exact package, revision, semantic state, dependency, compiler, bytecode, and closure identities.
- Strictly validate an artifact before execution or external publication.
- Production execution and canonical reference execution remain separate oracles for pure behavior.
- Never duplicate live external effects to obtain a differential result.
- Runtime handles are opaque process-local resources, not semantic identities.
- Release resources on cancellation, exhaustion, failure, and shutdown.

Do not add JIT, AOT, SIMD, memory mapping, custom allocators, resident sessions, or specialization without a measured maintained workload, independent oracle, and reversal condition.

## Public CLI and protocols

- The executable registry is the exhaustive owner of finite operations, grammar, request/response models, limits, diagnostics, authority effects, and security nonclaims.
- One behavior has one public name. Reject unknown commands and options; do not add fallback routing or compatibility aliases.
- Direct flags, compact records, and future transports are adapters to transport-neutral typed requests.
- Do not make raw JSON, storage objects, generated schemas, source text, or review projections the required authoring path.
- JSON may be used as a strict bounded runtime-value or deployment adapter when the normative operation requires it; it is not program authority.
- Finite output is deterministic, bounded, machine-readable, and names the observed revision when meaning is read.
- Classified finite outcomes use stdout; keep stderr empty unless the current contract explicitly says otherwise.
- Growing results use explicit budgets and continuations or an explicit bounded output file. Never silently truncate.
- Keep large payloads and logs in files referenced by path and digest, not repeated in stdout or agent context.
- A public operation is complete only when discovery, valid behavior, failure behavior, bounds, copied-binary use, documentation, and predecessor rejection are proved.
- Do not expose a private typed operation until its complete public workflow exists.
- Do not advertise an artifact-runtime compatibility boundary as current semantic or compiler authority.

## Errors, resources, security, and recovery

Diagnostics identify:

- a stable class and code;
- the failed boundary;
- the relevant exact identity when safe;
- an actionable correction.

Preserve useful source locations without leaking secrets or large payloads.

Keep resource dimensions separate. Distinguish as relevant:

- input bytes and records;
- authored operations and semantic work;
- affected owners, relations, objects, pages, compiler units, and selected tests;
- output bytes and records;
- wall time, CPU time, and peak RSS;
- filesystem bytes and synchronization operations;
- runtime tasks, connections, queue entries, retries, and cancellations.

Every numeric bound needs a unit, owning boundary, classification, override policy, and rationale. Do not substitute one `fuel` scalar for a multidimensional resource model. Do not infer provider tokens, cache use, requests, or monetary cost without direct telemetry.

Treat request files, paths, artifacts, backups, continuations, deployment descriptors, adapter input, and network data as hostile boundaries.

- Validate before unbounded allocation or external effect.
- Reject symlink and non-regular-file surprises at publication boundaries.
- Use private staging, synchronization, and atomic visibility for durable output.
- Redact secrets from output, logs, diagnostics, receipts, and fixtures.
- Keep application authorization in graph meaning and generic capability enforcement in host mechanisms.
- Do not claim sandboxing, multi-tenant isolation, encrypted graph storage, artifact provenance, distributed consensus, or platform support without implementation and evidence.
- Do not add speculative TLS machinery to the current plaintext HTTP/PostgreSQL boundary; transport encryption requires a separately selected trusted boundary.

Cancellation and exhaustion are typed outcomes. Release resources, leave accepted authority unchanged, and return enough evidence for deterministic recovery.

## Scope and design discipline

Each substantial campaign has one primary objective and one dependency-closed final state.

Before broad editing, identify:

- current and target authorities;
- maintained consumers;
- migration order;
- exact deletion targets;
- independent oracles;
- observable acceptance criteria;
- empirical questions that require local execution.

Prefer established technical vocabulary and one term per concept. Remove obsolete current terminology after cutover. Do not preserve work merely because it is recent, and do not replace correct work merely to create visible change.

Do not bundle independent reforms. Do not introduce sessions, daemons, registries, schedulers, caches, JIT, AOT, SIMD, memory mapping, custom allocators, `unsafe`, or distributed machinery without a measured problem, named consumer, independent oracle, and reversal condition.

Record serious unrelated findings with exact evidence and defer them unless they invalidate the active objective or present immediate authority, data-loss, or security risk.

When an assumption is disproved, make the smallest objective-preserving correction and record it. Do not leave a partial public cutover, dual authority, or temporary compatibility path.

## Documentation ownership

- Normative behavior: `docs/spec/`.
- Current facts and limitations: `docs/status.md`.
- Layer and dependency map: `docs/architecture.md`.
- Deferred work: `docs/roadmap.md`.
- Measurements: `docs/performance.md` and `docs/evidence/`.
- Generated contracts: `docs/generated/`, produced by the executable owner.
- Durable decisions and reversal conditions: `docs/decisions/`.
- Implementation mandates: `prompts/`.
- Concise campaign completion or termination evidence: `docs/campaigns/`.
- User-facing examples: `README.md` and maintained consumer READMEs.

Do not duplicate complete operation, field, diagnostic, or version catalogs across handwritten files. Do not paste raw logs, large manifests, full diffs, or binary inventories into Markdown. Historical prompts, campaigns, decisions, and comments do not override current executable contracts.

## Completion and handoff

Work is complete only when:

- the selected public workflow works through a copied released executable;
- maintained consumers in scope use it;
- predecessor readers, writers, adapters, aliases, fixtures, schemas, tests, and stale current documentation in scope are deleted;
- relevant valid, malformed, stale, exhausted, cancelled, interrupted, corrupt, and predecessor cases are tested;
- independent oracles agree;
- generated files match their owner;
- current documentation matches executable behavior;
- final verification is fresh for final content or accurately classified unavailable;
- commits are coherent;
- the working tree is clean or preserved user work is identified; and
- the final report gives exact commands, classifications, commit SHAs, receipt/log/artifact paths and digests, deviations, limitations, push status, and the smallest next candidates.

Before reporting completion, inspect:

```sh
git status --short
git diff --check
git log --oneline --decorate -20
```

Do not claim public completion from a private prototype, internal unit test, stale receipt, frozen predecessor artifact, documentation, or an unavailable gate.
