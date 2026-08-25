# Repository Agent Instructions

## Scope and precedence

This file applies to the repository root and every descendant unless a nearer `AGENTS.md` or `AGENTS.override.md` provides more specific instructions.

Apply instructions in this order:

1. the active user request;
2. the active campaign prompt explicitly selected by the user;
3. the nearest applicable agent instruction file;
4. executable public contracts, tests, and verification policy;
5. normative specifications under `docs/spec/`;
6. current status and architecture documents;
7. implementation and generated documentation;
8. historical prompts, ledgers, comments, and commits.

Historical prompts are evidence, not current requirements. When sources conflict, preserve the higher-precedence source and record the conflict rather than silently combining incompatible rules.

Read the active campaign and inspect the current Git state before editing. A recorded campaign baseline is orientation; audit later commits when HEAD has advanced.

## Mission and enduring invariants

`lkjscript` is an AI-first programming language and application platform.

- One accepted revision of the typed semantic graph is the sole authority for program meaning.
- Source text, compact requests, direct CLI flags, review projections, indexes, artifacts, bytecode, runtime handles, deployment descriptors, caches, logs, and receipts are not second editable program authorities.
- Ordinary application development must be possible through the distributed `lkjscript` executable.
- Application policy belongs in graph meaning. Rust owns generic host mechanisms.
- Names are mutable locators and presentation. Stable typed identities express continuity.
- Exact semantic references and dense runtime indexes are separate concerns.
- Logical meaning and physical layout are separate concerns.
- Accepted writes use an exact base, complete validation evidence, durable immutable objects, and one atomic HEAD visibility point.
- Failed, stale, cancelled, exhausted, corrupt, or interrupted work must not partially advance accepted authority.
- Backward compatibility is not a default objective. Move maintained consumers, reject predecessor input, and delete predecessor paths as one dependency-closed cutover.
- Keep representations replaceable so measured future optimization does not require changing language meaning.

AI-first does not justify ambiguous contracts, hidden state, excessive terminology, weak diagnostics, or an undiscoverable CLI.

## Repository map

Use these locations according to their authority:

- `src/platform/`: language, authority, repository, compiler, runtime, adapter, and public-control implementation.
- `src/bin/lkjscript.rs`: released process boundary.
- `tools/lkjscript-dev/`: contributor-only verification, evidence, scale, service, and policy tooling.
- `tests/`: black-box public CLI and service acceptance.
- `packages/standard/`: maintained standard-package consumer.
- `applications/lkjournal/`: maintained application and service consumer.
- `docs/spec/`: normative behavior.
- `docs/status.md`: implemented current facts and limitations.
- `docs/architecture.md`: current layer and dependency map.
- `docs/roadmap.md`: deferred evidence-gated work.
- `docs/performance.md` and `docs/evidence/`: reproduced measurements and retained evidence.
- `docs/generated/`: executable-generated contract documentation; never hand-edit it.
- `docs/decisions/`: durable design decisions and reversal conditions.
- `prompts/`: campaign implementation specifications and history.
- `docs/campaigns/`: concise campaign state and evidence indexes.

Do not turn `AGENTS.md` into a normative specification, campaign plan, status report, measurement log, or roadmap.

## Initial repository inspection

Before changing files, capture:

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

Then read only the smallest authoritative path set needed by the active campaign. Prefer exact symbol search over broad rescans.

Preserve pre-existing tracked and untracked work. Do not assume an unfamiliar change is disposable.

## Git and working-tree safety

Never use destructive commands such as:

```text
git reset --hard
git clean -fd
git checkout -- .
git restore .
git rebase
git push --force
git push --force-with-lease
```

unless the active user request explicitly requires the exact action and its consequences have been verified.

Additional rules:

- Do not rewrite history.
- Do not remove or overwrite unrelated user work.
- Do not use `git add .` or `git add -A`; stage explicit paths.
- Inspect the staged diff before every commit.
- Keep commits coherent, buildable where practical, and paired with their proving tests.
- Do not mix unrelated formatting or cleanup into a campaign commit.
- Use a normal fast-forward push only when authorized and the remote state is understood.
- End with a clean working tree unless an explicit preserved user change is documented.

