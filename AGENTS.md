# AGENTS.md

## Scope

This file governs the entire repository.

A deeper `AGENTS.md` may narrow local procedure, but it must not weaken the
repository-wide requirements for semantic correctness, safety, identity,
atomicity, determinism, verification, evidence, or architectural restraint.

Use English for maintained code, tests, diagnostics, documentation, commit
messages, and machine-readable fields unless an external protocol requires
another language.

Prompts, transcripts, scratch plans, raw logs, and temporary measurements are
not repository authority. Do not commit them unless the active task explicitly
makes one a maintained product artifact.

## Mission

Build `lkjscript` as an AI-primary, statically typed, memory-safe,
collector-free programming system.

AI-primary means an agent can discover, construct, inspect, revise, validate,
compile, and execute programs through deterministic typed semantic operations.
Model inference must never participate in parsing, name resolution, typing,
ownership, effects, capability enforcement, artifact acceptance, or runtime
correctness.

Source remains an import, package, review, and interoperability format. After
import, source text is not mutable semantic authority. The in-process semantic
workspace is the authoring authority.

Prefer one complete architecture and one dependency-closed vertical over broad
partial frameworks. Long-term performance matters, but correctness, safety,
failure atomicity, one authority, stable identity, deterministic behavior, and
a complete generic runtime route come first.

## Current Product Model

Read `docs/status.md` for current facts. The supported product is local package
checking and execution plus an in-process semantic workspace.

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

`SemanticProgram` is mutable only while one transaction is staged.
`WorkspaceSnapshot` is the immutable published authority. HIR, SSA, bytecode,
native images, projections, diagnostics, metrics, and presentation text are
derived.

Public semantic identities are opaque and stable according to explicit
continuity rules. Compiler IDs, dense indexes, ordinals, addresses, slots,
registers, source coordinates, and machine details remain private.

A successful transaction publishes one revision. Failure publishes nothing.
Old snapshots remain valid. Presentation and source provenance never become
semantic identity.

The following remain absent unless a current measured consumer proves a need:
daemon, RPC, wire protocol, persistence, collaboration, scheduler, plugin
platform, generalized cache, and compatibility layers for deleted designs.

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

Compatibility is intentionally last. Future platform ideas, prompt length,
diff size, crate count, file count, and line count are not product evidence.

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

Language semantics belong in `docs/spec/language.md`. Workspace semantics
belong in `docs/spec/workspace.md`. Checkout facts belong in code, tests,
manifests, and `docs/status.md`. Architecture owns responsibility and flow.
Performance documentation owns retained measurements. The roadmap owns ordering
and evidence gates. Superseded behavior belongs in Git history.

When artifacts disagree, classify the claim, inspect its owner and focused
executable evidence, preserve accepted semantics, and update or delete stale
material in the same change. Do not preserve an accident merely because it
exists. Do not rewrite a normative contract because its implementation is
inconvenient.

Do not manufacture authority through copied tables, prompt archives,
unconsumed registries, descriptive revisions, descriptors, or digests.

## Autonomy and Compatibility

Use the actual checkout, current consumers, specifications, focused tests,
recent history, and measurements. Do not ask the user to choose internal
alternatives that evidence can decide.

Broad authority permits one clean decision, not unrelated expansion. Ask only
when a genuinely external requirement is missing and no safe explicit
assumption can complete the selected vertical.

If the requested objective is already complete, verify it and stop. When the
roadmap has no `Now` item, select work only from a concrete consumer, focused
failure, accepted gap, measured bottleneck, or demonstrated maintenance burden.

Backward compatibility is not an objective unless the active task names an
independent persistent or external boundary. Syntax, provisional source
encoding, Rust APIs, commands, crate topology, internal forms, fixtures, tests,
and documentation may change incompatibly.

Prefer a direct cutover. Do not add `v2`, `next`, editions, legacy modes,
deprecated aliases, adapters, dual readers or writers, compatibility flags, or
migrations for nonexistent persistent data. Update every active producer and
consumer, then delete displaced code, stale tests, stale documentation, and
obsolete dependencies.

Preserve unrelated work and external state. Never reset, clean, overwrite,
force-push, or otherwise destroy work you did not create.

