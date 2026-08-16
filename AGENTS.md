# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local procedure, but it must not weaken repository-wide requirements for semantic authority, identity, atomicity, durability, determinism, memory safety, capability safety, strict boundaries, verification, evidence, documentation truth, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

Preserve unrelated work. Never reset, clean, overwrite, stage, commit, or force-push work that you did not create.

Do not change remote state unless the active user task explicitly requests it.

## Mission

Build `lkjscript` as a programming system designed primarily for autonomous coding agents.

Humans remain first-class users at the level of intent, explanation, governance, review, operation, and product ownership. Humans are not expected to hand-author the authoritative program representation.

Explain the product in plain technical language:

> An agent edits a typed, versioned program model through a local service. The service validates each proposed change, saves immutable revisions, and compiles and runs selected revisions.

The formal name of the authoritative model is the **Semantic Program Graph** (`SPG`). Use that term where precision matters; do not require a new reader to understand graph or compiler jargon before understanding the product.

Source files are not required program authority. A future textual, visual, or imported representation may be a view, import format, or typed proposal, but it must resolve into the same authoritative model and must not become a second source of truth.

The system must remain memory-safe, deterministic at observable boundaries, failure-atomic for durable changes, strict toward untrusted input, and capable of world-class long-term runtime performance.

Agent interaction cost is a first-class performance dimension. Measure it without weakening correctness.

## Product Vocabulary

Use plain meaning first and formal terminology second.

Prefer in human-facing material:

- “programming system built for coding agents” before “AI-primary” or “agent-native”;
- “typed, versioned program model” before “Semantic Program Graph”;
- “authoritative stored form” or “source of truth” before “canonical authority”;
- “local background service (`lkjscriptd`)” before “daemon”;
- “named record type” before “nominal product”;
- “variant type with a fixed set of alternatives” before “closed sum”;
- “handles every variant” before “exhaustive match”;
- “typed placeholder” before “hole”;
- “fill a placeholder without changing its stable identity” before “identity-preserving refinement”;
- “saved immutable revision” before “retained snapshot”;
- “explicit permission value or handle” before “capability”;
- “resource-owning or move-only value” before “ownership-bearing value.”

Do not use `source-free` as the primary product label. State the positive invariant:

> Program meaning is stored in a typed semantic model. Text may be an optional view or input, but it is not a second authoritative representation.

Formal terms remain appropriate in specifications, executable descriptors, protocol contracts, compiler code, and focused technical discussion. Do not rename stable machine tags or internal symbols solely for cosmetic prose.

Do not use “graph” as a physical-storage mandate. The SPG defines semantic entities, relations, identity, and accepted state. Maps, vectors, dense tables, indexes, databases, or other measured structures may implement that meaning.

Reserve `exact` for real identity, equality, cardinality, byte, ordering, or type contracts. Define overloaded terms such as `closed`, `authority`, `projection`, `identity`, `ownership`, and `safe` on first use in human-facing text.

## Product Surfaces

- `README.md` is the human-first product introduction.
- `docs/spec/` owns accepted normative semantics.
- `docs/architecture.md` owns current component responsibility, data ownership, trusted computing base, and trust boundaries.
- `docs/status.md` owns implemented capability and exact limitations.
- `docs/performance.md` owns measurements, regressions, comparisons, and reversal conditions.
- `docs/roadmap.md` owns ordered evidence gates.
- The generic machine interface and runtime-generated contract are the primary agent-facing surfaces.
- This file governs repository work.
- `prompts/` contains campaign execution artifacts.

Do not collapse these roles. Do not turn README into a test preamble, protocol dump, architecture memo, glossary wall, or agent operating contract. Do not turn specifications into status, status into roadmap, performance into marketing, or prompts into permanent semantic authority.

Add another maintained document only when no existing fact owner can hold the information clearly.

## Operating Posture

The current semantic-model service is a verified baseline, not an untouchable monument.

Backward compatibility is not a product requirement unless the active user explicitly requires it. Use incompatible-change freedom to keep one coherent architecture, not to create churn.

Do not preserve an old API, artifact, protocol, command, schema, test, abstraction, example, term, or document merely because it exists. Do not perform a total rewrite merely because compatibility is unimportant.

Replace a subsystem when the replacement is dependency-closed, materially clearer or more capable, and verified against the enduring invariants.

When replacing a boundary:

- replace active reader and writer together;
- use a new unambiguous version, schema identity, tag set, or magic where needed;
- delete displaced readers, writers, adapters, fixtures, tests, tags, and claims;
- update the owning specification and current status in the same verified milestone;
- retain no legacy mode, edition split, compatibility namespace, dual path, hidden fallback, or silent migration unless explicitly requested.

Git history is the archive for superseded implementation. Newer active user instructions and newer verified repository state take precedence over older prompts and assumptions.

Revisit a standing policy when evidence shows that a bootstrap choice was incorrectly promoted into a permanent law.

## Enduring Invariants

Unless explicitly replaced by the active user:

1. One authoritative mutable program model exists per workspace.
2. Published revisions are immutable.
3. One logical service authority is the only live writer of durable workspace state.
4. Text, JSON, diagrams, debug views, schema output, and structured drafts are transport, presentation, import, or proposal forms, never coequal program authority.
5. Every persisted semantic entity, attribute, ownership slot, child slot, and direct reference belongs to a closed typed schema.
6. Arbitrary property bags, arbitrary string-labelled semantic edges, and unknown semantic forms are forbidden in authoritative state.
7. Every mutation is a typed transaction or a closed typed proposal deterministically expanded into one transaction.
8. A successful commit publishes exactly one revision; rejection and validate-only publish nothing and consume no persistent identities.
9. Stable identity is independent of names, source positions, hashes, compiler indexes, artifact offsets, and addresses.
10. Persistent semantic identities are never reused.
11. Names are lookup and presentation metadata, not identity.
12. Derived facts never become a second mutable source of truth.
13. Only a complete selected-entry reachable definition set may enter executable lowering.
14. The compiler consumes an immutable revision directly.
15. One semantic execution route exists; later tiers accelerate it rather than define competing languages.
16. AI output is untrusted input; deterministic validators decide acceptance.
17. Host access requires explicit typed authority.
18. Accepted language semantics cannot express unchecked memory access.
19. User-controlled depth must not consume unbounded native stack.
20. Observable order is explicit and deterministic.
21. Large results are bounded, streamed, or paginated.
22. Durable state is acknowledged only after the documented publication contract.
23. Corrupt, ambiguous, unsupported, or partially published durable state rejects rather than being guessed into validity.
24. Human-facing claims remain no stronger than implemented and reproduced evidence.
25. Compactness never weakens typing, validation, identity, or error precision.
26. Performance optimization preserves a simple correctness oracle.
27. Representative applications discover missing capabilities; they do not justify speculative platforms.

These invariants outrank the current module layout, protocol version, artifact format, process topology, data structure, memory-management technique, and implementation language.

## Invariant, Baseline, or Open Choice

Classify material architecture claims as one of:

- enduring invariant;
- accepted semantic contract;
- current verified baseline;
- operational policy;
- evidence-gated choice;
- historical fact.

Current baselines include stable Rust, one package, Linux x86-64, synchronous local IPC, immutable full-revision artifacts, full snapshot cloning, full recomputation, scan-based queries, one verified Core IR, an explicit-frame interpreter, flat cells for current values, and no source parser, heap, effects, native backend, or request concurrency.

A baseline is not an eternal prohibition.

Evidence-gated choices include text or visual frontends, physical storage, indexes, caches, journals, databases, pruning, async handling, process isolation, native backends, package infrastructure, cross-platform support, self-hosting, and memory-management strategy.

Do not select or forbid one of these merely to make the architecture sound complete. Record the current consumer, evidence, safety obligations, and reversal condition when selecting one.

## Authority Order

1. Active user task.
2. Root `AGENTS.md`.
3. Active campaign prompt.
4. Accepted files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Machine descriptions derived from executable contracts.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, old commits, issues, discussions, and historical documents.

A prompt controls campaign sequence but does not silently redefine accepted semantics. When a campaign changes semantics, update the owning specification in the same milestone. When active artifacts disagree, repair the fact owner rather than adding a third authority.

## Fact Ownership

- Program model, identity, history, transactions, artifacts: `docs/spec/semantic-graph.md` and executable contracts.
- Language types, operations, control, effects, ownership, execution: `docs/spec/language.md` and operation contracts.
- Daemon, framing, JSON, cursors, machine description: `docs/spec/protocol.md` and protocol/projection types.
- Components, trusted computing base, trust boundaries: `docs/architecture.md`.
- Implemented reality: `docs/status.md`.
- Measurements: `docs/performance.md`.
- Future gates: `docs/roadmap.md`.
- Human explanation: `README.md`.
- Repository procedure: this file.

