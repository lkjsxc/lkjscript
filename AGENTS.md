# AGENTS.md

## Scope

This file applies to the entire `lkjscript` repository.
It governs inspection, design, implementation, testing, measurement, documentation, commits, subagent use, and final reports.
Use English for repository artifacts unless an active task explicitly requires another language for a user-facing artifact.
A subtree policy may narrow local procedure, but it must not create a second language definition, semantic authority, ownership model, compiler route, runtime route, or documentation authority.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free, high-performance programming system.
The project is not an AI wrapper around an ordinary text-first language.
Agents should be able to discover, construct, edit, inspect, leave incomplete, check, compile, run, review, and verify programs through deterministic semantic machinery.
A model may propose operations; deterministic local code decides correctness.
Model inference must never participate in parsing, typing, ownership, validation, optimization correctness, persistence correctness, runtime correctness, or another trusted boundary.
Do not optimize the language around one model, provider, tokenizer, context window, benchmark, prompt style, orchestration product, or current API price.

- Semantic meaning can exist without source text.
- Incomplete state is explicit and non-executable.
- Compilation consumes one complete semantic snapshot directly.
- One complete generic production route remains authoritative.
- Optional specialization cannot weaken correctness or duplicate effects.
- Review uses stable structured facts, deterministic projections, and semantic diffs.
- Claims require executable or reproducible evidence.

## Priority order

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. Scale-safe representations and algorithms.
5. One syntax-independent mutable semantic authority.
6. One complete source-free authoring, inspection, check, compile, and run route.
7. A small deterministic local workflow for agents and ordinary tools.
8. Representative evidence before caches, parallelism, incrementality, or services.
9. One complete generic production execution route.
10. Optional specialization behind that route.
11. Broader products only after current prerequisites exist.
12. Persistence, collaboration, daemonization, scheduling, and distribution only after measured demand.

A later platform idea is not permission to skip a prerequisite. A benchmark may expose a defect; it does not define the language.

## Authority

- The active task owns its objective and explicit acceptance criteria.
- This file owns repository-wide engineering procedure.
- Accepted files under `docs/spec/` own intended external semantics and target contracts.
- Executable code, tests, command definitions, schemas, manifests, and locks own checkout behavior.
- `README.md` owns the product introduction and first successful use.
- `docs/status.md` summarizes current implementation and known gaps.
- `docs/architecture.md` explains responsibilities, data flow, ownership, and trust boundaries.
- `docs/performance.md` owns measurement method, retained workloads, compact evidence, decisions, and reversal conditions.
- `docs/roadmap.md` owns ordering and intent only.
- Sparse accepted files under `docs/decisions/` own durable rationale when justified.
- Git history owns superseded implementation and prose.

A target specification may lead implementation. That difference is an implementation gap, not permission for silent contradiction.

1. Classify the conflicting claim.
2. Inspect the artifact that owns that dimension.
3. Inspect executable evidence for current behavior.
4. Update or delete stale material in the same coherent change.
5. Leave one intelligible authority.

Use labels such as `Current`, `Target`, `Hypothesis`, `Measured`, `Historical`, `Unknown`, and `Blocked` when unlabeled prose would mislead.

Do not create authority from prompt archives, transcripts, scratch plans, handoffs, fact ledgers, duplicated status tables, global revisions, closure registries, plan trees, or unconsumed semantic digests. Task prompts and subagent packets are transport, not durable repository authority.

## Autonomy and compatibility

Use autonomous engineering judgment from the actual checkout, accepted specifications, focused tests, measurements, current consumers, product requirements, and real failure boundaries.
Ask the user only when a genuinely external product requirement is missing and no safe explicit assumption can complete the task.
Do not ask the user to choose among internal alternatives the repository can decide.
Broad authority permits the right change; it does not require maximum scope.
Preserve unrelated tracked work, untracked work, credentials, host state, external data, and remote history.

Backward compatibility is not a project objective unless the active task names a currently consumed external boundary that must remain compatible.

