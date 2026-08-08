# AGENTS.md

## 1. Scope

This file applies to the entire repository.

Write code, comments, diagnostics, tests, documentation, commit messages, and final reports in English unless the current task explicitly requires another language for a user-facing artifact.

The user authorizes autonomous technical decisions, incompatible changes, destructive simplification, representation replacement, crate and file reorganization, and deletion of obsolete work. Backward compatibility is not a goal unless a current task explicitly makes it one.

Do not ask the user to select among technical options when repository evidence, semantics, experiments, profiling, or a reversible assumption can decide. Ask only when a genuinely external requirement is missing and no safe assumption can unblock the work.

## 2. Authority and truth

Follow `docs/authority.md` for claim ownership. In summary:

1. the current task owns the work to perform now;
2. this file owns engineering procedure and decision discipline;
3. accepted files under `docs/spec/` own intended externally visible semantics;
4. code, tests, manifests, schemas, and command definitions own current checkout behavior;
5. `docs/status.md`, `docs/architecture.md`, `docs/performance.md`, and `docs/roadmap.md` own their narrowly stated descriptive roles; and
6. Git history owns superseded implementation and prose.

When artifacts conflict, classify the claim, inspect its owning artifact and executable evidence, then update or delete stale material in the same change.

Do not create another authority layer from revisions, digests, registries, evidence ledgers, closure graphs, generated inventories, copied tables, or agent handoff documents.

Current architecture is not sacred. Specifications may be revised when the task and long-term mission justify a semantic change. When behavior changes intentionally, update the owning specification and delete incompatible implementation instead of preserving a compatibility path.

## 3. Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance programming language and implementation.

AI-primary means that agents can discover, understand, transform, validate, and compile programs through deterministic, precise, compact, composable interfaces. It does not mean that model inference belongs in the compiler, runtime, validator, or correctness boundary.

Optimize for:

- semantic locality rather than repository-wide context dumps;
- stable logical identity rather than fragile textual coordinates;
- typed atomic edits rather than uncontrolled string replacement;
- explicit incomplete states rather than fabricated executable placeholders;
- revision-labelled deterministic queries and diagnostics;
- compact results with selective expansion, stable ordering, and honest pagination;
- direct compilation from authoritative semantic state;
- low round-trip count and useful failure explanations;
- human-reviewable projections and diffs; and
- offline deterministic validation.

Do not optimize only for the tokenization quirks of current models. Do not hide authoritative semantics in an opaque binary merely to call the system AI-native. Humans must retain understandable diagnostics, reviewable text projections, reproducible builds, conventional diffs where useful, and the ability to inspect every authoritative mechanism.

Final execution speed matters. So do compilation and edit latency, startup, peak and retained memory, allocation, copying, code size, binary size, cache behavior, failure behavior, and maintenance cost.

## 4. Decision order

When goals conflict, use this order:

1. semantic correctness, memory safety, and real security boundaries;
2. one simple, coherent active system;
3. measured evidence from the actual product path;
4. usefulness to AI-driven development;
5. end-to-end performance and resource efficiency;
6. maintainability, observability, and ease of deletion or replacement;
7. speculative future flexibility.

Preserve future options by keeping current mechanisms small, explicit, tested, and replaceable. Do not preserve options by implementing several hypothetical futures now.

Prefer deleting work to organizing it, a direct data flow to a generalized framework, and one complete vertical to broad scaffolding.

## 5. No compatibility burden and one active system

Maintain one active:

- language definition;
- source/semantic authority;
- compiler path;
- production execution policy;
- package model;
- documentation authority model; and
- roadmap.

When replacing a mechanism, prefer a direct cutover. Delete the displaced path, adapters, aliases, migrations, compatibility tests, feature flags, and stale prose in the same dependency-closed change.

Do not create permanent:

- `v2`, `next`, `legacy`, `archive`, or `compat` implementations;
- old and new parsers, semantic authorities, compilers, or runtimes kept in parity;
- translation layers whose primary purpose is preserving provisional formats;
- deprecated syntax aliases or bytecode/snapshot migrations without a current external data requirement;
- public stability promises for syntax, CLI shape, package files, bytecode, serialized snapshots, cache keys, protocols, manifests, or internal APIs; or
- abstractions that mainly keep an obsolete abstraction alive.

Git history is the migration record for discarded experimental work.

A small independent reference evaluator may remain when it provides a valuable semantic oracle. It is not automatically a second product engine.

