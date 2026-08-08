# AGENTS.md

## 1. Scope and language

This file applies to the entire repository.

Write code, comments, diagnostics, tests, documentation, commit messages, and final reports in English unless the current task explicitly requires another language for a user-facing artifact.

The user authorizes autonomous technical decisions, incompatible changes, destructive simplification, representation replacement, crate and file reorganization, specification revision, and deletion of obsolete work. Backward compatibility is not a goal unless the current task explicitly makes it one.

Do not ask the user to choose among technical options when repository evidence, accepted semantics, experiments, profiling, or a reversible assumption can decide. Ask only when a genuinely external requirement is missing and no safe assumption can unblock the work.

Do not destroy unrelated local work, external user data, credentials, or host state. Broad authority over the repository is not authority to erase unrelated state.

## 2. Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance programming language and implementation.

AI-primary means that an agent can discover, understand, transform, validate, compile, execute, and review programs through deterministic, precise, compact, composable interfaces. It does not mean that model inference belongs in the compiler, runtime, validator, optimizer, database, or correctness boundary.

Optimize for:

- semantic locality instead of repository-wide context dumps;
- stable logical identity instead of fragile textual coordinates;
- typed atomic edits instead of uncontrolled string replacement;
- explicit incomplete states instead of fabricated executable placeholders;
- revision-labelled deterministic queries and diagnostics;
- compact headers with selective expansion, stable ordering, and honest pagination;
- direct compilation from authoritative semantic state;
- low round-trip count and actionable failure explanations;
- human-reviewable projections and diffs;
- offline deterministic validation;
- final execution speed;
- compilation, edit, query, startup, and time-to-first-result latency;
- peak and retained memory;
- allocation, copying, hashing, serialization, generated code, and binary size;
- predictable failure, cancellation, and cleanup;
- maintainability and ease of deletion.

Do not optimize for the tokenization quirks of one model generation. Do not hide authoritative semantics in an opaque binary merely to call the system AI-native. Humans must retain understandable diagnostics, reproducible builds, inspectable mechanisms, and reviewable projections and diffs.

## 3. Authority and truth

Follow `docs/authority.md` for ownership by claim dimension.

In summary:

1. the current task owns the work requested now;
2. this file owns engineering procedure and decision discipline;
3. accepted files under `docs/spec/` own intended external semantics and target contracts;
4. code, tests, manifests, schemas, and command definitions own current checkout behavior;
5. `docs/status.md`, `docs/architecture.md`, `docs/performance.md`, and `docs/roadmap.md` own their narrowly defined descriptive roles;
6. sparse accepted decisions own durable rationale only when such a decision exists;
7. Git history owns superseded implementation and prose.

Use precise state labels when needed:

- **Current**: implemented and verified in this checkout;
- **Target**: accepted intended contract not yet fully implemented;
- **Hypothesis**: an unverified design or performance claim;
- **Historical**: superseded evidence retained for context;
- **Unknown**: not yet inspected or measured;
- **Blocked**: prevented by a named external condition.

When artifacts conflict, classify the claim, inspect the owning artifact and executable evidence, then update or delete stale material in the same change.

Do not create another authority layer from global revisions, digests, registries, evidence ledgers, closure graphs, generated inventories, copied tables, task prompts, or handoff documents.

Current architecture is not sacred. Specifications may be deliberately revised when the task and long-term mission justify a semantic change. Update the owning specification and directly replace incompatible implementation; do not preserve a provisional compatibility path.

## 4. Decision order

When goals conflict, use this order:

1. semantic correctness, memory safety, and real security boundaries;
2. one simple, coherent active system;
3. measured evidence from the actual product path;
4. usefulness to AI-driven development;
5. end-to-end performance and resource efficiency;
6. failure atomicity, determinism, and observability;
7. maintainability and ease of deletion or replacement;
8. speculative future flexibility.

Preserve future options by keeping current mechanisms small, explicit, tested, and replaceable. Do not preserve options by implementing several hypothetical futures now.

Prefer:

- deleting unused work to organizing it;
- a direct data flow to a generalized framework;
- one complete vertical to broad scaffolding;
- a local data structure to a universal registry;
- a typed value to repeated serialization and reconstruction;
- a measured narrow cache to a general incremental engine;
- one supported target to a decorative target matrix.

