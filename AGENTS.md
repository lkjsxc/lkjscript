# AGENTS.md

## Scope

This file governs the entire repository. A deeper `AGENTS.md` may narrow local
procedure, but it must not weaken repository-wide requirements for semantic correctness,
memory safety, failure atomicity, stable identity, determinism, verification, evidence,
or architectural restraint.

Use English for maintained code, tests, diagnostics, documentation, commit messages, and
machine-readable fields unless an external protocol requires another language. Prompts,
transcripts, scratch plans, copied tool output, raw logs, and temporary measurements are
not repository authority and must not be committed unless the active task explicitly
makes one a maintained product artifact.

## Mission

Build `lkjscript` as an AI-primary, statically typed, memory-safe, collector-free
programming system. AI-primary means an agent can discover, construct, inspect, revise,
validate, compile, and execute programs through deterministic typed semantic operations.
Model inference must never participate in parsing, name resolution, typing, ownership,
effect inference, capability enforcement, artifact acceptance, or runtime correctness.

Source remains an import, package, review, and interoperability format. After import,
source text is not mutable semantic authority. The in-process semantic workspace is the
authoring authority.

Prefer one complete architecture and one dependency-closed vertical over broad partial
frameworks. Long-term performance matters, but correctness, safety, failure atomicity,
one authority, stable identity, deterministic behavior, exact generic semantics, and a
complete generic VM route come first.

## Current Product Model

Read `docs/status.md` for current facts. The supported product is local package checking
and execution plus an in-process semantic workspace.

```text
verified source import or source-free construction
    -> partial-capable SemanticProgram
    -> immutable WorkspaceSnapshot
    -> structured completeness gate
    -> source-optional complete HIR
    -> ownership and memory planning
    -> verified SSA
    -> validated bytecode
    -> generic VM or bounded baseline-native specialization
```

`SemanticProgram` is mutable only while one transaction is staged. `WorkspaceSnapshot`
is the immutable published authority. HIR, SSA, bytecode, native images, projections,
diagnostics, metrics, and presentation text are derived.

Public semantic identities are opaque and stable according to explicit continuity rules.
Compiler IDs, dense indexes, ordinals, addresses, slots, registers, source coordinates,
and machine details remain private.

A successful transaction publishes exactly one revision. Failure publishes nothing. Old
snapshots remain valid. Presentation and source provenance never become semantic
identity.

The following remain absent unless a current measured consumer proves a need: daemon,
RPC, wire protocol, persistent workspace format, collaboration, scheduler, plugin
platform, generalized cache, generalized incremental dependency engine, and
compatibility layers for deleted provisional designs.

## Priority Order

When requirements conflict, use this order:

1. semantic correctness;
2. memory and type safety;
3. transaction failure atomicity;
4. one semantic authority;
5. stable public identity;
6. deterministic observable behavior;
7. complete generic compiler and VM coverage;
8. exact capability, effect, ownership, and cleanup behavior;
9. stack safety and user-scale correctness;
10. direct AI-agent usability;
11. measured performance;
12. simplicity and low agent search cost;
13. documentation precision;
14. compatibility with provisional behavior.

Compatibility is intentionally last. Future platform ideas, prompt length, diff size,
crate count, file count, and line count are not product evidence.

## Authority

Use this order:

1. the active user task;
2. the nearest applicable `AGENTS.md`;
3. normative files under `docs/spec/`;
4. executable code and focused tests;
5. manifests, locks, schemas, commands, and generated contract inputs;
6. `README.md`;
7. `docs/status.md`;
8. `docs/architecture.md`;
9. `docs/performance.md`;
10. `docs/roadmap.md`;
11. comments, historical prompts, and stale prose.

Language semantics belong in `docs/spec/language.md`. Workspace semantics belong in
`docs/spec/workspace.md`. Checkout facts belong in code, tests, manifests, and
`docs/status.md`. Architecture owns responsibility, data flow, authority, and trust
boundaries. Performance documentation owns retained measurements and reversal
conditions. The roadmap owns ordering and evidence gates. Superseded behavior belongs in
Git history.