## Select One Verified Vertical

One implementation turn normally completes one dependency-closed vertical.
The vertical must have:

- one concrete user or agent operation;
- one demonstrated defect, accepted gap, consumer, bottleneck, or maintenance
  burden;
- one semantic authority and bounded producer-to-consumer path;
- one end-to-end success path;
- one end-to-end rejection path;
- exact identity and transaction behavior;
- focused executable evidence;
- explicit non-goals;
- an exact stop condition.

"Improve the architecture," "make the language AI-friendly," and "prepare for
persistence" are not implementation verticals.

A prerequisite representation cleanup is allowed only when the current
representation makes the selected operation dishonest or impossible, removes a
real contradiction, and remains inside the dependency closure.

Do not combine roadmap items because they touch the same file. Stop after the
selected vertical passes. Record the next evidenced problem without
implementing it.

Before implementation, answer privately and concretely:

1. What exact operation improves?
2. Which current consumer needs it?
3. Which test, example, query, or benchmark proves the gap?
4. Which artifact owns the semantics?
5. Which representation is authoritative?
6. Which identities survive, are created, or are tombstoned?
7. What remains failure-atomic?
8. Which compiler and runtime route proves success?
9. What is out of scope?
10. What is the stop condition?
11. Why is broader machinery unnecessary?

If these lack concrete answers, verify that the work is already complete or
stop after a focused audit with an exact handoff. Do not build infrastructure to
avoid stopping.

## Semantic and Transaction Invariants

Use typed operations, stable opaque identities, immutable snapshots,
revision-checked transactions, structured diagnostics, deterministic ordering,
revision-bound continuations, queryable incomplete states, explicit unsupported
results, direct source-free compilation, and failure-atomic publication.

Avoid heuristic rebinding, hidden resolution, implicit globals, unstable
coordinates, natural-language correctness paths, stale caches, opaque retries,
partial publication, and APIs that require callers to reconstruct private
compiler state.

Source parsing is a boundary. Resolve names during import, then use identities.
Post-import edits do not reload or reparse source. Source-free programs compile
without synthetic source. Do not render and reparse semantic edits, retain
parallel mutable HIR and workspace authorities, retain stale derived forms in
snapshots, or reconnect references by spelling.

Compiler-owned built-ins remain fixed context unless a normative contract makes
them mutable entities.

Public identities are namespace-scoped, generation-aware, kind-checked, stable
across unrelated edits and private compaction, invalid after deletion, preserved
in old snapshots, and checked before use.

Private identities may be dense and relocate. Reconcile every surviving public
entity explicitly. Never infer continuity from names, content, hashes, preorder,
spans, vector positions, private IDs, or addresses.

Survivors keep identities. Deletion tombstones identities. Same-name recreation
gets a fresh generation. Replacement descendants get new identities unless a
specific movement operation proves continuity. Private relocation emits no
semantic movement.

Names are presentation and lookup unless semantics explicitly say otherwise.
Renaming does not create entities, rebind by text, alter runtime identity,
rewrite unrelated structure, change nominal identity, or fabricate rewiring.
Collision rules belong to the owning namespace. Reject no-op edits unless the
contract explicitly publishes them.

Every transaction checks base revision and target namespace, generation,
liveness, kind, owner, and visibility as applicable. Stage allocator state and
all fallible work before publication.

Failure publishes nothing, consumes no public identity, and preserves the
current snapshot, revision, allocator, diagnostics, blockers, attachments,
provenance, and deterministic future allocation.

Success publishes one immutable revision and one deterministic base-to-final
semantic diff. Diffs describe semantic changes, not private addresses,
compaction, vector shifts, or compiler churn.

Reject unsupported overlap rather than adding a generalized edit planner.
Deletion owns only defined containment. Independent dependents do not disappear
transitively.

Incomplete snapshots are queryable values. Holes and unresolved references are
semantic nodes. Blockers are structured. Diagnostics derive from current state.
Incomplete snapshots do not compile. Do not install placeholders, fallback
values, or automatic ambiguity resolution.

Public types use stable semantic identities. Compiler-local types may use
private identities. User-depth operations must be stack-safe. A type-shape
change must inventory construction, matching, clone, destruction, equality,
hashing, substitution, validation, dependency tracking, compaction, queries,
projection, diagnostics, tests, and persistent boundaries.

