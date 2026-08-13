# AGENTS.md

## Scope and Language

This file governs the repository. A deeper `AGENTS.md` may narrow local procedure but must not weaken semantic, safety, identity, atomicity, determinism, verification, or evidence requirements.

Use English for maintained code, tests, diagnostics, documentation, commit messages, and machine-readable fields unless an external protocol requires another language.

Prompts, transcripts, scratch notes, and raw logs are not repository authority. Do not commit them unless the active task explicitly makes one a maintained product artifact.

## Mission

Build `lkjscript` as an AI-primary, statically typed, memory-safe, collector-free programming system.

AI-primary means deterministic typed semantic operations let agents discover, construct, inspect, revise, validate, compile, and execute programs. Model inference never participates in parsing, resolution, typing, ownership, effects, artifact acceptance, or runtime correctness.

Source remains an import, package, review, and interoperability format. After import, source text is not mutable semantic authority. The in-process semantic workspace is the authoring authority.

Prefer one complete coherent architecture and one dependency-closed vertical over broad partial frameworks. Long-term performance matters, but correctness, safety, atomicity, one authority, stable identity, deterministic behavior, and a complete generic runtime path come first.

## Current Product Boundary

Read `docs/status.md` for current facts. The supported product is local package checking/execution plus an in-process semantic workspace:

```text
verified source import or source-free construction
    -> partial-capable SemanticProgram
    -> immutable WorkspaceSnapshot
    -> completeness gate
    -> source-optional complete HIR
    -> ownership and memory planning
    -> verified SSA
    -> validated bytecode
    -> generic VM or bounded baseline-native specialization
```

`SemanticProgram` is mutable only during one staged transaction. `WorkspaceSnapshot` is the immutable published authority. HIR, SSA, bytecode, native images, projections, diagnostics, and metrics are derived.

Public semantic identities are opaque and stable according to explicit continuity rules. Compiler IDs, dense indexes, ordinals, addresses, slots, registers, source coordinates, and machine details remain private.

A successful transaction publishes one revision. Failure publishes nothing. Old snapshots remain valid. Presentation and source provenance never become semantic identity.

Absent unless a current measured consumer proves otherwise: daemon, RPC, wire protocol, persistence, collaboration, scheduler, plugin platform, generalized cache, and compatibility layers for deleted designs.

## Priority Order

1. semantic correctness;
2. memory and type safety;
3. transaction failure atomicity;
4. one semantic authority;
5. stable public identity;
6. deterministic observable behavior;
7. complete generic compiler/runtime coverage;
8. exact capability, effect, ownership, and cleanup behavior;
9. stack safety and user-scale correctness;
10. direct AI-agent usability;
11. measured performance;
12. simplicity and low agent search cost;
13. documentation precision;
14. compatibility with provisional behavior.

Compatibility is intentionally last. Future platform ideas, prompt length, diff size, crate count, file count, and line count are not product evidence.

## Authority

Use this order:

1. active user task;
2. nearest applicable `AGENTS.md`;
3. normative `docs/spec/`;
4. executable code and focused tests;
5. manifests, locks, schemas, commands, and generated contract inputs;
6. `README.md`;
7. `docs/status.md`;
8. `docs/architecture.md`;
9. `docs/performance.md`;
10. `docs/roadmap.md`;
11. comments, historical prompts, and stale prose.

Language semantics belong in `docs/spec/language.md`; workspace semantics in `docs/spec/workspace.md`; checkout facts in code/tests/manifests and `docs/status.md`; responsibility and flow in `docs/architecture.md`; measurements in `docs/performance.md`; ordering in `docs/roadmap.md`; superseded behavior in Git history.

When artifacts disagree, classify the claim, inspect its owner and focused executable evidence, preserve accepted semantics, and update or delete stale material in the same change. Do not preserve accidents merely because they exist or rewrite normative contracts because implementation is inconvenient. Do not manufacture authority through copied tables, prompt archives, unconsumed registries, descriptors, revisions, or digests.

## Autonomy and Compatibility

Use the actual checkout, current consumers, specifications, focused tests, recent history, and measurements. Do not ask the user to choose internal alternatives evidence can decide.

Broad authority permits one clean decision, not unrelated expansion. Ask only when a genuinely external requirement is missing and no safe explicit assumption can complete the selected vertical.

If the requested objective is already complete, verify it and stop. When roadmap `Now` is empty, select work only from a concrete consumer, focused failure, accepted gap, measured bottleneck, or demonstrated maintenance burden.

