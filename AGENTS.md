# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local procedure.
It may not weaken repository-wide requirements for semantic authority, identity, atomicity, durability, determinism, memory safety, capability safety, strict boundaries, verification, evidence, documentation truth, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

Preserve unrelated work.
Never reset, clean, overwrite, stage, commit, or force-push work that you did not create.
Do not change remote state unless the active user task explicitly requests it.
Never commit credentials, private transcripts, hidden model reasoning, or unrelated user data.

## Mission

Build `lkjscript` as a programming system designed primarily for autonomous coding agents.

Humans remain first-class users at the level of intent, explanation, governance, review, operation, and product ownership.
Humans are not expected to hand-author the authoritative program representation.

Use this plain explanation before specialized terminology:

> An agent edits a typed, versioned program model through a local service. The service validates proposed changes, saves immutable revisions, and compiles and runs selected revisions.

The formal name of the authoritative model is the **Semantic Program Graph** (`SPG`).

"Graph" describes semantic entities, stable identity, containment, ordering, and references.
It does not prescribe pointer-based storage, a graph database, or an in-memory layout.

Text, diagrams, generated source, imported syntax, tool calls, and JSON may be useful views or proposal forms.
They must resolve into the same authoritative semantic model.
They must not become a second source of truth.

The system must remain memory-safe, deterministic at observable boundaries, failure-atomic for durable publication, strict toward untrusted input, and capable of world-class long-term runtime performance.

Agent interaction cost is a first-class engineering dimension.
Measure it without weakening correctness.

## Instruction Precedence

Use this order when active artifacts disagree:

1. The active user task.
2. This root `AGENTS.md`.
3. The active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Machine descriptions derived from executable contracts.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, old commits, issues, discussions, and historical documents.

Newer active instructions and newer verified repository state outrank older prompts and assumptions.

A campaign prompt controls sequence and scope.
It does not silently redefine accepted semantics.
When a campaign changes semantics, update the owning specification in the same verified milestone.

Keep durable policy here.
Keep one-time plans, experiments, measurements, and task capsules in the campaign prompt or their existing fact-owning documents.

## Fact Ownership

Keep one owner for each maintained fact:

- `docs/spec/semantic-graph.md`: authoritative model, identity, revisions, transactions, history, and artifacts.
- `docs/spec/language.md`: types, operations, control, effects, ownership, and execution semantics.
- `docs/spec/protocol.md`: local transport, requests, responses, schema discovery, framing, and cursors.
- `docs/architecture.md`: component responsibility, trusted computing base, and trust boundaries.
- `docs/status.md`: exactly what the current checkout implements and does not implement.
- `docs/performance.md`: measurements, comparisons, regressions, and reversal evidence.
- `docs/roadmap.md`: ordered evidence gates and deferred choices.
- `README.md`: human-first product explanation and runnable entry points.
- This file: repository-wide operating policy.
- `prompts/`: campaign execution artifacts, not permanent semantic authority.

Do not maintain duplicate status catalogues, glossaries, version registries, roadmaps, schema copies, architecture inventories, or benchmark tables.

Generated machine descriptions must derive from executable contracts.
Do not commit a hand-maintained duplicate schema.

## Claims and Evidence

Classify material claims as an enduring invariant, accepted semantic contract, current verified baseline, operational policy, evidence-gated choice, experimental hypothesis, or historical fact.

Do not promote a bootstrap absence into an eternal prohibition.
Do not present a hypothesis as implemented reality.
Do not present one model run as a general benchmark.
Bytes are not tokens.
Safe Rust is not a complete proof.
Report only what the checkout and reproduced evidence support.

## Enduring Invariants

