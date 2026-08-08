# AGENTS.md

## 1. Scope

This file applies to the entire repository.

Write code, comments, diagnostics, documentation, tests, commit messages, and final reports in English unless a current task explicitly requires another language for a user-facing artifact.

The user authorizes autonomous technical decisions, destructive simplification, incompatible changes, crate and file reorganization, representation replacement, and deletion of obsolete work. Backward compatibility is not a goal unless the current task explicitly makes it one.

Do not ask the user to choose between implementation options when correctness, repository evidence, experiments, or profiling can decide. Ask only when a genuinely external requirement is missing and no safe, reversible assumption can unblock the work.

## 2. Authority

Use the artifact that owns the kind of claim being evaluated:

1. The current task owns what to do now.
2. This file owns engineering procedure and decision discipline.
3. Accepted files under `docs/spec/` own intended externally visible semantics.
4. Code, tests, manifests, schemas, and command definitions own what the current checkout does.
5. `docs/status.md` summarizes current behavior and known gaps.
6. Reproducible harnesses and `docs/performance.md` own performance evidence.
7. `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust boundaries.
8. `docs/roadmap.md` owns only current ordering: `Now`, `Next`, and `Later`.
9. Sparse accepted decisions under `docs/decisions/` own durable rationale when such a decision exists.
10. Git history owns superseded implementation and prose.

This file must not become a second language specification, architecture inventory, status report, roadmap, or exhaustive design document.

When artifacts conflict:

1. classify the claim;
2. inspect the artifact that owns it;
3. inspect executable evidence;
4. update or delete stale material in the same change; and
5. record a durable decision only when rediscovering the reasoning would be costly.

Do not manufacture a global authority system from revisions, hashes, registries, closure graphs, copied tables, or agent-produced evidence ledgers.

## 3. Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance programming language and implementation.

AI-primary means that an agent can understand and change programs through deterministic, precise, compact, composable interfaces. It does not mean that model inference belongs in the compiler, runtime, validator, or correctness boundary.

Optimize for:

- semantic locality instead of repository-wide context dumps;
- stable identities instead of fragile textual coordinates;
- typed atomic edits instead of uncontrolled string replacement;
- revision-labelled queries and diagnostics;
- compact summaries with selective expansion;
- explicit legal operations and failure reasons;
- deterministic validation that works offline;
- low round-trip count;
- reviewable semantic and textual projections; and
- complete, truthful results or explicit pagination and partial-result markers.

Humans must retain understandable diagnostics, useful text projections, reviewable diffs, reproducible builds, and the ability to inspect every authoritative mechanism.

Final execution speed matters, but so do edit latency, compilation latency, startup, peak memory, allocation, copying, generated-code size, binary size, cache behavior, and maintenance cost.

## 4. Decision order

When goals compete, use this order:

1. semantic correctness, memory safety, and real security boundaries;
2. one simple and coherent active system;
3. measured evidence from the current product path;
4. usefulness to AI-driven development;
5. end-to-end performance and resource efficiency;
6. maintainability and ease of deletion or replacement;
7. speculative future flexibility.

Long-term quality is not achieved by implementing every plausible future subsystem. Preserve future options by keeping current mechanisms small, explicit, well-tested, and replaceable.

## 5. One active system and no compatibility burden

Maintain one active:

- language definition;
- semantic program authority;
- compiler path;
- production execution policy;
- documentation authority model; and
- roadmap.

When replacing a mechanism, prefer a direct cutover. Delete the displaced path, adapters, compatibility tests, feature flags, and stale prose in the same dependency-closed change.

Do not create permanent:

- `v2`, `next`, `legacy`, `archive`, or `compat` implementations;
- shadow parsers, shadow compilers, or dual source authorities;
- old and new runtime products kept in parity;
- translation layers whose main purpose is preserving provisional formats;
- public compatibility promises for current syntax, bytecode, serialized snapshots, cache keys, protocols, manifests, or internal APIs; or
- abstractions that exist primarily to keep an obsolete abstraction alive.

A small reference evaluator may remain when it is a useful semantic oracle. It is not automatically a second product engine.

Names and historical components have no special authority. In particular, `lkjscript-sys` is an ordinary implementation component: preserve, merge, split, rename, or delete it according to present cohesion, safety, portability, and performance evidence. Do not change it merely for aesthetics, and do not preserve it merely because earlier plans used its name.

## 6. Anti-overengineering rules

Start from a demonstrated current problem, not from an imagined platform.

A new abstraction must do at least one concrete job now:

- remove demonstrated duplication;
- make an important invalid state unrepresentable;
- isolate a real trust, unsafe, platform, or ownership boundary;
- provide an independently useful API;
- enable a measured performance property; or
- substantially simplify testing and reasoning.

Otherwise keep the logic local or do not add it.

Do not build speculative:

- daemon or service frameworks;
- distributed stores, CRDTs, consensus, or replication;
- schedulers, topology models, NUMA policy, or process-cell systems;
- plugin frameworks or universal registries;
- general cache frameworks before a measured cache problem;
- proof or certificate ecosystems without a real trust boundary;
- wire protocols without a real process consumer;
- persistence layers without a measured recovery or retained-scale need;
- backend matrices without supported product targets; or
- generalized taxonomies for a single local distinction.

Do not solve complexity with more bookkeeping around the complexity. Delete redundant work first.

Do not impose numeric file length, directory width, directory depth, module count, or repository shape policies. Split and merge by cohesion, ownership, testability, retrieval quality, compile isolation, and platform boundaries.

Do not pursue a zero-dependency badge. Use a mature dependency when it removes substantial custom machinery or risk. Keep owned code when it is smaller, clearer, safer, or measurably better. Decide by evidence, not ideology.

Do not add parallelism, caching, interning, incremental computation, custom allocation, or custom scheduling before identifying the actual cost and the simplest algorithmic correction.

## 7. Selecting work

At the start of a substantial task:

1. inspect the current branch and worktree without destroying unrelated changes;
2. inspect recent history;
3. read the relevant authority documents;
4. trace the actual code path and its consumers;
5. identify the highest-leverage dependency-closed problem supported by evidence;
6. state a falsifiable hypothesis;
7. establish a focused baseline; and
8. define completion and reversal conditions.

Then implement. Do not spend the task constructing an elaborate planning hierarchy.

Prefer one complete vertical that removes a root cause over broad scaffolding for several future phases.

A task-local checklist may exist in the agent's working notes. Do not commit scratch plans, checkpoints, transcript summaries, or an active archive as product authority.

For multi-turn work, every turn must leave the repository in a coherent state. Do not leave two active architectures, a half-completed cutover, disabled correctness checks, or documentation that describes an uncommitted future.

## 8. Semantic validity and host resources

Program meaning is determined by language semantics, not by project-selected size quotas.

Do not make an otherwise valid trusted program invalid because it exceeds an arbitrary count of tokens, bytes, nesting, declarations, fields, variants, parameters, arguments, locals, functions, blocks, edges, IR nodes, identities, runtime values, diagnostics, files, modules, or analysis steps.

Do not disguise such a limit by raising it, widening its integer, moving it to a later phase, renaming it, or calling it a safety profile.

Trusted local work may end because of:

- success;
- explicit cancellation;
- allocation failure;
- operating-system or I/O failure;
- a genuine external representation boundary; or
- another real host failure.

Untrusted requests may have explicit coarse host policy for input bytes, memory, output, time, cancellation, and concurrency. Exhausting that policy is a typed host-resource result, not a semantic error.

Use checked arithmetic and checked narrowing for sizes, offsets, identities, and indexes. Keep compact representations behind a wide or generic fallback when they would otherwise restrict ordinary valid programs.

User-controlled depth must not consume unbounded native stack. Prefer iterative traversal or an explicitly heap-backed work stack.

Never silently truncate a result claimed as complete. Stream, paginate, return a continuation, mark the result as partial, or fail explicitly.

## 9. Semantic workspace and AI-facing interfaces

Follow the accepted workspace specification rather than duplicating it here.

Evaluate workspace and tooling changes by whether an agent can:

- discover the relevant semantic slice without loading the whole repository;
- refer to entities stably across unrelated presentation changes;
- query actual and expected meaning at a named revision;
- propose a typed batch of changes;
- receive deterministic diagnostics, semantic diff, and invalidation information; and
- compile a complete semantic snapshot without a text round trip.

Do not create another source authority. Text may remain an important import, export, debugging, interoperability, and review representation.

Do not hide authoritative semantics in an opaque blob merely to call the system AI-native.

Do not add persistence, collaboration, a protocol, a daemon, or remote execution until a measured consumer justifies the boundary. An in-process API is preferable while it serves the actual product.

## 10. Performance discipline

Profile before optimizing. Measure the selected product path, not a disconnected microbenchmark alone.

For relevant work, consider:

- end-to-end wall time;
- phase time;
- peak resident memory;
- allocation count and bytes;
- retained memory;
- bytes copied, serialized, and hashed;
- repeated whole-program traversals;
- code generation and installation cost;
- generated-code size;
- release binary size;
- cold and warm behavior;
- agent query and transaction latency; and
- failure and cancellation paths.

Before a comparison, state:

- the hypothesis;
- equivalent semantics;
- workload;
- machine and toolchain;
- sample protocol;
- selection criteria; and
- reversal condition.

Prefer, in order:

1. deleting work that has no consumer;
2. fixing poor asymptotic complexity;
3. avoiding whole-program clones and repeated reconstruction;
4. avoiding unconditional representations;
5. improving data layout and locality;
6. reducing allocation and copying;
7. adding narrowly justified caching or incremental work;
8. adding parallelism only when the remaining work is large and separable.

Never restore a validity quota to hide a performance defect.

Keep reproducible harnesses. Store raw output outside Git or in CI artifacts. Commit only compact results and the decisions they support.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code size, complexity, testing, and maintenance.

## 11. Validation, safety, and determinism

Preserve genuine trust boundaries while removing internal ceremony.

Validate fail-closed at untrusted boundaries such as:

- source and semantic-operation input;
- packages, paths, and imports;
- serialized or persisted data;
- bytecode or executable artifacts loaded from outside trusted construction;
- capabilities and host operations;
- relocation and executable-memory installation;
- generated entry points and FFI; and
- operating-system and database interfaces.

Inside one synchronous trusted pipeline, validated typed values should carry authority. Do not repeatedly serialize, hash, reconstruct, and independently verify the same value unless a real consumer, cache boundary, transfer boundary, or threat model requires it.

Unsafe code belongs in a narrow named mechanism with a documented safe caller contract, explicit invariants, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or property coverage.

Publication must be failure-atomic. Validation failure, cancellation, allocation failure, I/O failure, backend failure, or resource exhaustion must preserve the previous published snapshot, cache entry, executable mapping, or durable state.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling and cache state must not change completed program meaning.

## 12. Repository structure

Organize by coherent responsibility, not by counts.

A crate boundary should correspond to at least one real property:

- a trust or unsafe boundary;
- an independently useful library;
- a distinct build target or platform;
- measured compile isolation; or
- a low-coupling subsystem with clear ownership.

Merge crates that primarily exchange internal descriptors, digests, witnesses, re-exports, or compatibility adapters. Split a crate when a real boundary becomes clearer.

Remove numbered implementation shards, one-child directory ladders, include-only facades, and artificial fragments when recombination improves comprehension.

Do not reorganize unrelated code merely to make the tree look cleaner. Structural work must reduce current complexity or support the active vertical.

## 13. Documentation

Keep active documentation small, non-overlapping, and truthful.

- `README.md` introduces the product and first successful use.
- `docs/spec/` owns intended semantics.
- `docs/status.md` reports implemented behavior and known gaps.
- `docs/architecture.md` explains current responsibilities and boundaries.
- `docs/performance.md` records method and compact evidence.
- `docs/roadmap.md` contains only `Now`, `Next`, and `Later`.
- `docs/decisions/` contains only sparse durable decisions.

Update the document that owns a changed claim. Delete stale text rather than preserving it as active history.

Do not add prose digests, global platform revisions, fact shards, closure graphs, completion capsules, evidence records for every commit, copied Cargo graphs, or committed agent handoffs. Use executable sources and Git history.

Write an ADR only when the decision is durable, non-obvious, expensive to rediscover, and has a meaningful reversal condition.

## 14. Tests

Tests should protect semantic laws, safety boundaries, failure atomicity, deterministic behavior, and selected product behavior.

Add focused regression tests for every root cause fixed.

Use generated fixtures for scale. Keep fast default CI separate from explicitly ignored stress geometry when necessary, but ensure ordinary tests exercise the same algorithm on smaller input.

Use differential or property testing where independent semantics exist.

Delete tests whose main purpose is preserving:

- provisional syntax compatibility;
- old serialized bytes;
- obsolete engine parity;
- arbitrary count limits;
- deleted platform machinery;
- internal file topology; or
- accidental implementation details.

Never weaken a test merely to make a redesign pass. Replace it with a test of the intended invariant.

## 15. Change protocol

For a substantial change:

1. inspect `git status`, the branch, and recent history;
2. preserve unrelated work;
3. read the relevant spec, status, architecture, performance, and roadmap sections;
4. trace producers, consumers, ownership, and trust boundaries;
5. run a focused baseline;
6. implement the simplest coherent correction;
7. delete the displaced path in the same vertical;
8. add focused correctness, malformed-input, failure, and scale tests;
9. measure when performance is part of the claim;
10. update owning documentation;
11. run focused verification while iterating;
12. run the full relevant verification after the final change;
13. inspect the final diff for duplicate architecture, stale references, unchecked narrowing, accidental compatibility, and new speculative machinery;
14. commit cohesive changes; and
15. push without force when the task and environment permit.

Do not use destructive reset, checkout, clean, or force-push operations against work you did not create.

Do not claim a command passed unless it ran after the final relevant change.

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

- focused release stress tests;
- differential tests;
- Miri;
- ASan, LSan, or TSan;
- fuzzing;
- property tests;
- malformed decoder tests;
- cancellation and allocation-failure tests;
- release benchmarks; and
- documentation link or example checks.

When an environmental failure prevents a command, report the exact command, failure category, and the evidence that still succeeded.

## 17. Definition of done

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and the old path is gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning specification;
- no arbitrary validity limit substitutes for an algorithmic fix;
- failure cannot partially publish state;
- focused tests cover the changed invariant;
- performance claims have reproducible evidence;
- active documentation describes the checkout truthfully;
- the final relevant verification has run; and
- the report clearly separates implemented, measured, untested, planned, and intentionally deleted work.

The final report must include:

- the decision and why it was selected;
- important deletions and replacements;
- exact measurements and comparison protocol;
- tests and commands run;
- commit and push state;
- remaining risks;
- work deliberately deferred; and
- the next highest-leverage problem, without beginning another speculative subsystem.
