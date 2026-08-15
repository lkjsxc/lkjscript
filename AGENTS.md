# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local procedure but must not weaken repository-wide requirements for semantic authority, identity, atomicity, durability, determinism, safety, strict boundaries, verification, evidence, documentation truth, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

Preserve unrelated work.

Never reset, clean, overwrite, stage, commit, or force-push work that you did not create.

Do not change remote state unless the active user task requests it.

## Mission

Build `lkjscript` as an AI-primary programming system in which autonomous agents construct and maintain programs through an exact machine interface.

Humans remain first-class users at the level of intent, explanation, governance, review, and operation.

Humans are not expected to hand-author the canonical program representation.

The canonical program is one closed, strongly typed Semantic Program Graph owned by one logical daemon.

An external agent must be able to discover the exact active contract, create declarations and definitions, inspect bounded context, validate proposals, commit atomically, query retained revisions, compile, execute, diagnose, repair, restart, and continue without preserving or round-tripping source text.

Long-term runtime performance and agent interaction cost are first-class objectives.

Measure model-facing bytes, round trips, failed proposals, repeated discovery, repository search work, build latency, verification latency, compiler latency, runtime latency, memory, and elapsed task time.

Never claim runtime, build, token, cost, reliability, or usability leadership without reproducible evidence.

## Product Surfaces

- `README.md` is the human-first product introduction.
- `docs/spec/` owns accepted normative semantics.
- `docs/architecture.md` explains current component responsibility and trust boundaries.
- `docs/status.md` states exactly what the current checkout implements and does not implement.
- `docs/performance.md` retains measurements, regressions, and reversal conditions.
- `docs/roadmap.md` states ordered evidence gates.
- The generic machine interface and runtime-generated schema are the primary agent-facing surfaces.
- This file governs repository work.
- Files under `prompts/` are campaign execution artifacts.

Do not collapse these roles.

Do not turn README into a test preamble, raw protocol dump, internal architecture memo, or agent operating contract.

Do not turn specifications into status reports.

Do not turn status into roadmap.

Do not turn prompts into permanent semantic authority.

## Operating Posture

The source-free semantic-graph daemon vertical is the baseline, not an untouchable monument.

Backward compatibility is not a product requirement unless the active user explicitly requires it.

Use incompatible-change freedom to keep one coherent architecture.

Do not use it to create churn.

Do not preserve an old API, artifact, protocol, CLI command, schema, test, abstraction, or document merely because it exists.

Do not perform a total rewrite merely because compatibility is unimportant.

When replacing a boundary:

- replace the active reader and writer together;
- use a new unambiguous version, tag set, schema identity, or magic where needed;
- delete displaced readers, writers, adapters, tests, fixtures, tags, and claims;
- update the owning specification and status in the same verified milestone;
- retain no legacy mode, compatibility namespace, edition split, dual reader, dual writer, hidden fallback, or silent migration.

A compatibility bridge requires an explicit active-user instruction.

Newer user instructions and newer verified repository state take precedence over older prompts and assumptions.

## Product Laws

