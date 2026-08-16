# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local procedure.
It may not weaken repository-wide requirements for one semantic authority, stable identity, atomic publication, durability, determinism, memory safety, capability safety, strict boundaries, bounded resource use, verification, evidence, documentation truth, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, specifications, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

Preserve unrelated work.
Never reset, clean, overwrite, stage, commit, push, force-push, or delete work that you did not create.
Do not change remote state unless the active user task explicitly requests it.
Never commit credentials, private transcripts, hidden model reasoning, provider secrets, or unrelated user data.

## Mission

Build `lkjscript` as a programming system designed primarily for autonomous coding agents.

Humans remain first-class at the levels of intent, governance, explanation, review, operation, security policy, and product ownership.
Humans are not expected to hand-author or textually maintain the authoritative program representation.

Use this plain explanation before specialized terminology:

> A coding agent edits a typed, versioned program model through a local service. The service validates proposed changes, saves immutable revisions, and compiles and runs selected revisions.

The formal name of the authoritative model is the **Semantic Program Graph** (`SPG`).

“Graph” describes semantic entities, stable identity, containment, ordering, value flow, and direct references.
It does not prescribe pointer-based storage, a graph database, a mutable object graph, or a particular in-memory layout.

Text, diagrams, generated source, imported syntax, tool calls, and JSON may be useful views or proposal forms.
They must resolve into the same authoritative semantic model.
They must not become a second source of truth.

The system must remain memory-safe, deterministic at observable boundaries, failure-atomic for durable publication, strict toward untrusted input, and capable of excellent long-term runtime performance.

Agent interaction cost is a first-class engineering dimension.
Measure it without weakening correctness, evidence, or final verification.

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

A campaign prompt controls scope, sequence, evidence gates, and one-time work.
It does not silently redefine accepted semantics.
When a campaign changes semantics, update the owning specification in the same verified milestone.

Keep durable policy here.
Keep one-time plans, experiments, measurements, research notes, task capsules, and campaign-specific numbers in the campaign prompt or their existing fact-owning documents.

## Policy Stability and Context Economics

Treat the stable instruction prefix as shared infrastructure.

Do not rewrite this file cosmetically.
Change it only when durable repository policy materially changes.
Prefer a small precise amendment over broad rewording.
Do not duplicate campaign detail here.

Measure policy bytes and lines when replacing this file.
Do not infer model tokens or monetary cost from bytes.
Use provider-reported token classes and prices only when available.

A smaller policy is not better if it omits a correctness, durability, safety, evidence, or handoff obligation.
A larger policy is not better if it repeats specifications, status, roadmaps, or campaign procedure.

Keep stable names, ordering, and terminology where they remain correct.
Instruction-cache stability, repeated discovery cost, and agent orientation cost are legitimate engineering concerns.

## Fact Ownership

Keep one maintained owner for each fact:

- `docs/spec/semantic-graph.md`: authoritative model, identity, revisions, transactions, history, and semantic artifacts.
- `docs/spec/language.md`: types, values, operations, control, effects, ownership, lifetime, cleanup, and execution semantics.
- `docs/spec/protocol.md`: local transport, requests, responses, schema discovery, framing, cursors, strict JSON, and CLI projections.
- `docs/architecture.md`: component responsibility, trusted computing base, process topology, and trust boundaries.
- `docs/status.md`: exactly what the current checkout implements and does not implement.
- `docs/performance.md`: reproduced measurements, comparisons, regressions, resource observations, and reversal evidence.
- `docs/roadmap.md`: ordered evidence gates and deferred choices.
- `README.md`: human-first product explanation and runnable entry points.
- This file: repository-wide operating policy.
- `prompts/`: campaign execution artifacts, not permanent semantic authority.

Do not maintain duplicate status catalogues, glossaries, version registries, roadmaps, schema copies, architecture inventories, dependency inventories, memory-model tables, or benchmark tables.

Generated machine descriptions must derive from executable contracts.
Do not commit a hand-maintained duplicate schema.

## Claims and Evidence

Classify material claims as one of:

- enduring invariant;
- accepted semantic contract;
- current verified baseline;
- operational policy;
- evidence-gated choice;
- experimental hypothesis;
- historical fact.

