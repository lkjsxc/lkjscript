# AGENTS.md

## Scope

This file applies to the entire `lkjscript` repository.
Use English for repository artifacts unless the active task explicitly requires another language for a user-facing result.
A narrower subtree policy may refine local procedure, but it must not create a second language definition, semantic authority, identity model, ownership model, compiler route, runtime route, artifact authority, or documentation authority.
The active task chooses the objective; this file governs how repository work is selected, implemented, verified, documented, committed, and reported.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free, high-performance programming system.
AI-primary means deterministic semantic machinery makes programs economical for agents to discover, construct, edit, inspect, leave incomplete, check, review, compile, and run.
Model inference must not participate in parsing, typing, ownership, effect analysis, validation, optimization correctness, artifact acceptance, persistence correctness, or runtime correctness.
Optimize for long-term semantic coherence and low reasoning burden rather than compatibility with provisional repository history.
Prefer one small complete architecture over several ambitious partial architectures.

## Priority order

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. Scale-safe representations and algorithms.
5. One syntax-independent semantic authority.
6. One complete generic production compiler and runtime route.
7. Direct deterministic workflows for agents and ordinary tools.
8. Evidence before abstraction, caching, incrementality, parallelism, services, or distribution.
9. Optional specialization only behind the complete generic route.
10. Broader products only after present prerequisites and consumers exist.

A future platform idea is not evidence for present machinery.
Backward incompatibility is permission for a clean cutover, not uncontrolled scope.

## Authority

- The active task owns the objective, explicit acceptance criteria, product constraints, and stop condition.
- This file owns repository-wide engineering procedure and evidence discipline.
- `docs/spec/language.md` owns intended language semantics.
- `docs/spec/workspace.md` owns intended semantic-workspace behavior.
- Code, tests, commands, schemas, manifests, and locks own checkout behavior.
- `README.md` owns the product introduction and first successful use.
- `docs/status.md` owns concise current implementation and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns measurement protocol, retained evidence, and reversal conditions.
- `docs/roadmap.md` owns planned ordering only.
- Sparse accepted files under `docs/decisions/` own durable non-obvious rationale when justified.
- Git history owns superseded implementation and prose.

When claims conflict, classify the claim, inspect its owning artifact and executable evidence, then update or delete stale material in the same coherent change.
Do not manufacture authority through prompt archives, transcripts, handoff files, global revisions, copied status tables, unconsumed registries, or unconsumed digests.

## Autonomy and compatibility

- Use autonomous judgment from the actual checkout, accepted specifications, focused tests, measurements, current consumers, and real failure boundaries.
- Ask the user only when a genuinely external product requirement is missing and no safe explicit assumption can complete the selected vertical.
- Do not ask the user to choose among internal alternatives that repository evidence can decide.
- Backward compatibility is not an objective unless the active task names a current independent boundary that must remain compatible.
- Language syntax, source encoding, Rust APIs, commands, packages, locks, modules, crates, compiler forms, runtime forms, fixtures, tests, and documentation may change incompatibly.
- When cutting over, update every active producer and consumer, regenerate active artifacts when required, and delete displaced implementations, adapters, aliases, migrations, flags, and stale tests.
- Do not leave `legacy`, `v2`, `next`, edition, compatibility, dual-read, or dual-write architectures.
- The `.lkjscript` extension is fixed; other provisional representations remain replaceable unless accepted semantics fix them.
- Preserve unrelated tracked work, untracked work, credentials, host state, external data, and remote history.
- Never reset, clean, rewrite, force-push, or otherwise destroy work you did not create.

## Multi-turn operating loop

Work in dependency-closed verticals. One turn need not resolve the complete roadmap.

### Inspect

1. Inspect branch, worktree, upstream, and recent history.
2. Read the active task and only the authority needed for the selected problem.
3. Search owning symbols before opening large files.
4. Map producers, consumers, owner, lifetime, invalidation, trust boundaries, and failure paths.
5. Classify relevant values and files by boundary kind before proposing metadata or abstraction.
6. Separate current, target, hypothesis, measurement, historical, and unknown claims.

### Select

1. Choose one demonstrated defect, accepted gap, measured bottleneck, explicit product result, or current maintenance burden.
2. State a falsifiable hypothesis, completion criteria, reversal condition, stop condition, and non-goals.
3. Choose a vertical small enough to finish and verify in the current turn.
4. Prefer deletion and simplification before machinery.
5. Do not make crate count, file count, line count, symmetry, or naming aesthetics objectives by themselves.

### Implement

1. Add or identify the smallest focused evidence that can fail.
2. Fix the root cause rather than wrapping it.
3. Reuse canonical validation and invariants.
4. Keep one mutable authority and one active route.
5. Make invalid states unrepresentable when that is smaller than validating fabricated states.
6. Delete displaced paths completely.
7. Run the smallest command that can falsify the change.
8. Review the diff as a consumer, adversary, and future coding agent.
9. Update the owning documentation.

### Finish