1. The Semantic Program Graph is the only mutable program authority.
2. Source text is not required to construct, inspect, revise, validate, compile, execute, package, or debug a program.
3. Text, JSON, diagrams, diagnostics, debug views, structured drafts, and schema output are projections or transport.
4. No projection is a coequal program authority.
5. A structured draft is a typed transaction proposal and is never persisted as a second program representation.
6. The daemon is the sole live writer of durable workspace state.
7. Every persisted semantic node, attribute, ownership slot, child slot, and reference belongs to a closed typed schema.
8. Arbitrary property bags and string-labelled semantic edges are forbidden in canonical state.
9. Unknown semantic kinds, fields, slots, tags, operations, and value forms reject.
10. Every mutation is an ordered typed transaction or an exact structured proposal deterministically expanded into one.
11. Every successful commit publishes exactly one immutable snapshot.
12. Rejected and validate-only transactions publish nothing and consume no persistent identities.
13. Persistent Node IDs are workspace-scoped, monotonic, stable, and never reused.
14. Deletion tombstones identity and retained snapshots preserve historical meaning.
15. Names, positions, hashes, dense indexes, offsets, and addresses are not semantic identity.
16. Types, scope facts, effect facts, ownership facts, layouts, diagnostics, indexes, Core IR, and machine code are derived state.
17. Derived state is not a second mutable source of truth.
18. Incomplete programs use explicit typed holes or exact missing-definition states.
19. A published incomplete snapshot remains structurally valid and queryable.
20. Only a complete selected-entry dependency closure may enter executable lowering.
21. The compiler consumes an immutable semantic snapshot directly.
22. The production compiler has one canonical executable IR route.
23. Optimization tiers accelerate that route rather than implement competing languages.
24. AI output is an untrusted proposal and deterministic validators decide acceptance.
25. Structured authoring convenience never weakens final graph validation.
26. Host effects require explicit typed authority.
27. Ambient filesystem, network, process, terminal, database, clock, entropy, device, environment, and foreign-memory authority are forbidden.
28. No mandatory global stop-the-world tracing collector may be introduced.
29. User-controlled graph depth, type depth, call depth, control depth, and decoder depth must not consume unbounded native stack.
30. Operational quotas protect named request, decoder, runtime, workspace, or host boundaries.
31. Operational quotas are not semantic program-size limits.
32. Arbitrary semantic node-count, graph-depth, repository line-count, file-count, or fanout limits are forbidden.
33. Observable order is explicit and deterministic.
34. Hash-table, allocator, thread, filesystem, and process order are not semantics unless an explicit contract says otherwise.
35. Public mutation responses are bounded by explicit projections.
36. Potentially large detail belongs in exact revision-bound paginated queries.
37. The daemon preflights the exact committed response before durable publication.
38. Idempotent retry returns the same compact result or a structured conflict.
39. Durable state is acknowledged only after the documented publication contract is satisfied.
40. Corrupt, ambiguous, unknown-version, or partially authoritative durable state rejects.
41. Protocol, JSON, artifact, HEAD, cache, and future native-image decoders reject unknown variants and trailing data.
42. The repository retains one active implementation for each product path.
43. Human-facing documentation states material limitations directly.
44. Agent-interface compactness must not weaken typing, identity, validation, or explicitness.
45. Performance work follows representative evidence and preserves a simple correctness oracle.
46. Application work discovers missing capabilities but does not justify speculative frameworks.
47. A nominal type is identified by its declaration Node ID.
48. A nominal field or variant is identified by its member Node ID.
49. Nominal declaration identity does not silently change shape.
50. Field owner, ordinal, and value type do not silently change under one field identity.
51. Variant owner, ordinal, and payload contract do not silently change under one variant identity.
52. Rename may preserve nominal identity because names are presentation.
53. Shape-changing evolution creates new identity unless an accepted specification defines a narrower continuity rule.
54. By-value nominal cycles reject until an explicit indirection model exists.
55. Nominal layout is deterministic derived state and is not serialized as semantic authority.
56. Product construction checks exact field identities and types.
57. Field projection checks exact field ownership.
58. Variant construction checks exact variant ownership and payload contract.
59. Closed-sum matching is exhaustive by variant identity and executes exactly one selected arm.
60. Match payload binding follows the exact variant payload contract.
61. Aggregate runtime resource accounting reflects materialized footprint rather than only IR value count.
62. Machine schema output derives from executable contracts.
63. Schema manifests, sections, full descriptions, and digests are projections of one schema authority.
64. A known schema digest may produce a compact unchanged response.
65. Schema compactness must not omit contracts required to construct a valid request.
66. No model-token or API-cost claim may be inferred from bytes alone.

## Authority and Fact Ownership

Use this authority order:

1. active user task;
2. this root file;
3. active prompt selected by the task;
4. accepted files under `docs/spec/`;
5. executable code and focused invariant tests;
6. boundary schemas derived from executable contracts;
7. `docs/status.md`;
8. `docs/architecture.md`;
9. `docs/performance.md`;
10. `docs/roadmap.md`;
11. `README.md`;
12. comments and examples;
13. old prompts, commits, issues, discussions, and historical documents.

A newer active prompt supersedes older prompts for campaign sequencing.

A prompt does not silently supersede this file or an accepted specification.

When a campaign changes semantics, update the owning specification in the same verified milestone.

Do not create a third authority to reconcile disagreement.

Fact owners:

- canonical graph, identity, transactions, history, and semantic artifacts: semantic-graph specification plus executable graph, transaction, validation, history, and artifact code;
- language types, nominal declarations, operations, control, match, and execution semantics: language specification plus executable contracts;
- daemon and machine transport: protocol specification plus executable request, response, codec, and machine-projection definitions;
- current component responsibility and trust boundaries: architecture;
- implemented state and limitations: status;
- measurements and reversal conditions: performance;
- ordered future gates: roadmap;
- human introduction: README;
- repository operating policy: this file.

