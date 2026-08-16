# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local procedure.
It may not weaken repository-wide requirements for semantic authority, identity, atomicity, durability, determinism, memory safety, capability safety, strict boundaries, verification, evidence, documentation truth, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

Preserve unrelated work.
Never reset, clean, overwrite, stage, commit, or force-push work that you did not create.
Do not change remote state unless the active user task explicitly requests it.

## Mission

Build `lkjscript` as a programming system designed primarily for autonomous coding agents.

Humans remain first-class users at the level of intent, explanation, governance, review, operation, and product ownership.
Humans are not expected to hand-author the authoritative program representation.

Use this plain explanation before specialized terminology:

> An agent edits a typed, versioned program model through a local service. The service validates proposed changes, saves immutable revisions, and compiles and runs selected revisions.

The formal name of the authoritative model is the **Semantic Program Graph** (`SPG`).

“Graph” describes semantic entities, identity, containment, ordering, and references.
It does not prescribe pointer-based storage, a graph database, or a specific in-memory layout.

Text, diagrams, generated source, and imported syntax may become useful views or proposal formats.
They must resolve into the same authoritative semantic model.
They must not become a second source of truth.

The system must remain memory-safe, deterministic at observable boundaries, failure-atomic for durable publication, strict toward untrusted input, and capable of world-class long-term runtime performance.

Agent interaction cost is a first-class engineering dimension.
Measure it without weakening correctness.

## Authority Order

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

A campaign prompt controls sequence and scope.
It does not silently redefine accepted semantics.
When a campaign changes semantics, update the owning specification in the same verified milestone.

Newer active instructions and newer verified repository state outrank older prompts and assumptions.

## Fact Ownership

Keep one owner for each maintained fact:

- `docs/spec/semantic-graph.md`: authoritative model, identity, revisions, transactions, history, and artifacts.
- `docs/spec/language.md`: types, operations, control, effects, ownership, and execution semantics.
- `docs/spec/protocol.md`: local service transport, machine requests, responses, schema discovery, framing, and cursors.
- `docs/architecture.md`: component responsibility, trusted computing base, and trust boundaries.
- `docs/status.md`: exactly what the current checkout implements and does not implement.
- `docs/performance.md`: measurements, comparisons, regressions, and reversal conditions.
- `docs/roadmap.md`: ordered evidence gates.
- `README.md`: human-first product explanation and runnable entry points.
- This file: repository-wide operating policy.
- `prompts/`: campaign execution artifacts, not permanent semantic authority.

Do not maintain duplicate status catalogues, glossaries, version registries, roadmaps, schema copies, or architecture inventories.

Generated machine descriptions must derive from executable contracts.
Do not commit a hand-maintained duplicate schema.

## Enduring Invariants

The following requirements outrank the current module layout, protocol version, artifact format, process topology, storage engine, runtime representation, memory-management technique, and implementation language.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions are immutable.
3. Durable publication has one unambiguous logical commit authority per durable namespace.
4. Current single-head and single-writer mechanics are baselines, not eternal bans on branches, multiple clients, or isolated workers.
5. Any future branch, merge, replica, or worker design must preserve explicit parentage, deterministic conflict semantics, and freedom from split-brain authority.
6. Text, JSON, diagrams, generated source, structured drafts, caches, indexes, Core IR, and machine code are views, proposals, or derived state, never coequal program authority.
7. Every persisted semantic entity, attribute, ownership slot, child slot, and direct reference belongs to a closed typed schema.
8. Arbitrary property bags, arbitrary string-labelled semantic edges, and unknown semantic forms are forbidden in authoritative state.
9. Every mutation is a typed transaction or a closed typed proposal deterministically expanded into one transaction.
10. A successful commit publishes exactly one accepted revision.
11. Rejection and validate-only publish nothing and consume no persistent identities.
12. Stable identity is independent of names, source positions, hashes, compiler indexes, artifact offsets, memory addresses, and physical storage keys.
13. Persistent semantic identities are never reused within their defined identity domain.
14. Names are lookup and presentation metadata, not identity.
15. Identity-preserving change exists only where an accepted continuity rule defines it.
16. Derived facts never become a second mutable source of truth.
17. Only a complete selected-entry reachable definition set may enter executable lowering.
18. The compiler consumes an immutable revision directly.
19. One semantic execution route defines behavior.
20. Later interpreters, JITs, AOT backends, or native workers accelerate that route rather than define competing languages.
21. AI output is untrusted input.
22. Deterministic validators decide acceptance.
23. Host access requires explicit typed authority.
24. Accepted language semantics cannot express unchecked memory access.
25. User-controlled depth must not consume unbounded native stack.
26. Observable order is explicit and deterministic.
27. Potentially large results are bounded, streamed, or paginated.
28. Durable state is acknowledged only after the documented publication contract.
29. Corrupt, ambiguous, unsupported, or partially published durable state rejects rather than being guessed into validity.
30. Human-facing claims remain no stronger than implemented and reproduced evidence.
31. Compactness never weakens typing, validation, identity, authorization, or error precision.
32. Performance optimization preserves a simple correctness oracle.
33. Representative applications discover missing capabilities.
34. Representative applications do not justify speculative platforms.
35. Backward compatibility is not a product requirement unless the active user explicitly requires it.
36. Incompatible-change freedom is used to maintain one coherent path, not to create churn or parallel legacy paths.

