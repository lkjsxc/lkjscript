# AGENTS.md

## Scope

This file applies to the entire `lkjscript` repository. It governs coding, testing, measurement,
documentation, commits, publication, and final reports. Use English for code, APIs, diagnostics,
tests, documentation, commit messages, and engineering reports unless an active task explicitly
requires another language for a user-facing artifact.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free,
high-performance language and implementation. The goal is not a conventional text-first language
with an AI wrapper. The goal is a deterministic semantic programming substrate that agents can
inspect, construct, edit, validate, compile, execute, and review.

- Agents should discover supported behavior without reading the whole repository.
- Agents should create and edit source-free incomplete programs through typed semantic operations.
- Agents should query exact types, effects, ownership facts, identities, references, calls,
  blockers, and legal next actions.
- Checking must be deterministic and effect-free; running must be intentional.
- Failures must be compact, structured, deterministic, and revision-labelled.
- Local deterministic machinery decides validity. Model inference never belongs in a correctness
  boundary.
- Do not optimize the design around one model, provider, tokenizer, benchmark, prompt style, or
  current API price.

## Priority order

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. Scale-safe representations and algorithms.
5. One syntax-independent mutable semantic authority.
6. One complete source-free edit, check, compile, and run route.
7. A small deterministic local agent workflow.
8. Representative evidence before performance machinery.
9. One complete generic production execution path.
10. Optional specialization that never compromises correctness.
11. Broader products only after their prerequisites are real.
12. Persistence, collaboration, daemonization, and distribution only after measured demand.

## Authority and truth

Classify each claim before deciding which artifact owns it.

- The active task owns its objective and explicit acceptance criteria.
- This file owns repository-wide engineering procedure.
- `docs/spec/` owns intended external semantics and target contracts.
- Executable code, tests, command definitions, schemas, and manifests own checkout behavior.
- `docs/status.md` summarizes current behavior and known gaps.
- `docs/architecture.md` explains current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns measurement method, workloads, compact evidence, and reversal
  conditions.
- `docs/roadmap.md` owns ordering and intent only.
- Sparse accepted files under `docs/decisions/` own durable rationale when one exists.
- Git history owns superseded implementation and prose.

A target specification may lead implementation. That difference is an implementation gap, not
permission for silent contradiction. When claims conflict, identify the dimension, inspect its
owner, inspect executable evidence for current behavior, and update or delete stale material in
the same change.

- Use `Current`, `Target`, `Hypothesis`, `Measured`, `Historical`, `Unknown`, and `Blocked` when
  unlabelled prose would mislead.
- Do not create a second authority system from digests, ledgers, fact registries, closure graphs,
  global revisions, generated inventories, or copied tables.
- Task prompts, transcripts, scratch plans, raw reasoning, handoff capsules, and temporary reports
  are transport, not durable authority.

## Autonomous judgment

- Choose internal designs from the checkout, accepted specifications, focused experiments,
  measurements, and reversible assumptions.
- Ask the user only when a genuinely external requirement is missing and no safe assumption can
  complete the objective.
- Incompatible semantic, syntax, Rust API, command, package, cache, module, and crate changes are
  permitted.
- Obsolete features, tests, adapters, fixtures, exports, dependencies, and prose may be deleted.
- Broad authority is permission to make the right change, not an instruction to maximize scope.
- Preserve unrelated work, credentials, host state, external data, and remote history.

## Backward compatibility

Backward compatibility is not a project objective unless the active task identifies a currently
consumed external boundary that must remain compatible.

- Do not retain old syntax, bytes, APIs, commands, module paths, fixtures, aliases, adapters,
  migrations, or feature flags merely because they exist.
- When a better design requires a cutover, update the owning specification, every active producer
  and consumer, current tests and examples, then delete the displaced route.
- Do not create permanent `legacy`, `v2`, `next`, `new`, or edition-based parallel architectures.
- The `.lkjscript` extension is fixed. Other notation, schemas, bytes, commands, package models,
  representations, and persistence choices remain replaceable unless accepted semantics fix them.

## Multi-turn engineering

Work as a sequence of coherent dependency-closed verticals. One turn should not attempt the whole
roadmap.

