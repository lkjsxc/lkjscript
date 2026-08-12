# AGENTS.md

## Scope

This file applies to the entire `lkjscript` repository.
Use English for repository artifacts unless the active task explicitly requires another language for a user-facing result.
A narrower subtree policy may refine local procedure, but it must not create a second semantic authority, compiler route, runtime route, ownership model, or documentation authority.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free, high-performance programming system.
AI-primary means deterministic semantic machinery makes programs easy for agents to discover, construct, edit, inspect, check, review, compile, and run.
It does not mean model inference belongs inside parsing, typing, ownership, validation, optimization correctness, persistence correctness, or runtime correctness.
Optimize for long-term coherence rather than compatibility with provisional repository history.
Prefer one small complete architecture over several ambitious partial architectures.

## Priority order

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. Scale-safe representations and algorithms.
5. One syntax-independent semantic authority.
6. One complete generic production compiler and runtime route.
7. A compact deterministic workflow for agents and ordinary tools.
8. Evidence before abstraction, caching, incrementality, parallelism, or services.
9. Optional specialization only behind the complete generic route.
10. Broader products only after their prerequisites and consumers exist.

A future platform idea is not evidence for present machinery.
A benchmark may expose a defect; it does not define the language.

## Authority

Use the artifact that owns the claim:

- The active task owns its objective and explicit acceptance criteria.
- This file owns repository-wide engineering procedure.
- `docs/spec/language.md` owns intended language semantics.
- `docs/spec/workspace.md` owns intended semantic-workspace behavior.
- Code, tests, command definitions, schemas, manifests, and locks own checkout behavior.
- `README.md` owns product introduction and first successful use.
- `docs/status.md` summarizes current implementation and known gaps.
- `docs/architecture.md` explains current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns measurement method and retained evidence.
- `docs/roadmap.md` owns ordering and intent only.
- Sparse accepted files under `docs/decisions/` own durable rationale when one is genuinely needed.
- Git history owns superseded implementation and prose.

When claims conflict, classify the claim, inspect the owning artifact, inspect executable evidence, and update or delete stale material in the same change.
Do not manufacture authority through prompt archives, transcripts, handoff files, global revisions, fact registries, copied status tables, or unconsumed digests.

## Autonomy and compatibility

Use autonomous engineering judgment from the actual checkout, accepted specifications, tests, measurements, current consumers, and real failure boundaries.
Ask the user only when an external product requirement is genuinely missing and no safe explicit assumption can complete the task.
Do not ask the user to choose among internal alternatives that repository evidence can decide.

Backward compatibility is not a project objective unless the active task names a current external boundary that must remain compatible.
Language syntax, source encoding, Rust APIs, commands, packages, locks, modules, crates, compiler forms, runtime forms, fixtures, tests, and documentation may change incompatibly.
When cutting over, update every active producer and consumer, then delete the displaced implementation, adapter, alias, migration, feature flag, and stale test.
Do not leave `legacy`, `v2`, `next`, edition, compatibility, or dual-write architectures.
The `.lkjscript` extension is fixed; other provisional representations remain replaceable unless an accepted specification fixes them.

Preserve unrelated tracked work, untracked work, credentials, host state, external data, and remote history.
Never use destructive Git operations on work you did not create.

## Operating loop

### Inspect

1. Inspect branch, worktree, upstream, and recent history.
2. Read the active task and only the authority documents needed for the selected problem.
3. Search owning symbols before opening large files.
4. Map producers, consumers, ownership, lifetime, invalidation, trust boundaries, and failure paths.
5. Distinguish persistent or independent artifacts from typed values that remain inside one process and build.

### Select

1. Choose one demonstrated defect, accepted gap, measured bottleneck, explicit product result, or current maintenance burden.
2. State a falsifiable hypothesis.
3. State completion criteria, reversal condition, stop condition, and non-goals.
4. Choose a dependency-closed vertical small enough to finish and verify in the current turn.
5. Do not make crate count, file count, line count, or symmetry an objective by itself.

### Implement

1. Add or identify the smallest focused evidence that can fail.
2. Fix the root cause rather than wrapping it.
3. Reuse canonical validation and invariants.
4. Keep one mutable authority and one active route.
5. Delete displaced paths completely.
6. Run the smallest command that can falsify the change.
7. Review the diff as a consumer and adversary.
8. Update the owning documentation.

### Finish

1. Run the final relevant verification boundary once final relevant inputs are stable.
2. Commit cohesive changes when the task permits.
3. Push only when explicitly requested.
4. Report evidence, residual risk, commit state, and publication state.
5. Name the next highest-leverage problem without beginning it.