## 5. One active system and no compatibility burden

Maintain one active:

- language definition;
- semantic/source authority;
- compiler path;
- production execution policy;
- package model;
- runtime ownership model;
- documentation authority model;
- roadmap.

When replacing a mechanism, prefer a direct cutover. Delete the displaced path, adapters, aliases, migrations, compatibility tests, feature flags, and stale prose in the same dependency-closed change.

Do not create permanent:

- `v2`, `next`, `legacy`, `archive`, or `compat` implementations;
- old and new parsers, semantic authorities, compilers, or runtimes kept in parity;
- translation layers whose primary purpose is preserving a provisional format;
- deprecated syntax aliases or bytecode/snapshot migrations without a current external data requirement;
- public stability promises for provisional syntax, CLI shape, package files, bytecode, serialized snapshots, cache keys, metrics schemas, protocols, manifests, or internal APIs;
- abstractions that mainly keep an obsolete abstraction alive.

Git history is the migration record for discarded experimental work.

A small independent evaluator, model, or reference implementation may remain when it provides a valuable semantic oracle in tests. It is not automatically a second product engine.

Names have no authority. Preserve, merge, split, rename, or delete any crate or component according to cohesion, safety, platform ownership, compile isolation, measured cost, and current consumers.

## 6. Anti-overengineering gate

Start from a demonstrated current problem or an explicit current product requirement, not from an imagined platform.

A new abstraction must perform at least one concrete job now:

- remove demonstrated duplication or repeated work;
- make an important invalid state unrepresentable;
- isolate a real trust, unsafe, FFI, platform, ownership, or build boundary;
- expose an independently useful current API;
- enable a measured performance property;
- substantially simplify testing and reasoning.

It must also:

- be smaller than the problem it replaces;
- have a clear owner and current consumer;
- define failure and lifetime behavior;
- avoid duplicating authority;
- remain deletable or replaceable.

Otherwise keep the logic local or do not add it.

Do not build speculative:

- daemon or service frameworks;
- persistence, journaling, distributed storage, CRDT, consensus, or replication;
- schedulers, resource topologies, NUMA policy, process-cell systems, or custom execution fabrics;
- plugin frameworks or universal registries;
- general cache or incremental-computation frameworks before a measured repeated-computation problem;
- proof, witness, certificate, digest, or content-identity ecosystems without a real trust, cache, transfer, persistence, or executable-artifact boundary;
- wire protocols without a real process consumer;
- backend or target matrices without supported product targets;
- multi-tier JIT policy, deoptimization machinery, or speculative native artifacts without equivalent product measurements;
- custom allocators, memory managers, schedulers, or storage engines before a measured need;
- generalized taxonomies for one local distinction.

Do not solve complexity by adding bookkeeping around the complexity. First delete unused work, remove repeated scans and reconstruction, simplify ownership, and reduce the number of representations.

Do not impose numeric file length, directory width, directory depth, module count, plan count, or repository-shape policies. Split and merge by cohesion, ownership, retrieval quality, testability, compile isolation, and real boundaries.

Do not create bureaucracy proving that the project has avoided bureaucracy.

## 7. Long-horizon directions and admission gates

The user has durable long-horizon interests in:

1. runtime-informed adaptive optimization;
2. one shared local daemon that can coordinate `lkjscript` work;
3. a performance-oriented native database designed with `lkjscript` in mind.

These are directions to preserve, not current implementation mandates. Do not erase them from the roadmap merely because premature implementations were deleted. Do not use them to justify speculative machinery.

### 7.1 Runtime-informed optimization

Runtime observations may eventually guide later specialization, but must never determine language meaning.

Before adding a profiling or adaptive mechanism, require:

- a representative product workload corpus;
- stable baseline measurements for the generic path;
- one specific optimization hypothesis;
- a named owner and lifetime for profile data;
- bounded collection overhead and retained memory;
- explicit invalidation when code or target meaning changes;
- bounded generated-code memory and compile cost;
- a complete generic fallback;
- deterministic semantics independent of profile and cache state;
- evidence across more than one contrived fixture;
- a deletion or reversal condition.

Add one optimization mechanism at a time. Do not jump from a baseline compiler to a multi-tier runtime, deoptimization framework, on-stack replacement, global profile database, or persistent code cache.