Reuse existing identity domains and canonical generic resolution and witness
machinery. Do not build a second generic solver or weaken type and trait
validation. Unsupported construction rejects explicitly.

## Compiler, Capabilities, Ownership, and Runtime

`compile_snapshot` is the sole semantic-snapshot compilation boundary. It
rejects incompleteness before HIR, memory planning, SSA, bytecode, native
lowering, or execution.

Complete HIR passes canonical consistency and ownership validation. Memory
planning remains authoritative. SSA is verified. Bytecode is validated. Staging
must not duplicate compiler logic or bypass canonical validation.

Source-free tests prove zero post-construction source loading and parsing.
Imported and source-free equivalence compares semantic facts and observable
behavior, not private IDs.

Queries are revision-labelled, deterministic, stably ordered, and paginated
when large. Continuations bind namespace, revision, and query. Return compact
identities first and expand details selectively. Projection text never becomes
authority or identity.

A legal-constructor query must be honest. Do not advertise an edit the
transaction path cannot validate or omit supported behavior because a stale
second whitelist was not updated. Mark candidates provisional when canonical
validation remains required.

Capabilities are explicit typed values. Imported and source-free entry points
obey the same capability rules. Operation signatures, effects, ownership,
capability requirements, traps, divergence, and lowering come from canonical
operation contracts, not workspace copies.

Do not fabricate grants, suppress effects, copy affine values, suppress
cleanup, or weaken ownership validation.

Moves, borrows, drops, cleanup, control transfer, and failures remain exact.
Loop exits and backedges preserve outer ownership and loans. Early return and
failed calls use canonical cleanup exactly once.

The VM is the generic validated route. Baseline native execution is bounded
specialization. Eligibility, lowering, installation, or preparation may decline
before entry and then execute the unchanged program in the VM. After native
entry, never retry in the VM. Host effects occur exactly once.

Do not narrow valid behavior, add public engine selection, add transition
policy, or add compilation caches without a current requirement and evidence.

Unsafe code remains localized at genuine unsafe and FFI boundaries with a
stated safety argument. Validate FFI values and preserve W^X and pre-entry
installation atomicity.

## Scale and Performance

User-controlled depth must not consume unbounded native stack. Use explicit
work stacks or another proven bounded mechanism for expressions, patterns,
types, values, dependencies, compaction, projection, queries, validation,
diffs, and destruction.

Do not impose arbitrary semantic limits or turn benchmark sizes into language
maxima. Use wide identities and counts with checked host-index conversion.
Query and execution budgets limit one response or execution boundary, never
semantic authority.

Classify boundaries before adding machinery:

- persistent bytes need exact encoding, validation, corruption behavior,
  compatibility policy, and a current consumer;
- unsafe or FFI values need checked representation, safety invariants, and
  atomicity;
- machine-readable CLI output needs a deterministic schema and consumer
  decoding tests;
- same-build in-process values normally need Rust types and constructor
  validation, not digests, registries, or serialization;
- closed vocabularies normally need a closed enum, exhaustive matches, and
  canonical metadata, not a global registry.

Begin with in-memory immutable or copy-on-write snapshots. Add persistence only
for measured crash recovery, retained scale, restart continuity, or durable
consumers. Add collaboration only for defined multi-writer semantics. Add a
daemon only after local paths are complete enough to measure a process
boundary. The language SQLite capability is not workspace persistence.

Performance work requires a workload, baseline, suspected cost, structural
proposal, correctness oracle, and reversal condition. Measure the production
path and relevant phases.

Do not infer gains from code size, allocation count, or one sample. Do not
optimize inactive paths, narrow accepted programs for specialization, add
parallelism before independent work is measured, add caches before reuse is
measured, or use a warm service to hide local recomputation.

Incrementality requires measured repeated work, exact dependencies,
failure-safe publication, the full path as oracle, and demonstrated reduction.
Prefer a narrow proven fast path over a generalized dependency engine. Cache
presence and scheduling never change meaning, and cache keys never become
identity.

Record only retained measurements in `docs/performance.md`. Never fabricate or
overclaim.