When artifacts disagree, classify the claim, inspect its owner and focused executable
evidence, preserve accepted semantics, and update or delete stale material in the same
change. Do not preserve an accident merely because it exists. Do not rewrite a normative
contract merely because its implementation is inconvenient. Do not manufacture authority
through copied tables, prompt archives, unconsumed registries, descriptive revisions,
descriptors, or digests.

## Autonomy and Compatibility

Use the actual checkout, current consumers, specifications, focused tests, recent
history, and measurements. Do not ask the user to choose internal alternatives that
repository evidence can decide. Broad authority permits one clean decision, not
unrelated expansion.

If the requested objective is already complete, verify it and stop. When the roadmap has
no selected `Now` item, choose implementation work only from this evidence ladder:

1. a current user or agent operation that cannot complete;
2. a focused correctness or safety failure;
3. an accepted documented gap that blocks a current operation;
4. a measured production-path bottleneck;
5. a demonstrated maintenance burden that causes repeated defects or rework.

Symmetry, a pleasing abstraction, a future service, and a missing API without a current
caller are not evidence.

Backward compatibility is not an objective unless the active task names an independent
persistent or external boundary. Syntax, provisional source encoding, Rust APIs,
commands, crate topology, internal forms, fixtures, tests, and documentation may change
incompatibly.

Prefer a direct cutover. Do not add `v2`, `next`, editions, legacy modes, deprecated
aliases, adapters, dual readers or writers, compatibility flags, or migrations for
nonexistent persistent data. Update every active producer and consumer, then delete
displaced code, stale tests, stale documentation, and obsolete dependencies.

Preserve unrelated work and external state. Never reset, clean, overwrite, force-push,
or otherwise destroy work you did not create.

## Select One Verified Vertical

One implementation turn normally completes one dependency-closed vertical. It must have:

- one concrete user or agent operation;
- one demonstrated defect, accepted blocking gap, current consumer, bottleneck, or maintenance burden;
- one semantic authority and bounded producer-to-consumer path;
- one end-to-end success path;
- one end-to-end rejection path;
- exact identity and transaction behavior;
- focused executable evidence;
- explicit non-goals;
- an exact stop condition.

“Improve the architecture,” “make the language AI-friendly,” and “prepare for
persistence” are not implementation verticals.

A prerequisite representation cleanup is allowed only when the current representation
makes the selected operation dishonest or impossible, removes a real contradiction, and
remains inside the dependency closure. Do not combine roadmap items merely because they
touch the same file. Stop after the selected vertical passes and record the next
evidenced problem without implementing it.

Before implementation, answer privately and concretely:

1. What exact operation improves?
2. Which current consumer needs it?
3. Which focused test, example, query, or benchmark proves the gap?
4. Which artifact owns the semantics?
5. Which representation is authoritative?
6. Which identities survive, are created, or are tombstoned?
7. What remains failure-atomic?
8. Which compiler and runtime route proves success?
9. What is explicitly out of scope?
10. What is the stop condition?
11. Why is broader machinery unnecessary?

If these questions lack concrete answers, verify that the work is already complete or
stop after a focused audit with an exact handoff. Do not build infrastructure to avoid
stopping.

## AI-Primary Interface Standard

An AI-primary interface exposes semantic facts, not text-editing guesses. Agents
retrieve exact entities, owners, types, effects, dependencies, legal constructors,
diagnostics, completeness blockers, and revisions through typed APIs. Supported behavior
is authored through typed edits and transaction-local draft handles.

A transaction-local handle is not a public identity, and a published identity is not a
draft handle. Never accept one in the domain of the other. Names support presentation
and lookup; after resolution they do not replace identities.

Do not require callers to reconstruct private compiler indexes, ordinals, addresses,
source spans, preorder positions, runtime tags, slots, or registers. Do not render
semantic state as source and parse it again.

Every public field must have one exact semantic meaning. Do not expose ignored bounds,
ignored flags, dead metadata, or advisory values that canonical validation does not
consume.

A legal-constructor query must be honest: do not advertise an edit the transaction route
cannot validate, and do not omit supported behavior because a second whitelist became
stale. Mark candidates provisional when canonical validation remains required.

Prefer compact identity-first queries followed by selective expansion. Pagination is
deterministic and revision-bound. Projection is presentation, never authority, identity,
or a transport substitute. Natural-language goals may guide an agent but never determine
semantic correctness.

## Semantic Authority and Source Boundary