### 7.2 Shared local daemon

Prefer in-process APIs until a process boundary earns its cost.

A daemon is admissible only after:

- the semantic workspace and direct compiler input are mature;
- repeated edit, query, compile, and run workloads are measured;
- incremental recomputation has a selected, measured design;
- warm-state value is quantified;
- startup, IPC, retained RSS, crash isolation, cancellation, and security tradeoffs are known;
- the trust domain and ownership model are explicit.

The first daemon should be small: one coordinator per explicit local trust domain, probably an OS user unless evidence requires another scope. It should expose thin deterministic operations over existing in-process libraries.

Do not resurrect process cells, session brokers, scheduler platforms, resource topologies, service databases, protocol revisions, or orchestration layers as prerequisites.

### 7.3 Native database

The current direct SQLite capability is the baseline and oracle until evidence supports another design.

Before custom database work:

- define representative workloads;
- define durability, transaction, isolation, recovery, concurrency, indexing, query, and value-layout semantics;
- measure the direct SQLite path;
- identify a concrete bottleneck or semantic mismatch;
- decide whether better integration, a narrow embedded primitive, or a new engine is warranted;
- define capability and trust boundaries;
- test crash, corruption, recovery, cancellation, and failure atomicity.

A renamed SQLite wrapper, daemon metadata store, service database, registry, or control plane is not the desired native database.

## 8. Work selection and multi-turn discipline

At the start of a substantial task:

1. inspect branch, worktree, upstream state, and recent history without destroying unrelated changes;
2. read the relevant authority documents;
3. trace the actual producer/consumer path and ownership/trust boundaries;
4. identify the highest-leverage dependency-closed problem supported by evidence;
5. state a falsifiable hypothesis;
6. establish a focused baseline or semantic characterization;
7. define completion, reversal, and stop conditions;
8. implement the smallest coherent correction.

Do not spend the task constructing an elaborate hierarchy of planning files. Keep scratch plans in agent working state, not as product authority. Do not commit task prompts, checkpoints, transcript summaries, completion capsules, active archives, or copied context.

For multi-turn work, every turn must leave the repository coherent, documented, tested, and usable. Do not leave:

- two active architectures;
- a half-completed cutover;
- disabled correctness checks;
- a migration that is required for ordinary use but not completed;
- prose that describes uncommitted future behavior;
- hidden dependence on a scratch artifact.

Complete one coherent vertical, update the roadmap, report the next problem, and stop. Do not automatically begin the next roadmap item because time remains.

If evidence invalidates the task's suggested implementation, change course. The task owns the objective, not an unverified mechanism.

## 9. AI-facing semantic design

Maintain one syntax-independent semantic authority when the accepted workspace specification requires it. Text may be a persistent import/export, interoperability, review, and debugging representation without becoming a second compiler authority.

Evaluate AI-facing changes by whether an agent can:

- locate a relevant semantic slice without loading the entire repository;
- refer to entities stably across unrelated presentation changes;
- query actual and expected meaning at a named revision;
- receive compact headers first and expand selected detail;
- propose a typed batch of edits with explicit preconditions;
- receive deterministic diagnostics, semantic diff, and invalidation information;
- preserve useful incomplete states without pretending they are executable;
- compile a complete snapshot without rendering and reparsing text;
- understand why an operation failed and what legal operations remain;
- review the result through a deterministic human-readable projection.

Correctness must remain deterministic and local. A model may propose an operation but must never decide whether it is valid.

Text coordinates, names, paths, formatting, source order, and content hashes are not universal identity. Use opaque logical identity with namespace, generation, and revision defenses where mutable semantic identity is required.

Do not add persistence, collaboration, a protocol, a daemon, remote execution, or a cache merely because semantic snapshots exist. Prefer an in-process API until a measured consumer justifies another boundary.

Do not duplicate semantic facts across HIR, indexes, attachments, caches, projections, and compiled artifacts without clear ownership and invalidation. Derived state must be reproducible from its owner or explicitly versioned at a real boundary.

An incomplete editing snapshot may be valid and queryable. It must not be compiled through a fabricated placeholder.

## 10. Semantic validity, resources, and scale

Language meaning is determined by semantics, not project-selected size quotas.

