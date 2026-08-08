# AGENTS.md

## 1. Scope and fixed facts

This file applies to the entire repository.

Write code, comments, diagnostics, tests, documentation, commit messages, and final reports in English unless the current task explicitly requires another language for a user-facing artifact.

The user authorizes autonomous technical decisions, incompatible changes, destructive simplification, representation replacement, specification revision, crate and file reorganization, and deletion of obsolete work. Backward compatibility is not a goal unless the current task explicitly makes it one.

The only permanently fixed property of the program file format is the `.lkjscript` extension. Do not infer that its contents must remain textual, line-oriented, Unicode, human-authored, compatible with the current grammar, or compatible with any prior bytes.

Do not ask the user to choose among technical options when repository evidence, accepted semantics, focused experiments, profiling, or a reversible assumption can decide. Ask only when a genuinely external requirement is missing and no safe assumption can unblock the work.

Historical requests and preferences are context, not permanent architecture requirements. Re-evaluate them against the current task, specifications, checkout, measurements, and long-term product quality. Do not preserve or build a mechanism merely because the user once expressed interest in it.

Do not destroy unrelated local work, external user data, credentials, or host state. Authority to redesign this repository is not authority to erase unrelated state.

## 2. Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance programming language and implementation.

AI-primary means that an agent can discover, understand, construct, transform, validate, compile, execute, and review programs through deterministic, precise, compact, composable interfaces. It does not mean that model inference belongs in the compiler, runtime, validator, optimizer, storage engine, or correctness boundary.

Optimize for:

- one syntax-independent semantic authority;
- first-class program construction without source-text bootstrapping;
- stable logical identity instead of fragile textual coordinates;
- explicit incomplete states instead of dummy executable programs;
- typed, atomic, revision-checked edits instead of uncontrolled string replacement;
- expected and actual types, scope-correct bindings, effects, capabilities, ownership facts, and actionable diagnostics at the point of work;
- compact semantic headers followed by selective expansion;
- deterministic ordering, pagination, projections, and semantic diffs;
- direct compilation from complete semantic state without rendering and reparsing;
- low agent round-trip count and failures that explain legal next actions;
- human-reviewable projections and reproducible builds;
- final execution speed, startup, compilation, edit/query, and time-to-first-result latency;
- low peak memory, retained memory, allocation, copying, hashing, serialization, generated-code size, and binary size; and
- predictable cancellation, cleanup, and failure atomicity.

Correctness remains deterministic and local. A model may propose an operation; deterministic implementation machinery decides whether it is valid.

Do not optimize for one model's tokenizer. Do not make semantics opaque merely to call the system AI-native. Humans and tools must retain precise diagnostics, inspection, validation, semantic diffing, and review projections.

## 3. Authority and truth

Follow `docs/authority.md` for ownership by claim dimension.

In summary:

1. the current task owns the objective being pursued now;
2. this file owns repository-wide engineering procedure and decision discipline;
3. accepted files under `docs/spec/` own intended external semantics and target contracts;
4. code, tests, manifests, schemas, and command definitions own current checkout behavior;
5. `docs/status.md`, `docs/architecture.md`, `docs/performance.md`, and `docs/roadmap.md` own their narrowly defined descriptive roles;
6. sparse accepted decisions own durable rationale only when such a decision exists; and
7. Git history owns superseded implementation and prose.

Use precise labels when needed: **Current**, **Target**, **Hypothesis**, **Historical**, **Unknown**, and **Blocked**.

When artifacts conflict, classify the claim, inspect its owning artifact and executable evidence, then update or delete stale material in the same change.

Do not manufacture another authority layer from global revisions, prose digests, registries, evidence ledgers, closure graphs, generated inventories, copied tables, task prompts, or handoff documents.

Specifications and architecture are revisable. When a better design requires changing an accepted contract, update the owning specification and perform a direct implementation cutover. Do not silently contradict the specification or preserve an obsolete compatibility path.

## 4. One active system; no compatibility burden

Maintain one active language definition, semantic program authority, compiler path, production generic execution route, runtime ownership model, package model, documentation authority model, and roadmap.