Do not promote a bootstrap absence into an eternal prohibition.
Do not present a hypothesis as implemented reality.
Do not present one model run as a general benchmark.
Do not present one machine, one seed, or one warm-cache observation as universal performance.
Do not present a safe-Rust implementation as a complete proof.
Do not present bytes as tokens.
Do not present cached and uncached tokens as equivalent.
Do not present logical bytes, encoded bytes, retained bytes, and peak bytes as interchangeable.

Report only what the active checkout and reproduced evidence support.

## Enduring Invariants

The following outrank the current module layout, protocol version, artifact format, process topology, storage engine, runtime representation, memory-management technique, implementation language, and model provider.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions are immutable.
3. Durable publication has one unambiguous logical commit authority per durable namespace.
4. Current single-head and single-writer mechanics are baselines, not eternal bans on branches, clients, replicas, or isolated workers.
5. Future branch, merge, replica, or worker designs preserve explicit parentage, deterministic conflicts, and freedom from split-brain authority.
6. Text, JSON, diagrams, generated source, drafts, tool schemas, caches, indexes, Core IR, memory plans, profiles, and machine code are views, proposals, or derived state, never coequal authority.
7. Persisted semantic entities, attributes, ownership slots, child slots, value uses, and direct references belong to a closed typed schema.
8. Arbitrary property bags, arbitrary string-labelled semantic edges, and unknown semantic forms are forbidden in authority.
9. Mutation is a typed transaction or a closed typed proposal deterministically normalized into one transaction.
10. A successful commit publishes exactly one accepted revision.
11. Rejection and validate-only publish nothing and consume no persistent identities.
12. Stable semantic identity is independent of names, source positions, formatting, hashes, compiler indexes, artifact offsets, addresses, handles, and storage keys.
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
30. Compactness never weakens typing, validation, identity, authorization, durability, error precision, or verification.
31. Performance optimization preserves a simple correctness oracle.
32. Representative applications discover missing capabilities but do not justify speculative platforms.
33. Backward compatibility is not required unless the active user explicitly requires it.
34. Incompatible-change freedom is used to converge on one coherent path, not create churn or legacy paths.
35. Memory safety, resource exhaustion, deterministic cleanup, concurrency safety, permission security, and crash consistency remain separate contracts.
36. Tool adapters and frontends may disappear without changing program meaning.
37. Runtime optimizations may disappear without changing accepted semantics.
38. Every retained dependency has a named current consumer.
39. Every public boundary has exact versioning, bounds, rejection behavior, and output policy.
40. Every effect has explicit authority and outcome semantics.
41. No non-idempotent effect is silently retried after a possible partial action.
42. Immutable value equality and behavior do not depend on physical address, storage sharing, allocator choice, or reclamation time.
43. A logical value copy may be represented by physical sharing when semantics remain unchanged.
44. Physical sharing, uniqueness, moves, borrows, regions, reference counts, tracing, and reuse are implementation or derived-memory facts unless the language contract explicitly exposes them.
45. Resource ownership is semantic when duplication or cleanup changes behavior.
46. External resources never rely on nondeterministic garbage-collector finalization for correctness.
47. Untrusted or persisted references never expose raw process pointers.
48. Derived runtime handles are validated, domain-bound, kind-checked, and protected against stale reuse.
49. Logical work accounting remains deterministic even when physical memory optimization changes.
50. A faster memory strategy remains differential against a simpler safe oracle.

## Operating Posture

The current semantic-model service is a verified baseline, not an untouchable monument.

Do not restore displaced source-oriented authority.
Do not perform a total rewrite merely because compatibility is unimportant.
Do not preserve a mechanism because it was expensive to build.
Do not delete a mechanism merely because a simpler idea sounds attractive.
Do not generalize one consumer into a platform.
Do not defer an evidence-supported architectural correction merely because the current baseline is coherent.

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

Select an evidence-gated choice only after naming:

- its concrete consumer;
- semantic obligations;
- safety obligations;
- durability obligations;
- correctness oracle;
- measured baseline;
- expected benefit;
- implementation and trusted-surface cost;
- reversal condition.