Do not make an otherwise valid trusted program invalid because it exceeds an arbitrary count of:

- bytes or tokens;
- nesting;
- declarations, fields, variants, parameters, arguments, locals, functions, files, or modules;
- blocks, edges, IR nodes, identities, values, cleanup records, diagnostics, or analysis steps;
- runtime values, handles, structural nodes, or native bookkeeping entries.

Do not disguise a limit by raising it, widening its integer, moving it to another phase, renaming it, or calling it a safety profile.

Trusted local work may end because of:

- success;
- explicit cancellation;
- allocation failure;
- operating-system or I/O failure;
- a genuine external representation boundary;
- another real host failure.

An untrusted product may impose explicit coarse policy for input bytes, memory, output, elapsed time, cancellation, and concurrency. Exhaustion is a typed host-resource result, not a semantic error. Do not define a detailed untrusted policy before an untrusted product exists.

An optional specialization may decline because its private representation or resource policy cannot handle a program only when a complete generic path remains. The decline must be observable, typed, cheap enough to justify, and must not silently change meaning or duplicate effects.

Use checked arithmetic and checked narrowing for sizes, offsets, identities, code locations, handles, and indexes. Compact representations must decline or fall back before they restrict an otherwise valid generic program.

User-controlled depth must not consume unbounded native stack. Prefer iterative traversal or an explicitly heap-backed work stack. A private segment size is tuning, not a language-depth limit.

Never silently truncate a result claimed as complete. Stream, paginate, return a continuation, mark it partial, or fail explicitly.

## 11. Ownership and memory model

Follow the accepted language specification. Ordinary execution is currently specified as collector-free and non-tracing.

Do not introduce a tracing collector, hidden reference-counting semantic fallback, raw-pointer surface, retain/release API, general `free`, or runtime-engine-specific memory controls merely to simplify an implementation.

Use Rust ownership for compiler and host implementation state where appropriate; the collector-free language contract does not require custom manual management for every internal Rust value.

Preserve:

- exact move and borrow laws;
- deterministic cleanup where promised;
- cleanup on normal, failure, cancellation, and early-exit paths;
- no double release;
- failure-atomic publication;
- safe handling of deep destruction;
- explicit ownership of host resources.

A deliberate change to language memory semantics requires an owning specification change, complete implementation cutover, and evidence that the semantic and complexity trade is better. Do not maintain GC and non-GC language modes in parallel.

## 12. Runtime and specialization discipline

Maintain one production execution policy.

A complete generic route must own language coverage. Native compilation, specialization, vectorization, or target-specific layouts are optional accelerators unless the accepted specification deliberately says otherwise.

Before native entry:

- unsupported shapes must decline without effects;
- original validated program and inputs must remain available to the generic route;
- installation and accounting must be failure-atomic;
- cancellation, deadline, and resource policy must be coherent;
- decline reasons must be precise enough to measure.

After native entry:

- the result or failure is final;
- do not re-execute in another engine;
- observable effects must occur exactly once;
- cleanup and host-resource behavior must preserve semantics.

Do not add public engine-selection flags, forced execution helpers, tier names in user semantics, or benchmark-only product paths.

Runtime profile state, code cache state, scheduling, allocation addresses, and hash-table state must not alter completed language meaning or which deterministic diagnostic wins.

## 13. Performance discipline

Profile before optimizing. Measure the selected product path, not only a disconnected microbenchmark.

For relevant work consider:

- end-to-end wall time and phase time;
- startup and time to first result;
- steady-state throughput and latency;
- peak and retained memory;
- allocation count and bytes;
- bytes copied, serialized, hashed, and generated;
- repeated full-program or full-function traversals;
- asymptotic behavior across generated scale;
- generated-code and final binary size;
- cold and warm behavior;
- semantic edit, query, and compile latency;
- native admission and fallback costs;
- failure, cancellation, and cleanup paths.

Before a comparison, state:

- hypothesis;
- equivalent semantics;
- workload;
- machine, OS, architecture, and toolchain;
- build profile;
- cache state;
- sample protocol;
- selection criterion;
- reversal condition.

Prefer, in order:

1. delete work with no current consumer;
2. fix poor asymptotic complexity and repeated reconstruction;
3. avoid unnecessary whole-program clones and unconditional representations;
4. improve data layout and locality;
5. reduce allocation, copying, hashing, serialization, and generated bytes;
6. add a narrow cache or incremental mechanism only for a measured repeated computation;
7. add parallelism only when remaining work is large, separable, and worth synchronization;
8. add target-specific specialization only behind a generic correct path and measured product need.

Do not add a validity quota to hide a performance defect. Do not add a general framework to avoid one local scan. Do not optimize solely for one generated fixture; cross-check representative and adversarial behavior.

Use complexity or work counters when they provide more stable evidence than timing, but keep them local and purposeful. Do not create a global phase ledger.

Keep reproducible harnesses. Store raw profiles and samples outside Git or in CI artifacts. Commit only compact results and the decisions they support. Do not make noisy developer-machine timings hard correctness gates.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code size, complexity, test burden, and maintenance.

## 14. Safety, validation, determinism, and failure atomicity

Preserve genuine trust boundaries while removing internal ceremony.

Validate fail-closed at untrusted boundaries such as:

- text/source and semantic-operation input;
- packages, paths, imports, manifests, and locks;
- serialized, persisted, cached, or transferred data;
- bytecode or executable artifacts loaded from outside trusted construction;
- capabilities and host operations;
- relocation and executable-memory installation;
- generated entry points and FFI;
- operating-system, network, terminal, filesystem, and database interfaces.

Inside one synchronous trusted pipeline, validated typed values carry authority. Do not repeatedly serialize, hash, reconstruct, independently verify, or bind identities to the same value unless a real cache, transfer, persistence, executable-artifact, or threat boundary consumes the result.

Unsafe code belongs in a narrow named mechanism with:

- a documented safe-caller contract;
- explicit invariants;
- focused malformed-input tests;
- appropriate Miri, sanitizer, fuzz, or property coverage.

Do not spread unsafe code to save trivial wrapper cost. Do not build layers of ceremony around an already narrow safe abstraction.

Publication must be failure-atomic. Validation failure, cancellation, allocation failure, I/O failure, backend failure, or resource exhaustion must preserve the previous published snapshot, cache entry, executable mapping, database transaction, or durable state.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling, allocation addresses, hash-table state, profile state, and cache state must not change completed program meaning or which deterministic diagnostic wins.

## 15. Repository and component structure

Organize by coherent responsibility, not counts or aesthetic symmetry.

A crate boundary should correspond to at least one real property:

- a trust or unsafe boundary;
- an independently useful library;
- a distinct supported build target or platform;
- measured compile isolation;
- a low-coupling subsystem with clear ownership.

Merge crates that mainly exchange internal descriptors, digests, witnesses, re-exports, or compatibility adapters. Split a crate only when a real boundary becomes clearer.

Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny modules, and redundant model/conversion layers when recombination improves comprehension. Keep a larger file when it is one coherent mechanism; split a smaller file when responsibilities are genuinely independent.

Do not reorganize unrelated code merely to make the tree look cleaner. Structural work must remove current complexity or support the active vertical.

Use mature dependencies when they remove substantial custom machinery or risk. Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better. Pinning and supply-chain policy must be deliberate, not a zero-dependency badge.

Before adding a descriptor, registry, witness, identity, plan, contract, or cache object, identify its current producer, consumer, lifetime, boundary, and deletion condition.

## 16. Tests

Tests should protect:

- semantic laws;
- type, effect, capability, and ownership behavior;
- safety boundaries;
- failure atomicity;
- deterministic behavior;
- exactly-once observable effects;
- stack safety;
- scale behavior;
- selected product behavior;
- generic/specialized equivalence.

Add focused regression tests for every root cause fixed. Characterize subtle current semantics before replacing their implementation.

Use generated fixtures for scale. Keep fast default tests separate from explicitly ignored release stress geometry when necessary, but ensure ordinary tests exercise the same algorithm on smaller inputs.

Use differential, property, model, or test-only reference implementations where an independent oracle is cheap. A reference path must not become a second production architecture.

For performance corrections, prefer structural tests and work-shape assertions over unstable wall-clock CI gates. Retain a reproducible release harness for development evidence.

Test failure paths, not only success:

- malformed input;
- stale identity and revision;
- cancellation and deadline;
- allocation or resource exhaustion where injectable;
- partial installation or publication;
- cleanup failure;
- native decline before entry;
- entered failure without retry;
- deep input and destruction;
- host and database errors.