## Invariant, Contract, Baseline, or Open Choice

Classify material claims as one of:

- enduring invariant;
- accepted semantic contract;
- current verified baseline;
- operational resource policy;
- evidence-gated choice;
- historical fact.

Do not promote a bootstrap absence into an eternal prohibition.

Current verified baselines may include stable Rust, one package, Linux x86-64, local IPC, immutable full-revision artifacts, full snapshot cloning, full recomputation, scan-based queries, one Core IR, an explicit-frame interpreter, flat cells, and no effects or managed heap.

These are not permanent architecture mandates.

Evidence-gated choices include:

- text and visual frontends;
- physical storage layout;
- journals, databases, and chunk stores;
- indexes and caches;
- history pruning and compaction;
- concurrent requests;
- multiple branch heads;
- process isolation;
- package infrastructure;
- cross-platform support;
- self-hosting;
- managed heaps;
- reference counting;
- affine resources;
- regions;
- tracing collection;
- stable handles;
- native backends;
- JIT or AOT compilation.

Select or reject an evidence-gated choice only after naming its consumer, safety obligations, correctness oracle, measured benefit, implementation cost, and reversal condition.

## Operating Posture

The current semantic-model service is a verified baseline, not an untouchable monument.

Do not restore displaced source-oriented authority.

Do not perform a total rewrite merely because compatibility is unimportant.

Replace a subsystem when the replacement is dependency-closed, materially clearer or more capable, and verified against the enduring invariants.

When replacing an active boundary:

- replace every active reader and writer together;
- use a new unambiguous version, schema identity, tag set, or magic when old bytes would be ambiguous;
- delete displaced readers, writers, adapters, fixtures, tests, tags, and claims;
- update the owning specification and current status in the same milestone;
- retain no legacy mode, edition split, compatibility namespace, dual path, hidden fallback, or silent migration unless explicitly requested.

Git history is the archive for superseded implementation.

## Source-Independent Authority

The invariant is source independence, not hostility toward text.

An agent must be able to construct, inspect, revise, validate, compile, execute, package, and debug without preserving or round-tripping source files.

A future frontend may:

- import text;
- render explanatory source;
- provide a human review or diff view;
- attach documentation metadata;
- exchange code with another ecosystem;
- generate a compact proposal.

It may not:

- bypass validation;
- allocate persistent identities independently;
- own mutable semantic state;
- persist a parallel AST as coequal authority;
- make formatting or source position identity;
- require render-and-reparse for normal semantic editing.

Do not promise that lkjscript will never have syntax.
Promise that syntax will not become a second authoritative program.

## Semantic Model and Identity

The SPG is a closed typed semantic model, not a generic property graph.

Each semantic kind has explicit:

- attributes;
- owner rules;
- ordered child slots;
- direct reference slots;
- operands and results;
- cardinality;
- completeness;
- continuity;
- deletion behavior;
- lowering obligations.

Unknown schema elements reject.
Do not preserve unknown semantic fields for hypothetical forward compatibility.
Evolve through explicit direct replacement.

Use one code-owned contract to provide facts to validators, codecs, queries, machine descriptions, history checks, and lowering where practical.

Do not introduce runtime registration for a closed vocabulary.
Do not introduce a general graph framework when direct types and static descriptors are sufficient.

Physical traversal order is not observable unless the semantic contract defines it.