The workspace owns semantic authoring state. Source import and source-free construction
both produce that authority and then use the same transaction, query, compilation, and
execution routes.

Do not retain parallel mutable HIR and workspace authorities. Do not retain stale
derived compiler forms in published snapshots. Do not reconnect references by spelling
or infer continuity from source text. Compiler-owned built-ins remain fixed context
unless a normative contract makes them mutable entities.

Source parsing is a boundary. Post-import edits do not reload or reparse source.
Source-free programs compile without synthetic source. `compile_snapshot` is the sole
semantic-snapshot compilation boundary, and incomplete snapshots do not cross it.

## Public Identity

Public identities are namespace-scoped, generation-aware, kind-checked, owner-checked
where applicable, stable across unrelated edits and private compaction, invalid after
deletion, preserved in old snapshots, and checked before use.

Private identities may be dense and relocate. Reconcile every surviving public entity
explicitly. Never infer continuity from names, equal content, hashes, preorder, spans,
vector positions, private IDs, private addresses, or compiler allocation order.

Survivors keep identities. Deletion tombstones identities. Same-name recreation receives
a fresh generation. Replacement descendants receive fresh identities unless a specific
movement operation proves continuity. Private relocation emits no semantic movement.

Names are presentation and lookup unless semantics explicitly say otherwise. Renaming
does not create entities, rebind by text, alter runtime identity, rewrite unrelated
structure, or change nominal identity. Reject no-op edits unless the contract explicitly
publishes them.

## Transactions

Every transaction checks base revision, namespace, generation, liveness, kind, owner,
visibility, overlap, and canonical semantic validity as applicable. Stage allocator
state, semantic changes, derived indexes, completeness, diagnostics, and every fallible
operation before publication.

Failure publishes nothing, consumes no public identity, and preserves the current
snapshot pointer, revision, allocator, diagnostics, blockers, attachments, provenance,
and deterministic future allocation. A valid retry after failure must allocate and
behave exactly like a clean control run.

Success publishes one immutable revision and one deterministic base-to-final semantic
diff. Diffs describe semantic changes, not private addresses, vector shifts, compiler
churn, or compaction.

Reject unsupported overlap instead of adding a generalized edit planner. Deletion owns
only defined containment; independent dependents do not disappear transitively.

## Declaration Authoring and Generics

Creation APIs use exact typed drafts. A declaration-local binder handle is
invocation-local and never enters a published snapshot, query, projection, diagnostic
identity, diff identity, or compiler artifact. Publication maps each accepted local
binder to one stable type-parameter `EntityId`.

Declaration order is semantic where the language defines it. Canonicalize keyed
submissions to declaration order. Do not infer binder identity from a name or ordinal.

Do not copy a function-only rule into another declaration kind merely for symmetry.
Trait bounds, used-binder rules, and signature restrictions apply only where language
semantics define them. A phantom generic parameter is valid when the language permits
it.

A public draft type must distinguish already-published type parameters from
same-declaration local binders. User-controlled type depth must be processed
iteratively.

When a type boundary changes, inventory construction, matching, clone, destruction,
equality, hashing, substitution, validation, ownership restrictions, dependency
tracking, compaction, queries, projection, diagnostics, tests, and external or
persistent boundaries.

Reuse existing stable type-parameter identity domains, generic resolution, substitution,
trait witnesses, and validation. Do not build a second binder identity system or generic
solver. Unsupported construction rejects explicitly.

## Incomplete State

Incomplete snapshots are queryable values. Holes and unresolved references are semantic
nodes. Completeness blockers are structured facts. Diagnostics derive from current
semantic state.

Incomplete snapshots do not compile. Do not install fallback values, placeholder
compiler objects, or automatic ambiguity resolution. Failed completion attempts do not
consume identities.

A hole or unresolved reference removed with its defining subtree becomes stale.
Unrelated incomplete nodes survive unrelated edits.

## Queries and Projection

Queries are revision-labelled and validate namespace and revision before work.
Continuations bind workspace namespace, exact revision, exact query identity, and cursor
state.

Observable order must not depend on hash iteration. Sort by semantic keys where order is
otherwise undefined and preserve declaration or evaluation order where defined.

Return compact typed facts first and expand selectively. Do not expose private dense
indexes as identities. Do not require callers to parse projection text. Projection must
be deterministic but must not become a second schema.