The following outrank the current module layout, protocol version, artifact format, process topology, storage engine, runtime representation, memory-management technique, implementation language, and model provider.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions are immutable.
3. Durable publication has one unambiguous logical commit authority per durable namespace.
4. Current single-head and single-writer mechanics are baselines, not eternal bans on branches, clients, replicas, or isolated workers.
5. Future branch, merge, replica, or worker designs preserve explicit parentage, deterministic conflicts, and freedom from split-brain authority.
6. Text, JSON, diagrams, generated source, drafts, tool schemas, caches, indexes, Core IR, profiles, and machine code are views, proposals, or derived state, never coequal authority.
7. Persisted semantic entities, attributes, ownership slots, child slots, and direct references belong to a closed typed schema.
8. Arbitrary property bags, arbitrary string-labelled semantic edges, and unknown semantic forms are forbidden in authority.
9. Mutation is a typed transaction or a closed typed proposal deterministically normalized into one transaction.
10. A successful commit publishes exactly one accepted revision.
11. Rejection and validate-only publish nothing and consume no persistent identities.
12. Stable semantic identity is independent of names, source positions, formatting, hashes, compiler indexes, artifact offsets, addresses, and storage keys.
13. Persistent semantic identities are never reused within their identity domain.
14. Names are lookup and presentation metadata, not identity.
15. Identity-preserving change exists only where an accepted continuity rule defines it.
16. Derived facts never become a second mutable source of truth.
17. Only a complete selected-entry reachable definition set enters executable lowering.
18. The compiler consumes an immutable revision directly.
19. One semantic execution route defines behavior; later tiers accelerate it rather than define another language.
20. AI output and natural-language intent are untrusted input, not executable authority.
21. Deterministic validators decide acceptance.
22. Host access requires explicit typed authority.
23. Accepted language semantics cannot express unchecked memory access.
24. User-controlled depth does not consume unbounded native stack.
25. Observable order is explicit and deterministic.
26. Potentially large results are bounded, streamed, or paginated.
27. Durable state is acknowledged only after the documented publication contract.
28. Corrupt, ambiguous, unsupported, or partially published authority rejects rather than being guessed into validity.
29. Human-facing claims remain no stronger than implemented and reproduced evidence.
30. Compactness never weakens typing, validation, identity, authorization, durability, or error precision.
31. Performance optimization preserves a simple correctness oracle.
32. Representative applications discover missing capabilities but do not justify speculative platforms.
33. Backward compatibility is not required unless the active user explicitly requires it.
34. Incompatible-change freedom is used to converge on one coherent path, not create churn or legacy paths.
35. Memory safety, resource exhaustion, cleanup, concurrency safety, permission security, and crash consistency remain separate contracts.
36. Tool adapters and frontends may disappear without changing program meaning.
37. Runtime optimizations may disappear without changing accepted semantics.
38. Every retained dependency has a named current consumer.
39. Every public boundary has exact versioning, bounds, rejection behavior, and output policy.
40. Every effect has explicit authority and outcome semantics; no non-idempotent effect is silently retried after a possible partial action.

## Operating Posture

The current semantic-model service is a verified baseline, not an untouchable monument.

Do not restore displaced source-oriented authority.
Do not perform a total rewrite merely because compatibility is unimportant.
Do not preserve a mechanism because it was expensive to build.
Do not delete a mechanism merely because a simpler idea sounds attractive.

Replace a subsystem when the replacement is dependency-closed, materially clearer or more capable, and verified against the enduring invariants.

When replacing an active boundary:

- replace every active reader and writer together;
- use a new unambiguous version, schema identity, tag set, or magic where old bytes would be ambiguous;
- delete displaced readers, writers, adapters, fixtures, tests, tags, and claims;
- update the owning specification and current status in the same milestone;
- retain no legacy mode, edition split, compatibility namespace, dual path, hidden fallback, or silent migration unless explicitly requested.

Git history is the archive for superseded implementation.

Current stable Rust, one package, Linux x86-64, local IPC, one durable writer, full immutable artifacts, full history, full recomputation, scan queries, one Core IR, an explicit-frame interpreter, flat cells, and pure values are baselines rather than permanent mandates.

Text frontends, tool adapters, storage engines, indexes, compaction, branches, concurrency, process isolation, packages, cross-platform support, managed heaps, ownership systems, native backends, and self-hosting remain evidence-gated.