Backward compatibility is not an objective unless the task names a current independent boundary. Syntax, source encoding, Rust APIs, commands, crate topology, internal forms, fixtures, tests, and docs may change incompatibly. Persistent package/lock boundaries and the `.lkjscript` extension remain exact where current contracts require them.

Prefer direct cutover. Do not add `v2`, `next`, editions, legacy modes, deprecated aliases, adapters, dual readers/writers, compatibility flags, or migrations for nonexistent persistent data. Update all active producers/consumers and delete displaced code, stale tests/docs, and obsolete dependencies.

Preserve unrelated work and external state. Never reset, clean, overwrite, or force-push work you did not create.

## Select One Verified Vertical

One turn normally completes one dependency-closed vertical with:

- one concrete user or agent operation;
- one demonstrated defect, accepted gap, consumer, bottleneck, or maintenance burden;
- one semantic authority and bounded producer-to-consumer path;
- one success and one rejection path;
- exact identity and transaction behavior;
- focused executable evidence;
- explicit non-goals and stop condition.

“Improve architecture,” “make it AI-friendly,” and “prepare for persistence” are not verticals.

A prerequisite cleanup is allowed only when the current representation makes the selected operation dishonest or impossible, removes a real contradiction, and stays inside the dependency closure.

Do not combine roadmap items because they share a file. Stop after the selected vertical passes. Mention the next evidenced problem without implementing it.

Before implementation answer:

1. What exact operation improves?
2. Which current consumer needs it?
3. Which test/example/query/benchmark proves the gap?
4. Which artifact owns semantics?
5. Which representation is authoritative?
6. Which identities survive, are created, or are tombstoned?
7. What remains failure-atomic?
8. Which compiler/runtime route proves success?
9. What is out of scope?
10. What is the stop condition?
11. Why is broader machinery unnecessary?

If these lack concrete answers, either verify the work is already complete or stop after a focused audit with an exact handoff. Do not build infrastructure to avoid stopping.

## Multi-Turn Workflow

### Orient

1. Record commit, branch, and `git status --short`.
2. Read applicable instructions and only the required normative/status sections.
3. Inspect recent relevant commits.
4. Search owning symbols before opening large files.
5. Inspect representative producers, consumers, and tests.
6. Run the smallest characterization.
7. Preserve unrelated work.

Keep a compact task ledger in working memory or ignored scratch space: selected operation, contradiction/consumer, authority, producer/consumers, identities, atomicity, focused tests, non-goals, stop condition, and any measurement question.

### Characterize and Decide

Prefer one focused test, existing example, exact query assertion, imported/source-free equivalence case, malformed-input case at the owning boundary, deterministic work counter, or retained benchmark. Do not build a general harness for a focused defect or add measurement without a stated complexity question.

Make one coherent representation decision, record rejected alternatives briefly, and stop exploring once evidence selects it.

### Implement

1. Update the authoritative representation.
2. Update its producer and every active consumer.
3. Preserve/reconcile public identity explicitly.
4. Reuse canonical type, effect, ownership, and capability validation.
5. Delete displaced alternatives.
6. Add focused success and rejection evidence.
7. Run focused verification.
8. Update maintained docs.
9. Run the full boundary once.
10. Inspect the final diff and commit one cohesive change when permitted.

### Report

Report starting/ending commits, selected vertical, semantic result, representation decision, compatibility breaks, identity/atomicity behavior, tests, commands, unrun verification, measurements, remaining gaps, and worktree state.

## Evidence Before Abstraction

Before adding an abstraction, identify current producers/consumers, owned invariant, removed invalid state or repeated work, proving test/measurement, and deletion condition. Check whether it creates another authority/identity domain, requires serialization/versioning/process boundaries, narrows the generic path, or increases search space. Prefer a local helper when sufficient.

A new type is not automatically better modeling; a crate is not automatically modularity; a registry is not authority; a digest is not integrity; a protocol is not agent usability; a cache is not performance; a planner is not composability.

Preferred solution order:

1. delete obsolete code;
2. reuse an identity or typed value;
3. make one representation authoritative;
4. move validation to its owner;
5. replace repeated scans with one local index;
6. replace user-depth recursion with an explicit work stack;
7. simplify a structure;
8. add a narrow helper;
9. add focused measurement;
10. add cache/process/protocol machinery only after current measured need.