- Language semantics, syntax, source encoding, Rust APIs, commands, packages, locks, modules, crates, compiler forms, runtime forms, fixtures, examples, tests, and documentation may change incompatibly.
- When cutting over, update the owning specification, every active producer and consumer, fixtures, examples, tests, and documentation.
- Delete the displaced implementation, exports, dependencies, adapters, migrations, aliases, and feature flags.
- Leave exactly one active route.
- Do not create permanent `legacy`, `v2`, `next`, `new`, edition, compatibility, or dual-write architectures.
- The `.lkjscript` extension is fixed; other formats and representations remain replaceable unless accepted semantics fix them.

## Multi-turn operating method

Work in coherent, dependency-closed verticals. One turn should not attempt the whole roadmap.

### Start

1. Inspect branch, worktree, upstream, and recent history.
2. Read the relevant authority documents.
3. Search owning symbols before opening large files.
4. Map producers, consumers, mutable authority, derived facts, ownership, trust boundaries, and failure paths.
5. Select one demonstrated defect, accepted gap, measured bottleneck, explicit product result, or current maintenance burden.
6. Record a falsifiable hypothesis, completion criteria, reversal condition, stop condition, and non-goals in temporary state.

### Execute

1. Add the smallest focused evidence that can fail.
2. Fix the dependency-closed root cause.
3. Reuse canonical validation and invariants.
4. Delete displaced paths.
5. Run the smallest command that can falsify the hypothesis.
6. Review the diff as both consumer and adversary.
7. Update the owning documentation.
8. Stop when the selected product result is complete.

### Finish

1. Run the final relevant verification boundary once final relevant inputs are stable.
2. Commit cohesive changes when permitted.
3. Publish only when explicitly requested.
4. Report evidence, residual risk, commit state, and publication state.
5. Name the next highest-leverage problem without beginning it.

- Do not leave two mutable authorities, two active compiler routes, a half-cutover, disabled checks, hidden executable fallback, unfinished required migration, stale current prose, unowned cache, temporary compatibility layer, or unreviewed subagent branch.
- Do not turn incidental findings into unrelated rewrites.
- If scope expands materially beyond the opening hypothesis, reassess before continuing.

## Evidence and anti-overengineering

Begin from a failing test, accepted specification gap, roadmap item, explicit requirement, measured bottleneck, safety boundary, or concrete maintenance burden with current consumers. A hypothetical future consumer is not evidence. A design diagram is not an implementation result. One noisy timing is orientation, not a claim.

Before adding a nontrivial abstraction, identify the present problem, producer, consumer, owner, lifetime, invalidation, failure behavior, benefit, maintenance cost, reversal condition, deletion condition, and why direct local code is insufficient.

- A justified abstraction removes meaningful duplication, makes invalid state unrepresentable, isolates a real trust or unsafe boundary, exposes an independently useful API, enables a measured property, materially simplifies reasoning, or replaces a larger fragile mechanism.
- The mechanism must be smaller than the problem and must not duplicate authority.

1. Delete unused work.
2. Simplify semantics.
3. Simplify representation.
4. Reuse an invariant.
5. Reuse the canonical validator.
6. Add a small local derived fact.
7. Improve traversal or layout.
8. Make invalid state unrepresentable.
9. Cache only after measured repeated work.
10. Parallelize only after measured separable work.
11. Add incrementality only after measured recomputation dominates.
12. Specialize only behind the complete generic route.
13. Add a process boundary only after measured process cost or isolation demand.

Prefer explicit local code for one current use. Extract a framework only after multiple current consumers prove shared semantics.

Do not add speculative daemons, services, sessions, journals, databases, CRDTs, schedulers, registries, plugin systems, rewrite DSLs, cache or incremental frameworks, proof ecosystems, network protocols, target matrices, deoptimization, PGO, self-hosting scaffolding, orchestration platforms, or persistent agent state.

Do not build a universal graph engine for one traversal, a generic recovery framework for one incomplete state, an event system for one synchronous result, a trait hierarchy for two short functions, or a scheduler for a few checks.

## One active architecture

- Maintain one language definition, semantic authority, identity model, incomplete-state model, compiler input route, ownership model, generic production execution route, package model, verification contract, and documentation authority model.
- A small independent evaluator may remain a test oracle; it is not automatically a second production engine.
- Crate and module names have no authority by themselves.
- Preserve, merge, split, rename, or delete components according to cohesion, ownership, trust and unsafe boundaries, independently useful APIs, current consumers, coupling, compile isolation, and measured cost.
- When architecture causes a defect, replace it rather than surrounding it with adapters, registries, synchronization, dual writes, or migration scaffolding.