## Source-Independent Authority

The invariant is source independence, not hostility toward text.

An agent must be able to construct, inspect, revise, validate, compile, execute, package, and debug without preserving or round-tripping source files.

A frontend may import or render text, provide review views, exchange code, generate a typed proposal, or expose tools to a harness.

It may not:

- bypass validation;
- allocate persistent identities independently;
- own mutable semantic state;
- persist a parallel AST as coequal authority;
- make formatting identity;
- define behavior absent from the authoritative model;
- require render-and-reparse editing.

Do not promise that `lkjscript` will never have syntax.
Promise that syntax will not become a second authoritative program.

Do not use `source-free` as the primary product label.
State:

> Program meaning is stored in a typed semantic model. Text may be an optional view or input, but it is not a second authoritative representation.

## Semantic Model and Identity

The SPG is a closed typed semantic model, not a generic property graph.

Each semantic kind has explicit:

- attributes;
- owner rules;
- ordered children;
- direct references;
- operands;
- results;
- cardinality;
- completeness;
- continuity;
- deletion behavior;
- lowering obligations;
- query obligations;
- artifact obligations.

Unknown schema elements reject.
Do not preserve unknown semantic fields for hypothetical forward compatibility.
Evolve through explicit direct replacement.

Use one code-owned contract to supply facts to validators, codecs, queries, machine descriptions, history checks, and lowering where practical.

Do not add runtime registration for a closed vocabulary.
Do not add a general graph framework when direct types and static descriptors suffice.

Physical traversal order is not observable unless the semantic contract defines it.

Stable identity is valuable and costly.
Distinguish:

- persistent Node IDs;
- transaction-local symbols;
- query labels;
- revision numbers;
- hashes and digests;
- dense compiler IDs;
- runtime handles;
- names;
- view positions.

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

Every mutation names:

- a workspace;
- an exact base revision or accepted parent relation;
- commit or validate-only mode;
- an optional idempotency key where supported;
- an ordered closed proposal;
- a bounded response selection.

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

Provide:

- a closed executable contract;
- stable names and IDs;
- compact discovery;
- exact on-demand expansion;
- revision-bound reads;
- meaningful proposals;
- selected bindings;
- compact receipts;
- typed errors;
- legal constructors;
- visible values;
- repair context;
- paginated diffs;
- exact run values;
- digest-based unchanged results where useful.

A low-level node API is not agent-friendly merely because it is typed JSON.

Do not require agents to author:

- private compiler indexes;
- CFG details;
- artifact offsets;
- checksums;
- publication records;
- cache keys;
- memory-plan instructions;
- retain/release operations;
- allocator metadata;
- durability metadata.

Prefer standard inspectable control-plane representations until measurement justifies custom transport.
One request and response vocabulary should serve CLI, service, tests, and machine description.

Do not invent compact syntax from intuition.
Evaluate interface changes with equal-task agent trials and deterministic oracles.

## Tool Adapters and Interaction Cost

A Codex-native, MCP, app-server, shell, library, batch, session, or other harness adapter is optional projection infrastructure.
It is not program authority.
It must invoke or derive from the executable machine contract rather than maintain a second accepted vocabulary.

Keep retained tools small, stable, deterministic, and semantically meaningful.
Do not expose one tool per internal node kind.
Do not change tool names, order, descriptions, or schemas casually.
Dynamic tool refresh must be explicit and deterministic.

Do not add a large protocol stack, async runtime, SDK, or dependency tree solely for branding.
Prototype outside the retained product path.
Retain an adapter only when equal-task evidence justifies implementation, lifecycle, security, cancellation, and context cost.

Measure separately:

- policy bytes;
- campaign bytes;
- machine-contract bytes;
- tool-definition bytes;
- request and response bytes;
- provider-reported token classes and price when available;
- tool calls;
- shell launches;
- client processes;
- daemon connections;
- semantic round trips;
- failures;
- repeated discovery;
- files opened;
- elapsed time;
- implementation size;
- verification cost.

Bytes are not tokens.
Cached and uncached tokens are not equivalent.
Do not claim API-cost savings without provider telemetry or a clearly labelled proxy.

