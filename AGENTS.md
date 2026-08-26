# Repository Agent Instructions

## Scope and precedence

This file applies to the repository root and every descendant unless a nearer `AGENTS.md` or
`AGENTS.override.md` is more specific.

Apply instructions in this order:

1. active user instructions;
2. the active implementation mandate under `docs/campaigns/`;
3. the nearest applicable agent instruction file;
4. executable public contracts, tests, and verification policy;
5. normative specifications under `docs/spec/`;
6. current status and architecture documents;
7. implementation and generated documentation;
8. historical campaigns, prompts, comments, and commits.

Historical material is evidence, not current requirements. Read the active campaign and inspect the
current Git state before editing. A recorded campaign SHA is orientation; audit every later commit
when HEAD has advanced.

## Mission and enduring invariants

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole authority for program meaning.
- Source text, compact records, direct flags, projections, indexes, artifacts, bytecode, runtime
  handles, deployment descriptors, caches, logs, plans, and receipts are not second editable
  program authorities.
- Ordinary application development must be possible through the distributed `lkjscript`
  executable.
- Application policy belongs in graph meaning. Rust owns generic host mechanisms.
- Names are mutable locators. Stable typed identities express continuity.
- Exact semantic references, dense runtime indexes, and physical storage locations are separate.
- Accepted writes name an exact base, validate a complete candidate, make immutable data durable,
  and expose one atomic HEAD visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance
  accepted authority.
- Backward compatibility is not a default objective. Move every maintained consumer in scope,
  reject predecessor input, and delete predecessor paths as one dependency-closed cutover.
- Keep stored and hot representations replaceable without changing language meaning.

AI-first does not justify ambiguous contracts, hidden state, excessive terminology, weak
diagnostics, or an undiscoverable CLI.

## Repository authority map

Use these locations according to their role:

- `src/platform/`: semantic authority, language, repository, compiler, runtime, adapters, and
  public control.
- `src/bin/lkjscript.rs`: released process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, policy, service, scale, and evidence
  tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package consumer.
- `applications/lkjournal/`: maintained application and service consumer.
- `docs/spec/`: normative behavior.
- `docs/status.md`: current implemented facts and limitations.
- `docs/architecture.md`: current layers and dependency direction.
- `docs/roadmap.md`: deferred evidence-gated work.
- `docs/performance.md` and `docs/evidence/`: measurements and concise retained evidence.
- `docs/generated/`: executable-generated contract documentation; never hand-edit it.
- `docs/decisions/`: durable decisions and reversal conditions.
- `docs/campaigns/`: current implementation mandates and their concise completion evidence.
- historical `prompts/` content in Git history: background only; do not recreate it as a current
  authority.

Do not turn `AGENTS.md` into a campaign plan, architecture specification, status report,
measurement log, field catalog, or roadmap.

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

Identify the latest applicable file under `docs/campaigns/` and determine whether it is an active
mandate, a completed campaign, or historical evidence. Record pre-existing tracked and untracked
work. Read the smallest authoritative path set needed by the campaign and prefer exact
symbol/consumer search over broad rescans.

## Git and working-tree safety

Do not rewrite history or destroy unfamiliar work. Unless the active user explicitly requires the
exact action and its consequences are verified, do not use:

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
- Do not commit `.artifacts/` or large local logs. Retain only concise selected evidence in the
  documented evidence locations.
- Push only by normal fast-forward when authorized and remote state is understood.
- End clean unless preserved pre-existing work is identified explicitly.

## Build and verification

The workspace uses stable Rust 2024 and the locked dependency graph.

Use narrow checks during iteration:

```sh
cargo fmt --all -- --check
cargo test --locked --lib FILTER
cargo test --locked --test public_cli FILTER
cargo test --locked --test general_service FILTER
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
```

Repository profiles are:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

Run the narrowest relevant check while iterating. Run one fresh `full` profile only after final
code, tests, generated files, and documentation are complete.

The verification harness owns gate dependencies, exact fingerprints, bounded child logs, required
outputs, and fresh/reused classification. Reuse evidence only when the harness proves exact input
identity. Never report skipped, stale, unavailable, or failed evidence as passed. Keep large logs
under the harness artifact directory and return paths and digests rather than pasting them.

Environment-dependent checks may be unavailable. Report the exact environmental reason and all
independent evidence; do not weaken a gate or fabricate success.

## Rust and first-party tooling

- Rust is the default implementation and repository-tooling language.
- Do not add Python, replace typed first-party tooling with shell orchestration, or add a runtime
  dependency without campaign-specific necessity.
- Keep `Cargo.lock` authoritative and use stable Rust.
- Safe Rust is required; repository lint policy forbids `unsafe`.
- Production code must satisfy workspace denial of `unwrap`, `expect`, `panic`, `todo`, and
  `unimplemented`.
- Tests may use narrow local allowances with an explicit reason.
- Prefer typed domain objects, checked conversions, deterministic collections, explicit ownership,
  and bounded decoding.
- Reject malformed tags, foreign identity domains, duplicates, noncanonical order, overflow,
  trailing input, and exhausted limits at the owning boundary.
- Do not couple contract bytes or digest identity to Rust enum order, serde shape, memory layout,
  filesystem paths, or hash iteration.
- Regenerate owned artifacts through their executable owner.
- Avoid new dependencies when the standard library or an existing dependency is sufficient.
- Delete first-party predecessor tooling after all maintained consumers move in the same campaign.

## Semantic authority and publication

- Lower mutations to typed semantic intent before validation or publication.
- The repository publication API is the sole normal accepted-authority writer.
- Plan and apply for one operation share normalization, allocation, impact analysis, validation,
  and logical-result construction.