Add a query or dedicated view only when a current consumer cannot inspect a supported
operation through existing typed facts. Neighboring API symmetry is insufficient.

## Compiler, Runtime, Capabilities, and Ownership

Complete semantic snapshots pass canonical consistency and ownership validation. Memory
planning remains authoritative. SSA remains verified. Bytecode remains validated.
Staging must not duplicate compiler logic or bypass canonical validation.

Imported and source-free programs use the same complete HIR route. Source-free tests
prove zero post-construction source loading and parsing. Imported/source-free
equivalence compares semantic facts and observable behavior, not private compiler
identities.

Capabilities are explicit typed values. Operation signatures, effects, ownership,
capability requirements, traps, divergence, and lowering come from canonical operation
contracts, not workspace copies. Do not fabricate grants, suppress effects, copy affine
values, suppress cleanup, or weaken ownership validation.

Moves, borrows, drops, cleanup, control transfer, and failures remain exact. Loop exits
and backedges preserve outer ownership and loans. Early return and failed calls use
canonical cleanup exactly once.

The generic VM is the complete validated route. Baseline native execution is bounded
specialization. Eligibility, lowering, installation, or preparation may decline before
native entry and then execute the unchanged program in the VM. After native entry, never
retry in the VM. Host effects occur exactly once.

Do not narrow valid VM behavior, add public engine selection, add transition policy, or
add compilation caches without a current requirement and evidence.

Unsafe code remains localized at genuine unsafe and FFI boundaries with a local safety
argument. Validate FFI values and preserve W^X and pre-entry installation atomicity.

## Scale and Stack Safety

User-controlled depth must not consume unbounded native stack. Use explicit work stacks
or another proven bounded mechanism for expressions, patterns, types, values,
dependencies, compaction, projection, queries, validation, diffs, and destruction.

Do not impose arbitrary semantic depth limits or turn benchmark sizes into language
maxima. Use wide identities and counts with checked host-index conversion. Query and
execution budgets bound one response or execution, never semantic authority.

If a new traversal touches user-shaped recursive data, add or reuse small-stack
evidence. Do not add a recursive visitor framework when one local iterative traversal is
sufficient.

## Performance and Boundaries

Performance work requires a current workload, retained baseline, suspected cost,
structural proposal, correctness oracle, and reversal condition. Measure the production
path and relevant phases.

Do not infer speed from code size, allocation count alone, or one sample. Do not
optimize inactive paths, narrow accepted programs for specialization, add parallelism
before independent work is measured, add caches before reuse is measured, or use a warm
service to hide local recomputation.

Incrementality requires measured repeated work, exact dependencies, failure-safe
publication, the full path as oracle, and demonstrated work reduction. Prefer a narrow
proven fast path over a generalized dependency engine. Cache presence, scheduling, and
cache keys never change meaning or identity.

Classify boundaries before adding machinery:

- persistent bytes require exact encoding, validation, corruption behavior, compatibility policy, and a current consumer;
- unsafe or FFI values require checked representation, safety invariants, and atomicity;
- machine-readable CLI output requires a deterministic schema, a consumer, and consumer decoding tests;
- same-build in-process values normally need Rust types and constructor validation, not digests, registries, or serialization;
- closed vocabularies normally need a closed enum, exhaustive matches, and canonical metadata, not a global registry.

Begin with in-memory immutable or copy-on-write snapshots. Add persistence only for
measured crash recovery, retained scale, restart continuity, or a durable consumer. Add
collaboration only for defined multi-writer semantics. Add a daemon only after local
paths are complete enough to measure a process boundary. The language SQLite capability
is not workspace persistence.

Record only retained measurements in `docs/performance.md`. Never fabricate or
overclaim.

## Architecture Restraint

Before adding an abstraction, identify its current producer and consumer, invariant,
invalid state or repeated work removed, proof, whether a local helper is sufficient,
whether it creates another authority or identity domain, whether it requires
serialization or a process boundary, whether it narrows the generic route, whether it
increases agent search space, and its deletion condition.

A new type is not automatically better modeling. A crate is not automatically
modularity. A registry is not authority. A digest is not integrity. A protocol is not
agent usability. A cache is not performance. A planner is not composability.