## Semantic authority

One syntax-independent semantic state owns mutable program meaning. It must exist without source text, formatting, paths, spans, parser nodes, hashes, compiler-dense indexes, rendered diagnostics, or a running service.

Source, comments, files, formatting, spans, and hashes may be importer inputs, provenance, presentation, cache keys, review views, or interoperability forms. They are not semantic authority.

- Do not use dummy files, placeholder paths, fabricated hashes, synthetic declarations, fake entry points, reserved placeholder identities, hidden executable bodies, or render-and-reparse cycles.
- Every derived representation needs a current producer, consumer, owner, lifetime, invalidation rule, and deletion condition.
- Dense IDs, vector positions, slots, offsets, registers, layout indexes, and traversal coordinates remain private.
- Compilation consumes one complete semantic snapshot directly.
- Do not serialize and deserialize an in-process typed value merely to manufacture another authority token.

## Incomplete state

Incomplete state is valid editing state and never executable state. Introduce one concrete lifecycle at a time.

- Define known and unknown facts, identity, owner, context, expected and actual type where known, effects, scope, diagnostics, blockers, legal actions, queries, projection, diff, replacement, deletion, resolution, stale behavior, failure atomicity, old-snapshot behavior, compilation rejection, and independent downstream defense.
- Preserve sound facts and mark unknown facts explicitly.
- Never lower incompleteness to `unit`, zero, `false`, an empty value, a trap, a guessed declaration, the first candidate, an arbitrary candidate, or hidden fallback.
- Never retain the displaced executable expression behind an incomplete node.
- Reject incomplete snapshots before ownership planning, memory planning, SSA, bytecode, native lowering, executable installation, VM entry, or host effects.
- Do not build a universal incomplete-state framework before multiple current states prove shared lifecycle.
- A finite ambiguity is not automatically stored state; first prove deterministic candidate queries and explicit resolution are insufficient.

## Identity and revisions

Use opaque logical identity where meaning must survive rename, movement, replacement boundaries, or private compaction.

- Define namespace, allocator, kind, owner, uniqueness lifetime, generation checks, revision preconditions, continuity, removal, tombstone, slot reuse, and persistence lifetime only if persistence exists.
- Names, paths, spans, formatting, source order, and hashes are not universal mutable identity.
- Surviving identities remain stable across private relocation; removed identities tombstone; old immutable snapshots remain valid.
- Reject foreign namespace, stale revision, stale generation, wrong kind, and wrong owner before publication.
- A failed operation must not publish, consume identities, mutate allocator state, alter future allocation order, diagnostics, blockers, continuations, or derived state.
- Do not expose compiler-dense IDs or imply cross-process identity without a real cross-process lifetime and validation boundary.

## Transactions and public semantic APIs

Semantic edits are typed operations over identities. One successful transaction publishes one coherent revision; one failed transaction publishes nothing.

- Validate namespace, revision, generation, kind, owner, operation shape, disjointness, preconditions, draft connectivity, acyclicity, single-parent structure, child uniqueness, visibility, final dependency closure, types, effects, ownership, match usefulness and exhaustiveness, cleanup, and allocation failure as applicable.
- When batching promises order independence, validate the intended final graph rather than edit-list order.
- Containment-owned facts may cascade; independent dependents must not be silently deleted.
- Transaction-local handles must be typed, scoped, validated, non-persistent, and impossible to confuse with stable identity.
- Use one structured public model per concept unless input and output genuinely differ.
- Do not add transaction machinery solely to preserve obsolete routes.

Expose semantic meaning, not parser nodes, private addresses, dense indexes, debug formatting, or display strings as the only representation.

- Machine-facing results are deterministic, revision-labelled, stably ordered, completeness-explicit, bounded or paginated, structurally typed, and honest about provisional facts.
- Never silently truncate.
- Return compact headers and stable identities before expensive expansion.
- Expose legal next actions when deterministic machinery already knows them.
- Do not claim canonical legality before ownership, effect, scope, or capability validation runs.
- Public recursive values must be stack-safe to construct, clone, compare, hash when required, project, validate, convert, rewrite, and destroy.
- A real external consumer must earn a wire schema, daemon, or persistent identity lifetime.