Select an evidence-gated choice only after naming its consumer, safety obligations, correctness oracle, measured benefit, implementation cost, and reversal condition.

## Source-Independent Authority

The invariant is source independence, not hostility toward text.

An agent must be able to construct, inspect, revise, validate, compile, execute, package, and debug without preserving or round-tripping source files.

A frontend may import or render text, provide review views, exchange code, generate a typed proposal, or expose tools to a harness.

It may not bypass validation, allocate persistent identities independently, own mutable semantic state, persist a parallel AST as coequal authority, make formatting identity, or define behavior absent from the authoritative model.

Do not promise that `lkjscript` will never have syntax.
Promise that syntax will not become a second authoritative program.

## Semantic Model and Identity

The SPG is a closed typed semantic model, not a generic property graph.

Each semantic kind has explicit attributes, owner rules, ordered children, direct references, operands, results, cardinality, completeness, continuity, deletion behavior, lowering obligations, query obligations, and artifact obligations.

Unknown schema elements reject.
Do not preserve unknown semantic fields for hypothetical forward compatibility.
Evolve through explicit direct replacement.

Use one code-owned contract to supply facts to validators, codecs, queries, machine descriptions, history checks, and lowering where practical.

Do not add runtime registration for a closed vocabulary.
Do not add a general graph framework when direct types and static descriptors suffice.

Physical traversal order is not observable unless the semantic contract defines it.

Stable identity is valuable and costly.
Distinguish persistent Node IDs, transaction-local symbols, query labels, revision numbers, hashes, dense compiler IDs, runtime handles, names, and view positions.

Give persistent identity when continuity, direct targeting, sharing, repair, history, attribution, or external reference requires it.
Do not create persistent identity merely because an implementation uses a struct or a proposal needs a temporary name.
Do not remove persistent identity merely to reduce payload bytes when it materially improves repair, diffing, or maintenance.

Anonymous inline proposal terms may normalize into the same authoritative model.
They must not create a second expression language or validator.
Normalization order, allocation accounting, error paths, and canonical equivalence must be exact.

If identity granularity changes artifacts, history, diffs, repair, or fine-grained edits, treat it as a semantic-model change rather than interface cleanup.

## Incomplete Programs and Repair

Incomplete programs are valid semantic states.

Use typed placeholders or another exact accepted missing-definition form.
An incomplete definition blocks execution only when reachable from the selected entry.

Repair context is deterministic typed data.
Models remain outside the correctness authority path.

Identity-preserving repair requires an explicit one-way continuity rule.
Do not generalize one refinement into unrestricted semantic morphing.
A repairable entity remains addressable through stable identity or another equally explicit accepted mechanism.

## Transactions and Proposals

Every mutation names a workspace, exact base revision or accepted parent relation, commit or validate-only mode, optional idempotency key where supported, ordered closed proposal, and bounded response selection.

A successful commit:

1. validates the envelope;
2. resolves proposal-local references;
3. normalizes structured proposals;
4. allocates candidate identities deterministically;
5. applies canonical edits;
6. validates model and history;
7. derives deterministic change facts;
8. preflights response and durable bytes;
9. durably publishes the revision and authoritative head;
10. publishes in-memory state;
11. returns the preflighted receipt.

A rejected transaction changes none of those states.
Validate-only uses the same semantic preparation and publication preflight without writing or consuming identity.

Structured authoring may remove repeated scaffolding.
It is a closed typed proposal, not persisted authority.
Final semantic validation remains authoritative.

Prefer semantically meaningful proposals over transport-level setup operations.
Retain fine-grained edits for real maintenance.
Do not add a macro language, rewrite language, parser, template engine, or hidden model planner for one convenience case.

## Agent-Facing Interface

The external coding agent is the primary program author.
The machine interface is a product surface.

Provide a closed executable contract, stable names and IDs, compact discovery, exact on-demand expansion, revision-bound reads, meaningful proposals, selected bindings, compact receipts, typed errors, legal constructors, visible values, repair context, paginated diffs, exact run values, and digest-based unchanged results where useful.