Do not maintain duplicate catalogues, fact ledgers, digest registries, or documentation shards that repeat another owner.

## Data Classes

### Semantic State

Semantic state includes workspace identity, stable node identity, node kind, typed attributes, typed ownership, ordered child slots, direct semantic references, nominal declaration and member identity, immutable nominal shape, operation payloads, explicit holes, incomplete definitions, selected entry, allocation frontier, tombstones, and revision.

Semantic state belongs in immutable snapshots and canonical `.lkjscript` artifacts.

### Derived State

Derived state includes resolved types, binding and scope facts, blockers, incoming references, dependency closure, legal constructors, repair contexts, diffs, type dependency order, layouts, field offsets, discriminants, runtime-cell counts, Core IR, machine code, profiles, query indexes, schema projections, and schema digests.

Derived state may be recomputed or cached only when evidence and exact invalidation justify it.

### Executable State

Executable state includes verified Core IR, interpreter frames, initialized-value state, aggregate storage, runtime handles, active capabilities, and machine code.

Executable state is absent from the semantic artifact.

### Presentation and Transport

Presentation and transport include JSON, binary IPC, CLI text, README diagrams, debug views, schema projections, request IDs, pagination cursors, structured drafts, and runtime-value DTOs.

Presentation and transport never become semantic identity.

## Human and Agent Surfaces

README must explain, in ordinary technical language:

- what lkjscript is;
- why it exists;
- who it is for;
- what is unusual;
- how humans and agents use it;
- what a `.lkjscript` file is;
- what the current checkout can and cannot do;
- how to try the real product path;
- where specifications, architecture, status, performance, and roadmap live.

Explain the product before build and test commands.

Do not use giant inline JSON as the primary explanation.

Label explanatory pseudocode as non-canonical pseudocode.

Do not imply source syntax, a public network service, a sandbox, native code, packages, or production readiness when absent.

Update README in the same verified milestone when the public product path changes.

The agent-facing interface should:

- expose a complete closed vocabulary;
- derive schema descriptions from executable contracts;
- expose a compact schema manifest, exact sections, a canonical digest, and explicit full projection;
- allow several sections in one request;
- allow a compact unchanged response for a known digest;
- use stable canonical IDs and lowercase machine names;
- make compact summaries the default and exact expansion explicit;
- support revision-bound batches, bounded receipts, deterministic typed errors, repair context, legal constructors, visible values, and nominal type context;
- use aggregate typed writes when they remove deterministic storage scaffolding;
- never require agents to author compiler indexes, CFG blocks, predecessor lists, phi nodes, layout offsets, checksums, or durability metadata.

Before preserving public boilerplate, measure request bytes, response bytes, round trips, failed proposals, selected bindings, repeated schema bytes, and implementation complexity.

Prefer semantically meaningful proposals over transport scaffolding when expansion is deterministic and final validation remains authoritative.

Use identity-keyed field and variant references where ordinal-only authoring is fragile.

Canonical storage may normalize exact identity-keyed input into declaration order.

Do not abbreviate stable names merely to win a byte table.

## Machine Schema Discovery

One executable contract owns the complete machine schema.

A manifest, section, full description, and digest are deterministic projections of that contract.

Compute the digest from complete canonical facts while excluding the digest field itself.

The manifest should identify schema identity, digest, active versions, section codes, and boundary limits needed for projection.

Allow multiple exact sections per request.

Keep the section set small enough to avoid round-trip inflation.

A matching known digest may return a compact unchanged result.

Do not persist schema responses or add daemon-side caching until repeated computation is measured.

Do not hand-maintain a second JSON schema.

Do not make documentation the only complete schema.

Measure manifest, selected-section, full, and unchanged bytes.

Do not claim token savings without model telemetry.

## Canonical Graph and Nominal Data

The canonical graph is not a generic property graph.

Each node kind has explicit attributes, owner rules, child slots, order, reference slots, cardinality, completeness rules, and history rules.

Unknown schema elements reject.

One code-owned contract should serve validators, codecs, queries, legal constructors, repair context, machine descriptions, lowering, history checks, and diffs.

Prefer closed enums, direct structs, exhaustive matches, static descriptors, and small context-dependent helpers.

Do not add a registry or solver for a closed vocabulary.

