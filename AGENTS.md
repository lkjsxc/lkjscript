# AGENTS.md

## Scope

This file applies to the entire repository.

Write code, comments, diagnostics, tests, documentation, commit messages, and final reports in English unless the active task explicitly requires another language for a user-facing artifact.

The user authorizes autonomous technical judgment, incompatible changes, destructive simplification, specification revision, representation replacement, file and crate reorganization, and deletion of obsolete work. Backward compatibility is not a goal unless the active task explicitly makes it one.

Historical requests are context, not permanent architecture requirements. Re-evaluate them against the active task, accepted specifications, current checkout, executable evidence, and long-term product quality. Do not preserve or build a mechanism merely because it was previously discussed, requested, or implemented.

Do not ask the user to select among technical alternatives when repository evidence, tests, a focused experiment, profiling, or a reversible assumption can decide. Ask only when a genuinely external requirement is missing and no safe assumption can unblock the work.

Do not destroy unrelated local work, external data, credentials, host state, or remote history. Authorization to redesign this repository is not authorization to erase unrelated state.

The only permanently fixed property of the program file format is the `.lkjscript` extension. Current bytes, textuality, grammar, encoding, source layout, package format, CLI, bytecode, cache format, schemas, and internal Rust APIs are provisional unless an accepted specification explicitly says otherwise.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance programming language and implementation.

AI-primary means that an agent can discover, construct, inspect, transform, validate, compile, execute, test, and review programs through deterministic, precise, compact, composable interfaces.

AI-primary does not mean:

- model inference inside the compiler, runtime, validator, optimizer, storage layer, or correctness boundary;
- optimizing semantics for one model or tokenizer;
- making representation opaque without deterministic inspection and review;
- multiplying schemas, descriptors, protocols, or services;
- preserving source text as the only authoring authority; or
- replacing local deterministic checks with probabilistic judgment.

The product direction is:

- one syntax-independent semantic program authority;
- first-class source-free construction and editing;
- stable logical identities independent of names, paths, spans, formatting, hashes, and dense compiler indexes;
- explicit incomplete states instead of fabricated executable placeholders;
- typed, atomic, revision-checked semantic edits;
- direct access to types, scope, effects, capabilities, ownership, dependencies, diagnostics, and legal next actions;
- deterministic ordering, pagination, projections, and semantic diffs;
- direct compilation from complete semantic state without rendering and reparsing;
- one complete generic production execution route;
- optional specialization that may decline only before effects;
- low agent round-trip count and actionable failures;
- human-reviewable projections and reproducible builds; and
- strong end-to-end startup, compile, edit/query, runtime, memory, allocation, copying, serialization, generated-code, and binary-size behavior.

Correctness remains deterministic and locally decidable. A model may propose an operation; deterministic implementation machinery decides whether it is valid.

## Authority and truth

Use `docs/authority.md` for ownership by claim dimension.

In practical order:

1. the active task owns the current objective;
2. this file owns repository-wide engineering procedure;
3. accepted files under `docs/spec/` own intended language and workspace contracts;
4. code, tests, manifests, schemas, and command definitions own behavior in the checkout;
5. `docs/status.md` reports current implementation and known gaps;
6. `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust boundaries;
7. `docs/performance.md` owns measurement method and compact reproducible evidence;
8. `docs/roadmap.md` owns planned ordering only;
9. sparse accepted decisions own durable rationale; and
10. Git history owns superseded implementation and prose.

A roadmap item is not an architectural commitment. Re-evaluate it before implementation.

When claims conflict, classify the claim, inspect its owning artifact and executable evidence, then update or delete stale material in the same change.

Use labels such as **Current**, **Target**, **Hypothesis**, **Historical**, **Unknown**, and **Blocked** when ambiguity would otherwise remain.

Do not create another authority layer from task prompts, planning trees, global revisions, prose digests, registries, evidence ledgers, generated inventories, closure graphs, checkpoints, handoffs, or completion capsules.

Specifications are revisable. When a better design changes intended semantics, update the owning specification and perform one direct implementation cutover. Do not silently contradict the specification or retain an obsolete compatibility path.

## One active architecture

Maintain one active:

- language definition;
- mutable semantic program authority;
- compiler path;
- generic production execution route;
- ownership model;
- package model;
- documentation authority model; and
- implementation for each current product boundary.

When replacing a mechanism, prefer a dependency-closed cutover. Delete displaced implementations, adapters, aliases, feature flags, compatibility tests, stale documentation, dead dependencies, and obsolete data paths in the same coherent change.

Do not keep permanent `v2`, `next`, `legacy`, `archive`, or `compat` implementations. Do not keep old and new parsers, semantic authorities, compilers, runtimes, storage formats, or protocols in parity. Git history is the migration record for discarded experimental work.

A small independent evaluator, model, or reference implementation may remain when it is a useful test oracle. It is not automatically a second production engine.

Names and current crate boundaries have no authority. Preserve, merge, split, rename, or delete components according to cohesion, safety, real platform ownership, independently useful APIs, measured compile isolation, coupling, and current consumers.

When the current architecture causes the defect, replace it. Do not hide it behind bookkeeping.

## Representation discipline

The authoritative semantic state must be able to exist without source text, files, formatting, paths, spans, parser nodes, source hashes, or compiler-dense indexes. Those are optional importer, presentation, provenance, cache, or trust-boundary attachments.

Text import may create semantic state. Compilation, semantic editing, queries, and correctness checks must not require rendering and reparsing text.

Do not satisfy an invariant with fake semantic data. Do not invent dummy source files, placeholder paths, fabricated hashes, hidden valid bodies beneath holes, synthetic entry points, fake declarations, reserved identities, or fallback executable meaning because an internal type currently requires them. Correct the representation or boundary.

Use an honest source-optional origin model. A source-backed fact may name source provenance; a source-free fact remains source-free. Never smuggle source-free meaning through a dummy `SourceId`.

One representation owns mutable semantic facts. Other representations may be derived for analysis, compilation, execution, projection, caching, persistence, or transport, but each must have a current producer, consumer, lifetime, invalidation rule, and deletion condition. Derived representations are not coequal mutable authorities.

Public stable identity does not require every internal object to be persistent. Dense compiler IDs, vector positions, physical slots, code offsets, and layout indexes should be derived, compact, and replaceable. Preserve logical identity explicitly when meaning survives; tombstone it when meaning does not.

Names, paths, spans, formatting, semantically irrelevant order, and content hashes are not universal mutable identities. Use opaque logical identities with namespace, generation, and revision defenses where stable editable identity is required.

Incomplete semantic state is valid editing state. Missing declarations, bodies, expressions, references, choices, or conflict resolutions must be explicit blockers, holes, or recovery facts. Do not retain executable fallback meaning behind an incomplete node.

The `.lkjscript` encoding is deliberately open. Do not redesign it without a current storage, transfer, integrity, startup, interoperability, or tooling requirement. An opaque or binary primary representation must still provide deterministic first-party inspection, validation, querying, editing, semantic diffing, projection/export, malformed-data handling, and failure-atomic publication.

Source-free authoring does not imply a database, journal, daemon, protocol, distributed store, CRDT, or collaboration system. Complete and measure the in-process semantic workflow first.

## Decision discipline

Resolve conflicts in this order:

1. semantic correctness, memory safety, and real security boundaries;
2. one simple and coherent active system;
3. evidence from the actual product path;
4. usefulness to AI-driven development;
5. end-to-end performance and resource efficiency;
6. failure atomicity, determinism, and observability;
7. maintainability and ease of replacement; and
8. speculative future flexibility.

Preserve future options by keeping current mechanisms small, explicit, tested, and replaceable, not by implementing several hypothetical futures.

Start from a demonstrated current defect or an explicit current product requirement. Correct the dependency-closed root cause, not one visible symptom.

Before adding an abstraction, identify:

- the concrete current problem;
- its current producer and consumer;
- the authority it owns or derives;
- lifetime and invalidation behavior;
- failure behavior;
- measured or structural benefit;
- why local code is insufficient; and
- the condition under which the abstraction should be deleted.

A new abstraction should remove duplication or repeated work, make an important invalid state unrepresentable, isolate a real boundary, expose an independently useful current API, enable a measured property, or materially simplify reasoning and testing. It must be smaller than the problem it replaces and must not duplicate authority.

Otherwise keep the logic local or do not add it.

Prefer, in order:

1. delete unused work;
2. remove a redundant representation or round trip;
3. correct poor asymptotic behavior;
4. make ownership and authority direct;
5. improve data layout and locality;
6. reduce allocation, copying, hashing, serialization, and generated bytes;
7. add a narrow cache only for measured repeated computation;
8. add parallelism only when remaining work is large and separable; and
9. add target-specific specialization only behind a complete generic route and measured need.

Do not solve complexity by adding bookkeeping around it. First remove repeated scans, reconstruction, duplicated facts, unnecessary boundaries, and dead work.

## Anti-overengineering

Do not build speculative:

- daemons, services, sessions, or process boundaries;
- persistence layers, journals, databases, distributed stores, or CRDTs;
- schedulers, resource topologies, process-cell systems, or custom allocators;
- universal registries, descriptor systems, plugin frameworks, or generic rewrite DSLs;
- general incremental-computation or cache frameworks;
- proof or certificate ecosystems;
- wire protocols or target matrices;
- multi-tier JIT policy, deoptimization, or PGO machinery; or
- platform products without a present end-to-end consumer.

A current task may introduce one of these only with a demonstrated boundary, present consumer, measured need, acceptance criteria, and reversal condition.

Do not impose numeric file-length, directory-width, directory-depth, module-count, plan-count, or repository-shape rules. Split and merge by cohesion, ownership, retrieval quality, testability, compile isolation, and real boundaries.

Do not reorganize unrelated code for symmetry or aesthetics.

Temporary planning belongs in agent working state. Do not commit planning hierarchies or bureaucracy to prove that the project avoided bureaucracy.

## Semantics, safety, and resources

Language validity is determined by semantic laws, not project-selected size quotas. Do not reject an otherwise valid trusted program because it exceeds an arbitrary number of source bytes, tokens, nesting levels, declarations, fields, variants, parameters, arguments, locals, functions, files, modules, blocks, IR nodes, identities, values, diagnostics, handles, or analysis steps.

Do not disguise a semantic limit by raising it, widening an integer, moving it to another phase, renaming it, or calling it a safety profile.

Use checked arithmetic and checked narrowing for sizes, offsets, identities, code locations, handles, and indexes. User-controlled depth must not consume unbounded native stack. Use iterative traversal or a justified heap-backed work stack.

Never silently truncate a complete result. Paginate, stream, return an explicit partial result, or fail.

An untrusted product may impose explicit coarse host-resource policy for input, memory, output, elapsed time, cancellation, and concurrency. Resource exhaustion is a typed host result, not a semantic error. Do not design detailed untrusted policy before such a product exists.

Follow the accepted ownership specification. Ordinary execution is collector-free and non-tracing. Do not introduce tracing collection, hidden reference-counting semantics, a raw-pointer language surface, retain/release, or general `free` merely to simplify implementation. A memory-semantic change requires a specification change and one complete cutover, not parallel GC and non-GC modes.

Preserve exact move and borrow laws, deterministic cleanup where promised, cleanup on normal, trap, error, cancellation, allocation failure, and early-exit paths, no double release, stack-safe destruction, explicit host-resource ownership, and failure-atomic publication.

Maintain one complete generic production execution route. An optional native or specialized path may decline only before effects and must leave the generic route intact. After specialized entry, its result or failure is final; never re-execute effects through fallback.

Validate fail-closed at real untrusted boundaries. Inside one synchronous trusted typed pipeline, validated values carry authority; do not repeatedly serialize, hash, reconstruct, or independently revalidate them without a real boundary consumer.

Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented safe-caller contract, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or property coverage.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling, allocation addresses, hash-table state, profile state, and cache state must not change completed meaning or deterministic diagnostic selection.

## Performance and evidence

Profile before optimizing. Measure the selected product path rather than a detached surrogate.

Relevant evidence may include:

- process wall time and phase time;
- startup and throughput;
- edit and query latency;
- peak and retained memory;
- allocations and allocated bytes;
- bytes copied, parsed, rendered, serialized, hashed, or generated;
- repeated traversal counts;
- scale behavior;
- generated-code size; and
- binary size.

Before a comparison, state the hypothesis, equivalent semantics, workload, environment, build and cache state, sample protocol, selection criterion, and reversal condition.

Prefer deterministic structural work counters over noisy timings when they answer the question. Generated scale tests establish correctness and complexity shape; they are not substitutes for representative application benchmarks.

Keep raw samples outside Git. Commit only compact reproducible evidence. Do not turn noisy developer-machine timing into a correctness gate.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code size, complexity, test burden, and maintenance. Do not add a validity quota to hide a performance defect or a framework to avoid one local scan.

## Work selection and execution

At the start of a substantial task:

1. inspect branch, worktree, upstream state, and recent history without destroying unrelated changes;
2. read the authority documents relevant to the task;
3. trace producers, consumers, mutable authority, derived representations, ownership, trust boundaries, and failure paths;
4. characterize current behavior with focused tests or measurements;
5. identify the highest-leverage dependency-closed problem;
6. state a falsifiable hypothesis, completion criteria, reversal condition, and stop condition in temporary working state;
7. implement the smallest coherent correction that removes the root cause;
8. delete the displaced path and stale claims;
9. add focused tests and update owning documentation;
10. run final verification after the final relevant change; and
11. commit cohesive changes and verify publication state when publication is requested.

The active task owns the objective, not an unverified proposed mechanism. Change course when executable evidence invalidates the suggested implementation.

If a more severe correctness, safety, or authority defect blocks the selected vertical, fix that dependency as part of the same vertical. Do not use incidental findings as permission for an unrelated rewrite.

Every turn must leave the repository coherent, documented, tested, and usable. Do not leave two active architectures, a half-cutover, disabled correctness checks, a required unfinished migration, stale prose presented as current, a compatibility layer hiding an incomplete replacement, or dependence on a scratch artifact.

Prefer a smaller complete vertical over a larger half-implemented program. Complete one coherent vertical, update the roadmap, identify the next problem, and stop. Do not begin the next roadmap item merely because time remains.

Do not commit task prompts, checkpoints, transcript summaries, copied context, handoff files, completion capsules, or prompt archives. If a task prompt was placed inside the repository for transport, remove it from the intended commit.

Preserve unrelated work. Do not use destructive reset, checkout, clean, history rewrite, or force push against work you did not create.

## Structure and dependencies

Organize code by coherent responsibility, not counts or symmetry.

A crate boundary should represent a real trust or unsafe boundary, an independently useful library, a supported target, measured compile isolation, or a low-coupling subsystem. Merge crates that mainly exchange internal descriptors, witnesses, re-exports, or compatibility adapters.

Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny modules, redundant models, and conversion layers when recombination improves comprehension. Do not reorganize unrelated code for aesthetics.

Use mature dependencies when they remove substantial custom machinery or risk. Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better.

Before adding a descriptor, registry, witness, identity, plan, contract, cache, or conversion layer, identify its current producer, consumer, lifetime, boundary, invalidation rule, and deletion condition.

## Tests

Tests should protect intended semantics and public invariants, not accidental topology.

Cover, as relevant:

- type, effect, capability, ownership, control-flow, and cleanup laws;
- completeness and explicit incomplete states;
- stable identity, namespaces, generations, revisions, deletion, and deterministic ordering;
- malformed input and stale, foreign, or wrong-kind identities;
- transaction and artifact failure atomicity;
- exactly-once effects and generic/specialized equivalence;
- cancellation, resource failure, host errors, and cleanup;
- deep input, deep destruction, scale behavior, and checked representation boundaries; and
- real product integration.

Add a focused regression test for each root cause. Use generated fixtures for scale. Keep fast default tests separate from ignored locked-release stress geometry while exercising the same algorithm at smaller sizes.

Use differential, property, model, or test-only reference implementations when an independent oracle is cheap and useful.

Delete tests whose purpose is preserving provisional syntax, old serialized bytes, obsolete APIs, deleted machinery, arbitrary limits, internal topology, or accidental details. Replace them with tests of intended invariants. Never weaken a test merely to make a redesign pass.

## Documentation

Keep active documentation small, non-overlapping, and truthful:

- `README.md`: product introduction and first successful use;
- `docs/spec/`: intended external semantics and target contracts;
- `docs/status.md`: current implementation and known gaps;
- `docs/architecture.md`: current responsibilities, data flow, ownership, and trust boundaries;
- `docs/performance.md`: method, reproducible workloads, compact evidence, and reversal conditions;
- `docs/roadmap.md`: only `Now`, `Next`, and `Later`; and
- `docs/decisions/`: sparse durable decisions.

Update the owning document and delete stale text in the same change. Do not add prose digests, global revisions, fact shards, generated inventories, copied tables, per-commit evidence records, transcripts, handoffs, prompt archives, completion capsules, or duplicate roadmaps.

Write a decision record only when a choice is durable, non-obvious, expensive to rediscover, and has a meaningful reversal condition.

Do not describe a target architecture as current, a hypothesis as measurement, private implementation movement as a public feature, or a planned subsystem as supported.

## Verification

After the final relevant change, run at least:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

Run retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run additional focused release stress, differential or property tests, small-stack and deep-input tests, malformed-boundary tests, cancellation or allocation-failure tests, Miri, sanitizer, fuzz, benchmark, documentation, and example checks when relevant.

If the environment prevents a command, report the exact command, failure category, and successful evidence that remains. Do not silently substitute a weaker command. Do not claim a command passed unless it ran after the final relevant change.

## Definition of done

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and displaced paths are gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning specification;
- no fake semantic data, retained dead authority, or arbitrary validity limit substitutes for a representation or algorithmic correction;
- failure cannot partially publish state, consume stable identities, poison an earlier snapshot, or duplicate effects;
- focused tests cover the changed invariant and important failure paths;
- performance claims have reproducible equivalent evidence;
- active documentation describes the checkout truthfully;
- final relevant verification ran after the final change;
- intended changes are committed and branch/upstream state is verified when publication was requested; and
- the repository is coherent for the next independent turn.

The final report must separate:

- implemented work;
- semantic and architectural changes;
- important deletions;
- measurements;
- tests and exact commands;
- commit and push state;
- untested paths;
- remaining risks;
- deliberately deferred work; and
- the next highest-leverage problem.

Stop after one coherent vertical.