1. Inspect branch, worktree, upstream, and recent history.
2. Read the relevant authority documents.
3. Map producers, consumers, mutable authority, derived facts, ownership, trust boundaries, and
   failures.
4. State a falsifiable hypothesis, completion criteria, reversal condition, and stop condition in
   temporary working state.
5. Implement the smallest root-cause correction that creates a complete product result.
6. Delete displaced paths and update owning documentation.
7. Run focused checks during iteration and complete verification at the final relevant state.
8. Commit cohesive changes when permitted and publish only when requested.
9. Report remaining risk and the next highest-leverage problem.
10. Stop before starting that next problem merely to appear ambitious.

- Do not leave a half-cutover, two mutable authorities, disabled checks, stale current prose, or
  hidden executable fallback.
- Do not turn incidental findings into unrelated rewrites.

## Evidence-first work selection

- Start from a demonstrated defect, accepted specification gap, current roadmap item, explicit
  product requirement, measured bottleneck, or blocking safety boundary.
- Characterize current behavior with the smallest useful test, counter, or measurement before
  choosing a mechanism.
- Fix the dependency-closed root cause rather than the most visible symptom.
- Prefer semantic simplification and deletion before new machinery.
- A focused failing test, deterministic work count, or representative measurement is evidence.
- A vague concern that a system may become large is not evidence.

## Anti-overengineering gate

Before adding an abstraction, identify the present problem, producer, consumer, owner, lifetime,
invalidation rule, failure behavior, benefit, why local code is insufficient, and the deletion
condition. Keep this in temporary working state unless the decision is durably non-obvious.

- An abstraction is justified when it removes meaningful duplication, makes invalid state
  unrepresentable, isolates a real boundary, exposes a useful API, enables a measured property, or
  materially simplifies reasoning.
- The mechanism must be smaller than the problem and must not duplicate authority.

Use this escalation order:

1. Delete unused work.
2. Simplify semantics.
3. Simplify representation.
4. Reuse an existing invariant or canonical validator.
5. Add a small local derived fact.
6. Improve traversal or layout.
7. Make an invalid state unrepresentable.
8. Add caching only after measured repeated work.
9. Add parallelism only for measured separable work.
10. Add specialization behind a complete generic route.

- Prefer explicit local code for one current use; extract a framework only after multiple current
  consumers prove shared semantics.
- Do not add speculative daemons, services, persistence, journals, databases, CRDTs, schedulers,
  registries, plugin systems, rewrite DSLs, incremental frameworks, cache frameworks, proof
  ecosystems, protocols, broad target matrices, deoptimization, PGO, platform products, or
  self-hosting scaffolding.
- Such machinery requires a demonstrated current boundary, end-to-end consumer, measured need,
  explicit ownership, acceptance criteria, failure behavior, and reversal condition.
- Do not create a universal graph engine for one traversal, a rewrite framework for a few remaps,
  an event system for one synchronous result, or a trait hierarchy to share two short functions.

## One active architecture

- Maintain one active language definition, semantic authority, identity model, incomplete-state
  model, compiler input path, ownership model, generic production route, package model, and
  documentation authority model.
- A small independent evaluator may remain as a test oracle; it is not automatically a second
  production engine.
- Crate and module names have no authority by themselves. Preserve, merge, split, rename, or
  delete according to cohesion, ownership, trust, unsafe boundaries, current consumers, coupling,
  and measured compile isolation.
- When architecture causes a defect, replace it rather than surrounding it with adapters,
  registries, synchronization bookkeeping, or migration scaffolding.

## Semantic authority

- One syntax-independent semantic state owns mutable program meaning.
- Semantic state must be able to exist without source text, formatting, paths, spans, parser
  nodes, source hashes, compiler-dense indexes, or rendered diagnostics.
- Source, files, comments, formatting, spans, and hashes may be attachments, provenance, cache
  keys, or interoperability views; they are not semantic authority.
- Do not use dummy files, placeholder paths, fabricated hashes, synthetic declarations, fake entry
  points, reserved placeholder identities, or hidden executable bodies to satisfy invariants.
- Every derived representation needs a current producer, consumer, lifetime, invalidation rule,
  and deletion condition.
- Dense IDs, vector positions, slots, offsets, and layout indexes remain private.
- Compilation consumes one complete semantic snapshot directly; do not render and reparse source
  internally.