A low-level node API is not agent-friendly merely because it is typed JSON.

Do not require agents to author private compiler indexes, CFG details, artifact offsets, checksums, publication records, cache keys, or durability metadata.

Prefer standard inspectable control-plane representations until measurement justifies custom transport.
One request and response vocabulary should serve CLI, service, tests, and machine description.

Do not invent compact syntax from intuition.
Evaluate interface changes with equal-task agent trials and deterministic oracles.

## Tool Adapters and Context Economics

A Codex-native, MCP, app-server, shell, library, or other harness adapter is optional projection infrastructure.
It is not program authority.
It must invoke or derive from the executable machine contract rather than maintain a second accepted vocabulary.

Keep retained tools small, stable, deterministic, and semantically meaningful.
Do not expose one tool per internal node kind.
Do not change tool names, order, descriptions, or schemas casually.
Dynamic tool refresh must be explicit and deterministic.

Do not add a large protocol stack, async runtime, SDK, or dependency tree solely for branding.
Prototype outside the retained product path.
Retain an adapter only when equal-task evidence justifies implementation, lifecycle, security, and context cost.

Measure policy bytes, campaign bytes, machine-contract bytes, tool definitions, request and response bytes, provider-reported token classes and price when available, tool calls, round trips, launches, failures, repeated discovery, searches, files opened, elapsed time, implementation size, and verification cost.

Bytes are not tokens.
Cached and uncached tokens are not equivalent.
Do not claim API-cost savings without provider telemetry or a clearly labelled proxy.

Keep this root policy stable and no larger than necessary.
Put campaign detail in campaign prompts.
Do not rewrite stable instruction prefixes cosmetically.

Never save cost by weakening correctness or final verification.

## Protocol and Untrusted Boundaries

Treat CLI input, IPC, proposals, artifacts, head metadata, cursors, runtime values, imports, packages, caches, native images, permission handles, effect receipts, and FFI values as untrusted or corruptible.

Every boundary defines version or schema identity, framing, lengths, counts, numeric domains, canonical IDs, unknown and duplicate policy, trailing-data policy, allocation and depth policy, correlation, output bounds, timeout or cancellation, and failure behavior.

Unknown forms reject.
Use closed DTOs or direct closed types, not arbitrary property bags.

Machine stdout contains one structured response.
Human diagnostics belong on stderr or in a presentation layer.

Protocol replacement is direct when compatibility is not required.
Persistent formats use unambiguous magic, explicit version, canonical order, checked counts, strict trailing policy, bounded decoding, no Rust-layout dependence, no pointers, no compiler-private IDs, and deterministic content identity.

A commit is acknowledged only after the documented crash contract.
If outcome becomes unknowable, stop the writer rather than continue with ambiguous authority.

## Memory Safety

Memory safety is an enduring product requirement, not one implementation technique.

Valid use must prevent use-after-free, double free, invalid dereference, out-of-bounds access, uninitialized reads, type confusion, data races, invalid aliasing, use-after-move, duplicated exclusive ownership, and double close.

Malformed inputs may reject.
They must not corrupt authority, violate memory safety, allocate without checked bounds, consume unbounded native stack, or continue after authority becomes ambiguous.

Keep package-wide `unsafe_code = "forbid"`.
Do not weaken it in ordinary campaigns.
"Written in Rust" is not a proof.

Review separately:

1. language expressiveness;
2. semantic validation;
3. boundary decoding;
4. implementation memory safety;
5. resource and allocation bounds;
6. stack safety;
7. lifetime and aliasing;
8. deterministic cleanup;
9. concurrency safety;
10. permission security;
11. native and foreign isolation;
12. crash and durability behavior.

A future unsafe boundary requires explicit active-task authorization, proof that safe alternatives are inadequate, narrow isolation, a complete safety invariant, validation before entry, applicable lifetime/aliasing/overflow/concurrency tests, relevant Miri/sanitizer/fuzz/differential evidence, documented assumptions, and a safe public default.

## Value Classes and Memory Management

Do not choose one universal memory manager before representative value classes exist.