Delete tests whose primary purpose is preserving:

- provisional syntax or CLI compatibility;
- old serialized bytes, cache identities, metrics schemas, or manifests;
- obsolete engine parity;
- arbitrary count limits;
- deleted platform machinery;
- internal file topology;
- accidental implementation details.

Never weaken a test merely to make a redesign pass. Replace it with a test of the intended invariant.

## 17. Documentation

Keep active documentation small, non-overlapping, and truthful.

- `README.md` introduces the product and first successful use.
- `docs/spec/` owns intended external semantics and target contracts.
- `docs/status.md` reports implemented behavior and known gaps.
- `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust boundaries.
- `docs/performance.md` records method, reproducible workloads, compact evidence, and reversal conditions.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later` ordering.
- `docs/decisions/` contains only sparse durable decisions.

Update the document that owns a changed claim. Delete stale text rather than retaining it as active history.

Do not add:

- prose digests;
- global platform revisions;
- fact shards;
- closure graphs;
- generated inventories;
- copied Cargo, CLI, schema, operation, or diagnostic tables;
- per-commit evidence records;
- task transcripts;
- committed agent handoffs;
- prompt archives;
- completion capsules;
- duplicate roadmaps.

Write a decision record only when the decision is durable, non-obvious, expensive to rediscover, and has a meaningful reversal condition. Most implementation choices do not need one.

Do not document a future architecture as current. Clearly label target and hypothesis material.

Preserve the long-horizon adaptive-runtime, shared-daemon, and native-database directions in concise roadmap language, but do not turn the roadmap into a speculative design document.

## 18. Change protocol

For a substantial change:

1. inspect `git status`, branch, upstream state, and recent history;
2. preserve unrelated work;
3. read relevant specification, status, architecture, performance, and roadmap sections;
4. trace producers, consumers, ownership, trust boundaries, and failure paths;
5. establish a focused baseline or semantic characterization;
6. define the smallest dependency-closed vertical;
7. implement the simplest coherent correction;
8. delete the displaced path in the same vertical;
9. add focused correctness, malformed-input, failure, stack, and scale tests;
10. measure when performance is part of the claim;
11. update owning documentation;
12. run focused verification while iterating;
13. run full relevant verification after the final change;
14. inspect the final diff for duplicate architecture, stale references, unchecked narrowing, accidental compatibility, benchmark-specific paths, and speculative machinery;
15. commit cohesive changes;
16. push the current branch without force when the task and environment permit;
17. verify branch and upstream state.

Do not use destructive reset, checkout, clean, history rewrite, or force push against work you did not create.

Do not claim a command passed unless it ran after the final relevant change. Do not claim a commit was pushed without verifying branch and upstream state.

## 19. Standard verification

Before completion, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

Run the retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run additional relevant tools at changed boundaries, such as:

- focused release stress and benchmark harnesses;
- differential and property tests;
- small-stack/deep-input tests;
- malformed decoder, validator, executable, or transaction tests;
- cancellation, allocation-failure, and publication-atomicity tests;
- Miri;
- ASan, LSan, or TSan;
- fuzzing;
- documentation link and example checks.

When an environmental failure prevents a command, report the exact command, failure category, and successful evidence that remains. Do not silently substitute a weaker verification command.

## 20. Definition of done

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and the old path is gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning specification;
- no arbitrary validity limit substitutes for an algorithmic correction;
- failure cannot partially publish state;
- observable effects cannot execute twice through fallback;
- focused tests cover the changed invariant and important failure paths;
- performance claims have reproducible equivalent evidence;
- active documentation describes the checkout truthfully;
- final relevant verification has run after the final change;
- intended changes are committed and push state is verified when publication is requested;
- the repository is left coherent for the next independent turn.

The final report must separate:

- implemented work;
- measured results;
- important deletions and replacements;
- tests and commands run;
- commit and push state;
- untested environments or paths;
- remaining risks;
- deliberately deferred work;
- the next highest-leverage problem.

Do not describe a plan as implementation, a hypothesis as measurement, a single noisy sample as a stable result, or an intentionally deleted subsystem as still supported.

Stop after one coherent vertical.