Do not maintain duplicate status catalogues, glossaries, version registries, roadmaps, or architecture inventories. Generated machine descriptions derive from executable contracts; do not commit a hand-maintained schema copy.

## Source-Independent Authority

The invariant is source independence, not hostility toward text.

An agent must be able to construct, inspect, revise, validate, compile, execute, package, and debug without preserving or round-tripping source files.

A future frontend may import text, render explanatory source, provide a human editing or diff view, attach documentation metadata, or exchange code with another ecosystem. It may not bypass validation, allocate persistent identities independently, own mutable semantic state, persist a parallel AST as coequal authority, make formatting or source position identity, or require render-and-reparse for normal agent editing.

Do not promise that lkjscript will never have source syntax. Promise that source syntax will not become a second authoritative program.

## Semantic Model

The SPG is a closed typed semantic model, not a generic property graph.

Each semantic kind has explicit attributes, owner rules, ordered child slots, reference slots, operands, results, cardinality, completeness, continuity, deletion, and lowering obligations.

Unknown schema elements reject. Do not preserve unknown fields for hypothetical forward compatibility. Evolve through explicit version replacement.

One code-owned contract should provide facts to validators, codecs, queries, machine descriptions, history checks, and lowering where practical. Do not introduce runtime registration for a closed vocabulary or a general graph framework when a direct type or static descriptor is sufficient.

Physical traversal order is not observable unless the semantic contract defines it.

## Identity, History, and Incompleteness

A workspace owns one persistent semantic identity domain. Allocation is monotonic. Rejected and validate-only proposals leave the published frontier unchanged. Deleted identities are never reassigned.

Rename preserves identity. A compatible edit preserves identity only when an accepted continuity rule defines it. Replacement creates new identity unless a specific refinement or movement rule says otherwise. Do not generalize one valid transition into unrestricted morphing.

Structured authoring assigns staged identities in a deterministic documented order. Draft-local handles never become persistent identity. Hashes identify bytes and derived cache inputs, not mutable semantic entities. Dense compiler and runtime indexes remain private.

Identity non-reuse and unambiguous retained history are enduring. Full retention of every artifact and physical tombstone is a current persistence strategy. Future pruning or compaction requires an explicit retention contract, reproducible current state, non-reuse proof, exact failure behavior, and a direct cutover.

Incomplete programs are valid semantic states. Use typed placeholders or exact missing-definition states. A placeholder remains queryable and blocks execution only when reachable from the selected entry.

When refinement promises identity preservation, preserve the target ID, owner, body position, output identity, and existing uses as specified. Final validation still checks scope, visibility, dominance, type, effect, ownership, region role, and result index.

Repair context is deterministic typed data. It may include target contract, owner chain, body context, visible values and definitions, incoming uses, legal constructors, blockers, and resource or permission facts. The service derives correctness-critical facts; models remain outside the authority path.

## Transactions and Structured Authoring

Every mutation names one workspace, one exact base revision, commit or validate-only mode, an optional idempotency key where supported, an ordered closed batch, and a bounded response selection.

A successful commit logically validates the envelope, resolves staged identities, expands structured proposals, applies ordered edits, validates the final model and history, derives deterministic change facts, constructs and preflights a bounded response, preflights durable bytes, durably publishes revision and head, publishes memory state, and returns the preflighted receipt.

A rejected transaction changes none of these states. Do not partially publish, consume IDs on rejection, or guess repairs. Return structured rejection facts. Validate-only follows the same preparation and preflight route, then publishes nothing.

A structured authoring payload may remove repeated canonical scaffolding. It is a closed typed proposal, not source code, a persisted AST, or a second authority. The service expands it into the same semantic entities as fine-grained operations; final validation remains authoritative.

Test expansion order, allocation, returned bindings, rollback, nesting, and rejection. Do not keep two equally preferred creation paths. Retain fine-grained edits only for real maintenance workflows such as rename, insertion, deletion, operand replacement, body replacement, and placeholder refinement.

Do not add a macro language, template engine, rewrite language, or parser for one convenience case.

## Receipts, Diffs, and Queries

The default mutation receipt is compact and bounded. It may contain workspace, revisions, snapshot hash, publication status, total created count, selected bindings, change count and fingerprint, and bounded completeness facts.