1. Run the final relevant verification boundary once final relevant inputs are stable.
2. Commit cohesive changes when permitted.
3. Push only when explicitly requested.
4. Report evidence, residual risk, commit state, worktree state, and publication state.
5. Name the next highest-leverage problem without beginning it.

Stop rather than opening an unrelated second vertical.
Do not leave a half-cutover, hidden fallback, disabled check, stale current prose, temporary compatibility layer, unowned cache, or unreviewed subagent branch.

## Boundary classification

Classify a value before deciding whether it needs a contract, version, digest, codec, registry entry, or independent validation.

### Persistent or transmitted artifact
- Bytes can outlive the producing process or binary, and producer and consumer may come from different builds.
- Use explicit canonical encoding, exact identity, bounded decoding, and fail-closed validation appropriate to the threat model.

### Untrusted or unsafe boundary
- The consumer crosses FFI, executable memory, generated entry, operating-system input, external bytes, or another safety boundary.
- Validate structure and policy at the actual boundary.
- Same-build typing does not replace validation of untrusted bytes; a digest does not replace structural validation.

### Independent machine-facing output
- A real script, tool, test harness, or user can retain and parse output independently.
- Give it a stable schema identity only when compatibility or exact interpretation is a current requirement.
- Do not add a schema merely because JSON can be emitted.

### Same-build typed value
- Producer and consumer are compiled together and exchange a private or typed in-memory value.
- Rust type identity, private construction, canonical validation, and rebuilding normally provide continuity.
- Do not serialize, hash, register, version, or compare the value with the current build merely to prove it came from the current build.

### Shared canonical vocabulary
- Capability kinds, resource kinds, operation identities, semantic traits, and similar terms may be shared typed vocabulary.
- Vocabulary does not automatically require a descriptor registry or content digest.
- Keep one direct owner.

### Derived observation
- Timings, counts, indexes, projections, candidates, and diagnostics derive from authority.
- Do not promote them into mutable authority.
- Retain them only while a current consumer justifies their lifecycle and cost.

A future serialization, daemon, cache, plugin, or distributed consumer does not make a current same-build value an artifact.
Introduce an explicit boundary when that consumer is actually implemented and tested.

## Architectural invariants

- Maintain one language definition, semantic authority, identity model, incomplete-state model, compiler input route, ownership model, generic production execution route, package model, and documentation authority model.
- Semantic meaning must be able to exist without source text, paths, spans, formatting, parser nodes, hashes, or a running service.
- Source and files are importer inputs, provenance, presentation, persistence, and interoperability forms; they are not mutable semantic authority.
- Incomplete state is valid editing state and never executable state.
- Compilation consumes one complete immutable semantic snapshot directly; never render and reparse semantic state.
- Dense IDs, slots, offsets, registers, layouts, and traversal positions remain private.
- Stable public identities survive private relocation; removals tombstone; old immutable snapshots remain valid.
- One successful transaction publishes one revision; failure publishes nothing and consumes no future identity state.
- Queries and diffs are deterministic, revision-labelled, structured, stably ordered, and never silently truncated.
- Keep one complete generic production execution route.
- Checking must not execute effects.
- Baseline-native specialization may decline only before effects and generated entry.
- After native entry begins, its result or failure is final; never rerun effects through VM fallback.
- A test-only evaluator may remain an independent oracle, not a production engine.
- Ordinary execution is collector-free and non-tracing.
- Preserve exact move, borrow, loan, cleanup, return, trap, cancellation, allocation, resource, host-resource, and teardown laws.
- Unsafe code belongs in narrow named mechanisms with explicit invariants and malformed-input evidence.
- Keep FFI, executable-memory, relocation, W^X, and generated-entry boundaries fail-closed.
- Language validity follows semantic laws, not arbitrary project quotas.
- Use checked arithmetic, checked narrowing, and iterative traversal for user-controlled depth.
- An untrusted product may impose explicit resource policy without redefining language validity.

## Evidence and anti-overengineering

A hypothetical consumer, design diagram, desired symmetry, old prompt, or anticipated platform is not evidence.
Before adding a nontrivial abstraction, identify its present producer, consumer, owner, lifetime, invalidation, failure behavior, benefit, maintenance cost, reversal condition, and deletion condition.

1. Delete unused work.
2. Simplify semantics.
3. Simplify representation.
4. Reuse an invariant or canonical validator.
5. Improve a local traversal or layout.
6. Make invalid state unrepresentable.
7. Extract shared machinery only after multiple current consumers prove shared semantics.
8. Cache only after measured repeated work.
9. Parallelize only after measured separable work.
10. Add incrementality only after measured recomputation dominates.
11. Add a process boundary only after measured demand.

Do not add speculative daemons, services, sessions, journals, databases, CRDTs, schedulers, registries, plugin systems, rewrite DSLs, cache frameworks, proof ecosystems, wire protocols, target matrices, deoptimization, PGO, self-hosting scaffolding, or orchestration platforms.
Do not build a universal framework for one current traversal, state, event, recovery case, descriptor, validation step, or transaction.

## Crates and dependencies