Prefer, in order:

1. delete obsolete code;
2. reuse an existing identity or typed value;
3. make one representation authoritative;
4. move validation to its owner;
5. replace repeated scans with one local index;
6. replace user-depth recursion with an explicit work stack;
7. simplify a data structure;
8. add a narrow helper;
9. add focused measurement;
10. add cache, process, or protocol machinery only after measured need.

Do not add a generic tree editor for one edit, a visitor framework for one traversal, a
planner for one overlap, a registry for a closed enum, a serializer for a same-build
value, a cache for unmeasured work, a service for an in-process consumer, persistence
for an editing test, or collaboration state without defined writers.

A crate boundary must earn itself through unsafe or FFI isolation, an independently
useful API, a supported target boundary, measured compile isolation, low coupling, or a
current product boundary.

A module owns one coherent responsibility. Large files are not automatically wrong, and
small files are not automatically modular. Do not impose line limits. Split only when
the selected vertical reveals a stable responsibility with a narrow interface and
reduced search fan-out.

Prefer mature dependencies when they remove substantial machinery or risk. Keep local
code when it is smaller, clearer, safer, or measurably better. Do not add dependencies
for trivial transformations, procedural macros for small closed vocabularies, async
runtimes without an async product, or serialization for in-process values.

## Multi-Turn Workflow

### Orient

1. Record the starting commit, branch, and `git status --short`.
2. Read applicable instructions and only required normative and status sections.
3. Inspect recent relevant commits.
4. Search owning symbols before opening large files.
5. Inspect representative producers, consumers, and tests.
6. Run the smallest characterization.
7. Preserve unrelated work.

Keep a compact task ledger in working memory or ignored scratch space: operation,
consumer, contradiction, authority, producer, consumers, identities, atomicity, focused
tests, non-goals, stop condition, and measurement question. Do not commit it.

### Characterize and Decide

Prefer one focused test, existing example, exact query assertion, imported/source-free
equivalence case, malformed-input case at the owning boundary, deterministic work
counter, or retained benchmark.

Do not build a general harness for a focused defect or add measurement without a stated
complexity question. Make one coherent representation decision, record rejected
alternatives briefly, and stop exploring after evidence selects the design.

### Implement

1. update the authoritative representation;
2. update its producer and every active consumer;
3. preserve or reconcile public identity explicitly;
4. reuse canonical type, generic, effect, ownership, and capability validation;
5. delete displaced alternatives;
6. add focused success and rejection evidence;
7. run focused verification;
8. update maintained documentation;
9. run the full boundary once;
10. inspect the final diff;
11. commit one cohesive local change when permitted.

Do not implement the next roadmap item in the same turn.

### Continue Across Turns

A handoff states the exact starting and ending commits, selected vertical, completed
behavior, deliberate non-goals, first remaining evidenced blocker, exact paths and
symbols, smallest next characterization command, environment limitations, and worktree
state.

Do not leave a broad “continue improving” instruction or speculative scaffolding. A
later turn must re-read current evidence rather than assume the previous proposed next
step is still correct.

### Report

Report semantic results and evidence, not hidden reasoning or a transcript. Distinguish
product failures from environment failures and state exact unrun verification.

## Errors and Tests

Reject invalid input at the owning boundary. Use structured errors when callers need
facts. Do not stringify and reparse typed errors, fabricate paths or identities, swallow
host failures, or panic on user-controlled input. Error wording must identify the actual
declaration or operation; do not retain function-specific wording after an error becomes
shared.

Observable output must not depend on hash iteration. Sort by semantic keys where order
is undefined and preserve declaration and evaluation order where defined.

Tests protect intended semantics, not obsolete topology. Add the smallest focused
evidence and prefer coherent table-driven scenarios with independent oracles.

Cover the relevant subset of:

- success and owning-boundary rejection;
- namespace, generation, kind, owner, visibility, and revision checks;
- failure atomicity and allocator rollback;
- identity continuity, tombstones, and old snapshots;
- deterministic diffs, queries, and projections;
- exact types, effects, capabilities, ownership, and cleanup;
- source-free compilation and zero parser or source-loading work;
- VM behavior and bounded native behavior;
- stack safety, private compaction, and complexity shape.