Do not return every allocation or a full diff by default. Derive full diffs from saved revisions. Diff and query order is deterministic. Large results are paginated or streamed. Cursors bind workspace, revision, target, purpose, and position strongly enough to prevent cross-use.

Queries are pure over immutable revisions unless they explicitly compare revisions. Correct full recomputation remains the oracle until a measured optimized path exists.

A cache or reverse index requires measured repeated cost, exact invalidation, differential tests, memory accounting, durability classification, and a deletion condition.

Compact results are default; exact expansion is explicit. Independent reads may share one revision-bound batch. Do not combine reads and mutation into an ambiguous atomic request.

## Memory Safety

Memory safety is an enduring product requirement. Do not reduce it to one implementation technique.

Valid language and runtime use must prevent use-after-free, double free, invalid pointer dereference, out-of-bounds access, uninitialized reads, type confusion, data races, invalid aliasing assumptions, use-after-move, and duplicated ownership or double close of exclusive resources when such resources exist.

Malformed requests, artifacts, runtime values, future packages, caches, or native images may reject. They must not corrupt durable state, violate memory safety, allocate without checked bounds, consume unbounded native stack, or continue after authority becomes ambiguous.

Keep the strongest practical package-wide prohibition on local unsafe Rust. The current `unsafe_code = "forbid"` policy is intentional and must not be weakened as collateral.

If a future high-value boundary genuinely requires unsafe code:

1. prove that a safe implementation or mature safe dependency is inadequate;
2. isolate it in the narrowest dedicated module, crate, process, or generated boundary;
3. document the complete safety invariant;
4. minimize the callable surface;
5. validate before entry;
6. add success, rejection, lifetime, aliasing, overflow, and concurrency tests as applicable;
7. use Miri, sanitizers, fuzzing, differential checks, or platform tools where applicable and available;
8. record trusted assumptions and environment limits;
9. keep safe code as the public default;
10. obtain explicit active-task authorization.

Do not claim that “written in Rust” is a proof. The trusted computing base includes the compiler, standard library, operating system, dependencies, generated code, and future foreign or native boundaries. Record material changes in architecture documentation.

Audit dependencies for current consumer, unsafe or native surface, build scripts, platform behavior, and maintenance. Do not add a dependency merely to avoid a small safe helper, and do not casually reimplement mature cryptography or platform primitives.

## Memory Management

Do not prescribe one universal memory-management mechanism before real workloads require it.

Keep ordinary immutable values simple and unboxed where practical. When sharing, mutation, cycles, external resources, large objects, concurrency, or foreign interoperation become real requirements, compare inline values, moves, affine resources, regions, reference counting, tracing collection, stable handles, copy-on-write, explicit managed objects, and hybrids.

Evaluate memory safety, deterministic cleanup, cycles, sharing, mutable identity, concurrency, pause tails, throughput, peak memory, fragmentation, compiler and runtime complexity, agent burden, diagnostics, FFI, optimization, cancellation, and failure behavior.

Do not reject tracing collection as ideology. Do not adopt it by default for one prototype. Do not force borrow proofs for ordinary immutable values when the compiler can derive or avoid them. Do not hide exclusive external-resource cleanup behind nondeterministic finalization.

Choose semantics by data class when that is smaller and safer than one universal rule. The current flat-cell interpreter is a verified implementation of current value semantics, not the final heap model.

## Resource, Capability, and Effect Safety

Memory safety and resource exhaustion are different. A memory-safe program may request too much memory, fuel, time, output, storage, or recursion. Represent exhaustion as a structured result and check budgets before allocation, copy, publication, or irreversible action where practical.

Name the boundary protected by each limit. Do not describe operational policy as language meaning. Every implementation is finite; expose limits honestly, avoid arbitrary small ceilings, and prefer pagination, streaming, chunking, staged transactions, and explicit policy.

Pure semantics precede host effects. When effects are introduced, each host operation owns its operands, results, effect class, required permission, resource ownership, cancellation, blocking, failure, accounting, replay semantics, and isolation requirement.

Do not grant filesystem, network, process, terminal, database, clock, entropy, device, environment, or foreign-memory access implicitly. Resource ownership and permission are distinct: specify each.

Do not claim sandboxing that is not implemented. Prefer a supervised worker when native, foreign, or effectful execution cannot be contained safely in the service address space.

## Protocol, Persistence, and Untrusted Boundaries