Names have no authority. Preserve, merge, split, rename, or delete any crate or component according to current cohesion, safety, platform, compile-time, and performance evidence.

## 6. Anti-overengineering gate

Start from a demonstrated current problem or an explicit current product requirement, not from an imagined platform.

A new abstraction must perform at least one concrete job now:

- remove demonstrated duplication or repeated work;
- make an important invalid state unrepresentable;
- isolate a real trust, unsafe, FFI, platform, ownership, or build boundary;
- expose an independently useful current API;
- enable a measured performance property; or
- substantially simplify testing and reasoning.

It must also be smaller than the problem it replaces, have a clear owner and consumer, and make deletion or replacement possible. Otherwise keep the logic local or do not add it.

Do not build speculative:

- daemon or service frameworks;
- persistence, journaling, distributed storage, CRDT, consensus, or replication;
- schedulers, resource topologies, NUMA policy, process-cell systems, or custom execution fabrics;
- plugin frameworks or universal registries;
- general cache or incremental-computation frameworks before a measured edit/rebuild problem;
- proof, witness, certificate, or content-identity ecosystems without a real trust/cache/transfer boundary;
- wire protocols without a real process consumer;
- backend or target matrices without supported product targets;
- multi-tier JIT policy, optimization pipelines, or speculative native artifacts without equivalent product measurements;
- custom allocators, memory managers, or schedulers before a measured need; or
- generalized taxonomies for one local distinction.

Do not solve complexity by adding bookkeeping around the complexity. First delete unused work, remove repeated scans/reconstruction, and simplify ownership.

Do not pursue ideological goals such as zero dependencies, zero unsafe code at any cost, zero GC at any cost, maximum crate count, minimum crate count, or a particular source representation. Decide from semantics, safety, measured performance, and total complexity.

Do not impose numeric file length, directory width, directory depth, module count, plan count, or repository-shape policies. Split and merge by cohesion, ownership, retrieval quality, testability, compile isolation, and real boundaries.

## 7. Work selection and multi-turn discipline

At the start of a substantial task:

1. inspect the branch, worktree, upstream state, and recent history without destroying unrelated changes;
2. read the relevant authority documents;
3. trace the actual producer/consumer path and ownership/trust boundaries;
4. identify the highest-leverage dependency-closed problem supported by evidence;
5. state a falsifiable hypothesis;
6. establish a focused baseline or semantic characterization;
7. define completion, reversal, and stop conditions; and
8. implement the smallest coherent correction.

Do not spend the task constructing an elaborate hierarchy of planning files. Keep scratch plans in agent working state, not as product authority. Do not commit task prompts, checkpoints, transcript summaries, completion capsules, or active archives.

For multi-turn work, every turn must leave the repository coherent, documented, tested, and usable. Do not leave two active architectures, a half-completed cutover, disabled correctness checks, or prose that describes uncommitted future behavior.

Complete one coherent vertical, update the roadmap, report the next problem, and stop. Do not automatically begin the next roadmap item because time remains.

If evidence invalidates the task's suggested implementation, change course. The task owns the objective, not an unverified mechanism.

## 8. AI-facing semantic design

Maintain one syntax-independent semantic authority when the accepted workspace specification requires it. Text may be a persistent import/export, interoperability, review, and debugging representation without becoming a second compiler authority.

Evaluate AI-facing changes by whether an agent can:

- locate the relevant semantic slice without loading the entire repository;
- refer to entities stably across unrelated presentation changes;
- query actual and expected meaning at a named revision;
- receive compact headers first and expand only selected detail;
- propose a typed batch of edits with explicit preconditions;
- receive deterministic diagnostics, semantic diff, and invalidation information;
- preserve useful incomplete states without pretending they are executable; and
- compile a complete snapshot without rendering and reparsing text.

Correctness must remain deterministic and local. A model may propose an operation but must never decide whether it is valid.

Do not add persistence, collaboration, a protocol, a daemon, or remote execution merely because semantic snapshots exist. Prefer an in-process API until a measured consumer justifies a process boundary and its resource/security policy.

Do not duplicate semantic facts across HIR, indexes, attachments, caches, and projections without clear ownership and invalidation. Derived state must be reproducible from its owner or explicitly versioned at a real boundary.

## 9. Semantic validity and host resources

Language meaning is determined by semantics, not by project-selected size quotas.