A workspace owns a persistent semantic identity domain.
Allocation is monotonic under the current contract.
Rejected and validate-only proposals leave the published frontier unchanged.
Deleted identities are never reassigned.

Rename preserves identity.
Replacement creates new identity unless an accepted continuity rule says otherwise.
Do not generalize one valid refinement into unrestricted semantic morphing.

Hashes identify bytes and cache inputs.
Dense compiler and runtime indexes remain private.

Full retention of every artifact and physical tombstone is a current persistence strategy.
Future pruning or compaction requires:

- an explicit retention contract;
- reproducible current state;
- non-reuse proof;
- exact historical-query behavior;
- exact failure behavior;
- one direct cutover.

Incomplete programs are valid semantic states.
Use typed placeholders or exact missing-definition states.
An incomplete definition blocks execution only when reachable from the selected entry.

Repair context is deterministic typed data.
Models remain outside the correctness authority path.

## Transactions and Structured Proposals

Every mutation names:

- one workspace;
- one exact base revision or accepted parent relation;
- commit or validate-only mode;
- an optional idempotency key where supported;
- an ordered closed proposal;
- a bounded response selection.

A successful commit logically:

1. validates the envelope;
2. resolves draft-local references;
3. expands structured proposals;
4. applies ordered edits;
5. validates the final model and history;
6. derives deterministic change facts;
7. constructs and preflights a bounded response;
8. preflights durable bytes;
9. durably publishes the accepted revision and authoritative head metadata;
10. publishes in-memory state;
11. returns the preflighted receipt.

A rejected transaction changes none of those states.

A structured authoring payload may remove repeated canonical scaffolding.
It is a closed typed proposal.
It is not persisted as a second program representation.
Final semantic validation remains authoritative.

Prefer one semantically meaningful proposal over repeated transport-level setup operations.

Retain fine-grained edits for real maintenance workflows such as rename, insertion, deletion, operand replacement, body replacement, and typed placeholder refinement.

Do not add a macro language, rewrite language, parser, or template engine for one convenience case.

## Agent-Facing Interface

The external coding agent is the primary program author.

The public machine interface is a product surface, not an incidental serialization detail.

It should provide:

- a closed executable contract;
- stable names and IDs;
- compact discovery;
- exact on-demand expansion;
- revision-bound read batches;
- semantically meaningful structured proposals;
- selected returned bindings;
- compact receipts;
- deterministic typed errors;
- legal constructors;
- visible values and definitions;
- bounded repair context;
- paginated diffs;
- exact run values;
- digest-based unchanged responses where useful.

A low-level node-by-node API is not agent-friendly merely because it uses typed JSON.

Do not require agents to author private compiler indexes, CFG predecessor lists, phi nodes, block layout, artifact offsets, checksums, publication records, cache keys, or durability metadata.

Prefer standard, inspectable control-plane representations until measurements justify custom transport.

One logical request and response type vocabulary should serve the CLI, local service, tests, and machine description.

A custom binary control protocol is not an enduring goal.
Retain or introduce one only when measured latency, throughput, memory, or isolation needs outweigh duplicate codecs, schema surface, test burden, and agent search cost.

Do not invent a compact syntax from intuition.

Evaluate interface changes with controlled agent tasks and deterministic success oracles.

Measure:

- policy and documentation bytes;
- machine-contract bytes;
- request and response bytes;
- provider-reported tokens when available;
- tool calls;
- round trips;
- process launches;
- failed proposals;
- repeated discovery;
- repository searches;
- files opened;
- context reconstruction;
- selected bindings;
- elapsed task time;
- implementation size;
- build and verification cost.

Bytes are not tokens.
Tokens are not correctness.
Do not claim API-cost savings without provider telemetry or a clearly labelled proxy.

Do not reduce cost by weakening validation, skipping rejection tests, hiding limitations, or avoiding final verification.

## Protocol and Untrusted Boundaries

Treat CLI input, IPC frames, structured proposals, artifacts, head metadata, cursors, runtime values, imports, packages, caches, native images, permission handles, and FFI values as untrusted or potentially corrupt.

Every boundary defines:

- version or schema identity;
- framing;
- length and count checks;
- numeric domains;
- canonical IDs;
- unknown-form policy;
- trailing-data policy;
- allocation and depth policy;
- error correlation;
- output bounds;
- timeout or cancellation behavior;
- failure behavior.

Unknown forms reject.