Do not add a generic tree editor for one edit, visitor framework for one traversal, planner for one overlap, registry for a closed enum, serializer for a same-build value, cache for unmeasured work, service for an in-process consumer, persistence for an editing test, or collaboration state without writers.

Do not expose private coordinates, make names identity, infer continuity from equal content, turn observations into semantic limits, turn fixtures into architecture, preserve history as compatibility, or generalize around hypothetical consumers.

## AI-Primary Semantic Rules

Prefer typed operations, stable opaque identities, immutable snapshots, revision-checked transactions, structured diagnostics, exact continuations, deterministic ordering/pagination, finite candidate sets, explicit unsupported results, queryable incomplete states, direct source-free compilation, failure atomicity, and compact selective queries.

Avoid heuristic rebinding, hidden resolution, implicit globals, unstable coordinates, natural-language correctness paths, stale caches, opaque retries, partial publication, and APIs requiring reconstruction of private compiler state.

Source parsing is a boundary. Resolve names during import, then use identities. Post-import edits do not reload or reparse source. Source-free programs compile without synthetic source. Do not render/reparse semantic edits, retain parallel mutable HIR/workspace authorities, retain stale derived forms in snapshots, or reconnect references by spelling.

Compiler-owned built-ins remain fixed context unless a normative contract makes them mutable entities.

## Identity, Names, and Transactions

Public identities are opaque, namespace-scoped, generation-aware, kind-checked, stable across unrelated edits/private compaction, invalid after deletion, preserved in old snapshots, and checked before use.

Private identities may be dense and relocate. Reconcile every surviving public entity explicitly. Never infer continuity from names, content, hashes, preorder, spans, vector positions, private IDs, or addresses. Survivors keep identities; deletion tombstones; same-name recreation gets a fresh generation; replacement descendants get new identities unless an operation proves movement continuity. Private relocation emits no semantic move.

Names are presentation/lookup unless semantics say otherwise. Renaming does not create entities, rebind by text, alter runtime identity, rewrite unrelated structure, change nominal/member identity, or fabricate rewiring. Collision rules belong to the owning namespace. Reject no-op edits unless their contract publishes them.

Every transaction checks base revision and target namespace, generation, liveness, kind, owner, and visibility as applicable. Stage allocator and all fallible work before publication.

Failure publishes nothing, consumes no identity, and preserves the current snapshot, revision, allocator, diagnostics, blockers, attachments, provenance, and future allocation. Success publishes one immutable revision and deterministic base-to-final semantic diff.

Diffs describe semantic changes, not private addresses, compaction, vector shifts, or compiler churn. Reject unsupported overlap rather than adding a generalized planner. Follow existing final-state versus ordered semantics. Deletion owns only defined containment; independent dependents do not disappear transitively.

## Partial Programs, Types, and Compiler Boundary

Incomplete snapshots are queryable values. Holes and unresolved references are semantic nodes; blockers are structured; diagnostics derive from current state. Incomplete snapshots do not compile. Do not install placeholders, fallback values, or automatic ambiguity resolution. Requested-name intent is not a selected target; candidate sets remain derived unless an author records a stable constrained choice.

Parser recovery requires a source-import consumer. Conflict state requires real merge or concurrent-writer semantics.

Public types use stable semantic identities; compiler-local types may use private identities. User-depth operations must be stack-safe. A type-shape change inventories constructors, matches, clone/drop/equality/hash, substitution, validation, dependencies, compaction, queries, projection, diagnostics, tests, and persistent boundaries.

Reuse existing identity domains and canonical generic resolution/witness machinery. Semantic edits use exact type arguments where required. Do not build a second generic solver or weaken type/trait validation. Unsupported construction rejects explicitly.

`compile_snapshot` is the sole snapshot compilation boundary. It rejects incompleteness before HIR, memory planning, SSA, bytecode, native lowering, or execution. Complete HIR passes canonical consistency/ownership validation; memory planning remains authoritative; SSA is verified; bytecode is validated. Staging does not duplicate compiler logic or bypass validation.

Source-free tests prove zero post-construction source loading/parsing. Imported/source-free equivalence compares semantic facts and behavior, not private IDs.

## Queries, Capabilities, Ownership, and Runtime

Queries are revision-labelled, deterministic, stably ordered, paginated when large, and continuation-bound to namespace/revision/query. Return compact identities first; selectively expand details. Projection text never becomes authority or identity.

A legal-constructor query is honest: do not advertise an edit its path cannot validate, or omit supported behavior because a stale second whitelist was not updated. Expose provisional status when canonical validation remains. Do not create a global registry for a closed query.