When replacing a mechanism, prefer a dependency-closed direct cutover. Delete the displaced path, adapters, aliases, feature flags, migrations, compatibility tests, and stale prose in the same coherent change.

Do not create permanent:

- `v2`, `next`, `legacy`, `archive`, or `compat` implementations;
- old and new parsers, semantic authorities, compilers, runtimes, or storage formats kept in parity;
- translation layers whose primary purpose is preserving provisional bytes or APIs;
- deprecated syntax aliases or readers for old `.lkjscript` files without a current external data requirement;
- stability promises for provisional syntax, CLI shape, package files, bytecode, snapshots, cache keys, metrics schemas, protocols, manifests, or internal APIs; or
- abstractions that mainly keep an obsolete abstraction alive.

Git history is the migration record for discarded experimental work.

A small independent evaluator, model, or reference implementation may remain when it provides a valuable test oracle. It is not automatically a second production engine.

Names have no authority. Preserve, merge, split, rename, or delete any crate or component according to cohesion, safety, platform ownership, compile isolation, measured cost, and current consumers.

## 5. Representation discipline

The authoritative semantic state must not require source text, formatting, paths, spans, parser nodes, or source hashes in order to exist. Those are optional presentation, import, provenance, or trust-boundary attachments.

Text import may create semantic state, but text must not remain the only way to create a program. Compilation, analysis, semantic edits, and queries must not render and reparse text.

Do not satisfy an internal invariant with fake semantic data. Do not invent dummy source files, placeholder paths, hidden valid bodies beneath holes, synthetic executable entry points, or fabricated declarations merely because an internal IR currently requires them. Correct the representation or boundary.

One representation owns mutable semantic facts. Other representations may be derived for analysis, compilation, execution, projection, caching, persistence, or transport, but each must have a clear producer, consumer, lifetime, invalidation rule, and deletion condition. They are not coequal mutable authorities.

Names, paths, formatting, semantically irrelevant source order, spans, and content hashes are not universal mutable identity. Use opaque logical identity with namespace, generation, and revision defenses where stable editable identity is required.

The `.lkjscript` encoding is deliberately open. Do not redesign it unless the active vertical has a measured storage, transfer, integrity, startup, or tooling requirement. Before adopting an opaque or binary primary representation, provide deterministic first-party inspection, validation, semantic querying, editing, diffing, projection/export, malformed-data handling, and failure-atomic publication.

Do not create a storage database, content-addressed codebase, journal, protocol, or daemon merely because source text is not authoritative. First complete and measure the in-process semantic workflow.

## 6. Decision discipline and anti-overengineering

When goals conflict, use this order:

1. semantic correctness, memory safety, and real security boundaries;
2. one simple, coherent active system;
3. measured evidence from the actual product path;
4. usefulness to AI-driven development;
5. end-to-end performance and resource efficiency;
6. failure atomicity, determinism, and observability;
7. maintainability and ease of deletion or replacement; and
8. speculative future flexibility.

Preserve future options by keeping current mechanisms small, explicit, tested, and replaceable, not by implementing several hypothetical futures.

Start from a demonstrated current problem or an explicit current product requirement.

A new abstraction must do concrete work now: remove duplication or repeated work, make an important invalid state unrepresentable, isolate a real boundary, expose an independently useful current API, enable a measured performance property, or substantially simplify testing and reasoning. It must be smaller than the problem it replaces, have a current owner and consumer, define failure and lifetime behavior, avoid duplicating authority, and remain deletable.

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
9. add target-specific specialization only behind a complete generic route and measured product need.

Do not build speculative daemons, persistence, journals, distributed storage, CRDTs, schedulers, resource topologies, process-cell systems, plugin frameworks, universal registries, general incremental frameworks, proof/certificate ecosystems, wire protocols, target matrices, multi-tier JIT policy, deoptimization machinery, custom allocators, custom schedulers, or storage engines.

A current task may introduce one of those only after demonstrating its present consumer, boundary, measured need, acceptance criteria, and reversal condition.

Do not solve complexity by adding bookkeeping around it. First delete unused work, repeated scans, reconstruction, duplicated facts, and unnecessary boundaries.