- A reviewed plan binds every semantic effect and validation or test claim offered for review.
  Keep witness maintenance, compiler scheduling, physical layout, and volatile work outside review
  identity unless a normative contract explicitly includes them.
- Reject request components that can be checked before repository access as early as possible.
  Reprepare and compare repository-dependent facts before publication.
- Recheck the exact accepted base under the publication lock.
- Make durable canonical data precede the single HEAD visibility change.
- After uncertain interruption, inspect current HEAD and retained receipts before retrying.
- Derived summaries, witnesses, indexes, compiler state, artifacts, review files, plans, and
  deployment data may rebuild but cannot silently alter accepted meaning.
- Missing disposable state may rebuild. Inconsistent canonical authority is corruption, not a cache
  miss.
- Keep implementation-disjoint complete oracles until differential evidence justifies removal.

## Public CLI and protocols

- The executable registry is the exhaustive owner of finite operations, grammar, request/response
  models, limits, diagnostics, authority effects, and security nonclaims.
- One behavior has one public name. Reject unknown commands and options; do not add fallback routing
  or compatibility aliases.
- Direct flags, compact records, and future transports are adapters to transport-neutral typed
  requests.
- Do not make raw JSON, storage objects, generated schemas, source text, or review projections the
  required authoring path.
- Finite output is deterministic, bounded, machine-readable, and names the observed revision when
  meaning is read.
- Classified finite outcomes use stdout; keep stderr empty unless the current contract explicitly
  says otherwise.
- Growing results use explicit budgets and continuations or an explicit bounded output file. Never
  silently truncate.
- Keep large payloads and logs in files referenced by path and digest, not repeated in stdout or
  agent context.
- A public operation is complete only when discovery, valid behavior, failures, bounds,
  copied-binary use, documentation, and predecessor rejection are proved.
- Do not expose a private typed operation until its complete public workflow exists.

## Errors, resources, security, and recovery

Diagnostics identify a stable class and code, the failed boundary, and an actionable correction.
Preserve useful source locations without leaking secrets or large payloads.

Keep resource dimensions separate. Distinguish as relevant:

- input bytes and records;
- authored operations and semantic work;
- affected owners, relations, objects, pages, and selected tests;
- output bytes and records;
- wall time, CPU time, and peak RSS;
- filesystem bytes and synchronization operations;
- runtime tasks, connections, queue entries, retries, and cancellations.

Every numeric bound needs a unit, owning boundary, classification, override policy, and rationale.
Do not substitute one `fuel` scalar for a multidimensional resource model. Do not infer provider
tokens, cache use, or monetary cost without direct telemetry.

Treat request files, paths, artifacts, backups, continuations, deployment descriptors, adapter
input, and network data as hostile boundaries.

- Validate before unbounded allocation or external effect.
- Reject symlink and non-regular-file surprises at publication boundaries.
- Use private staging, synchronization, and atomic visibility for durable output.
- Redact secrets from output, logs, diagnostics, receipts, and fixtures.
- Keep application authorization in graph meaning and generic capability enforcement in host
  mechanisms.
- Do not claim sandboxing, multi-tenant isolation, encrypted graph storage, artifact provenance,
  distributed consensus, or platform support without implementation and evidence.
- Do not add speculative TLS machinery to the current plaintext HTTP/PostgreSQL boundary;
  transport encryption requires a separately selected trusted boundary.

Cancellation and exhaustion are typed outcomes. Release resources, leave accepted authority
unchanged, and return enough evidence for deterministic recovery.

## Scope and design discipline

Each substantial campaign has one primary objective and one dependency-closed final state.
Identify authorities, maintained consumers, migration order, deletion targets, independent oracles,
and observable acceptance criteria before broad editing.

Prefer established technical vocabulary and one term per concept. Remove obsolete terms after
cutover. Do not preserve work merely because it is recent, and do not replace correct work merely to
create visible change.

Do not introduce sessions, daemons, registries, schedulers, caches, JIT, AOT, SIMD, memory mapping,
custom allocators, `unsafe`, or distributed machinery without a measured problem, named consumer,
independent oracle, and reversal condition.

Record serious unrelated findings with exact evidence and defer them unless they invalidate the
active objective or present immediate authority, data-loss, or security risk.

## Documentation ownership

- Normative behavior: `docs/spec/`.
- Current facts and limitations: `docs/status.md`.
- Layer and dependency map: `docs/architecture.md`.
- Deferred work: `docs/roadmap.md`.
- Measurements: `docs/performance.md` and `docs/evidence/`.
- Generated contracts: `docs/generated/`, produced by the executable owner.
- Durable decisions: `docs/decisions/`, including reversal conditions.
- Campaign mandate and concise completion evidence: `docs/campaigns/`.
- User-facing examples: `README.md`.

Do not duplicate complete operation, field, diagnostic, or version catalogs across handwritten
files. Historical prompts, campaigns, and comments do not override current executable contracts.

## Completion and handoff

Work is complete only when:

- the selected workflow works through the released executable;
- maintained consumers in scope use it;
- predecessor readers, writers, adapters, aliases, fixtures, schemas, tests, and stale
  documentation in scope are deleted;
- relevant valid, malformed, stale, exhausted, interrupted, corrupt, and predecessor cases are
  tested;
- independent oracles agree;
- generated files match their owner;
- final verification is fresh for final content;
- commits are coherent;
- the working tree is clean or preserved user work is identified; and
- the final report gives exact commands, classifications, commit SHAs, receipt/log/artifact paths
  and digests, deviations, limitations, push status, and the smallest next candidates.

Do not claim public completion from a private prototype, internal unit test, stale receipt,
documentation, or an unavailable gate.