## Architecture Restraint

Before adding an abstraction, identify:

- its current producer and consumer;
- the invariant it owns;
- the invalid state or repeated work it removes;
- the test or measurement proving its value;
- whether a local helper is sufficient;
- whether it creates another authority or identity domain;
- whether it requires serialization, versioning, or a process boundary;
- whether it narrows the generic route;
- whether it increases agent search space;
- its deletion condition.

A new type is not automatically better modeling. A crate is not automatically
modularity. A registry is not authority. A digest is not integrity. A protocol
is not agent usability. A cache is not performance. A planner is not
composability.

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

Do not add a generic tree editor for one edit, a visitor framework for one
traversal, a planner for one overlap, a registry for a closed enum, a serializer
for a same-build value, a cache for unmeasured work, a service for an in-process
consumer, persistence for an editing test, or collaboration state without
writers.

A crate boundary must earn itself through unsafe or FFI isolation, an
independently useful API, a supported target boundary, measured compile
isolation, low coupling, or a current product boundary.

A module owns one coherent responsibility. Large files are not automatically
wrong, and small files are not automatically modular. Do not impose line limits.
Split only when the selected vertical reveals a stable responsibility with a
narrow interface and reduced search fan-out.

Prefer mature dependencies when they remove substantial machinery or risk.
Keep local code when it is smaller, clearer, safer, or measurably better. Do not
add dependencies for trivial transformations, procedural macros for small
closed vocabularies, async runtimes without async products, or serialization for
in-process values.

## Multi-Turn Workflow

### Orient

1. Record the starting commit, branch, and `git status --short`.
2. Read applicable instructions and only the required normative and status
   sections.
3. Inspect recent relevant commits.
4. Search owning symbols before opening large files.
5. Inspect representative producers, consumers, and tests.
6. Run the smallest characterization.
7. Preserve unrelated work.

Keep a compact task ledger in working memory or ignored scratch space:
operation, consumer, contradiction, authority, producer, consumers, identities,
atomicity, focused tests, non-goals, stop condition, and measurement question.

### Characterize and Decide

Prefer one focused test, existing example, exact query assertion,
imported/source-free equivalence case, malformed-input case at the owning
boundary, deterministic work counter, or retained benchmark.

Do not build a general harness for a focused defect. Do not add measurement
without a stated complexity question.

Make one coherent representation decision. Record rejected alternatives
briefly. Stop exploring once evidence selects the design.

### Implement

1. Update the authoritative representation.
2. Update its producer and every active consumer.
3. Preserve or reconcile public identity explicitly.
4. Reuse canonical type, effect, ownership, and capability validation.
5. Delete displaced alternatives.
6. Add focused success and rejection evidence.
7. Run focused verification.
8. Update maintained documentation.
9. Run the full boundary once.
10. Inspect the final diff and commit one cohesive change when permitted.

### Report

Report the starting and ending commits, selected vertical, current consumer,
semantic result, representation decision, compatibility breaks, identity and
atomicity behavior, compiler/runtime evidence, tests, commands, unrun
verification, measurements, documentation, remaining gaps, and worktree state.

## Errors, Tests, and Verification

Reject invalid input at the owning boundary. Use structured errors when callers
need facts. Do not stringify and reparse typed errors, fabricate paths, spans, or
identities, swallow host failures, or panic on user-controlled input.

Observable output must not depend on hash iteration, allocator accidents, or
private vector order. Sort by semantic keys when order is undefined. Preserve
declaration and evaluation order when it is defined.

Tests protect intended semantics, not obsolete topology. Add the smallest
focused evidence. Prefer coherent table-driven scenarios with independent
oracles rather than one test per syntactic case.

Cover the relevant subset of:

- success and owning-boundary rejection;
- namespace, generation, kind, owner, visibility, and revision checks;
- atomicity and allocator rollback;
- identity, tombstones, and old snapshots;
- deterministic diffs, queries, and projections;
- exact types, effects, capabilities, ownership, and cleanup;
- source-free compilation and zero parser/loading work;
- VM behavior and bounded native behavior;
- stack safety and private compaction;
- complexity shape when performance is part of the change.