## Types, ownership, and execution

Generic declarations, substitutions, bounds, instantiations, and witnesses are semantic facts. Source inference and source-free edits converge on one exact resolver. Keep type identity stable and syntax-independent; use checked conversion at compact and host boundaries. Do not add general inference, higher-rank, or recovery frameworks before a current requirement needs them. Do not impose arbitrary type-depth quotas.

Ordinary execution is collector-free and non-tracing.

- Do not add tracing GC, hidden language-visible reference counting, raw-pointer language surfaces, retain/release APIs, general `free`, parallel GC/non-GC modes, or ownership escape hatches.
- Preserve exact move, shared and mutable borrow, loan, cleanup, early return, trap, cancellation, allocation, resource, host-resource, and teardown laws.
- Prevent leaks, double release, use after move or owner end, stale loans, duplicated effects, and stack-overflow destruction.
- Unsafe code belongs in a narrow named mechanism with explicit invariants, safe-caller contract, focused malformed-input tests, and suitable Miri, sanitizer, fuzz, property, or differential evidence.

Maintain one complete generic production execution route. Optional specialization may decline only before effects; the unchanged generic route remains available. Once specialized entry begins, its result or failure is final. Never rerun effects through fallback. Checking must not execute program effects or construct execution state unnecessarily.

Validate fail-closed at real untrusted boundaries. Inside one trusted typed synchronous pipeline, do not repeatedly serialize, hash, reconstruct, clone, render, parse, or independently revalidate the same value without a consumer boundary or measured need.

## Scale and resource policy

Language validity follows semantic laws, not project-selected quotas.

- Do not reject trusted valid programs because of arbitrary byte, token, nesting, declaration, field, variant, parameter, local, function, file, module, IR-node, identity, value, diagnostic, handle, or work counts.
- Do not disguise a limit by raising, widening, moving, renaming, or profiling it.
- Use checked arithmetic, checked narrowing, and iterative traversal or a justified heap-backed work stack for user-controlled depth.
- An untrusted product may impose explicit coarse input, memory, output, time, cancellation, concurrency, handle, fuel, or artifact-size policy.
- Resource exhaustion is a typed host result, not a semantic error.
- Do not design detailed untrusted policy before an actual untrusted product exists.

## Agent workflow and attention budget

The smallest complete workflow is discover, inspect, edit, query, check, review, run intentionally, and verify.

- Prefer executable examples, concise authoring docs, focused symbol search, effect-free checks, structured diagnostics, one-shot semantic operations, deterministic projections and diffs, direct in-process APIs, and quiet success.
- Command names, arguments, exits, stdout, and stderr must be deterministic and tested.
- A one-shot command must not pretend identities survive across processes without a real lifetime.
- Agent use does not imply a database, journal, session broker, scheduler, protocol, CRDT, persistent store, remote executor, or broad agent framework.
- Add a daemon only when representative measurements show startup, repeated import, or isolation demand dominates.

Model context, tool output, developer attention, wall time, CPU, memory, disk, CI minutes, and API spend are engineering resources.

1. Search before opening large files.
2. Read focused ranges and diffs before full material.
3. Keep one compact orientation packet and reuse it.
4. Run one focused test before a target, crate, workspace, or container boundary.
5. Do not repeat an identical successful command after unchanged inputs.
6. Do not dump unchanged files, repository-wide diffs, generated IR, bytecode, machine code, massive JSON, complete projections, successful logs, or duplicate reports without a consumer.
7. Use native quiet flags; capture full failure logs outside Git and surface relevant excerpts.
8. Never hide nonzero status, diagnostics, sanitizer or fuzz findings, malformed output, or environment errors.
9. After the same failed approach twice, reassess the hypothesis.
10. Do not add runners, brokers, logging frameworks, caches, services, protocols, or persistent state merely to reduce prompt or log volume.

When efficiency is an objective, measure model round trips, tool calls, commands, output bytes, duplicate diagnostics, wall and CPU time, RSS, repeated compilation/parsing/serialization/validation, cache misses, required decision context, and critical-path duration as applicable. Provider-token or billing claims must be measured, explicitly estimated, or labelled unknown.

## Performance and verification throughput