Use closed DTOs or direct closed types.
Do not deserialize semantic requests into arbitrary property bags.

Machine stdout contains one structured response.
Human diagnostics belong on stderr or in a presentation layer.

Protocol replacement is direct when compatibility is not required.
Do not retain old success readers.

Persistent formats require unambiguous magic or schema identity, explicit version, canonical order, checked counts, strict trailing policy, bounded decoding, no Rust-layout dependency, no pointers, no private compiler IDs, and deterministic content identity.

A commit is acknowledged only after the documented crash contract.
If outcome becomes unknowable, stop the writer rather than continue with ambiguous authority.

## Memory Safety

Memory safety is an enduring product requirement.
Do not reduce it to one implementation technique.

Valid language and runtime use must prevent:

- use-after-free;
- double free;
- invalid pointer dereference;
- out-of-bounds access;
- uninitialized reads;
- type confusion;
- data races;
- invalid aliasing assumptions;
- use-after-move;
- duplicated ownership;
- double close of exclusive resources.

Malformed requests, artifacts, runtime values, packages, caches, and native images may reject.
They must not corrupt durable state, violate memory safety, allocate without checked bounds, consume unbounded native stack, or continue after authority becomes ambiguous.

Keep package-wide `unsafe_code = "forbid"`.

Do not weaken it in ordinary campaigns.

A future high-value unsafe boundary requires explicit active-task authorization and must:

1. prove safe implementation or a mature safe dependency is inadequate;
2. isolate unsafe code in the narrowest dedicated crate, process, generated boundary, or module;
3. document the complete safety invariant;
4. minimize its callable surface;
5. validate before entry;
6. test success, rejection, lifetime, aliasing, overflow, and concurrency behavior as applicable;
7. use Miri, sanitizers, fuzzing, differential checks, or platform tools where applicable and available;
8. record trusted assumptions and environment limits;
9. keep safe code as the public default.

“Written in Rust” is not a proof.

The trusted computing base includes the compiler, standard library, build tooling, operating system, dependencies, generated code, native code, and foreign boundaries.

Audit each dependency for:

- current consumer;
- unsafe and native surface;
- build scripts;
- transitive cost;
- platform behavior;
- maintenance;
- security history;
- license;
- replacement cost.

Do not casually reimplement mature cryptography or platform primitives.
Do not add a dependency merely to avoid a small safe helper.

## Memory Management and Resource Ownership

Do not prescribe one universal memory-management mechanism before real workloads require it.

Keep ordinary immutable values simple and unboxed where practical.

When sharing, mutation, cycles, external resources, large objects, concurrency, or foreign interoperation become real requirements, compare:

- inline values;
- moves;
- affine resources;
- regions;
- reference counting;
- tracing collection;
- stable handles;
- copy-on-write;
- explicit managed objects;
- hybrids.

Evaluate:

- memory safety;
- deterministic cleanup;
- cycles;
- sharing;
- mutable identity;
- concurrency;
- pause tails;
- throughput;
- peak memory;
- fragmentation;
- compiler complexity;
- runtime complexity;
- agent burden;
- diagnostics;
- FFI;
- optimization;
- cancellation;
- failure behavior.

Do not reject tracing collection as ideology.
Do not adopt it by default for one prototype.
Do not force borrow proofs for ordinary immutable values when the compiler can derive or avoid them.
Do not hide exclusive external-resource cleanup behind nondeterministic finalization.

Choose semantics by data class when that is smaller and safer than one universal rule.

The current flat-cell runtime is a verified implementation baseline, not the final heap model.

Memory safety, resource exhaustion, resource ownership, deterministic cleanup, aliasing, concurrency safety, and permission security are separate contracts.

## Effects, Capabilities, and Isolation

Pure semantics precede host effects.

When an effect is introduced, define:

- operands;
- result;
- effect class;
- required permission;
- resource ownership;
- cleanup;
- cancellation;
- blocking behavior;
- failure domain;
- accounting;
- replay semantics;
- isolation requirement.

Do not grant filesystem, network, process, terminal, database, clock, entropy, device, environment, or foreign-memory access implicitly.

Resource ownership and permission are distinct.
Specify both.

Do not claim sandboxing that is not implemented.

Prefer a supervised worker when native, foreign, or effectful execution cannot be contained safely in the service address space.

Do not silently retry effectful execution in another engine after effects begin.

## Compiler and Runtime

The semantic route is:

```text
immutable program revision
    -> completeness and semantic validation
    -> deterministic reachable definitions
    -> private executable IR
    -> independent IR verification
    -> interpreter or later acceleration tier
```

Executable IR is derived state.
It is not persisted as program authority.

Dense IDs and locality-oriented layouts remain private.

The interpreter remains the complete semantic oracle during bootstrap.

A future native tier is differential-tested against the oracle.

Agents author structured semantics, not CFG mechanics.

Optimization failure cannot redefine validity.

Runtime traps, domain failure, permission denial, cancellation, and resource exhaustion are distinct structured outcomes.

Calls, recursion, branches, loops, aggregate traversal, decoding, validation, and user-scalable control must not recurse through the Rust stack according to user depth.

Use explicit frames, work stacks, queues, or iterative algorithms.

Avoid universal boxing and unaccounted large-value copying without measured need.

Keep copy, move, drop, layout, ownership, and permission rules explicit before optimization.

## Determinism

Deterministic observable facts include:

- acceptance or rejection;
- persistent identity allocation;
- structured expansion;
- parent revision selection;
- diff order;
- change fingerprints;
- query order;
- repair-context selection;
- artifact bytes;
- snapshot identity;
- reachable-definition order;
- executable lowering;
- interpreter result;
- public collection order;
- diagnostic target selection.

Internal scheduling, hash placement, allocator addresses, process IDs, and filesystem enumeration may vary when they are not observable semantics.

Use semantic order where defined.
Otherwise sort or use ordered collections at public boundaries.

Test insertion-order variation where practical.

Do not pay unnecessary global sorting cost for facts already in semantic order.

## Persistence, Branching, and Concurrency

Full artifact rewrite, full history, full snapshot cloning, one head, one request per connection, and synchronous handling are current baselines.

They are not enduring requirements.

A journal, chunk store, database, compaction scheme, branch graph, merge operation, concurrent read service, parallel compiler, or worker pool requires a named workload and explicit invariants.

Any future branch or merge design must define:

- revision parentage;
- identity domains;
- allocation behavior;
- conflict detection;
- semantic merge rules;
- deterministic ordering;
- publication atomicity;
- retained-history behavior;
- query semantics;
- garbage-collection or pruning behavior;
- failure recovery.

Any future concurrency design must define:

- authority serialization;
- snapshot isolation;
- cancellation;
- backpressure;
- resource accounting;
- deterministic externally observable behavior;
- safe shutdown;
- failure containment.

Do not add async or concurrency because it sounds modern.
Do not forbid it because the bootstrap is synchronous.

## Performance Evidence

Long-term runtime performance is a first-class objective.
Ambition is not a current claim.

Measure representative end-to-end workloads before replacing a baseline.

Record:

- starting commit and dirty state;
- hardware;
- operating system;
- toolchain;
- build mode;
- workload;
- input;
- output oracle;
- warmup;
- sample count;
- statistic;
- tails;
- memory when available;
- environment limits.

Relevant observations include:

- clean and incremental build;
- verification;
- dependencies and target size;
- binaries;
- service start and restart;
- workspace creation;
- transaction preparation and commit;
- queries;
- machine-contract discovery;
- structured expansion;
- reachable-definition construction;
- IR lowering and verification;
- interpreter throughput;
- runtime memory;
- artifact growth;
- agent interaction cost;
- native tiers when introduced.

Label single observations and microbenchmarks honestly.

Do not calculate regression ratios between unequal workloads as if they were equal-work comparisons.

An index, cache, journal, database, async runtime, native backend, allocator, memory manager, or custom protocol needs a named consumer, before-and-after evidence, a preserved oracle, and a reversal condition.

## Application-Driven Development

Use representative applications to expose missing semantics and interface friction.

A retained application must exercise the real service, public machine interface, persistence, compiler, verifier, and runtime.

Do not satisfy acceptance only through private Rust constructors.

When an application reveals a blocker:

1. prove it through the public path;
2. classify it as semantic, interface, diagnostic, performance, documentation, safety, or operational friction;
3. select the smallest dependency-closed repair;
4. reject speculative collateral features;
5. rerun the application and baselines;
6. record cost and remaining limitations.

Prefer one broad honest example plus focused tests over many disconnected demonstrations.

## Architecture Restraint

Before adding a durable concept, identify:

- its data class;
- owner;
- validator;
- producer;
- consumer;
- removed invalid state;
- removed repeated work;
- identity domain;
- serialization need;
- process-boundary need;
- safety obligation;
- agent cost;
- deletion evidence.

Prefer, in order:

1. deletion;
2. an existing closed type;
3. a direct struct or enum;
4. a local helper;
5. a static descriptor;
6. a sorted vector or explicit work stack;
7. one narrow measured index or cache;
8. a process for a real isolation boundary;
9. a general framework only after repeated concrete use.

Do not add without evidence:

- a generic property graph;
- an open semantic dialect registry;
- plugin-defined authoritative node kinds;
- a general constraint solver;
- a visitor framework for one traversal;
- a serializer for same-build private values;
- a database;
- an async runtime;
- a scheduler;
- a cache;
- a reverse index;
- a custom JIT;
- a formal-proof framework;
- a second program or IR authority;
- a hypothetical abstraction;
- documentation machinery.

Multiple consumers are strong evidence for abstraction.
One high-risk safety boundary may justify a focused abstraction when it centralizes a critical invariant.

## Dependencies and Repository Shape

Every dependency has a named current consumer.

Use exact resolved versions through `Cargo.lock`.

Do not change unrelated dependencies.

Use one package until a boundary earns a split through:

- unsafe or FFI isolation;
- an independently useful stable API;
- target isolation;
- a process protocol;
- measured compile-time isolation;
- material dependency isolation.

Split modules by semantic ownership and change locality, not arbitrary line quotas.

Large coherent files are acceptable.
Large files mixing transport, schema description, conversion, validation, and tests are not.

Measure repository comprehensibility through:

- search cost;
- ownership clarity;
- number of files opened for one task;
- duplicated contract facts;
- change locality;
- controlled agent-task evidence.

Delete superseded code in the same milestone.

Do not leave `old`, `legacy`, `compat`, disabled duplicate paths, commented-out replacements, or versioned namespaces without an explicit compatibility requirement.

## Documentation

Keep the maintained documentation set small and role-specific.

Write plain language before formal terminology.

Prefer:

- “programming system built for coding agents” before “AI-primary” or “agent-native”;
- “typed, versioned program model” before “Semantic Program Graph”;
- “source of truth” before “canonical authority”;
- “local background service” before “daemon” in introductory material;
- “named record type” before “nominal product”;
- “variant type with a fixed set of alternatives” before “closed sum”;
- “typed placeholder” before “hole”;
- “saved immutable revision” before “retained snapshot”;
- “explicit permission value or handle” before “capability”;
- “resource-owning or move-only value” before “ownership-bearing value.”

Do not use `source-free` as the primary product label.

Do not imply that a source frontend, sandbox, public network service, native backend, package ecosystem, heap, effect system, or production platform exists when it does not.

Do not present explanatory pseudocode as actual syntax.

When the public path changes, update the README, owning specifications, architecture, status, performance evidence, roadmap, examples, and generated descriptions as applicable in the same milestone.

Documentation review is product acceptance.

## Testing and Verification

Test success, rejection, rollback, restart, corruption, ordering, and boundary behavior as applicable.

Important categories include:

- closed schema coverage;
- strict transport decoding;
- artifact decoding;
- containment;
- scope;
- visibility;
- type contracts;
- operation contracts;
- region contracts;
- named-data contracts;
- placeholder refinement;
- stable identity;
- identity non-reuse;
- allocation rollback;
- deterministic structured expansion;
- stale revisions;
- idempotency;
- bounded receipts;
- selected bindings;
- response preflight;
- diffs;
- queries;
- cursors;
- repair context;
- artifact determinism;
- durable failure atomicity;
- restart;
- writer exclusion;
- direct compilation;
- IR verification;
- explicit-frame runtime;
- resource exhaustion;
- malformed-input memory safety;
- generated sequences;
- deterministic boundary mutation;
- representative applications;
- controlled coding-agent tasks.

Use generated or model-based sequences where many operation orders share an invariant.

Retain failing seeds or minimized corpora.

Use real binaries for the principal end-to-end path.

Do not claim an unrun command.

Run focused checks during development.
Run the full boundary once after the change is coherent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run real service/client integration when protocol, persistence, query, compiler, runtime, CLI, structured authoring, examples, or runnable README paths change.

Run retained malformed-boundary mutation evidence when a trust boundary changes.

Run memory-safety tools when applicable and available.

Distinguish environment limitations from product failures.
Record exact failed commands and relevant output.