Do not make an otherwise valid trusted program invalid because it exceeds an arbitrary count of tokens, bytes, nesting, declarations, fields, variants, parameters, arguments, locals, functions, files, modules, blocks, edges, IR nodes, identities, values, cleanup records, diagnostics, or analysis steps.

Do not disguise such a limit by raising it, widening its integer, moving it to another phase, renaming it, or calling it a safety profile.

Trusted local work may end because of success, explicit cancellation, allocation failure, operating-system/I/O failure, a genuine external representation boundary, or another real host failure.

Untrusted products may impose explicit coarse policy for input bytes, memory, output, time, cancellation, and concurrency. Exhaustion is a typed host-resource result, not a semantic error. Do not define an untrusted policy before an untrusted product exists.

Use checked arithmetic and checked narrowing for sizes, offsets, identities, and indexes. Compact representations must decline or fall back before they restrict an otherwise valid generic program.

User-controlled depth must not consume unbounded native stack. Prefer iterative traversal or an explicitly heap-backed work stack. A private segment size is tuning, not a language-depth limit.

Never silently truncate a result claimed as complete. Stream, paginate, return a continuation, mark it partial, or fail explicitly.

## 10. Performance discipline

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
- semantic edit/query/compile latency; and
- failure, cancellation, and cleanup paths.

Before a comparison, state the hypothesis, equivalent semantics, workload, machine/toolchain, profile, cache state, sample protocol, selection criterion, and reversal condition.

Prefer, in order:

1. delete work with no current consumer;
2. fix poor asymptotic complexity and repeated reconstruction;
3. avoid unnecessary whole-program clones and unconditional representations;
4. improve data layout and locality;
5. reduce allocation, copying, hashing, and serialization;
6. add a narrow cache or incremental mechanism only for a measured repeated computation;
7. add parallelism only when the remaining work is large, separable, and worth synchronization; and
8. add target-specific specialization only behind a generic correct path and measured product need.

Do not add a validity quota to hide a performance defect. Do not add a general framework to avoid one local scan. Do not optimize solely for one generated fixture; cross-check representative and adversarial behavior.

Use complexity/work counters when they provide more stable evidence than timing, but keep them local and purposeful. Do not create a global phase ledger.

Keep reproducible harnesses. Store raw profiles and samples outside Git or in CI artifacts. Commit only compact results and the decisions they support. Do not make noisy developer-machine timings hard correctness gates.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code size, complexity, test burden, and maintenance.

## 11. Safety, validation, determinism, and failure atomicity

Preserve genuine trust boundaries while removing internal ceremony.

Validate fail-closed at untrusted boundaries such as:

- text/source and semantic-operation input;
- packages, paths, imports, manifests, and locks;
- serialized, persisted, cached, or transferred data;
- bytecode or executable artifacts loaded from outside trusted construction;
- capabilities and host operations;
- relocation and executable-memory installation;
- generated entry points and FFI; and
- operating-system, network, terminal, filesystem, and database interfaces.

Inside one synchronous trusted pipeline, validated typed values carry authority. Do not repeatedly serialize, hash, reconstruct, independently verify, or bind identities to the same value unless a real cache, transfer, persistence, executable-artifact, or threat boundary consumes the result.

Unsafe code belongs in a narrow named mechanism with a documented safe-caller contract, explicit invariants, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or property coverage. Do not spread unsafe code to save trivial wrapper cost; do not build layers of ceremony around an already narrow safe abstraction.

Publication must be failure-atomic. Validation failure, cancellation, allocation failure, I/O failure, backend failure, or resource exhaustion must preserve the previous published snapshot, cache entry, executable mapping, or durable state.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling, allocation addresses, hash-table state, and cache state must not change completed program meaning or which deterministic diagnostic wins.

## 12. Repository and component structure

Organize by coherent responsibility, not counts or aesthetic symmetry.

A crate boundary should correspond to at least one real property:

- a trust or unsafe boundary;
- an independently useful library;
- a distinct supported build target or platform;
- measured compile isolation; or
- a low-coupling subsystem with clear ownership.

Merge crates that mainly exchange internal descriptors, digests, witnesses, re-exports, or compatibility adapters. Split a crate only when a real boundary becomes clearer.

Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny modules, and redundant model/conversion layers when recombination improves comprehension. Keep a larger file when it is one coherent mechanism; split a smaller file when responsibilities are genuinely independent.