- Do not serialize and deserialize an in-process typed value merely to manufacture another
  authority token.

## Incomplete semantic state

Incomplete state is valid editing state and never executable state.

- Represent missing, unresolved, ambiguous, conflicting, or recovered meaning explicitly and one
  concrete lifecycle at a time.
- For each state define known and unknown facts, identity, owner, expected type, effects, scope,
  diagnostic, blocker, legal actions, query view, projection, diff, deletion, replacement,
  resolution, failure atomicity, and compile rejection.
- Preserve every sound fact and mark unknown facts explicitly.
- Never lower incomplete state to `unit`, zero, `false`, an empty value, a trap, or a guessed
  declaration.
- Never retain an executable fallback behind an incomplete node.
- Reject incomplete snapshots before ownership planning, memory planning, SSA, bytecode, native
  lowering, executable installation, or execution.
- Do not build a universal incomplete-state framework before distinct current states prove shared
  semantics.

## Identity and revisions

- Use opaque logical identity only where meaning must survive rename, movement, or private
  compaction.
- Define namespace, allocator, uniqueness lifetime, generation validation, ownership, continuity,
  removal, tombstone, and persistence lifetime for each public identity.
- Names, paths, spans, formatting, source order, and hashes are not universal mutable identity.
- Surviving identities remain stable across private relocation; old immutable snapshots remain
  valid.
- Foreign namespace, stale revision, stale generation, wrong kind, and wrong owner fail before
  publication.
- A failed operation must not consume identities or change future allocation order.
- Do not expose compiler-dense IDs or imply cross-process identity without a real lifetime.

## Transactions

- Semantic edits are typed operations over identities and publish one coherent revision or
  nothing.
- Validate revision, namespace, generation, kind, owner, operation shape, preconditions, draft
  connectivity, acyclicity, child uniqueness, visibility, and final dependency closure before
  publication.
- Failure must preserve the published snapshot, allocator, diagnostics, blockers, continuations,
  and derived state.
- When batching promises order independence, validate the intended final semantic graph rather
  than edit-list order.
- Containment-owned facts may cascade with their owner; independent dependents must not be
  silently deleted.
- Transaction-local handles may exist before stable entities, but must be typed, scoped,
  validated, non-persistent, and impossible to confuse with stable identity.
- Use one structured public model per concept unless input and output genuinely have different
  semantics.

## Public semantic APIs

- Expose semantic meaning, not parser nodes, private addresses, dense indexes, debug formatting,
  or display strings as the only data.
- Machine-facing results must be deterministic, revision-labelled, stably ordered,
  completeness-explicit, and bounded or paginated.
- Never silently truncate.
- Return compact headers and stable identities before expensive expansion.
- Expose legal next actions where deterministic machinery already knows them.
- Do not claim a candidate is legal when canonical ownership or effect validation has not run;
  label provisional candidates accurately.
- Public recursive values must be stack-safe to construct, clone, compare, hash when required,
  project, validate, convert, and destroy.

## Types and generics

- Generic declarations, substitutions, bounds, instantiations, and witnesses are semantic facts,
  not parser decoration.
- Source import and source-free editing converge on one exact instantiation and trait-selection
  path.
- Inference is an authoring convenience; exact substitutions and compiler-derived witnesses are
  the semantic result.
- Keep type identity stable and syntax-independent.
- Use checked conversion at compact or host representation boundaries.
- Do not add a general inference or higher-rank framework before a current language requirement
  needs it.
- Do not impose arbitrary type-depth quotas.

## Ownership and memory

- Ordinary execution is collector-free and non-tracing.
- Do not add tracing GC, hidden language-visible reference counting, raw-pointer language
  surfaces, retain/release, general `free`, or parallel GC and non-GC modes.
- Preserve exact move, borrow, loan, cleanup, early-exit, trap, cancellation, resource-failure,
  and host-resource laws.
- Prevent leaks, double release, and stack-overflow destruction.
- Unsafe code belongs in a narrow named mechanism with explicit invariants, a safe-caller
  contract, focused malformed-input tests, and suitable Miri, sanitizer, fuzz, or property
  coverage.

## Compilation and execution