A nominal product or sum declaration is a persistent semantic node.

A product field and sum variant are persistent semantic children.

A semantic nominal type names the declaration Node ID.

A product declaration owns an ordered field sequence.

A field owns one product, ordinal, display name, and exact value type.

A sum declaration owns an ordered variant sequence.

A variant owns one sum, ordinal, display name, and zero or one payload type.

Use a product payload when a variant needs several named values.

Names remain presentation.

Nominal shape is immutable under one declaration identity.

Create declarations atomically through closed structured transaction operations.

Do not publish half-created nominal declarations.

Forward type references may resolve through staged transaction identities.

Final graph validation remains authoritative.

Primitive and nominal declarations form one exact by-value dependency graph.

Product fields and variant payloads contribute edges.

Reject direct and indirect by-value cycles.

Do not add implicit boxing, tracing, reference counting, or ownership solely to accept recursion.

Cycle detection is deterministic and iterative.

A rejected cycle publishes nothing and consumes no IDs.

Acyclic semantic depth has no arbitrary maximum.

Layout is deterministic derived state.

It is absent from canonical artifacts.

Use checked arithmetic and report exact unrepresentable layout rather than wrapping or silently saturating.

At minimum derive size or unrepresentable state, alignment, runtime-cell count, product field offsets, sum discriminant, and payload layout.

If optimized layout caching appears later, differential-test it against full recomputation.

## Operation Contracts

Each operation constructor owns one authoritative contract covering stable code and tag, machine name, operand arity and types, use modes, result types, literal fields, direct declaration references, owned region rules, terminator status, completeness, effects, capability requirements, lowering obligations, and continuity rules.

Validators, queries, codecs, lowering, interpreters, history checks, and schema descriptions consume the same facts.

Use narrow closed dynamic rules for calls, product fields, optional variant payloads, and match variants.

Do not build a general constraint language.

The immutable nominal kernel uses:

- product construction naming one product and every exact field;
- field projection naming one exact field;
- variant construction naming one exact variant and optional payload;
- closed-sum match with one arm per exact variant.

Product construction rejects missing, duplicate, foreign, extra, invisible, or wrongly typed fields.

Field projection requires an operand of the owning product type.

Nullary variants accept no payload.

Payload variants accept exactly one value of the exact type.

Match arm order is canonical declaration order.

Each variant appears exactly once.

Payload arms receive exactly one typed block argument.

Nullary arms receive none.

Every arm yields one exact result type.

Only the selected arm executes.

Do not add guards, wildcards, open sums, overlapping patterns, or a general pattern engine as collateral.

## Identity, History, Holes, and Transactions

Persistent allocation is staged and monotonic.

Rejected and validate-only requests leave the published frontier unchanged.

Deletion tombstones every deleted ID.

Rename preserves identity.

A same-constructor edit preserves identity only under its exact continuity rule.

A replacement creates new identity unless a specific refinement rule says otherwise.

Structured expansion assigns staged IDs in one documented deterministic order.

Dense compiler and layout IDs are private derived identities.

Nominal history requires stable declaration kind, owner, ordered member IDs, member ordinals, field types, and variant payload contracts for every surviving identity.

Do not generalize one valid continuity rule into unrestricted morphing.

Typed holes record one exact primitive or nominal expected type.

A hole blocks executable lowering only when reachable from the selected entry.

The default identity-preserving refinement is one-way from a hole to a complete regionless non-terminator operation with the same single result type.

It preserves Node ID, owner, body position, output identity, and existing uses.

Nominal holes may refine to exact product or variant construction.

Scalar holes may refine to exact projection when types agree.

Refinement to another hole, a terminator, a different result type, an already complete operation, or a region-owning match rejects unless a future accepted specification defines exact continuity.

Every mutation request names workspace, exact base revision, mode, optional committed idempotency key, ordered closed batch, and bounded response projection.

Preparation stages graph changes, IDs, tombstones, declaration resolution, validation, change facts, artifact bytes, HEAD bytes, and exact response bytes.

A successful commit logically performs:

1. boundary and idempotency validation;
2. explicit local-handle scan;
3. staged identity allocation;
4. deterministic structured expansion;
5. primitive and nominal type-target resolution;
6. field and variant resolution and canonical normalization;
7. canonical edit application;
8. final graph, type-cycle, and history validation;
9. deterministic diff and receipt derivation;
10. exact response, artifact, and HEAD preflight;
11. durable publication;
12. in-memory publication;
13. acknowledgement.