Do not weaken tests to make a redesign pass. Delete tests for obsolete APIs.
Quiet-success CLI tests assert both streams are empty. Machine-output tests decode
output as consumers. Ignored stress tests state why they are ignored and how to run
them. Executable source-free features require production compilation and execution
evidence.

Use this iteration ladder:

1. one narrow affected test;
2. affected module or crate tests;
3. one compiler check after a deliberate type-shape migration;
4. fix warnings immediately;
5. format after the representation stabilizes;
6. workspace verification once.

Capture long output once, inspect the causal region, and do not rerun unchanged
commands.

## Required Verification

Before completion, run:

```bash
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
git diff --check
```

Run `cargo fmt --all` first when formatting is needed. Do not omit `--locked`,
`--all-targets`, `--all-features`, or the release build. Run Docker verification when
the environment supports it and the repository still defines it. Report exact
environment failures and never claim unrun verification.

Inspect final status and diff for prompts, logs, caches, generated files, build
artifacts, and unrelated changes.

## Documentation and Git

Documentation roles are:

- `docs/spec/`: normative contracts;
- `docs/status.md`: implemented checkout facts;
- `docs/architecture.md`: responsibility, flow, ownership, and trust boundaries;
- `docs/performance.md`: retained measurements and reversal conditions;
- `docs/roadmap.md`: ordering and evidence gates;
- `README.md`: concise product entry point.

Do not present future architecture as current, duplicate authority, or retain stale
claims. A completed item leaves roadmap `Now`; do not invent speculative work merely to
keep that section populated.

Add a decision record only for a non-obvious cross-cutting choice likely to be reopened.
Do not create permanent task summaries that immediately become stale.

Record the starting commit and inspect the worktree. Preserve unrelated changes. Keep
prompts, logs, caches, generated files, and build artifacts out of product commits.

Use one cohesive commit per verified vertical when permitted. The subject describes the
semantic result, not the prompt. Before commit, inspect status, diff stat, substantive
diff, `git diff --check`, verification, documentation, and staging. After commit,
inspect status and record the ending commit.

Do not push, force-push, open a pull request, or alter remote state unless the user
explicitly requests it.

## Agent Attention and API Cost

Treat context, tool calls, attention, and API spend as finite. Save cost through less
search and rework, never through weaker correctness or skipped final verification.

- search before reading;
- use exact symbols and narrow line ranges;
- use recent diffs to avoid rediscovering settled decisions;
- keep one compact task ledger;
- do not reread this file repeatedly in one turn;
- do not dump whole compiler files, generated fixtures, or full logs;
- capture long output once and inspect the causal region;
- batch mechanical representation updates;
- use one compiler pass as a migration inventory;
- run focused tests before workspace tests;
- do not rerun commands with unchanged inputs;
- stop generating alternatives after evidence selects one;
- do not add API-cost instrumentation without a consumer;
- hand off exact paths, symbols, invariants, failures, and commands instead of copied source.

The lead agent owns design, integration, and final verification. Use subagents only for
independent bounded investigations or disjoint implementation. Give each subagent one
question, exact scope, required evidence, a stop condition, and a compact output format.
Do not assign overlapping implementation or competing architectures.

## Completion and Stop Rules

Complete only when the selected operation works end to end; invalid input rejects
deterministically; failed transactions publish nothing and preserve allocation;
identities follow continuity rules; old snapshots remain valid; canonical validation
still runs; runtime effects occur exactly once; focused tests protect the root cause;
documentation matches the checkout; required verification passed or is reported
honestly; obsolete paths are deleted; and no speculative adjacent system appears.

Stop and narrow if the change introduces a second authority, unjustified identity
domain, generic planner for one edit, unmeasured cache, consumerless protocol,
serializer without a durable boundary, daemon without a process-boundary consumer,
persistence without retained-state evidence, narrowed VM route, weakened atomicity,
invalid old snapshots, user-depth recursion, arbitrary validity quota, unrelated feature
expansion, compatibility layer, or broad repository reorganization.

Usually correct by reusing an identity or canonical validator, localizing a helper,
deleting a redundant representation, deferring an unproven system, or stopping after a
focused audit. Do not solve uncertainty by building more infrastructure.

Use this final report shape:

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

Report evidence and decisions, not hidden reasoning or a transcript.