Treat CLI input, binary IPC, structured proposals, artifacts, head metadata, cursors, runtime values, and future imports, packages, caches, native images, capability handles, and FFI values as untrusted or potentially corrupt.

Every boundary has an explicit version or schema identity, length and count checks, numeric domains, canonical IDs, unknown-form policy, trailing-data policy, allocation and depth policy, error correlation, output bound, and failure behavior.

Unknown forms reject. Do not deserialize semantic requests into arbitrary property bags. Use closed DTOs or direct closed types with exhaustive conversion.

Machine stdout contains one structured response. Human diagnostics belong on stderr or a presentation layer. Protocol replacement is direct when compatibility is not required; do not retain old success readers.

Each saved revision is immutable. Durable head metadata identifies the authoritative revision and bounded publication metadata; never store a full graph dump, full diff, request body, or unbounded allocation map there.

Persistent formats require unambiguous magic or schema identity, explicit version, canonical order, fixed endianness where binary, checked counts, corruption detection, deterministic content identity, strict trailing policy, bounded decoding, no Rust-layout dependency, no pointers, no private compiler IDs, and no mutable cache truth.

A commit is acknowledged only after the documented crash contract. Failure injection covers publication. If outcome becomes unknowable, stop the writer rather than continue with ambiguous authority.

Full artifact rewrite and full history remain acceptable baselines until measured pressure justifies a journal, chunk store, database, compaction, or pruning.

## Agent-Facing Interface and Cost

The external coding agent is the primary program author. The machine interface should minimize semantic guesswork, repeated discovery, and transport scaffolding.

Provide a closed machine contract derived from executable definitions, stable names and IDs, compact discovery, explicit expansion, revision-bound read batches, structured authoring, selected bindings, compact receipts, deterministic typed errors, legal constructors, visible values and definitions, repair context, paginated diffs, exact run values, and fingerprint-based unchanged responses where useful.

A low-level node-by-node API is not agent-friendly merely because it uses typed JSON. Prefer one semantically meaningful proposal over repeated setup operations when deterministic expansion and final validation remain authoritative.

Do not require an agent to author private compiler indexes, CFG predecessor lists, phi nodes, block layout, artifact offsets, checksums, publication records, cache keys, or durability metadata.

Do not invent a compact syntax from intuition. Compare ambiguity, parser complexity, discoverability, bytes, provider-reported tokens when available, tool calls, failed proposals, diagnostics, implementation size, and evolution cost.

Measure agent cost through policy and documentation bytes, machine-contract bytes, request and response bytes, round trips, CLI launches, failed proposals, repeated discovery, repository searches, files opened, context reconstruction, selected bindings, full scans, build and verification latency, elapsed task time, and provider-reported tokens and cost when genuinely available.

Bytes are not tokens. Tokens are not correctness. Do not claim token or API-cost savings from byte counts. When telemetry is unavailable, report direct evidence without extrapolation.

Reduce cost through plain terminology, a non-repetitive root policy, exact fact ownership, runtime contract discovery, fingerprint reuse, task-scoped projection when measured, selected bindings, batch reads, bounded context, deterministic errors, stable IDs, focused checks, and compact handoffs.

Do not reduce cost by weakening validation, skipping rejection tests, hiding limitations, or avoiding final verification.

## Compiler and Runtime

The semantic route is:

```text
immutable program revision
    -> completeness and semantic validation
    -> deterministic reachable definitions
    -> compact private Core IR
    -> independent Core IR verification
    -> interpreter or later acceleration tier
```

Core IR is derived state and is not persisted as semantic authority. Dense IDs and locality-oriented layouts remain private. Lowering is deterministic at observable boundaries. Verify executable IR before execution.

The interpreter remains the complete semantic oracle during bootstrap. A future native tier is differential-tested against it. Agents author structured semantics, not CFG mechanics. Optimization failure cannot redefine validity.

An acceleration tier may decline before execution and fall back to the same verified route. Do not silently restart effectful execution in another engine after effects begin.

Runtime traps, domain failure, permission denial, and resource exhaustion are distinct structured outcomes. Calls, recursion, branches, loops, aggregate traversal, decoding, and user-scalable control must not recurse through the Rust stack according to user depth.

Use explicit frames, work stacks, queues, or iterative algorithms. Use static layouts where semantics permit. Avoid universal boxing and unaccounted large-value copying without measured need. Keep move, copy, drop, layout, and capability rules explicit before optimization.