A rejected request changes none of those states.

Validate-only uses the same semantic preparation and byte-preflight route but publishes nothing.

Do not partially publish or heuristically repair invalid requests.

## Structured Authoring, Receipts, Queries, and Repair

A structured draft is allowed when it removes repeated canonical scaffolding.

It is a closed typed transaction payload, never persisted, hashed as meaning, queried after expansion, or compiled directly.

Drafts may describe nominal declarations, members, function signatures, structured bodies, expressions, nested regions, match arms, payload bindings, and terminators.

The transaction layer expands them deterministically into canonical nodes.

All explicit handles are scanned before allocation.

This includes declarations, fields, variants, functions, parameters, operations, loop arguments, and match payloads.

Reject zero, duplicate, undeclared, wrong-category, over-depth, and over-item handles before publication.

A type draft may name a primitive or existing or local nominal declaration.

Product-construction drafts should key values by field identity.

Match drafts should key arms by variant identity.

The expander may normalize exact entries into declaration order.

Do not retain two equally preferred public creation paths.

Remove or privatize displaced low-level scaffolding once structured authoring covers the active language.

Retain narrow maintenance edits such as refinement, operand replacement, insertion, deletion, rename, and body replacement where they have current consumers.

The default receipt is compact and bounded.

It may contain identity and revision facts, publication status, total created count, only selected explicit bindings, exact change count and digest, and bounded completeness facts.

Full diffs and all allocations are not default response content.

Full semantic diffs are deterministic, revision-bound, and paginated.

A hole becoming a product or variant constructor remains a refinement fact rather than delete/create churn.

Queries are pure over exact immutable revisions.

Correct full scans are the oracle.

Do not add caches or reverse indexes before representative repeated cost is measured and invalidation is exact.

Potentially large collections are deterministic and paginated.

Independent reads may share one revision-bound batch.

A nominal type query should provide declaration identity, kind, owner, member count, member IDs and types, and derived layout summary without a whole-workspace dump.

Repair context is deterministic typed context for one target and may include expected type, owner chain, body window, visible values, incoming uses, legal constructors, blockers, exact nominal declaration facts, field requirements, variant requirements, and match-arm template.

A product-typed hole should be repairable after one bounded context query whenever its member list fits policy.

Do not invoke an LLM inside the daemon for correctness-critical context selection.

## Boundaries, Runtime Values, and Persistence

The daemon protocol is closed, typed, bounded, and versioned.

Unknown versions, fields, variants, tags, counts, indexes, IDs, cursors, and trailing data reject.

A protocol replacement is direct.

Binary IPC and the generic JSON CLI are transport only.

Machine mode emits one compact structured response on stdout.

Diagnostics belong on stderr.

Pretty output is explicit.

Do not accept arbitrary JSON property bags inside semantic requests.

Use canonical machine ID forms and stable lowercase enum names.

Public runtime values are typed projections bound to one exact workspace revision.

Primitive forms are exact.

Product values identify the product declaration and exact field IDs.

Output fields use declaration order.

Identity-keyed input may normalize after exact validation.

Sum values identify the sum declaration and exact variant ID, with payload exactly when required.

Runtime input validation checks declaration, member ownership, count, nested types, and operational depth and item policies.

Runtime output never exposes dense compiler IDs.

Boundary depth and item limits are operational, not semantic type-depth limits.

Avoid unbounded native recursion in runtime-value JSON conversion, binary encoding and decoding, type validation, normalization, internal flattening, output reconstruction, and adversarial formatting.

A mandatory result that cannot fit response policy should reject before entry whenever exact result type makes this predictable.

Each retained revision is immutable.

HEAD remains independently bounded and contains no full diff, graph dump, request body, or allocation map.

Persistent format changes use new unambiguous identities and reject old bytes when compatibility is not required.

Canonical formats have fixed magic, explicit version and schema identity, canonical order, fixed endianness, checked counts and lengths, corruption detection, deterministic hash, strict trailing policy, defensive decoding, no Rust-layout dependency, no pointers, no compiler IDs, and no derived layout.

A commit is acknowledged only after the documented crash contract.

Failure injection covers publication steps.

If outcome becomes genuinely unknowable, stop authority rather than continue ambiguously.

Full snapshot rewrite, retained history, and scan queries remain acceptable until measurements justify replacement.