Do not impose numeric file-length, directory-width, directory-depth, module-count, plan-count, or repository-shape policies. Split and merge by cohesion, ownership, retrieval quality, testability, compile isolation, and real boundaries.

Do not spend a task constructing planning hierarchies or bureaucracy that proves the project avoided bureaucracy. Keep temporary plans in agent working state, not in the product.

## 7. Correctness, resources, runtime, and performance

Language validity is determined by semantic laws, not project-selected size quotas. Do not make an otherwise valid trusted program invalid because it exceeds an arbitrary count of bytes, tokens, nesting, declarations, fields, variants, parameters, arguments, locals, functions, files, modules, blocks, IR nodes, identities, values, diagnostics, handles, or analysis steps.

Do not disguise a limit by raising it, widening its integer, moving it to another phase, renaming it, or calling it a safety profile.

Use checked arithmetic and narrowing for sizes, offsets, identities, code locations, handles, and indexes. User-controlled depth must not consume unbounded native stack. Prefer iterative traversal or an explicit heap-backed work stack. Never silently truncate a complete result; paginate, stream, return a continuation, mark it partial, or fail explicitly.

An untrusted product may impose explicit host-resource policy for input, memory, output, elapsed time, cancellation, and concurrency. Resource exhaustion is a typed host result, not a semantic error. Do not design detailed untrusted policy before such a product exists.

Follow the accepted ownership specification. Ordinary execution is currently collector-free and non-tracing. Do not introduce tracing collection, hidden reference-counting semantics, raw-pointer language surface, retain/release, or general `free` merely to simplify implementation. A memory-semantic change requires a specification change, one complete cutover, and evidence; do not maintain GC and non-GC modes in parallel.

Preserve exact move/borrow laws, deterministic cleanup where promised, cleanup on normal/failure/cancellation/early-exit paths, no double release, safe deep destruction, explicit host-resource ownership, and failure-atomic publication.

Maintain one complete generic production execution route. Optional native or specialized paths may decline only before effects and must leave the generic route intact. After specialized entry, the result or failure is final; never re-execute effects through fallback.

Validate fail-closed at real untrusted boundaries. Inside one synchronous trusted typed pipeline, validated values carry authority; do not repeatedly serialize, hash, reconstruct, or independently verify them without a real boundary consumer.

Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented safe-caller contract, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or property coverage.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling, allocation addresses, hash-table state, profile state, and cache state must not change completed meaning or which deterministic diagnostic wins.

Profile before optimizing. Measure the selected product path and relevant wall time, phase time, startup, throughput, edit/query latency, peak/retained memory, allocations, bytes copied/parsed/rendered/serialized/hashed/generated, repeated traversals, scale behavior, generated-code size, and failure paths.

Before a comparison, state the hypothesis, equivalent semantics, workload, environment, build/cache state, sample protocol, selection criterion, and reversal condition. Prefer structural work counters over noisy timings when they answer the question. Keep raw samples outside Git; commit compact reproducible evidence. Do not use noisy developer-machine timings as correctness gates.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code size, complexity, test burden, and maintenance. Do not add a validity quota to hide a performance defect or a framework to avoid one local scan.

## 8. Work selection and multi-turn discipline

At the start of a substantial task:

1. inspect branch, worktree, upstream state, and recent history without destroying unrelated changes;
2. read the relevant authority documents;
3. trace producers, consumers, semantic authority, ownership, trust boundaries, and failure paths;
4. identify the highest-leverage dependency-closed problem supported by evidence;
5. state a falsifiable hypothesis;
6. establish a focused baseline or semantic characterization;
7. define completion, reversal, and stop conditions; and
8. implement the smallest coherent correction that removes the root cause.

The task owns the objective, not an unverified proposed mechanism. Change course when evidence invalidates the suggested implementation.

Every turn must leave the repository coherent, documented, tested, and usable. Do not leave two active architectures, a half-completed cutover, disabled correctness checks, required unfinished migration, future prose described as current, a compatibility layer hiding an incomplete replacement, or dependence on a scratch artifact.

Complete one coherent vertical, update the roadmap, report the next problem, and stop. Do not automatically begin the next roadmap item because time remains.