Distinguish immediate copy values, fixed immutable aggregates, large immutable managed values, shared graphs, unique mutable values, cycles, borrowed views, external resources, and foreign memory.

Current copy semantics for primitives and acyclic immutable named values do not decide every future class.

Tracing GC, reference counting, affine ownership, regions, arenas, handles, borrowing, copy-on-write, process isolation, and hybrids remain evidence-gated.

Choose semantics before representation.
State whether an operation copies, shares immutably, moves, clones, borrows, or consumes.

Account separately for logical size, retained physical size, peak scratch, shared backing storage, and external resources.
Never hide unbounded cloning behind a cheap-looking semantic operation.

A first managed value should be narrow, bounded, memory-safe, and driven by a representative application.
It should not force a universal GC, ownership system, or generic collection framework prematurely.

## Effects, Permissions, and Resources

Pure computation is a baseline, not an eternal prohibition.

Every host effect requires explicit typed authority.
No ambient filesystem, network, clock, entropy, process, environment, or device access.

Permission values state what may be attempted.
Resource-owning values state what must be released or consumed.
Do not conflate them.

An effect contract defines authority, inputs, outputs, validation, order, cancellation, timeout, partial-action behavior, retry semantics, cleanup, audit, sandbox, and crash behavior.

Do not silently replay a non-idempotent effect after an unknown outcome.

Move-only or affine resources prevent duplication, use-after-move, double close, and leaks across normal return, rejection, trap, cancellation, and process failure.

Do not add effects before required value, permission, cleanup, and failure contracts are dependency-closed.

## Compiler, Runtime, and Performance

The route is:

```text
immutable semantic revision
    -> completeness and semantic validation
    -> derived executable IR
    -> independent verification
    -> execution engine
```

Core IR is derived, not program authority.
Keep one simple executable oracle.
Optimized interpreters, JITs, AOT backends, native workers, and specialized kernels remain differential against it.

Do not expose private block IDs, registers, layouts, or machine offsets to agents.
Do not preselect LLVM, Cranelift, a custom backend, or a custom JIT without workload evidence.

Native acceleration expands the trusted computing base.
Isolate it, validate boundaries, and bind code identity to semantic revision, target, policy, and backend version.

Fuel, frames, cells, bytes, allocations, stack, wall time, and external resources are distinct policies.
Performance work preserves deterministic failure behavior.

## Persistence, Determinism, and Concurrency

Published semantic revisions are immutable.
Full snapshots and full retention are current baselines.

Any journal, chunk store, database, checkpoint, compaction, or pruning replacement preserves commit authority, revision identity, stable identity, non-reuse, crash consistency, restart validation, corruption rejection, bounded recovery, historical-query behavior, and a simple reconstruction oracle.

Caches and indexes are disposable derived state.
They never decide program meaning.

Do not repair corrupt authority heuristically.
Use rejection or a separately authorized recovery tool.

Observable acceptance, allocation, artifacts, diffs, query order, lowering, execution, and specified errors are deterministic.

Internal map order, allocation, scheduling, filesystem enumeration, and timing must not leak accidentally.
Future time, entropy, network, device, and scheduler input is explicit data or authority.

Do not add async or threads without a measured queueing, latency, throughput, isolation, or utilization problem.
Concurrent designs define snapshot selection, writer serialization, read consistency, cancellation, lock ordering, conflicts, publication order, shutdown, recovery, and deterministic tests.
Keep a sequential oracle.

## Dependencies and Supply Chain

Every dependency has a named current consumer.

Before adding or upgrading one, inspect license, lockfile impact, transitive packages, build scripts, native code, unsafe code, platform behavior, maintenance, features, binary size, compile cost, and current security information where available.

Prefer the standard library or existing dependencies when adequate.
A mature dependency may beat custom security-sensitive code.
A small direct implementation may beat a large protocol stack.
Decide from the actual boundary.

Delete unused dependencies in the same milestone.
Keep `Cargo.lock` authoritative for the Rust build.

## Testing and Representative Applications

Test acceptance and rejection.