## Compiler, Core IR, and Runtime

The canonical route is:

```text
immutable Semantic Program Graph snapshot
    -> completeness and semantic validation
    -> deterministic function and nominal-type closure
    -> deterministic derived layouts
    -> dense private Core IR
    -> independent Core IR verification
    -> interpreter or later native lowering
```

Core IR is private derived state and absent from semantic artifacts.

Dense function, block, value, type, field, and variant IDs never escape as semantic identity.

Every lowering is deterministic.

The verifier rejects malformed IR independently from compiler construction.

Agents author semantic regions, not CFG bookkeeping.

Closed-sum match may lower to one private exhaustive switch or equivalent closed terminator.

A private type table may describe reachable primitive and nominal types and layouts.

The verifier checks every type, field, variant, aggregate instruction, match arm, branch argument, and recomputable layout fact.

The interpreter remains the complete semantic oracle.

Runtime traps and resource exhaustion are structured and leave semantic and durable state unchanged.

Calls, recursion, branches, loops, match, aggregate traversal, and user-depth processing use explicit frames and work structures rather than Rust recursion.

Use deterministic fuel or another exact operational budget.

Aggregate values use static derived layouts where practical.

Avoid universal boxing without evidence.

Do not count a large product as one live scalar slot.

Frame accounting includes materialized aggregate footprint.

Entry and call admission check footprint before allocation.

Returned frames release it.

Product construction initializes every field exactly once.

Projection reads the exact field.

Variant construction initializes exact discriminant and payload and canonicalizes inactive storage.

Match reads a validated discriminant, binds only selected payload, and executes one arm.

The public runtime DTO need not be the private interpreter representation.

Prefer the smallest private representation satisfying stack safety, exact accounting, deterministic behavior, and reasonable copy cost.

All values in the pure nominal bootstrap may retain copy semantics.

Do not infer the final ownership model from that bootstrap.

Pure semantics precede effects.

Do not add effects, capabilities, ownership, FFI, native code, or a managed heap as collateral to nominal data.

## Security, Resources, Determinism, and Evidence

Treat JSON, binary IPC, schema projection requests, structured drafts, declaration lists, field bindings, match arms, runtime aggregates, artifacts, HEAD, durable directories, cursors, Core IR, caches, native code, and foreign values as untrusted.

Malformed input may reject but must not panic, corrupt state, publish partially, allocate without a checked boundary, hang, or consume unbounded native stack.

Do not claim sandboxing that is not implemented.

Fuzz or deterministically mutate active decoders, transactions, Core verification, runtime-value decoding, and native boundaries.

Retain failing seeds.

Policy constants belong near named boundaries and have focused tests.

Examples include frame bytes, CLI bytes and nesting, schema section count, page size, batch size, returned bindings, artifact bytes, runtime arguments, runtime value items and nesting, result bytes, fuel, frames, live cells, and structured-draft depth and items.

For identical accepted input, snapshot, build contract, and explicit policy, make deterministic:

- acceptance;
- staged IDs;
- draft expansion;
- type and member resolution;
- field and arm normalization;
- cycle error selection;
- layout;
- member offsets and discriminants;
- diffs and digests;
- queries;
- repair context;
- artifacts and snapshot hashes;
- function and type closure;
- dense mappings;
- Core IR;
- interpreter result and trap origin;
- runtime aggregate output;
- schema digest and projection order.

Use ordered collections or explicit sorting at every observable boundary.

Measure before replacing a baseline.

Relevant evidence includes fresh and incremental builds, dependency count, target and binary size, daemon start and restart, transaction and durability cost, query bytes and latency, schema bytes and latency, repair context, artifacts, type closure, layout, Core lowering and verification, interpreter startup and throughput, aggregate operations, copy cost, calls, branches, loops, recursion, and memory.

Record hardware, OS, toolchain, commit, build mode, warmup, samples, workload, oracle, median, tails, and memory when available.

Label microbenchmarks.

Record regressions as well as improvements.

Do not claim model tokens without real telemetry.

Track coding-agent request and response bytes, round trips, process launches, repeated schema bytes, files opened, large files reread, failed mutations, unnecessary bindings, rediscovery, API calls, verification commands, and rebuilds.

Reduce cost through exact semantics, compact defaults, schema digests, multi-section projection, selected bindings, batch reads, aggregate writes, stable IDs, repair context, nominal type context, deterministic errors, one fact owner, focused checks, and compact handoffs.