- Maintain one complete generic production execution route.
- Optional specialization may decline only before program effects and must leave the unchanged
  generic route available.
- Once specialized entry begins, its result or failure is final; never rerun effects through
  fallback.
- Checking must not execute program effects.
- Do not construct execution state in an effect-free check path when compilation can finish
  without it.
- Validate fail-closed at real untrusted boundaries.
- Inside one trusted typed synchronous pipeline, do not repeatedly serialize, hash, reconstruct,
  or independently revalidate values without a consumer boundary.

## Scale and resource policy

- Language validity follows semantic laws, not project-selected quotas.
- Do not reject trusted valid programs because of arbitrary byte, token, nesting, declaration,
  field, variant, parameter, local, function, file, module, IR-node, identity, value, diagnostic,
  handle, or work counts.
- Do not disguise a limit by raising, widening, moving, renaming, or profiling it.
- Use checked arithmetic and checked narrowing.
- Use iterative traversal or justified heap-backed work stacks for user-controlled depth.
- An untrusted product may impose explicit coarse input, memory, output, time, cancellation,
  concurrency, or fuel policy.
- Resource exhaustion is a typed host result, not a semantic error.
- Do not design detailed untrusted policy before an actual untrusted product exists.

## AI-facing local workflow

The smallest complete agent workflow is discover, edit, inspect, check, review, run, and verify.

- Prefer executable examples, concise authoring documentation, effect-free compile-only commands,
  structured diagnostics, one-shot operations, deterministic projections, and direct in-process
  semantic APIs.
- Add a daemon only after measurements show process startup or repeated import dominates a real
  workflow.
- Agent use does not imply a database, journal, session broker, scheduler, network protocol, CRDT,
  persistent semantic store, or broad agent framework.
- Command names, arguments, exit behavior, stdout, and stderr must be deterministic and tested.
- Successful high-frequency validation is quiet by default.
- A one-shot command must not pretend identities survive across invocations without a real
  lifetime.

## Attention and API-cost discipline

- Model context, tool output, developer attention, wall time, and API spend are engineering
  resources.
- Search before opening large files; read focused ranges and diffs before full material.
- Run the smallest command that can falsify the current hypothesis.
- Prefer a focused test, then a crate test, then the workspace suite, then retained container
  verification.
- Do not repeat an identical successful command when no relevant input changed.
- Do not dump unchanged files, repository-wide diffs, generated HIR, SSA, bytecode, machine code,
  massive JSON, or complete projections without need.
- Never hide a non-zero status, diagnostic, sanitizer finding, fuzz failure, malformed output, or
  environment error.
- Use native quiet flags for successful commands; otherwise capture full logs outside Git and
  expose the relevant section and log path on failure.
- Do not add a runner, registry, broker, logging framework, cache, or service merely to silence
  commands.
- When efficiency is an objective, measure command count, round trips, stdout and stderr bytes,
  lines, duplicate diagnostics, wall time, repeated work, and context needed for the next
  decision.
- Do not infer provider-token or billing savings from byte counts alone.

## Performance

- Profile before optimizing and measure the selected product path.
- State hypothesis, equivalent semantics, workload, environment, build profile, cache state,
  sample protocol, selection criterion, reversal condition, and stop condition.
- Use wall and phase time, startup, throughput, edit/query/compile/run latency, memory,
  allocations, copied/parsed/serialized/rendered bytes, output, command count, work counts, code
  size, and scale shape as appropriate.
- Prefer deterministic work counters when they answer the question better than noisy timing.
- Generated scale tests establish correctness and complexity shape, not application performance.
- Keep raw samples outside Git and commit only compact reproducible evidence.
- Do not turn developer-machine noise into a correctness gate.
- Keep an optimization only when end-to-end benefit justifies compile time, memory, code size,
  complexity, tests, and maintenance.
- Full recomputation may remain correct until representative edits justify incrementality.
- Remove temporary instrumentation without a continuing consumer.
- Do not claim improvement without equivalent evidence.

## Repository structure and dependencies

- Organize by coherent responsibility, not arbitrary line counts, directory width, depth, or
  symmetry.
- A crate boundary needs a real trust or unsafe boundary, independently useful library, supported
  target, measured compile isolation, or low-coupling subsystem.