Never save cost by weakening correctness or final verification.

## Protocol and Untrusted Boundaries

Treat CLI input, IPC, proposals, artifacts, head metadata, cursors, runtime values, imports, packages, caches, native images, permission handles, effect receipts, managed handles, and FFI values as untrusted or corruptible.

Every boundary defines:

- version or schema identity;
- framing;
- lengths;
- counts;
- numeric domains;
- canonical IDs;
- unknown and duplicate policy;
- trailing-data policy;
- allocation and depth policy;
- correlation;
- output bounds;
- timeout or cancellation;
- failure behavior.

Unknown forms reject.
Use closed DTOs or direct closed types, not arbitrary property bags.

Machine stdout contains only structured machine responses.
Human diagnostics belong on stderr or in a presentation layer.

Protocol replacement is direct when compatibility is not required.

Persistent formats use:

- unambiguous magic;
- explicit version;
- canonical order;
- checked counts;
- strict trailing policy;
- bounded decoding;
- no Rust-layout dependence;
- no pointers;
- no compiler-private IDs;
- deterministic content identity.

A commit is acknowledged only after the documented crash contract.
If outcome becomes unknowable, stop the writer rather than continue with ambiguous authority.

## Memory Safety

Memory safety is an enduring product requirement, not one implementation technique.

Valid use must prevent:

- use-after-free;
- double free;
- invalid dereference;
- out-of-bounds access;
- uninitialized reads;
- type confusion;
- data races;
- invalid aliasing;
- use-after-move;
- duplicated exclusive ownership;
- double close;
- stale-handle access.

Malformed inputs may reject.
They must not corrupt authority, violate memory safety, allocate without checked bounds, consume unbounded native stack, or continue after authority becomes ambiguous.

Keep package-wide `unsafe_code = "forbid"`.
Do not weaken it in ordinary campaigns.
“Written in Rust” is not a proof.

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

A future unsafe boundary requires:

- explicit active-task authorization;
- proof that safe alternatives are inadequate;
- narrow isolation;
- a complete safety invariant;
- validation before entry;
- applicable lifetime, aliasing, overflow, and concurrency tests;
- relevant Miri, sanitizer, fuzz, or differential evidence;
- documented assumptions;
- a safe public default.

## Value Semantics and Memory Architecture

Do not collapse value semantics, usage discipline, lifetime, cleanup, representation, and reclamation into one word such as “ownership” or “GC”.

For every value class, specify separately:

1. semantic equality;
2. observable identity, if any;
3. mutability;
4. duplication semantics;
5. operation access semantics;
6. escape and lifetime rules;
7. cleanup obligations;
8. physical representation;
9. reclamation mechanism;
10. logical and physical accounting;
11. concurrency properties;
12. foreign-boundary behavior.

Distinguish at least:

- immediate copy values;
- fixed immutable aggregates;
- variable-size immutable managed values;
- immutable shared graphs;
- unique mutable values;
- cyclic values;
- borrowed views;
- external resources;
- foreign memory.

Current copy semantics for primitives and acyclic immutable named values do not decide every future class.

Immutable ordinary values should not gain observable address or object identity merely because an implementation stores them in managed memory.
A semantic duplicate may share physical storage.
A semantic operation may reuse unique storage while preserving immutable observable behavior.

Choose semantics before representation.

Do not choose one universal memory manager before representative value classes exist.
Do not use that rule as an excuse to postpone every concrete memory decision.

Evaluate mechanisms in this preferred order when applicable:

1. immediate or fixed inline representation;
2. compiler-proven nonescaping region, stack, or invocation arena;
3. exact ownership or precise reference counting for escaping cycle-free values;
4. uniqueness-guided or drop-guided reuse;
5. tracing or cycle collection inside an isolated class or region that actually permits cycles;
6. process isolation for foreign or high-risk memory.

This is an evaluation order, not a mandate that every layer must exist.

Prefer inference from the authoritative use graph, control flow, types, and escape facts over agent-authored lifetime syntax.
Do not expose Rust-like lifetime parameters, explicit retain/release, allocator calls, raw regions, or borrow choreography merely to make implementation easier.