## Build and verification entry points

The workspace is stable Rust 2024 and uses the locked dependency graph.

Useful narrow checks include:

```sh
cargo fmt --all -- --check
cargo test --locked --lib FILTER
cargo test --locked --test public_cli FILTER
cargo test --locked --test general_service FILTER
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
```

Repository-level profiles are:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

During iteration, run the narrowest relevant unit or black-box test. Do not run the full suite after every edit. Run the fresh `full` profile once the final content, generated files, and documentation are complete.

The verification harness owns gate dependencies, exact fingerprints, bounded child logs, required outputs, and fresh/reused classification. Reuse evidence only when the harness proves exact input identity. Never describe skipped, stale, unavailable, or failed evidence as passed. Store large logs under the harness artifact directory and return paths rather than pasting them.

Environment-dependent service checks may be unavailable. Report the environmental reason and all independent evidence; do not weaken the gate or fabricate success.

## Rust and first-party tooling

- Rust is the default implementation and repository-tooling language.
- Do not add Python, shell orchestration that replaces typed first-party tooling, or a new runtime dependency without a campaign-specific necessity.
- Use stable Rust and keep `Cargo.lock` authoritative.
- Safe Rust is the default; repository lint policy forbids `unsafe`.
- Production code must satisfy the workspace denial of `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.
- Tests may use local allowances only with a narrow reason.
- Prefer typed domain objects, checked conversions, deterministic collections, explicit ownership, and bounded decoding.
- Reject malformed tags, foreign identity domains, duplicate or noncanonical data, overflow, trailing bytes, and limit exhaustion at the owning boundary.
- Do not couple contract bytes or digest identity to Rust enum order, serde representation, memory layout, filesystem paths, or hash iteration.
- Keep generated artifacts reproducible and regenerate them through their owning executable command.
- Avoid new dependencies when the standard library or an existing dependency is sufficient.
- Remove first-party predecessor tooling in the same campaign after all maintained consumers move.

## Semantic authority and publication

- Mutations must lower to typed semantic intent before validation or publication.
- The repository publication API remains the sole normal accepted-authority writer.
- Plan and apply for the same operation must share normalization, allocation, impact analysis, validation, and response semantics.
- A plan binds the exact semantic base and normalized request. Apply must reject a mismatched plan before repository access when the plan can be checked without it.
- Under the publication lock, recheck the accepted base before advancing HEAD.
- Durable canonical objects precede the single HEAD visibility change.
- A retry after uncertain interruption begins by reading current HEAD and retained receipts; never blindly replay.
- Derived summaries, indexes, compiler state, artifacts, review files, and deployment data may rebuild but cannot silently alter accepted meaning.
- Missing disposable state may rebuild. Inconsistent canonical authority is corruption, not a cache miss.
- Keep independent complete oracles until differential evidence justifies removal.

## Public CLI and control protocols

- The executable registry is the exhaustive owner of finite public operations, request/response models, vocabulary, limits, diagnostics, and security nonclaims.
- One behavior has one public name. Reject unknown commands and options; do not add compatibility aliases or fallback routing.
- Direct flags, compact records, and any future transport are adapters to transport-neutral typed requests.
- Do not make raw JSON, storage objects, generated schemas, source text, or review text the required authoring path.
- Public finite output is deterministic, bounded, and machine-readable. It names the observed revision when meaning is read.
- Use stdout for the classified machine result. Keep stderr empty for classified finite outcomes unless the current contract explicitly states otherwise.
- Growing results use explicit budgets and continuations or write to an explicit output file.
- Large payloads and logs belong in files referenced by digest and path, not repeated in process output or agent context.
- A public operation is complete only when discovery, valid behavior, failures, bounds, copied-binary use, documentation, and predecessor rejection are all proven.
- Do not expose a private typed operation until its complete public workflow exists.

## Errors, resources, and recovery

Diagnostics must identify a stable class and code, the failed boundary, and an actionable correction. Preserve useful locations without leaking secrets or large payloads.

Keep resource dimensions separate. At minimum distinguish, when relevant:

- input bytes and records;
- semantic operations;
- semantic work;
- affected owners;
- output bytes and records;
- wall and CPU time;
- peak RSS;
- filesystem bytes and synchronization operations;
- runtime task, connection, queue, object, page, and retry counts.

Every numeric bound must have a unit, owning boundary, classification, override policy, and rationale. Do not use one `fuel` value as a substitute for a multidimensional resource model. Do not infer provider tokens, cache usage, or monetary cost without direct telemetry.

Cancellation and exhaustion are normal typed outcomes. Release owned resources, leave authority unchanged, and return enough evidence for deterministic recovery.

## Security and external effects

Treat request files, paths, artifacts, backups, continuations, deployment descriptors, adapter input, and network data as hostile decoding boundaries.

- Validate before unbounded allocation or external effect.
- Reject symlink and non-regular-file surprises at publication boundaries.
- Use private staging, synchronization, and atomic visibility for durable output.
- Redact secrets from output, logs, diagnostics, receipts, and test fixtures.
- Keep application authorization in graph meaning and generic capability enforcement in host mechanisms.
- Do not claim hostile-code sandboxing, multi-tenant isolation, encrypted graph storage, artifact provenance, distributed consensus, or platform support that has not been implemented and verified.
- The current plaintext HTTP and PostgreSQL `NoTls` boundary is not permission to add speculative TLS machinery. Transport encryption belongs at an explicitly selected trusted boundary or a separately justified adapter.

## Design and scope discipline

For a substantial campaign:

1. define one primary objective;
2. identify current authorities and maintained consumers;
3. compare the smallest credible alternatives;
4. select one dependency-closed target;
5. classify mandates, selected design, empirical questions, and non-goals;
6. implement vertical, verifiable milestones;
7. migrate maintained consumers and delete predecessors;
8. update the correct documentation layer;
9. verify the final public workflow and complete repository policy.

Prefer established technical vocabulary. Use one term per concept. Remove obsolete terms from current code and documentation after cutover.

Do not introduce sessions, daemons, additional registry layers, schedulers, caches, JITs, AOT, SIMD, memory mapping, custom allocators, `unsafe`, or distributed machinery without a measured problem, a named consumer, an independent oracle, and a deletion or reversal condition.

Do not preserve a design merely because it is implemented or recent. Do not replace correct work merely to create visible change. Avoid speculative abstraction and arbitrary structural limits.

When a serious unrelated problem is found, record exact evidence and defer it unless it invalidates the active objective or presents an immediate authority, data-loss, or security risk.

## Documentation ownership

- Change normative behavior only in `docs/spec/`.
- Change current implementation facts only in `docs/status.md`.
- Change the layer map only in `docs/architecture.md`.
- Record actual measurements in `docs/performance.md` or `docs/evidence/`.
- Keep generated contract tables executable-derived.
- Keep durable decisions concise and include reversal conditions.
- Keep campaign ledgers concise and evidence-oriented.
- Keep user examples in `README.md`.
- Do not duplicate complete field catalogs, operation catalogs, or version tables across handwritten files.

Historical comments and prompts must not override current executable contracts.

## Completion conditions

Work is complete only when:

- the selected public workflow works through the released executable;
- maintained consumers use the selected path;
- predecessor readers, writers, adapters, aliases, fixtures, schemas, tests, and stale documentation in scope are deleted;
- valid, malformed, stale, exhausted, interrupted, corrupt, and predecessor cases relevant to the change are tested;
- independent oracles agree;
- generated files match their owner;
- final verification is fresh for the final content;
- commits are coherent;
- the working tree is clean or preserved user work is explicitly identified; and
- the final report returns exact commands, outcomes, commit SHAs, receipt/log/artifact paths, deviations, limitations, and the smallest next candidates.

Do not claim public completion from a private prototype, an internal unit test, a stale receipt, or documentation alone.