Profile before optimizing. Measure the selected product path under equivalent semantics.

- Before comparison, state hypothesis, workload, environment, profile, cache state, sample protocol, selection criterion, reversal condition, and stop condition.
- Use metrics that answer the question: wall and phase time, startup, throughput, edit/query/compile/run latency, memory, allocations, copied/parsed/serialized/rendered bytes, output, commands, deterministic work, code and binary size, and scale shape.
- Prefer deterministic work counters when they answer better than noisy timing.
- Generated scale tests establish correctness and complexity, not application performance.
- Keep raw samples outside Git; commit only compact reproducible evidence.
- Do not turn developer-machine noise into a correctness gate.
- Keep an optimization only when end-to-end benefit justifies complexity, memory, code size, dependencies, portability, tests, and maintenance.
- Full recomputation and serial execution remain valid until representative evidence justifies incrementality or parallelism.
- Remove temporary instrumentation without a continuing consumer.

Verification speed is a workflow property; coverage and failure visibility remain correctness properties.

- Before changing topology, measure the critical path, compilation versus execution, duplicate work, shared-resource constraints, local latency versus CI compute, selection criterion, and reversal condition.
- Prefer deleting duplicate work, reusing built binaries, narrowing iteration, correcting cache keys, and isolating post-build smokes before adding machinery.
- Do not assume more jobs are faster.
- Do not add `cargo-nextest`, `sccache`, a custom scheduler, test protocol, or persistent runner without measured net benefit and a maintenance owner.
- Do not run full Clippy, tests, release/LTO builds, Miri, sanitizers, fuzzing, containers, or large stress tests concurrently in one target directory unless measured safe and faster.
- Separate target directories avoid interference but duplicate work; use them only when measured beneficial.

## Repository structure and dependencies

Organize by coherent responsibility, not arbitrary line counts, width, depth, symmetry, or visual uniformity.

- A crate boundary needs a trust or unsafe boundary, independently useful library, supported target boundary, measured compile isolation, low-coupling subsystem, or current product boundary.
- Merge crates that mainly exchange internal descriptors, re-exports, or adapters.
- Remove numbered shards, include-only facades, one-child ladders, artificial tiny modules, redundant models, conversion-only layers without a boundary, and empty placeholders.
- Split a large module only when the split establishes ownership and reduces change coupling.
- Use mature dependencies when they remove substantial machinery or risk; keep local code when it is smaller, clearer, safer, easier to audit, or measurably better.
- Do not add benchmark, logging, serialization, allocation, orchestration, or parallelism frameworks when a small current-purpose mechanism suffices.

## Tests

Tests protect intended semantics and public invariants, not accidental topology.

- Cover types, generics, traits, effects, capabilities, ownership, control flow, cleanup, completeness, identity, namespace, generation, revision, deletion, replacement, movement, ordering, malformed input, stale/foreign/wrong identities, visibility, exactly-once effects, cancellation, resource and allocation failure, deep operations, checked boundaries, machine output, effect-free checking, integration, convergence, failure atomicity, and old snapshots as relevant.
- Add one focused regression for each root cause.
- Use generated fixtures for scale; keep fast defaults separate from ignored locked-release stress while exercising the same algorithm.
- Use differential, property, model, or test-only reference implementations when a clear independent oracle is cheap.
- Delete tests that preserve provisional syntax, old bytes, obsolete APIs, deleted machinery, arbitrary limits, private topology, or accidental behavior.
- Never weaken a test merely to make a redesign pass.
- Convergence compares semantic outcomes, not only text.
- Failure-atomicity tests verify the prior snapshot and allocator state.
- Stack-safety tests cover construction, transformation, and destruction on a small native stack.
- Decode machine output as a consumer would; do not rely only on substrings.
- Quiet-success tests assert both streams are empty; no-effects checks use observable would-be effects.
- Parallel tests must not depend on uncontrolled global state or order.

## Documentation