Do not reduce cost by weakening validation, rejection evidence, final verification, or failure reporting.

## Architecture, Dependencies, Repository, and Documentation

Before adding an abstraction, identify its current producer, current consumer, invalid state removed, repeated work removed, proving test, proving measurement, authority effect, identity effect, serialization effect, process-boundary effect, agent-cost effect, and deletion condition.

Prefer, in order:

1. delete dead code;
2. closed enum;
3. authoritative struct;
4. local helper;
5. static descriptor;
6. sorted vector;
7. explicit work stack;
8. one narrow measured index;
9. one measured cache;
10. a process for a real boundary;
11. a general framework only after multiple real consumers exist.

Do not add without evidence a generic property graph, open dialect system, plugin system, general solver, visitor framework for one traversal, registry for a closed vocabulary, serializer for same-build private values, database, async runtime, scheduler, cache, reverse index, custom JIT backend, proof infrastructure, second graph or IR authority, generic pattern framework, or hypothetical abstraction.

Every dependency has one named current consumer.

Consider transitive count, fresh build cost, binary size, activity, trust boundary, unsafe code, platform impact, features, and license.

Do not add parser, schema, graph, async, database, compiler-backend, tokenization, benchmark, snapshot-testing, or big-integer dependencies as collateral.

Use exact lockfile versions.

Use one Rust package until unsafe isolation, independent stable API, process boundary, target-specific dependency isolation, or measured compile isolation earns a split.

Split modules on semantic ownership, not line count.

Delete superseded code in the same milestone.

Do not retain active `old`, `legacy`, `compat`, disabled, or commented-out implementations.

Maintain a small role-specific documentation set:

- semantic graph specification;
- language specification;
- protocol specification;
- architecture;
- status;
- performance;
- roadmap;
- README.

Add another maintained document only when none can own the fact.

Specifications state accepted semantics.

Status states implementation and limitations.

Architecture states responsibility and trust boundaries.

Performance states measurements.

Roadmap states future gates.

README explains the product.

Generated machine descriptions derive from code.

## Testing and Verification

Maintain focused evidence for:

- schema and descriptor exhaustiveness;
- schema digest and projection consistency;
- strict protocol, JSON, artifact, HEAD, and runtime-value rejection;
- containment, ownership, reference-slot, type, scope, visibility, dominance, and terminator rules;
- nominal declaration identity, member order, immutable shape, deletion blocking, and history;
- by-value cycle rejection and deep iterative traversal;
- deterministic layout and unrepresentable handling;
- product construction, projection, variant construction, exhaustive match, payload binding, and laziness;
- structured draft allocation, forward targets, normalization, rollback, validate-only, and idempotency;
- holes, nominal refinement, retained old revisions, and diff continuity;
- query pagination, cursor binding, nominal type context, visible values, legal constructors, and repair context;
- artifact determinism, corruption rejection, old-format rejection, publication failure atomicity, restart, and competing writer;
- exact function and nominal-type closure;
- malformed Core type tables, aggregate instructions, switches, branches, and value use;
- interpreter aggregates, explicit stack safety, fuel, frames, live-cell accounting, traps, and continued daemon usability;
- public nominal Run input and output;
- real daemon and generic client end-to-end behavior;
- generated transaction sequences and deterministic boundary mutation.

Use generated or model-based sequences when many operation orders share an invariant.

Retain failing seeds.

The principal vertical must use the real daemon and public machine client.

Run focused checks while editing.

After a coherent milestone, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run real integration tests when protocol, persistence, queries, compiler, runtime, CLI, structured authoring, nominal declarations, runtime aggregates, schema discovery, or runnable examples change.

Run the required deterministic mutation smoke when a trust boundary changes.

Never claim an unrun command.

Distinguish product failure from environment failure.

Use focused tests first and one full final boundary rather than repeated expensive full runs.

## Workflow

### Orient

1. Record branch, starting commit, and `git status --short`.
2. Read this file once.
3. Read README, status, roadmap, and only relevant specification sections.
4. Inspect newer relevant commits.
5. Search exact symbols before opening large files.
6. Preserve unrelated work.
7. State one dependency-closed acceptance gate.
8. Record active protocol, JSON, artifact, HEAD, and semantic schema identities.
9. Run the smallest current product-path smoke.
10. Record required baseline measurements.

### Decide