A surface ownership or locality annotation requires a real semantic consumer and evidence that deterministic inference cannot express the necessary choice clearly.

A derived memory plan may classify values as immediate, managed, borrowed, transferred, shared, unique, escaping, or region-local.
It must be independently verifiable.
It is derived compiler state, not program authority.

Do not let an unverified optimization pass decide memory safety.

### Managed Handles

Use validated handles rather than raw pointers at interpreter, protocol, artifact, cache, and untrusted boundaries.

A handle contract defines:

- domain;
- kind;
- index or identity encoding;
- invalid sentinel policy;
- generation or no-reuse rule;
- bounds;
- owner arena or region;
- escape policy;
- validation before access;
- stale-handle rejection;
- serialization prohibition unless explicitly specified.

A copied handle does not by itself prove semantic copyability or physical ownership.

Do not expose process addresses through runtime values, diagnostics, artifacts, hashes, or public IDs.

### Regions and Arenas

Regions and arenas are strong candidates for values whose escape boundary is statically known.

A retained region design defines:

- region creation;
- region parentage;
- permitted references;
- escape rules;
- tail-call interaction;
- cancellation and trap cleanup;
- peak reservation;
- object and byte limits;
- destruction order where observable;
- public-value materialization before destruction;
- differential oracle.

Invocation-scoped arenas are not a universal heap.
They are appropriate only when every retained reference dies or is safely materialized at the invocation boundary.

Do not hide unbounded retention behind a region.
A view retaining a large backing allocation must be accounted as retained backing, not only visible bytes.

### Reference Counting and Reuse

Precise reference counting, borrowed-reference inference, uniqueness, and reuse are legitimate future techniques for cycle-free managed values.

Do not add pervasive reference counts merely because the first managed value can be shared.

A retained reference-counting design defines:

- which references are counted;
- which uses borrow;
- insertion and elimination rules;
- cycle policy;
- overflow behavior;
- thread-sharing policy;
- deterministic destruction;
- panic, trap, and cancellation cleanup;
- verifier obligations;
- peak-memory effect;
- reuse safety;
- differential evidence.

Reuse must preserve semantic immutability.
A failed uniqueness check must fall back safely.
An optimization must not make peak memory unbounded or transform a bounded evaluation into an unbounded one.

### Tracing and Cycles

Tracing collection remains an option for real cyclic or long-lived graph workloads.

Do not introduce a tracing collector for a cycle-free bounded bytes vertical.
Do not permanently prohibit tracing because an earlier value class did not need it.

A tracing design requires:

- exact roots;
- object layout metadata;
- movement or pinning policy;
- write-barrier policy;
- stop-the-world or concurrent semantics;
- weak-reference and finalization policy;
- foreign-reference policy;
- latency and throughput evidence;
- a much larger trusted-computing-base review.

Do not use finalizers for required external-resource cleanup.

### Affine Resources

Resource-owning values are distinct from immutable managed data.

A file, socket, process, device, lock, transaction, or foreign allocation may need affine or linear semantics because duplication and cleanup are observable.

A resource contract defines:

- creation authority;
- unique owner;
- borrow operations;
- consuming operations;
- explicit close or commit;
- automatic fallback cleanup;
- normal-return behavior;
- rejection behavior;
- trap and cancellation behavior;
- process-failure behavior;
- unknown-outcome behavior;
- audit facts.

Do not represent a resource as an ambient integer handle.
Do not make garbage collection the only cleanup path.

### Memory Accounting

Account separately for:

- logical value bytes;
- visible slice or view bytes;
- distinct retained backing bytes;
- retained bytes caused by views;
- object or handle count;
- fixed cell or slot footprint;
- peak scratch;
- allocator capacity and fragmentation where measured;
- encoded boundary bytes;
- copied bytes;
- shared bytes;
- external-resource units.

Do not double-count shared backing as distinct physical memory.
Do not under-count a small view that pins a large backing allocation.
Do not hide unbounded cloning behind a cheap-looking semantic operation.

Every potentially allocating operation defines:

- checked size arithmetic;
- preflight point;
- failure code;
- fuel or work charge;
- memory-policy charge;
- partial-allocation rollback;
- output accounting.

Physical optimization must not silently alter logical fuel, branch selection, trap ordering, or other observable semantics.

## Effects, Permissions, and Resources

Pure computation is a baseline, not an eternal prohibition.

Every host effect requires explicit typed authority.
No ambient filesystem, network, clock, entropy, process, environment, or device access.

Permission values state what may be attempted.
Resource-owning values state what must be released or consumed.
Do not conflate them.

An effect contract defines:

- authority;
- inputs;
- outputs;
- validation;
- order;
- cancellation;
- timeout;
- partial-action behavior;
- retry semantics;
- cleanup;
- audit;
- sandbox boundary;
- crash behavior.

Do not silently replay a non-idempotent effect after an unknown outcome.

Do not add effects before required value, permission, cleanup, and failure contracts are dependency-closed.

## Compiler, Runtime, and Performance

The semantic route is:

```text
immutable semantic revision
    -> completeness and semantic validation
    -> derived executable IR
    -> independent verification
    -> execution engine
```

Core IR is derived, not program authority.
A memory plan or ownership IR is also derived, not authority.

Keep one simple executable oracle.
Optimized interpreters, memory planners, JITs, AOT backends, native workers, and specialized kernels remain differential against it.

Do not expose private block IDs, registers, layouts, handles, memory plans, or machine offsets to agents.

Do not preselect LLVM, Cranelift, a custom backend, a custom JIT, MMTk, or another runtime framework without workload evidence.

Native acceleration expands the trusted computing base.
Isolate it, validate boundaries, and bind code identity to semantic revision, target, policy, backend version, and memory contract.

Fuel, frames, cells, logical bytes, retained bytes, handles, allocations, stack, wall time, and external resources are distinct policies.

Performance work preserves deterministic failure behavior.

## Persistence, Determinism, and Concurrency

Published semantic revisions are immutable.
Full snapshots and full retention are current baselines.

Any journal, chunk store, database, checkpoint, compaction, or pruning replacement preserves:

- commit authority;
- revision identity;
- stable identity;
- non-reuse;
- crash consistency;
- restart validation;
- corruption rejection;
- bounded recovery;
- historical-query behavior;
- a simple reconstruction oracle.

Caches and indexes are disposable derived state.
They never decide program meaning.

Do not repair corrupt authority heuristically.
Use rejection or a separately authorized recovery tool.

Observable acceptance, allocation, artifacts, diffs, query order, lowering, execution, memory-policy failures, and specified errors are deterministic.

Internal map order, allocation address, scheduling, filesystem enumeration, reclamation timing, and wall-clock timing must not leak accidentally.

Future time, entropy, network, device, and scheduler input is explicit data or authority.

Do not add async or threads without a measured queueing, latency, throughput, isolation, or utilization problem.

Concurrent designs define:

- snapshot selection;
- writer serialization;
- read consistency;
- cancellation;
- lock ordering;
- conflicts;
- publication order;
- memory ownership across threads;
- shutdown;
- recovery;
- deterministic tests.

Keep a sequential oracle.

## Dependencies and Supply Chain

Every dependency has a named current consumer.

Before adding or upgrading one, inspect:

- license;
- lockfile impact;
- transitive packages;
- build scripts;
- native code;
- unsafe code;
- platform behavior;
- maintenance;
- enabled features;
- binary size;
- compile cost;
- current security information where available.

Prefer the standard library or existing dependencies when adequate.
A mature dependency may beat custom security-sensitive code.
A small direct implementation may beat a large protocol stack.
Decide from the actual boundary.

Delete unused dependencies in the same milestone.
Keep `Cargo.lock` authoritative for the Rust build.

A memory-management framework is not “just a dependency”.
Its VM binding, root contract, barriers, object model, unsafe assumptions, threads, pauses, and debugging obligations are part of the product trusted surface.

## Testing and Representative Applications

Test acceptance and rejection.

Cover, as applicable:

- semantic validity;
- malformed boundaries;
- wrong kinds and types;
- scope;
- identity;
- rollback;
- validate-only;
- idempotency;
- diffs;
- artifact round trips;
- restart;
- corruption;
- incompleteness and repair;
- old revisions;
- compiler rejection;
- verifier rejection;
- runtime traps;
- resource policies;
- stack safety;
- managed-handle validation;
- memory accounting;
- cleanup;
- aliasing;
- last-use analysis;
- cancellation.

Protocol changes cover strict fields and variants, duplicates, trailing data, framing, bounds, correlation, timeout, dropped responses, and publication preflight.

Memory changes cover:

- logical versus physical accounting;
- sharing;
- cloning;
- moves;
- borrows;
- region escape;
- view retention;
- cleanup;
- cancellation;
- traps;
- recursion;
- adversarial sizes;
- stale handles;
- exact policy boundaries.

Optimizations, indexes, and memory plans compare against a simple oracle.

Use generated sequences, property tests, fuzzing, Miri, sanitizers, or model checking when they address a named retained risk.
Do not call deterministic mutation testing fuzzing.
Do not claim unavailable tools ran.

A retained application:

- uses the public production path;
- has a deterministic oracle;
- exercises interacting features and rejection;
- includes restart or revision behavior where relevant;
- avoids private constructors and semantic-state fixtures;
- remains maintainable;
- justifies the retained capability.

Do not build a general framework from one consumer.

## Repository and Documentation

File boundaries follow fact ownership.
Do not impose arbitrary line, file, directory, or byte limits.

Split only for a real:

- ownership boundary;
- API boundary;
- target boundary;
- dependency boundary;
- unsafe boundary;
- compile-isolation boundary;
- process boundary;
- change-locality boundary.

Do not create forwarding forests or duplicate helpers.

Use plain meaning before formal terminology.

Prefer:

- “typed, versioned program model” before “Semantic Program Graph”;
- “named record type” before “nominal product”;
- “variant type” before “closed sum”;
- “typed placeholder” before “hole”;
- “explicit permission value” before “capability”;
- “immutable managed value” before naming its reclamation technique.

README is not an agent operating manual.
Specifications are not status.
Status is not roadmap.
Performance evidence is not marketing.
Campaign prompts are not permanent specifications.

## Work Discipline

Before substantial work:

1. inspect branch, commit, and worktree;
2. read this file and the active prompt;
3. inspect owning specifications and current status;
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

When a contract changes, update:

- direct types;
- validators;
- normalization;
- queries;
- machine descriptions;
- formats or versions where required;
- compiler and runtime paths;
- memory plans or accounting when affected;
- tests;
- examples;
- specifications;
- status;
- architecture;
- performance;
- roadmap;
- README.

Delete displaced code in the same milestone.

Do not leave commented alternatives, stale readers, dead flags, stale generated output, or TODOs in place of a campaign requirement.

Use temporary directories outside the repository for throwaway prototypes, generated payloads, benchmark output, and transcripts.

## Verification and Handoff

The normal final boundary is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run retained production examples and focused boundary, corruption, restart, rollback, memory, or performance commands required by the campaign.

A failed final command invalidates the boundary.
Fix it and rerun the complete boundary from the start.
Do not report stale results after later changes.

If Miri, sanitizers, fuzzers, model checkers, or provider telemetry are unavailable, record the exact limitation.
Do not weaken policy to manufacture a green claim.

Before handoff:

- inspect `git status --short`;
- inspect the complete diff;
- confirm unrelated work is unchanged;
- confirm no dual path remains after cutover;
- confirm fact ownership is singular;
- confirm documentation matches the checkout;
- confirm claims match evidence;
- confirm memory and resource boundaries are explicit;
- confirm the agent contract is discoverable without source;
- confirm no credentials or temporary transcripts entered the repository.

The handoff states:

- starting commit;
- final worktree state;
- decisions and rejected alternatives;
- semantic, protocol, artifact, tool, and memory changes;
- memory-safety and trusted-computing-base effects;
- exact verification results;
- application results;
- measured interaction, memory, and performance changes;
- unresolved risks;
- next evidence gate.

Do not claim implementation for work that was only designed.
Do not hide partial completion.
Do not push or publish unless the active user task explicitly requests it.