- A crate boundary must earn itself through a trust or unsafe boundary, independently useful API, supported target boundary, measured compile isolation, low-coupling subsystem, or current product boundary.
- Do not merge genuine unsafe or FFI boundaries merely to reduce member count.
- Do not add a crate merely to hide fan-in or move a dependency edge.
- Do not move types solely to improve graph aesthetics.
- Prefer mature dependencies when they remove substantial machinery or risk; keep local code when it is smaller, clearer, safer, or measurably better.
- Do not add a dependency for a trivial transformation.

## Agent attention and API spend

- Treat model context, tool output, attention, wall time, CPU, memory, disk, CI minutes, and API spend as engineering resources.
- Reduce them through deletion, direct ownership, focused inspection, and nonduplicated verification, never by hiding failures or weakening evidence.
- Search before opening large files; read focused ranges, symbols, call sites, and diffs.
- Keep one compact temporary orientation note and delete it before committing.
- Reuse facts already established in the turn.
- Do not ask multiple agents to rediscover the same map.
- Use the smallest useful set of read-only subagents for genuinely independent questions; one lead owns architecture, writes, integration, verification, commits, and the report.
- Do not let two agents edit the same file or evolve the same semantic model independently.
- Run focused tests before crate, workspace, release, or container boundaries.
- Do not repeat an identical successful command after unchanged relevant inputs.
- Use quiet commands while preserving status and diagnostics.
- Keep full failure logs outside Git and surface only relevant excerpts.
- Do not dump unchanged files, repository-wide diffs, generated IR, bytecode, machine code, large JSON, complete projections, or successful logs without a consumer.
- Do not create a service, cache, broker, database, protocol, logging framework, or agent-facing abstraction merely to reduce prompt volume.
- Do not commit prompt archives, transcripts, raw subagent packets, token ledgers, or per-turn fact registries.
- Do not present estimates as measurement.
- Label removed bytes, lines, symbols, edges, and command invocations as structural proxies, not direct API-cost measurements.

## Tests

- Tests protect intended semantics and public invariants, not provisional topology or compatibility.
- Add one focused regression or characterization for each selected root cause.
- Preserve failure atomicity, stable identities, old snapshots, ownership and cleanup, deterministic output, effect-free checking, and exactly-once effects as relevant.
- Decode machine output as a consumer would.
- Quiet-success tests assert both streams are empty.
- Use generated fixtures for scale; keep costly equivalent geometry in explicit locked-release stress when justified.
- Use differential, property, model, fuzz, Miri, sanitizer, or small-stack evidence when it is the cheapest independent oracle.
- Delete tests that preserve obsolete APIs, fabricated boundaries, old formats, arbitrary limits, or private topology.
- Never weaken a test merely to make a redesign pass.
- Do not preserve a configurable impossible state solely so a rejection test can manufacture it.

## Verification

Escalate only after focused evidence passes. Do not repeat the full boundary after unchanged relevant inputs.

### Focused and affected

Run the smallest relevant test, then affected crates, binaries, features, integration targets, package fixtures, and machine-output consumers.

### Native repository boundary

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

### Retained product and container boundary

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run the container boundary when changes can affect dependencies, packaging, release compilation, compiler or executable behavior, VM, native code, host capabilities, examples, smokes, system libraries, generated package artifacts, or publication confidence.
Documentation-only work need not rebuild it.
If the environment blocks a command, report the exact command, failure category, relevant output, successful remaining evidence, and residual risk.
Never claim a command passed unless it completed successfully.

## Documentation

- Update the owning document and delete stale text in the same change.
- Do not copy implementation inventories into multiple files.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later` ordering.
- Create a decision record only for durable, non-obvious, expensive-to-rediscover rationale with a meaningful reversal condition.
- Do not describe target as current, hypothesis as measurement, planned systems as supported, private relocation as public movement, or one-host observation as portable behavior.
- Keep examples active and mechanically checked where practical.
- Documentation length is not rigor; prefer one compact authoritative statement.

## Git and publication

- Inspect status before editing and before committing.
- Preserve unrelated work.
- Do not reset, clean, rewrite history, or force-push work you did not create.
- Commit one cohesive semantic, architectural, or measured result rather than splitting by file type.
- Exclude task prompts, raw logs, scratch notes, generated temporary files, credentials, and unrelated work.
- Use a commit message that names the actual result.
- Push only when explicitly requested.
- After requested publication, verify the local commit, tracking branch, and remote result.

## Final report

- State the completed objective and evidence gate.
- State starting and final commits, branch, upstream, and worktree state.
- State the principal design and every displaced path deleted.
- State boundary classifications governing retained and removed validation.
- State focused tests, measurements when relevant, and exact final verification outcomes.
- State environment-limited checks and residual risk.
- State documentation, commit, and publication status.
- State material subagent use and integration result.
- Name the next highest-leverage problem and explain why it was not started.

Keep the report factual and compact.
Do not reproduce the task prompt, paste successful logs, claim unmeasured savings, or describe future work as implemented.