- `README.md` owns introduction and first use.
- `docs/spec/` owns intended semantics and target contracts.
- `docs/status.md` owns current implementation and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns method, workloads, compact evidence, decisions, and reversal conditions.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later`.
- `docs/decisions/` contains sparse durable decisions.

Update the owning document and delete stale text in the same change. Do not add digests, global revisions, fact shards, copied tables, transcripts, handoffs, prompt archives, completion capsules, duplicate roadmaps, or raw logs.

Write a decision only when a choice is durable, non-obvious, expensive to rediscover, and governed by a meaningful reversal condition. Do not describe target as current, hypothesis as measurement, private relocation as public movement, planned systems as supported, developer observation as guarantee, or one host result as portable. Keep examples active and mechanically checked where practical.

## Lead agent and subagents

One lead owns the objective, architecture, worktree awareness, decomposition, write ownership, integration, final verification, commits, and final report. A subagent recommendation is evidence, not authority.

- Use the smallest useful set of subagents only when independent work reduces the critical path.
- Good tasks are focused read-only mapping, specification comparison, test-gap analysis, measurement, isolated implementation with narrow ownership, and independent invariant or final-diff review.
- Avoid duplicate broad reading, duplicate design attempts, splitting one coupled semantic edit among writers, long generic summaries, outsourced global decisions, and multiple full verification runs.
- Prefer parallel read-only discovery before writing.
- Parallel writers require isolated worktrees, non-overlapping files, no shared generated outputs, cohesive commits, lead review, one-at-a-time integration, and affected tests.
- Never let two agents edit the same file or evolve the same semantic model independently.
- Do not use a shared dirty worktree or create an orchestration service, task database, protocol, or registry.
- Evidence packets should contain the question, conclusion, exact files and symbols, executable evidence, uncertainty, recommendation, and reversal condition; writing packets also name worktree, files, commit, tests, assumptions, hazards, and risk.
- Do not return complete files or successful logs when a compact packet is sufficient.

## Git and publication

- Inspect branch and worktree before editing; preserve unrelated tracked and untracked work.
- Do not destructively reset, checkout, clean, rewrite history, or force push work you did not create.
- Commit cohesive repository changes; do not split merely by file type.
- Exclude task prompts, raw logs and samples, scratch plans, credentials, temporary generated files, subagent packets, and unrelated work.
- Use commit messages that name the semantic, architectural, or measured result.
- Push only when explicitly requested; never force push for convenience.
- After requested publication, verify local branch, tracking branch, pushed commit, and remote result.
- If publication fails, preserve the verified local commit and report the exact failure.

## Verification strategy

Run the smallest focused command during iteration. Escalate only after focused evidence passes. Do not repeat the full boundary after unchanged inputs.

### Tier 0: inspection

Use focused search, dependency and diff inspection, static reasoning, existing measurements, and current test inventory. Inspection does not replace executable evidence when behavior changes.

### Tier 1: focused

Run the smallest relevant unit, integration, generated, compile-only, convergence, machine-output, property, or smoke test.

### Tier 2: affected component

Run the affected crate, binary, feature, or integration target with the production feature shape needed by the change.

### Tier 3: native repository boundary

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Run these against final relevant inputs. Do not repeat them merely because non-compiled documentation changed after code-complete success.

### Tier 4: retained container and product boundary

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run Tier 4 when changes can affect dependencies, Docker inputs, packaging, release compilation, compiler or executable behavior, VM, native, host capabilities, examples, smokes, system libraries, or publication confidence. Documentation-only work need not rebuild it. Omit it for a narrow internal change only with concrete dependency/risk reasoning and native end-to-end evidence; do not omit it merely because it is slow.

Run relevant locked-release stress, differential/property tests, small-stack/deep/malformed tests, cancellation/allocation tests, Miri, sanitizers, fuzzers, benchmarks, documentation, and machine-output checks when justified. If an environment blocks a command, report the command, failure category, relevant output, causality, successful remaining evidence, and residual risk. Never claim a command passed unless it completed successfully.

## Final report

- Completed objective and demonstrated root cause or evidence gate.
- Principal design and replaced or deleted paths.
- Focused tests, convergence evidence, and measurements when relevant.
- API-cost or output claims only when measured, explicitly estimated, or unknown.
- Exact verification commands and outcomes.
- Environment-limited checks and remaining risk.
- Documentation, commit, worktree, and publication state.
- Material subagent use and integration result.
- Next highest-leverage problem and why work stopped before it.

Keep the report factual and compact. Do not reproduce the task prompt, paste complete successful logs, or claim future work is implemented.