Do not commit task prompts, checkpoints, transcript summaries, completion capsules, copied context, active archives, or agent handoff files.

Preserve unrelated work. Do not use destructive reset, checkout, clean, history rewrite, or force push against work you did not create.

Commit cohesive changes. Push the current branch without force when the task requests publication and the environment permits it. Verify branch/upstream state before claiming success.

## 9. Structure, tests, and documentation

Organize by coherent responsibility, not counts or symmetry. A crate boundary should represent a real trust/unsafe boundary, independently useful library, supported target, measured compile isolation, or low-coupling subsystem. Merge crates that mainly exchange internal descriptors, witnesses, re-exports, or compatibility adapters.

Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny modules, and redundant model/conversion layers when recombination improves comprehension. Do not reorganize unrelated code for aesthetics.

Use mature dependencies when they remove substantial custom machinery or risk. Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better.

Before adding a descriptor, registry, witness, identity, plan, contract, cache, or conversion layer, identify its current producer, consumer, lifetime, boundary, invalidation, and deletion condition.

Tests should protect semantic laws, type/effect/capability/ownership behavior, completeness, stable identity and revisions, safety boundaries, failure atomicity, deterministic behavior, exactly-once effects, stack safety, scale behavior, product behavior, and generic/specialized equivalence.

Add a focused regression test for every root cause. Test malformed input, stale/foreign identity, stale revision, type mismatch, cancellation/resource failure where injectable, partial publication, cleanup, native decline/entered failure, deep input/destruction, and host errors as relevant.

Use generated fixtures for scale. Keep fast default tests separate from ignored locked-release stress geometry, while exercising the same algorithm at smaller sizes. Use differential, property, model, or test-only reference implementations when an independent oracle is cheap.

Delete tests whose purpose is preserving provisional syntax, old serialized bytes, obsolete APIs, deleted machinery, arbitrary limits, internal topology, or accidental details. Replace them with tests of intended invariants. Never weaken a test merely to make a redesign pass.

Keep active documentation small, non-overlapping, and truthful:

- `README.md`: product introduction and first successful use;
- `docs/spec/`: intended external semantics and target contracts;
- `docs/status.md`: implemented behavior and known gaps;
- `docs/architecture.md`: current responsibilities, data flow, ownership, and trust boundaries;
- `docs/performance.md`: method, reproducible workloads, compact evidence, and reversal conditions;
- `docs/roadmap.md`: only `Now`, `Next`, and `Later`; and
- `docs/decisions/`: sparse durable decisions.

Update the owning document and delete stale text. Do not add prose digests, global revisions, fact shards, generated inventories, copied tables, per-commit evidence records, transcripts, handoffs, prompt archives, completion capsules, or duplicate roadmaps.

Write a decision record only when a decision is durable, non-obvious, expensive to rediscover, and has a meaningful reversal condition. Do not describe a future architecture as current.

## 10. Verification and definition of done

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

Run additional focused release stress, benchmark, differential/property, small-stack/deep-input, malformed-boundary, cancellation/allocation-failure/publication-atomicity, Miri, sanitizer, fuzz, and documentation/example checks when relevant.

If the environment prevents a command, report the exact command, failure category, and successful evidence that remains. Do not silently substitute a weaker command. Do not claim a command passed unless it ran after the final relevant change, or a push succeeded without verifying branch/upstream state.

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and the displaced path is gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning specification;
- no fake semantic data or arbitrary validity limit substitutes for a representation or algorithmic correction;
- failure cannot partially publish state, consume identities, or duplicate effects;
- focused tests cover the changed invariant and important failure paths;
- performance claims have reproducible equivalent evidence;
- active documentation describes the checkout truthfully;
- final relevant verification ran after the final change;
- intended changes are committed and push state is verified when requested; and
- the repository is coherent for the next independent turn.

The final report must separate implemented work, semantic/architectural changes, important deletions, measurements, tests and exact commands, commit/push state, untested paths, remaining risks, deliberately deferred work, and the next highest-leverage problem.

Do not describe a plan as implementation, a hypothesis as measurement, a noisy sample as a stable result, or an intentionally deleted subsystem as supported.

Stop after one coherent vertical.