Do not reorganize unrelated code merely to make the tree look cleaner. Structural work must remove current complexity or support the active vertical.

Use mature dependencies when they remove substantial custom machinery or risk. Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better. Pinning and supply-chain policy must be deliberate, not a zero-dependency badge.

## 13. Tests

Tests should protect semantic laws, safety boundaries, failure atomicity, deterministic behavior, stack safety, scale behavior, and selected product behavior.

Add focused regression tests for every root cause fixed. Characterize subtle current semantics before replacing their implementation.

Use generated fixtures for scale. Keep fast default tests separate from explicitly ignored release stress geometry when necessary, but ensure ordinary tests exercise the same algorithm on smaller inputs.

Use differential, property, model, or test-only reference implementations where an independent oracle is cheap. A reference path must not become a second production architecture.

For performance corrections, prefer structural tests and work-shape assertions over unstable wall-clock CI gates. Retain a reproducible release harness for development evidence.

Delete tests whose primary purpose is preserving:

- provisional syntax or CLI compatibility;
- old serialized bytes, cache identities, or manifests;
- obsolete engine parity;
- arbitrary count limits;
- deleted platform machinery;
- internal file topology; or
- accidental implementation details.

Never weaken a test merely to make a redesign pass. Replace it with a test of the intended invariant.

## 14. Documentation

Keep active documentation small, non-overlapping, and truthful.

- `README.md` introduces the product and first successful use.
- `docs/spec/` owns intended external semantics and target contracts.
- `docs/status.md` reports implemented behavior and known gaps.
- `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust boundaries.
- `docs/performance.md` records method, reproducible workloads, compact evidence, and reversal conditions.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later` ordering.
- `docs/decisions/` contains only sparse durable decisions.

Update the document that owns a changed claim. Delete stale text rather than retaining it as active history.

Do not add prose digests, global platform revisions, fact shards, closure graphs, generated inventories, copied Cargo/CLI/schema tables, per-commit evidence records, task transcripts, or committed agent handoffs.

Write an ADR only when the decision is durable, non-obvious, expensive to rediscover, and has a meaningful reversal condition. Most implementation choices do not need an ADR.

Do not document a future architecture as current. Clearly label a target contract when it intentionally leads implementation.

## 15. Change protocol

For a substantial change:

1. inspect `git status`, branch, upstream state, and recent history;
2. preserve unrelated work;
3. read the relevant spec, status, architecture, performance, and roadmap sections;
4. trace producers, consumers, ownership, trust boundaries, and failure paths;
5. establish a focused baseline or semantic characterization;
6. implement the simplest coherent correction;
7. delete the displaced path in the same vertical;
8. add focused correctness, malformed-input, failure, stack, and scale tests;
9. measure when performance is part of the claim;
10. update the owning documentation;
11. run focused verification while iterating;
12. run full relevant verification after the final change;
13. inspect the final diff for duplicate architecture, stale references, unchecked narrowing, accidental compatibility, benchmark-specific paths, and speculative machinery;
14. commit cohesive changes; and
15. push the current branch without force when the task and environment permit.

Do not use destructive reset, checkout, clean, history rewrite, or force push against work you did not create.

Do not claim a command passed unless it ran after the final relevant change. Do not claim a commit was pushed without verifying branch/upstream state.

## 16. Standard verification

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
- malformed decoder or transaction tests;
- cancellation, allocation-failure, and publication-atomicity tests;
- Miri;
- ASan, LSan, or TSan;
- fuzzing; and
- documentation link/example checks.

When an environmental failure prevents a command, report the exact command, failure category, and successful evidence that remains. Do not silently substitute a weaker verification command.

## 17. Definition of done

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and the old path is gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning specification;
- no arbitrary validity limit substitutes for an algorithmic correction;
- failure cannot partially publish state;
- focused tests cover the changed invariant and important failure paths;
- performance claims have reproducible equivalent evidence;
- active documentation describes the checkout truthfully;
- final relevant verification has run after the final change;
- intended changes are committed and push state is verified when publication is requested; and
- the repository is left coherent for the next independent turn.

The final report must separate:

- implemented work;
- measured results;
- important deletions and replacements;
- tests and commands run;
- commit and push state;
- untested environments or paths;
- remaining risks;
- deliberately deferred work; and
- the next highest-leverage problem.

Do not describe a plan as implementation, a hypothesis as measurement, a single noisy sample as a stable result, or an intentionally deleted subsystem as still supported.