Capabilities are explicit typed values. Imported and source-free entry points obey the same capability rules. Operation signatures, effects, ownership, capability requirements, traps/divergence, and lowering come from canonical operation contracts, not workspace copies. Do not fabricate grants, suppress effects, copy affine values, suppress cleanup, or weaken ownership validation.

Moves, borrows, drops, cleanup, control transfer, and failures remain exact. Loop exits/backedges preserve outer ownership/loans. Early return and failed calls use canonical cleanup exactly once.

The VM is the generic validated route. Baseline native execution is bounded specialization. Eligibility/lowering/installation/preparation may decline before entry and then run the unchanged program in VM. After native entry, never retry in VM. Do not narrow valid behavior, add engine selectors, transition policy, or compilation caches without a current requirement and evidence. Host effects occur exactly once.

Unsafe code remains localized at real unsafe/FFI boundaries with a stated safety argument. Validate FFI values; preserve W^X and pre-entry installation atomicity.

## Scale, Boundaries, and Performance

User-controlled depth never consumes unbounded native stack. Use explicit work stacks or another proven bounded mechanism for expressions, patterns, types, values, dependencies, compaction, projection, queries, validation, diffs, and destruction.

Do not impose arbitrary semantic limits or turn benchmark sizes into language maxima. Use wide identities/counts and checked host-index conversion. Query budgets limit one response/work unit, never semantic authority.

Classify boundaries:

- persistent bytes need exact encoding, validation, corruption behavior, explicit compatibility policy, and a current consumer;
- unsafe/FFI values need checked representation, safety invariants, and atomicity;
- machine-readable CLI needs a deterministic schema and consumer decoding tests;
- same-build in-process values normally need Rust types and constructor validation, not digests/registries/serialization;
- closed vocabularies normally need a closed enum, exhaustive matches, and canonical metadata, not a global registry.

Begin with in-memory immutable/copy-on-write snapshots. Add persistence for measured crash recovery, retained scale, restart continuity, or durable consumers; collaboration for defined multi-writer semantics; daemon only after local paths are complete enough to measure a process boundary. The language SQLite capability is not workspace persistence.

Performance work requires workload, baseline, suspected cost, structural proposal, correctness oracle, and reversal condition. Measure the production path and relevant phases. Do not infer gains from code size, allocation count, or one sample; optimize inactive paths; narrow programs for specialization; add parallelism before independent work is measured; add caches before reuse is measured; or use warm services to hide local recomputation.

Incrementality requires measured repeated work, exact dependencies, failure-safe publication, the full path as oracle, and demonstrated reduction. Prefer a narrow proven fast path over a generalized dependency engine. Cache presence/schedule never changes meaning and cache keys never become identity.

Record only retained measurements in `docs/performance.md`; never fabricate or overclaim.

## Crates, Modules, Files, and Dependencies

A crate boundary must earn itself through unsafe/FFI isolation, independently useful API, supported target boundary, measured compile isolation, low coupling, or current product boundary. Do not add crates for aesthetics or hide fan-in.

A module owns one coherent responsibility. Large files are not automatically wrong; small files are not automatically modular. Do not impose line limits.

Split only when the selected vertical reveals a stable responsibility with a narrow interface, reduces search fan-out/conflicting ownership, creates no second authority/framework, and tests can follow it. Do not perform repository-wide shuffles. Search exact symbols and read narrow ranges in large files.

Prefer mature dependencies when they remove substantial machinery/risk; keep local code when smaller, clearer, safer, or measurably better. Do not add dependencies for trivial transformations, proc macros for small closed vocabularies, async runtimes without async products, or serialization for in-process values. Update dependencies and lock data atomically; remove unused edges.

## Errors, Tests, and Verification

Reject invalid input at the owning boundary. Use structured errors where callers need facts. Do not stringify and reparse typed errors, fabricate paths/spans/identities, swallow host failures, or panic on untrusted/user-scale input. Avoid new production `unwrap`/`expect` without a proven invariant and repository convention.

Observable output must not depend on hash iteration, allocator accidents, or private vector order. Sort by semantic keys when undefined; preserve declaration/evaluation order when defined.

Tests protect intended semantics, not obsolete topology. Add the smallest focused evidence and prefer coherent/table-driven scenarios with independent oracles.

Cover relevant success/failure, namespace/generation/kind/owner/visibility, revision, atomicity/allocation rollback, identity/tombstones/old snapshots, deterministic diffs/queries/projections, types/effects/capabilities/ownership/cleanup/exactly-once behavior, source-free compilation, VM/native behavior, stack safety, compaction, and complexity shape.