Stop rather than opening an unrelated second vertical.
Do not leave a half-cutover, hidden fallback, disabled check, stale current prose, temporary compatibility layer, unowned cache, or unreviewed subagent branch.

## Architecture rules

Maintain one language definition, semantic authority, identity model, incomplete-state model, compiler input route, ownership model, generic production execution route, package model, and documentation authority model.

- Semantic meaning must be able to exist without source text, paths, spans, formatting, parser nodes, hashes, or a running service.
- Source and files are importer inputs, provenance, presentation, and interoperability forms; they are not semantic authority.
- Incomplete state is valid editing state and never executable state.
- Compilation consumes one complete immutable semantic snapshot directly.
- Do not render and reparse semantic state.
- Do not keep parallel source-shaped and semantic mutable authorities.
- Dense IDs, indexes, slots, offsets, registers, and traversal coordinates remain private.
- Stable public identities survive private relocation; removed identities tombstone; old immutable snapshots remain valid.
- One successful transaction publishes one coherent revision; one failed transaction publishes nothing and consumes no future identity state.
- Machine-facing queries and diffs are deterministic, revision-labelled, structured, stably ordered, and never silently truncated.

Keep one complete generic production execution route.
Baseline-native specialization may decline only before effects and before generated entry.
After native entry begins, its result or failure is final; never rerun effects through VM fallback.
Checking must not execute program effects or construct execution state unnecessarily.
A test-only evaluator may remain an independent oracle, not a second production engine.

Treat a package file, lock, serialized object, FFI input, executable mapping, or independently produced artifact as a real boundary when it is one.
Do not attach versions, hashes, registries, schemas, codecs, or validation passes to an in-process typed value merely because a future boundary could exist.
Conversely, do not remove validation from a genuine untrusted, persistent, unsafe, or independently produced boundary.

A crate boundary must earn itself through a trust or unsafe boundary, independently useful API, supported target boundary, measured compile isolation, low-coupling subsystem, or current product boundary.
Do not merge a genuine unsafe or FFI boundary merely to reduce member count.
Do not add a crate merely to hide fan-in or move a dependency edge.
Merge, split, rename, or delete components only when ownership and current consumers become clearer.

## Types, ownership, and resources

Generic declarations, substitutions, bounds, instantiations, and witnesses are semantic facts.
Source inference and source-free edits must converge on one exact resolver.
Keep type identity stable and syntax-independent.
Use checked conversion at compact machine and host boundaries.

Ordinary execution is collector-free and non-tracing.
Do not add tracing GC, hidden language-visible reference counting, raw-pointer language surfaces, general `free`, retain/release APIs, ownership escape hatches, or parallel GC/non-GC modes.
Preserve exact move, borrow, loan, cleanup, early-return, trap, cancellation, allocation, resource, host-resource, and teardown laws.
Prevent leaks, double release, use after move, stale loans, duplicated effects, and stack-overflow destruction.

Unsafe code belongs in a narrow named mechanism with explicit invariants, a safe-caller contract, and focused malformed-input evidence.
Keep FFI, executable-memory, and generated-entry boundaries fail-closed.
Inside one trusted typed synchronous pipeline, do not repeatedly serialize, hash, reconstruct, clone, render, parse, or independently revalidate the same value without a real consumer boundary or measured need.

Language validity follows semantic laws, not arbitrary project quotas.
Do not reject trusted valid programs because of selected source, token, depth, declaration, field, variant, function, local, IR-node, identity, or value counts.
Use checked arithmetic, checked narrowing, and iterative traversal for user-controlled depth.
An untrusted product may impose explicit memory, output, time, cancellation, concurrency, handle, fuel, or artifact-size policy.
Resource exhaustion is a typed host result, not a language-semantic error.

## Evidence and anti-overengineering

Begin from current evidence.
A hypothetical consumer, design diagram, desired symmetry, or old prompt is not evidence.
Before adding a nontrivial abstraction, identify its present producer, consumer, owner, lifetime, failure behavior, benefit, maintenance cost, reversal condition, and deletion condition.

Prefer this order:

1. Delete unused work.
2. Simplify semantics.
3. Simplify representation.
4. Reuse an invariant.
5. Reuse the canonical validator.
6. Improve local traversal or layout.
7. Make invalid state unrepresentable.
8. Cache only after measured repeated work.
9. Parallelize only after measured separable work.
10. Add incrementality only after measured recomputation dominates.
11. Add a process boundary only after a measured consumer requires it.