Identify the human-visible and agent-visible operation, semantic authority, producers, consumers, stable identity, history, success and rejection behavior, durability, response bounds, security boundary, representative evidence, non-goals, and stop condition.

Do not ask the user to choose internal details that semantics, tests, measurements, or product laws can decide.

### Implement

1. Change the authoritative contract.
2. Change every active producer and consumer.
3. Delete displaced code and tests.
4. Add success, rejection, boundary, restart, and corruption evidence.
5. Run focused checks.
6. Update owning specifications, status, architecture, performance, roadmap, and README as applicable.
7. Run one full verification boundary.
8. Inspect final diff and worktree.

### End a Turn

End only at a buildable, testable, dependency-closed boundary.

Do not leave dual semantic, protocol, artifact, authoring, compiler, runtime-value, or schema authorities.

Do not leave codecs ahead of validation, validation ahead of compiler classification, aggregate IR ahead of verification, runtime aggregates ahead of resource accounting, or README ahead of implementation.

Do not leave disabled legacy code, an intentionally failing branch, undocumented durable changes, unpreflighted committed responses, or unrelated work modified.

A handoff names exact paths, symbols, commands, results, failures, unresolved decisions, next gate, and worktree state.

Keep it compact.

## Multi-Agent Use

The lead owns architecture, semantic consistency, integration, documentation truth, and final verification.

Delegate only bounded independent questions or disjoint implementation areas.

A subagent task states one exact question, paths, required evidence, non-goals, stop condition, and compact output format.

Do not ask subagents to invent competing graph, protocol, identity, persistence, schema, compiler, or README authorities.

Review every result against the actual checkout.

## Git

Inspect the worktree before edits.

Use targeted deletion.

Do not use `git reset --hard`.

Do not use `git clean -fd`.

Do not force push.

Incompatible-change permission does not authorize destruction of unrelated work.

Prefer one cohesive commit per verified milestone when commits are permitted.

Do not create empty planning commits.

## Stop Conditions

Stop and narrow the edit if any of these appears:

- source text becomes required program authority;
- a second mutable graph, AST, or structured draft becomes authoritative;
- arbitrary semantic properties or string-labelled edges enter canonical state;
- IDs are reused or names, hashes, positions, or dense indexes become semantic identity;
- generic constructor morphing appears without exact continuity;
- nominal shape changes under stable identity;
- by-value cycles are accepted through implicit boxing;
- product construction accepts missing or foreign fields;
- variant construction accepts the wrong payload;
- match becomes non-exhaustive, wildcarded, overlapping, or eager;
- aggregate processing consumes unbounded native stack;
- aggregate resource use is counted as one scalar;
- private layouts or Core IDs escape as semantic identity;
- derived layout enters semantic artifacts;
- mutation responses become unbounded;
- full diffs or request bodies enter HEAD;
- unknown semantic fields are preserved;
- graph-wide results become unpaginated;
- schema projections gain a second hand-maintained authority or disagree;
- a cache, index, database, async runtime, native tier, or global collector appears without evidence;
- ambient host authority appears;
- a compatibility bridge or dual version remains;
- a generic framework has one consumer;
- README becomes a build preamble or raw protocol dump;
- documentation machinery multiplies;
- repository fragmentation increases agent search cost;
- performance claims lack evidence;
- token claims lack telemetry;
- an example pulls unrelated platform systems into the gate.

Usually correct by deleting the parallel path, narrowing the contract, using one closed enum or explicit transaction, creating new nominal identity, rejecting by-value cycles, using an explicit work stack, deriving layout, returning a compact receipt or schema projection, moving detail to pagination, or postponing the subsystem.

## Completion Report

```text
Starting commit:
Ending commit:
Milestone:
Human-facing outcome:
Agent-facing outcome:
Authority changed:
Compatibility breaks:
Protocol and JSON version:
Artifact and semantic schema:
HEAD result:
Schema manifest and digest result:
Canonical graph result:
Nominal declaration and history result:
Type-cycle and layout result:
Structured-authoring result:
Transaction and receipt result:
Query and repair result:
Hole-refinement result:
Compiler and Core IR result:
Runtime aggregate and match result:
Representative application:
Agent-cost evidence:
Performance evidence:
Focused tests:
Generated or mutation evidence:
Full verification:
Environment limitations:
README:
Specifications and status:
Deleted code:
Remaining gaps:
Next acceptance gate:
Worktree state:
```

Report observable evidence and decisions.

Do not report hidden chain-of-thought.