Do not weaken tests to make a redesign pass. Delete tests for obsolete APIs.
Quiet-success CLI tests assert both streams are empty. Machine-output tests
decode output as consumers. Ignored stress tests state why they are ignored and
how to run them. Executable source-free features require production compilation
and execution evidence.

Use this iteration ladder:

1. one narrow affected test;
2. affected crate or module tests;
3. one compiler check after a deliberate type-shape migration;
4. fix warnings immediately;
5. format after the representation stabilizes.

Capture long output once and inspect the causal region. Do not repeatedly dump
logs or rerun unchanged commands.

Before completion, run:

```bash
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
git diff --check
```

Run `cargo fmt --all` first when formatting is needed. Do not omit
`--locked`, `--all-targets`, `--all-features`, or the release build. Run Docker
verification when available and relevant. Report exact environment failures and
never claim unrun verification.

Inspect final status and diff for prompts, logs, caches, generated files,
build artifacts, and unrelated changes.

## Documentation and Git

Documentation roles are:

- `docs/spec/`: normative contracts;
- `docs/status.md`: implemented facts;
- `docs/architecture.md`: responsibility, flow, ownership, and trust boundaries;
- `docs/performance.md`: retained measurements and reversal conditions;
- `docs/roadmap.md`: ordering and evidence gates;
- `README.md`: concise product entry point.

Do not present future architecture as current, duplicate authorities, or retain
stale claims. A completed item leaves roadmap `Now`. Do not invent speculative
work merely to keep that section populated.

Add a decision record only for a non-obvious cross-cutting choice likely to be
reopened. Do not create permanent task summaries that immediately become stale.

Record the starting commit and inspect the worktree. Keep generated artifacts
and prompts out of product commits. Use one cohesive commit per verified
vertical when permitted. The subject describes the semantic result, not the
prompt.

Do not push, force-push, open a pull request, or alter remote state unless the
user explicitly requests it.

Before commit, inspect status, diff stat, substantive diff, and
`git diff --check`. Confirm verification, documentation, and the absence of
unrelated staging. After commit, inspect status and record the commit.

## Agent Attention and API Cost

Treat context, tool calls, attention, and API spend as finite. Save cost through
less search and rework, never through weaker correctness or skipped final
verification.

- search before reading;
- use exact symbols and narrow line ranges;
- use recent diffs to avoid rediscovering settled decisions;
- keep a compact task ledger;
- do not reread this file repeatedly in one turn;
- do not dump whole compiler files, generated fixtures, or full logs;
- capture long output once and inspect the causal region;
- batch mechanical representation updates;
- use one compiler pass as a migration inventory;
- run focused tests before workspace tests;
- do not rerun commands with unchanged inputs;
- stop generating alternatives after evidence selects one;
- do not add API-cost instrumentation without a consumer;
- hand off exact paths, symbols, invariants, failures, and commands instead of
  copied source.

The lead agent owns design, integration, and final verification. Use subagents
only for independent bounded investigations or disjoint implementation. Give
each subagent one question, exact scope, required evidence, a stop condition,
and a compact output format. Do not assign overlapping implementation or
competing architectures.

## Completion and Stop Rules

Complete only when the selected operation works end to end; invalid input
rejects deterministically; failed transactions publish nothing and preserve
allocation; identities follow continuity rules; old snapshots remain valid;
canonical validation still runs; runtime effects occur exactly once; focused
tests protect the root cause; documentation matches the checkout; required
verification passed or is reported honestly; obsolete paths are deleted; and no
speculative adjacent system appears.

Stop and narrow if the change introduces a second authority, unjustified
identity domain, generic planner for one edit, unmeasured cache, consumerless
protocol, serializer, daemon, or persistence layer, narrowed VM route, weakened
atomicity, invalid old snapshots, user-depth recursion, arbitrary validity
quota, unrelated feature expansion, compatibility layer, or broad repository
reorganization.

Usually correct by reusing an identity or canonical validator, localizing a
helper, deleting a redundant representation, deferring an unproven system, or
stopping after a focused audit. Do not solve uncertainty by building more
infrastructure.

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

Distinguish product failures from environment failures. State exact unrun
verification. Report evidence and decisions, not hidden reasoning or a
transcript.