Do not weaken tests to make redesign pass. Delete obsolete API tests. Quiet-success CLI tests assert both streams empty. Machine-output tests decode as consumers. Ignored stress tests state why and how. Executable source-free features require production compilation/execution evidence.

Iteration ladder:

1. narrow affected test;
2. affected crate/module tests;
3. one compiler check after a deliberate type-shape migration;
4. warnings fixed immediately;
5. formatting when stable.

Capture long output once and inspect the causal region. Do not repeatedly dump logs or rerun unchanged commands.

Before completion:

```bash
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
git diff --check
```

Run `cargo fmt --all` first when needed. Do not omit locked/all-targets/all-features/release. Run Docker verification when available and relevant; report exact environment failures and never claim unrun verification. Inspect final status/diff for prompts, logs, caches, generated files, artifacts, and unrelated changes.

## Documentation and Git

Docs describe current behavior:

- `docs/spec/`: normative contracts;
- `docs/status.md`: implemented facts;
- `docs/architecture.md`: responsibility, flow, ownership, trust boundaries;
- `docs/performance.md`: measurements and reversal conditions;
- `docs/roadmap.md`: ordering and evidence gates;
- `README.md`: concise product entry point.

Do not present future architecture as current, duplicate authorities, or keep stale claims. A completed item leaves roadmap `Now`; do not invent speculative work to keep it populated. Add decision records only for non-obvious cross-cutting choices likely to be reopened.

Record the starting commit and inspect worktree. Keep artifacts out of commits. Use one cohesive commit per verified vertical when permitted; subject describes the semantic result, not the prompt. Do not push or open a PR unless requested.

Before commit inspect status, stat, substantive diff, and `git diff --check`; confirm verification/docs and no prompt/artifact/unrelated staging. After commit inspect status, record the commit, and report intentionally untracked files and unrun verification.

## Agent Attention and API Cost

Treat context, tool calls, attention, and API spend as finite. Save cost through less search/rework, never less correctness or final verification.

- search before reading; use exact symbols and narrow ranges;
- use recent diffs to avoid rediscovering settled decisions;
- keep a compact task ledger;
- do not repeatedly restate the mission or reread this file in one turn;
- do not dump whole compiler files, generated fixtures, or full logs;
- capture long output once and inspect the causal region;
- batch mechanical representation updates;
- use one compiler pass as migration inventory;
- run focused tests before workspace tests;
- do not rerun commands with unchanged inputs;
- stop generating alternatives after evidence selects one;
- do not create API-cost instrumentation without a consumer;
- do not create permanent summaries that immediately stale;
- hand off exact paths, symbols, invariants, failures, and commands instead of copied source.

The lead agent owns design, integration, and final verification. Use subagents only for independent bounded investigations or disjoint work, with one question, exact scope, required evidence, stop condition, and compact output. Do not assign overlapping implementation or competing architectures.

## Completion and Stop Rules

Complete only when the selected operation works end to end; invalid input rejects deterministically; failed transactions publish nothing and preserve allocation; identities follow continuity rules; old snapshots remain valid; canonical validation still runs; runtime and exactly-once effects are correct; focused tests protect the root cause; docs match checkout; required verification passed or is honestly reported; obsolete paths are deleted; and no speculative adjacent system appears.

Stop and narrow if the change introduces a second authority, unjustified identity domain, generic planner for one edit, unmeasured cache, consumerless protocol/serializer/daemon/persistence, narrowed VM path, weakened atomicity, invalid old snapshots, user-depth recursion, arbitrary quota, unrelated feature expansion, compatibility layer, or broad unrelated file reorganization.

Usually correct by reusing identity/canonical validation, localizing a helper, deleting redundant representation, deferring unproven systems, or stopping after focused audit. Do not solve uncertainty by building more infrastructure.

Final report:

```text
Starting commit:
Ending commit:
Selected vertical:
Current consumer:
Semantic result:
Representation decision:
Deleted obsolete path:
Compatibility breaks:
Identity behavior:
Atomicity behavior:
Compiler/runtime evidence:
Focused tests:
Focused commands:
Full verification:
Docker verification:
Measurements:
Documentation:
Remaining gaps:
Worktree state:
```

Distinguish product and environment failures. State exact unrun verification. Report evidence and decisions, not hidden reasoning or a transcript.