- Merge crates that mainly exchange internal descriptors, re-exports, or adapters.
- Remove numbered shards, include-only facades, one-child ladders, artificial tiny modules,
  redundant models, and conversions without a boundary.
- Split a large module only when the split establishes ownership and reduces change coupling.
- Use mature dependencies when they remove substantial machinery or risk; keep local code when
  smaller, clearer, safer, easier to audit, or measurably better.
- Do not add benchmark, logging, serialization, or allocation frameworks when a small
  current-purpose harness suffices.

## Tests

- Tests protect intended semantics and public invariants, not accidental topology.
- Cover relevant type, generic, trait, effect, capability, ownership, control, cleanup,
  completeness, identity, namespace, generation, revision, deletion, ordering, malformed-input,
  stale/foreign/wrong-kind/wrong-owner, visibility, exactly-once effects, cancellation, resource
  failure, deep operations, checked boundaries, machine output, effect-free checking, integration,
  and failure atomicity.
- Add a focused regression test for each root cause.
- Use generated fixtures for scale; keep fast defaults separate from ignored locked-release stress
  while exercising the same algorithm at smaller scale.
- Use differential, property, model, or test-only reference implementations when an independent
  oracle is cheap.
- Delete tests that preserve provisional syntax, old bytes, obsolete APIs, deleted machinery,
  arbitrary limits, private topology, or accidental behavior.
- Never weaken a test merely to make a redesign pass.
- Convergence compares semantic outcomes, not only text.
- Failure-atomicity verifies prior snapshot and allocator state.
- Stack-safety covers construction, transformation, and destruction on a small native stack.
- Machine-output tests decode as a consumer would; do not validate JSON only by substring.
- Quiet-success tests assert both streams empty; no-effects checks use observable would-be
  effects.

## Documentation

- `README.md` owns product introduction and first successful use.
- `docs/spec/` owns intended semantics and target contracts.
- `docs/status.md` owns current implementation and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns method, workloads, compact evidence, and reversal conditions.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later`.
- `docs/decisions/` contains sparse durable decisions.
- Update the owning document and delete stale text in the same change.
- Do not add digests, global revisions, fact shards, copied tables, transcripts, handoffs, prompt
  archives, completion capsules, or duplicate roadmaps.
- Write a decision only when a choice is durable, non-obvious, expensive to rediscover, and has a
  meaningful reversal condition.
- Do not describe target as current, hypothesis as measurement, private relocation as public
  movement, planned systems as supported, or a developer observation as a guarantee.
- Examples must use active APIs and should be mechanically checked where practical.

## Git and publication

- Inspect worktree and branch state before editing and preserve unrelated tracked and untracked
  work.
- Do not use destructive reset, checkout, clean, history rewrite, or force push against work you
  did not create.
- Commit only cohesive repository changes; exclude prompts, raw logs, raw samples, scratch plans,
  credentials, generated temporary files, and unrelated work.
- Use a commit message naming the semantic or architectural result.
- Push only when explicitly requested; never force push for convenience.
- After a requested push, verify the local branch, tracking branch, and pushed commit.
- If publication fails, preserve the verified local commit and report the exact failure.

## Verification

During iteration, run the smallest focused command that can disprove the change. At the final
relevant state, run at least:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Run retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

- Run additional relevant release stress, differential, property, small-stack, deep-input,
  malformed-boundary, cancellation, allocation-failure, Miri, sanitizer, fuzz, benchmark,
  documentation, and machine-output checks.
- Run the full suite after the final relevant edit, not repeatedly after unchanged inputs.
- If the environment blocks a command, report the exact command, failure category, relevant
  output, whether the change caused it, successful remaining evidence, and unverified risk.
- Never claim a command passed when it did not complete successfully.

## Final report

- Report the completed objective and demonstrated root cause.
- Report the principal design and replaced or deleted paths.
- Report focused tests, convergence evidence, and measurements when relevant.
- Report output-volume evidence only when measured.
- Report exact verification commands and outcomes.
- Report environment-limited checks and remaining risk.
- Report documentation, commit, and publication state.
- Name the next highest-leverage problem and why work stopped before beginning it.
- Keep the report factual and compact; do not reproduce the prompt or paste complete successful
  logs.
- Do not claim future work is implemented.