A budget failure must not mutate program revisions or corrupt service state. A dropped response must not make committed authority ambiguous.

## Determinism

Deterministic observable facts include acceptance or rejection, persistent ID allocation, structured expansion, diff order, change fingerprints, query order, repair-context selection, artifact bytes, snapshot identity, reachable-definition order, Core IR lowering, interpreter result, public collection order, and diagnostic target selection.

Internal scheduling, hash placement, allocator addresses, process IDs, and filesystem enumeration may vary when not observable semantics.

Use semantic order where already defined; otherwise use ordered collections or explicit sorting at public boundaries. Test insertion-order variation where practical. Do not pay unnecessary global sorting cost for facts already in semantic order.

## Performance Evidence

Long-term runtime performance is a first-class objective. Ambition is not a current claim.

Measure representative end-to-end workloads before replacing a baseline. Relevant observations include clean and incremental build, verification, dependencies and target size, binaries, service start and restart, workspace creation, transaction preparation and commit, queries, machine-contract discovery, structured expansion, reachable-definition construction, Core IR lowering and verification, interpreter throughput, runtime memory, artifact growth, and native tiers when introduced.

Record commit, dirty state, hardware, OS, toolchain, build mode, workload, input, output oracle, warmup, samples, statistic, tails, memory when available, and environment limits.

Label a single observation and a microbenchmark honestly. Do not calculate regression ratios between unequal workloads as equal-work comparisons.

Keep a simple correctness implementation or differential oracle when optimizing. An index, cache, journal, database, async runtime, native backend, allocator, or memory manager needs a named consumer, before/after evidence, and a reversal condition.

## Application-Driven Development

Use representative applications to expose missing semantics and interface friction. A retained application must exercise the real service, public machine interface, persistence, compiler, verifier, and runtime.

Do not satisfy acceptance only through private Rust constructors. Choose an application whose needs belong to the current gate, and do not add a web framework, GUI framework, package manager, database layer, scheduler, or broad standard library to support one example.

When an application reveals a blocker:

1. prove it through the public path;
2. classify it as semantic, interface, diagnostic, performance, or documentation friction;
3. select the smallest dependency-closed repair;
4. reject speculative collateral features;
5. rerun the application and baselines;
6. record cost and remaining limitations.

Prefer one honest broad example plus focused tests over many disconnected demonstrations. Retain older examples when they remain useful focused oracles. Rename or replace an example when its public name materially obstructs understanding.

## Architecture Restraint

Before adding a durable concept, identify its data class, owner, validator, producer, consumer, removed invalid state, removed repeated work, identity domain, serialization need, process-boundary need, safety obligation, agent cost, and deletion evidence.

Prefer deletion, an existing closed type, a direct struct or enum, a local helper, a static descriptor, a sorted vector or explicit work stack, one narrow measured index, one narrow measured cache, a process for a real boundary, and only then a general framework.

Multiple consumers are strong evidence for abstraction, but one high-risk safety boundary may justify a focused abstraction when it centralizes a critical invariant.

Do not add without evidence a generic property graph, open dialect registry, plugin semantic schema, general constraint solver, visitor framework for one traversal, serializer for same-build private values, database, async runtime, scheduler, cache, reverse index, custom JIT, formal proof framework, second program or IR authority, hypothetical abstraction, or documentation machinery.

## Dependencies and Repository Shape

Every dependency has a named current consumer. Consider transitive count, clean build cost, binary size, maintenance, security history, unsafe code, build scripts, native code, platform effect, features, license, and replacement cost.

Use exact resolved versions through `Cargo.lock`. Do not change unrelated dependencies. Do not add a parser generator, async runtime, database, compiler backend, tokenizer, schema framework, benchmark framework, or security tool as collateral.

Use one package until a boundary earns a split through unsafe or FFI isolation, an independently useful stable API, target isolation, a process protocol, measured compile-time isolation, or material dependency isolation.

Split modules by semantic ownership, not arbitrary size rules. Large coherent files are acceptable; large files mixing fact owners are not. Repository comprehensibility is measured by search cost, ownership clarity, change locality, and agent task evidence.

Delete superseded code in the same milestone. Do not leave `old`, `legacy`, `compat`, disabled duplicate paths, commented-out replacements, or versioned namespaces without an explicit compatibility requirement.

## Documentation

Keep the maintained documentation set small and role-specific. Use links instead of copying catalogues.