Cover semantic validity, malformed boundaries, wrong kinds and types, scope, identity, rollback, validate-only, idempotency, diffs, artifact round trips, restart, corruption, incompleteness and repair, old revisions, compiler and verifier rejection, runtime traps, resource policies, and stack safety as applicable.

Protocol changes cover strict fields and variants, duplicates, trailing data, framing, bounds, correlation, timeout, dropped responses, and publication preflight.

Memory changes cover peak accounting, sharing, cloning, moves, cleanup, cancellation, traps, recursion, and adversarial sizes as applicable.

Optimizations and indexes compare against a simple oracle.

Use generated sequences, property tests, fuzzing, Miri, sanitizers, or model checking when they address a named retained risk.
Do not call deterministic mutation testing fuzzing.
Do not claim unavailable tools ran.

A retained application uses the public production path, has a deterministic oracle, exercises interacting features and rejection, includes restart or revision behavior where relevant, avoids private constructors and semantic-state fixtures, and remains maintainable.

Do not build a general framework from one consumer.

## Repository and Documentation

File boundaries follow fact ownership.
Do not impose arbitrary line, file, directory, or byte limits.
Split only for a real ownership, API, target, dependency, unsafe, compile-isolation, process, or change-locality boundary.
Do not create forwarding forests or duplicate helpers.

Use plain meaning before formal terminology.

Prefer "typed, versioned program model" before "Semantic Program Graph", "named record type" before "nominal product", "variant type" before "closed sum", "typed placeholder" before "hole", and "explicit permission value" before "capability" in introductory prose.

Do not use `source-free` as the primary product label.
State:

> Program meaning is stored in a typed semantic model. Text may be an optional view or input, but it is not a second authoritative representation.

README is not an agent operating manual.
Specifications are not status.
Status is not roadmap.
Performance evidence is not marketing.
Campaign prompts are not permanent specifications.

## Work Discipline

Before substantial work:

1. inspect branch, commit, and worktree;
2. read this file and the active prompt;
3. inspect owning specifications and status;
4. reproduce the smallest relevant public-path baseline;
5. identify the contract or limitation being changed;
6. compare alternatives, consumer, evidence, and reversal;
7. name every reader, writer, validator, schema, artifact, example, and document affected;
8. choose the smallest dependency-closed milestone;
9. define correctness, rejection, and measurement oracles;
10. record non-goals.

Do not implement from an old prompt without checking the active checkout.
Resolve implementation details from repository evidence rather than asking the user unnecessarily.

Keep vertical milestones truthful and verified.
When a contract changes, update direct types, validators, normalization, queries, machine descriptions, formats or versions where required, compiler/runtime paths if affected, tests, examples, specifications, status, architecture, performance, roadmap, and README.
Delete displaced code in the same milestone.

Do not leave commented alternatives, stale readers, dead flags, stale generated output, or TODOs in place of a campaign requirement.
Use temporary directories outside the repository for throwaway prototypes and transcripts.

## Verification and Handoff

The normal final boundary is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run retained production examples and focused boundary, corruption, restart, rollback, or performance commands required by the campaign.

A failed final command invalidates the boundary.
Fix it and rerun the complete boundary from the start.
Do not report stale results after later changes.

If Miri, sanitizers, fuzzers, or provider telemetry are unavailable, record the exact limitation.
Do not weaken policy to manufacture a green claim.

Before handoff, inspect `git status --short` and the full diff.
Confirm no unrelated work changed, no dual path remains after cutover, fact ownership is singular, documentation matches the checkout, claims match evidence, memory and resource boundaries are explicit, the agent contract is discoverable without source, and no credentials or temporary transcripts entered the repository.

The handoff states the starting commit, final worktree state, decisions and rejected alternatives, semantic/protocol/artifact/tool changes, memory-safety and TCB effects, exact verification results, application results, measured interaction or performance changes, unresolved risks, and next evidence gate.

Do not claim implementation for work that was only designed.
Do not hide partial completion.
Do not push or publish unless the active user task explicitly requests it.