Do not add speculative daemons, services, sessions, journals, databases, CRDTs, schedulers, registries, plugin systems, rewrite DSLs, cache frameworks, proof ecosystems, wire protocols, target matrices, deoptimization, PGO, self-hosting scaffolding, or orchestration platforms.
Do not build a universal framework for one current traversal, state, event, recovery case, or validation step.
Keep a performance change only when end-to-end benefit justifies complexity, memory, code size, dependencies, portability, tests, and maintenance.

## Agent attention and API spend

Model context, tool output, wall time, CPU, memory, disk, CI minutes, and API spend are engineering resources.
Reduce them by removing duplicate work, not by hiding failures or weakening evidence.

- Search before opening large files.
- Read focused ranges and diffs before full documents.
- Keep one compact temporary orientation note and delete it before committing.
- Reuse facts already established in the current turn.
- Run focused tests before crate, workspace, release, or container boundaries.
- Do not repeat an identical successful command after unchanged inputs.
- Use native quiet flags while preserving nonzero status and diagnostics.
- Keep full failure logs outside Git and surface only relevant excerpts.
- Do not dump unchanged files, repository-wide diffs, generated IR, bytecode, machine code, large JSON, complete projections, or successful logs without a consumer.
- Do not create a service, cache, broker, database, protocol, or logging framework to reduce prompt volume.
- Keep this file concise; put task-specific detail in the active task, not permanent policy.
- Do not create or commit prompt archives, transcripts, handoffs, raw samples, or token ledgers.
- Estimate provider tokens or billing only when explicitly labelled as an estimate; do not present it as measurement.

Use the smallest useful set of subagents only when independent work shortens the critical path.
Prefer parallel read-only mapping before writing.
One lead owns architecture, write scope, integration, final verification, commits, and the final report.
Do not let two agents edit the same file or evolve the same semantic model independently.

## Tests

Tests protect intended semantics and public invariants, not provisional topology.
Add one focused regression or characterization for each selected root cause.
Preserve failure atomicity, stable identities, old snapshots, exact ownership and cleanup, deterministic queries and output, effect-free checking, and exactly-once effects as relevant.
Decode machine output as a consumer would; do not rely only on substrings.
Quiet-success tests assert both streams are empty.
Use generated fixtures for scale and keep the largest equivalent geometry in explicit locked-release stress when default cost is not justified.
Use differential, property, model, fuzz, Miri, sanitizer, or small-stack evidence when it is the cheapest independent oracle for the selected boundary.
Delete tests that preserve obsolete APIs, fabricated boundaries, deleted formats, arbitrary limits, or private topology.
Never weaken a test merely to make a redesign pass.

## Verification

Escalate only after focused evidence passes.
Do not repeat the full boundary after unchanged inputs.

### Tier 1: focused

Run the smallest relevant unit, integration, compile-only, convergence, malformed-input, machine-output, or smoke test.

### Tier 2: affected components

Run the affected crates, binaries, features, and integration targets with the production feature shape needed by the change.

### Tier 3: native repository boundary

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

### Tier 4: retained product/container boundary

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run Tier 4 when changes can affect dependencies, Docker inputs, packaging, release compilation, compiler or executable behavior, VM, native code, host capabilities, examples, smokes, system libraries, or publication confidence.
Documentation-only work need not rebuild it.
If an environment blocks a command, report the exact command, failure category, relevant output, successful remaining evidence, and residual risk.
Never claim a command passed unless it completed successfully.

## Documentation

Update the owning document and delete stale text in the same change.
Do not copy implementation inventories into multiple files.
`docs/roadmap.md` contains only `Now`, `Next`, and `Later` ordering.
Create a decision record only when a choice is durable, non-obvious, expensive to rediscover, and governed by a meaningful reversal condition.
Do not describe target as current, hypothesis as measurement, planned systems as supported, or one-host observation as a portable guarantee.
Keep examples active and mechanically checked where practical.

## Git and publication

Inspect status before editing and before committing.
Preserve unrelated work.
Do not reset, clean, rewrite history, or force-push work you did not create.
Commit one cohesive semantic, architectural, or measured result rather than splitting by file type.
Exclude task prompts, raw logs, scratch notes, temporary generated files, credentials, and unrelated work.
Use a commit message that names the actual result.
Push only when explicitly requested.
After requested publication, verify the local commit, tracking branch, and remote result.

## Final report

Report:

- the completed objective and evidence gate;
- the principal design and deleted or replaced paths;
- focused tests and measurements when relevant;
- exact verification commands and outcomes;
- environment-limited checks and residual risk;
- documentation, commit, worktree, and publication state;
- material subagent use and integration result; and
- the next highest-leverage problem and why it was not started.

Keep the report factual and compact.
Do not reproduce the task prompt, paste successful logs, or claim future work is implemented.