Do not repeatedly run the complete expensive boundary after each small edit.

## Workflow

### Orient

1. Record branch, commit, and `git status --short`.
2. Read this file once.
3. Read README, current status, and roadmap.
4. Read only relevant specification sections.
5. Inspect relevant recent commits.
6. Search exact symbols before opening large files.
7. Preserve unrelated work.
8. State one dependency-closed acceptance gate.
9. Record the baseline needed to evaluate it.

Do not read old prompts unless the active task requires historical reconstruction.

### Decide

Identify:

- the user-visible operation;
- the fact owner;
- producers and consumers;
- identity effects;
- success and rejection behavior;
- durability;
- memory and resource safety;
- authorization;
- response bounds;
- trust boundary;
- compatibility cutover;
- representative evidence;
- non-goals;
- stop condition.

Resolve internal choices through semantics, focused prototypes, tests, and measurements.

Do not ask the user to choose details that the active contract and evidence can decide.

### Implement

1. Change the authoritative type or contract.
2. Change every active producer and consumer.
3. Delete displaced code and tests.
4. Add success and rejection evidence.
5. Add rollback, restart, corruption, and boundary evidence where applicable.
6. Run focused checks.
7. Update fact-owning documentation.
8. Run representative application evidence.
9. Run controlled agent evidence when the interface changes.
10. Run the full verification boundary once.
11. Inspect the final diff and worktree.

### End a Turn

End at a buildable, testable, dependency-closed boundary.

Do not leave:

- two authorities;
- two active protocol or artifact versions;
- two preferred authoring paths;
- two semantic execution routes;
- half-migrated requests;
- disabled legacy code;
- undocumented durable changes;
- unpreflighted commits;
- README claims ahead of implementation;
- memory-safety claims ahead of evidence;
- prompt-only planning when implementation was requested and achievable.

A handoff names exact paths, symbols, commands, failures, unresolved evidence, and the next gate.

Keep the handoff compact enough to avoid repository rediscovery.

## Multi-Agent and Git

The lead agent owns architecture, semantic integration, documentation truth, and final verification.

Use subagents for bounded independent questions, controlled black-box trials, or disjoint implementation areas.

Give each subagent:

- one exact question;
- allowed paths and commands;
- evidence requirements;
- non-goals;
- stop condition;
- compact output format.

Do not let subagents create independent schema, identity, persistence, compiler, runtime, or README authority.

Inspect and integrate results against the actual checkout.

Never use `git reset --hard`, `git clean -fd`, or force push.

Prefer one cohesive local commit per verified milestone when commits are requested or permitted.

Do not create empty planning commits.

Do not change remote state without explicit instruction.

## Stop Conditions

Stop, narrow, or reverse if:

- source text becomes required authority;
- a second mutable graph or AST becomes authoritative;
- a structured proposal is persisted as a competing program;
- arbitrary semantic properties enter authoritative state;
- IDs can be reused;
- names, hashes, offsets, or addresses become mutable identity;
- generic identity-preserving morphing appears without exact semantics;
- default responses or graph-wide queries become unbounded;
- memory safety is weakened;
- unsafe code spreads;
- implicit host authority appears;
- a compatibility bridge remains without a requirement;
- custom transport survives without measured value;
- a cache, index, database, async runtime, native tier, memory manager, or process split appears without measured need;
- a general framework has no demonstrated state-space reduction;
- terminology becomes more impressive but less understandable;
- README becomes a test preamble or protocol dump;
- documentation machinery multiplies;
- repository fragmentation raises search cost;
- performance claims lack evidence;
- token or cost claims lack telemetry;
- an example pulls unrelated platform systems into the gate.

The usual correction is to delete, narrow, return typed facts, use one closed type, move detail to a paginated query, preserve an oracle, or postpone the subsystem.

## Completion Report

Report:

- starting and ending commit;
- milestone;
- user-visible outcome;
- agent-visible outcome;
- authoritative model changes;
- compatibility breaks;
- identity and history effects;
- protocol and machine-contract effects;
- memory and resource safety;
- compiler and runtime effects;
- representative application evidence;
- controlled agent evidence;
- performance and cost evidence;
- focused and full verification;
- environment limitations;
- documentation changes;
- deleted code;
- remaining gaps;
- next evidence gate;
- final worktree state.

Report observable decisions and evidence.
Do not report hidden chain-of-thought.