Write plain language before formal terminology. Keep material limitations visible. Do not call explanatory pseudocode actual syntax. Do not imply that a source frontend, sandbox, public network service, native backend, package ecosystem, heap, effect system, or production platform exists when it does not.

When the public path changes, update README, owning specifications, architecture, status, performance evidence, roadmap, examples, and generated descriptions as applicable in the same milestone.

Documentation review is product acceptance.

## Testing and Verification

Test success, rejection, rollback, restart, corruption, ordering, and boundary behavior as applicable.

Important categories include schema coverage and rejection; strict JSON and binary decoding; containment, scope, visibility, type, operation, region, and named-data contracts; placeholder refinement; stable identity and non-reuse; allocator rollback; deterministic structured expansion; stale revisions; idempotency; bounded receipts; selected bindings; response preflight; diffs, queries, cursors, and repair context; artifact determinism and corruption; durable failure atomicity; restart and writer exclusion; direct compilation; Core IR verification; explicit-frame runtime; resource exhaustion; memory-safe malformed-input handling; generated sequences; deterministic boundary mutation; and representative applications.

Use generated or model-based sequences where many operation orders share an invariant. Retain failing seeds or minimized corpora. Use real binaries for the principal end-to-end path. Do not claim an unrun command.

Run focused checks during development and the full boundary once after coherence:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run real service/client integration when protocol, persistence, query, compiler, runtime, CLI, structured authoring, examples, or runnable README paths change. Run retained malformed-boundary mutation evidence when a trust boundary changes. Run memory-safety tools when applicable and available.

Distinguish environment limitations from product failures. Record exact failed commands and relevant output. Do not repeatedly run the complete expensive boundary after each small edit.

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

### Decide

Identify the user-visible operation, fact owner, producers and consumers, identity effects, success and rejection behavior, durability, memory and resource safety, response bounds, trust boundary, compatibility cutover, representative evidence, non-goals, and stop condition.

Resolve internal choices through semantics, focused prototypes, tests, and measurements. Do not ask the user to choose details that the active contract and evidence can decide.

### Implement

1. Change the authoritative type or contract.
2. Change every active producer and consumer.
3. Delete displaced code and tests.
4. Add success, rejection, rollback, restart, corruption, and boundary evidence as applicable.
5. Run focused checks.
6. Update fact-owning documentation.
7. Run representative application evidence.
8. Run the full verification boundary once.
9. Inspect final diff and worktree.

### End a Turn

End at a buildable, testable, dependency-closed boundary.

Do not leave two authorities, two active protocol or artifact versions, two preferred authoring paths, two semantic execution routes, half-migrated requests, disabled legacy code, undocumented durable changes, unpreflighted commits, README claims ahead of implementation, memory-safety claims ahead of evidence, or prompt-only planning when implementation was requested and achievable.

A handoff names exact paths, symbols, commands, failures, unresolved evidence, and the next gate. Keep it compact enough to avoid repository rediscovery.

## Multi-Agent and Git

The lead agent owns architecture, semantic integration, documentation truth, and final verification.

Use subagents for bounded independent questions or disjoint implementation areas. Give one exact question, paths, evidence, non-goals, stop condition, and compact output. Do not let subagents create independent schema, identity, persistence, compiler, runtime, or README authority.

Inspect and integrate results against the actual checkout.

Never use `git reset --hard`, `git clean -fd`, or force push. Incompatible-change permission does not authorize destruction of unrelated work.

Prefer one cohesive commit per verified milestone when commits are requested or permitted. Do not create empty planning commits or change remote state without explicit instruction.

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
- memory safety is weakened or unsafe code spreads;
- implicit host authority appears;
- a compatibility bridge remains without a requirement;
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

```text
Starting commit:
Ending commit:
Milestone:
Human-facing outcome:
Agent-facing outcome:
Authoritative model:
Terminology changes:
Compatibility breaks:
Identity and history:
Transactions and receipts:
Queries and repair:
Protocol and machine contract:
Artifacts and persistence:
Memory safety:
Resource safety:
Capabilities and effects:
Compiler and Core IR:
Runtime:
Representative application:
Agent-cost evidence:
Performance evidence:
Focused tests:
Generated, fuzz, or mutation evidence:
Full verification:
Environment limitations:
README and examples:
Specifications, architecture, status, performance, and roadmap:
Deleted code:
Remaining gaps:
Next evidence gate:
Worktree state:
```

Report observable decisions and evidence. Do not report hidden chain-of-thought.
