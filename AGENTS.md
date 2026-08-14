# AGENTS.md

## Scope

This file governs the entire repository. A deeper `AGENTS.md` may narrow local procedure, but it
must not weaken repository-wide requirements for semantic correctness, safety, stable public
identity, transaction failure atomicity, determinism, exact capability/effect/ownership behavior,
complete generic VM behavior, stack safety, verification, evidence, or architectural restraint.

Use English for maintained code, tests, diagnostics, documentation, commit messages, and
machine-readable fields unless an external protocol requires another language.

Prompts, model deliberation, transcripts, scratch plans, raw logs, temporary probes, and unretained
measurements are not repository authority. Do not commit them unless the active task explicitly
defines one as a maintained artifact.

Preserve unrelated work. Never reset, clean, overwrite, force-push, or otherwise destroy work you
did not create.

## Mission

Build `lkjscript` as an AI-primary, statically typed, memory-safe, collector-free programming
system. AI-primary means an agent can discover, construct, inspect, revise, validate, compile, and
execute programs through deterministic typed semantic operations.

A model may propose operations. Model inference must never decide parsing, name or trait resolution,
typing, ownership, effects, capabilities, artifact acceptance, executable validation, or runtime
meaning.

Source is an import, package, review, and interoperability format. After import, source text is not
mutable semantic authority. The semantic workspace is the authoring authority.

Prefer one coherent architecture and one dependency-closed vertical over broad partial frameworks.
Long-term performance matters, but correctness, safety, failure atomicity, one authority, stable
identity, deterministic behavior, and a complete generic VM route come first.

User trust grants decision authority, not evidence for speculative machinery. Bold changes are
welcome when evidence selects them. Large changes are not inherently better changes.

## Current Product Model

Read `docs/status.md` for current checkout facts. The supported product is local package checking
and execution plus an in-process semantic workspace.

```text
verified source import or source-free construction
    -> one partial-capable SemanticProgram
    -> immutable WorkspaceSnapshot
    -> structured completeness gate
    -> one source-optional complete HIR
    -> ownership and memory planning
    -> verified SSA
    -> validated bytecode
    -> generic VM or bounded baseline-native specialization
```

`SemanticProgram` is mutable only while one transaction is staged. `WorkspaceSnapshot` is the
immutable published authority. HIR, SSA, bytecode, native images, projections, diagnostics, metrics,
and presentation text are derived.

Public semantic identities are opaque and stable only according to explicit continuity rules.
Compiler IDs, dense indexes, ordinals, addresses, source coordinates, slots, registers, opcodes, and
machine details remain private.

A successful transaction publishes exactly one revision. Failure publishes nothing. Old snapshots
remain valid. Presentation and source provenance never become semantic identity.

The following remain absent unless a current measured consumer proves a need:

- daemon, RPC, or wire protocol
- persistent workspace storage or transaction journal
- collaboration or conflict state
- scheduler or plugin platform
- generalized cache
- generalized incremental dependency engine
- compatibility layers for deleted provisional designs
The absence of one of those systems is not itself a defect.

## Priority Order

1. semantic correctness
2. memory and type safety
3. transaction failure atomicity
4. one semantic authority
5. stable public identity
6. deterministic observable behavior
7. complete generic compiler and VM coverage
8. exact capability, effect, ownership, and cleanup behavior
9. stack safety and user-scale correctness
10. direct AI-agent usability
11. measured production-path performance
12. simplicity and low agent search cost
13. documentation precision
14. compatibility with provisional behavior
Compatibility is intentionally last. Future platform ideas, prompt length, diff size, crate count,
file count, and line count are not product evidence.

## Authority

1. the active user task
2. the nearest applicable `AGENTS.md`
3. accepted normative files under `docs/spec/`
4. executable code and focused tests
5. manifests, lock files, schemas, commands, and generated contract inputs
6. `README.md`
7. `docs/status.md`
8. `docs/architecture.md`
9. `docs/performance.md`
10. `docs/roadmap.md`
11. comments, historical prompts, issue prose, and stale documentation
Language semantics belong in `docs/spec/language.md`. Workspace semantics belong in
`docs/spec/workspace.md`. Checkout facts belong in code, tests, manifests, and `docs/status.md`.
Architecture owns responsibilities, flow, and trust boundaries. Performance documentation owns
retained measurements and reversal conditions. The roadmap owns ordering and evidence gates.
Superseded behavior belongs in Git history.

When artifacts disagree, classify the claim, identify its owner, inspect focused executable
evidence, preserve accepted semantics, and update or delete stale material in the same change. Do
not preserve an accident merely because it exists. Do not rewrite a normative contract merely
because implementation is inconvenient.

Do not manufacture authority through copied tables, prompt archives, unconsumed registries,
descriptive revision tokens, redundant digests, unused schemas, or same-build descriptors.

## Autonomy and Compatibility

Use the actual checkout, current consumers, accepted specifications, focused tests, recent relevant
history, and measurements. Do not ask the user to choose internal alternatives that repository
evidence can decide.

Ask only when a genuinely external requirement is missing and no safe explicit assumption can
complete the selected vertical. Broad authority permits one clean decision, not unrelated expansion.

If the requested objective is already complete, verify it and stop. When roadmap `Now` is empty,
select implementation work only from a concrete operation, focused failure, accepted blocking gap,
measured bottleneck, demonstrated maintenance burden, or direct authority contradiction.

Symmetry, a pleasing abstraction, a future service, and a missing API without a current caller are
not evidence.

Backward compatibility is not an objective unless the active task names an independent persistent or
external boundary. Syntax, provisional source encoding, Rust APIs, commands, crate topology,
internal forms, fixtures, tests, and documentation may change incompatibly.

Prefer direct cutover. Do not add `v2`, `next`, editions, legacy modes, deprecated aliases,
adapters, dual readers or writers, compatibility flags, or migrations for nonexistent durable data.
Update every active producer and consumer, then delete displaced code, stale tests, stale
documentation, obsolete dependencies, and obsolete configuration.

## Select One Verified Vertical

Before changing product code, answer privately and concretely:

1. What exact operation improves?
2. Which current consumer needs it?
3. Which focused test, example, query, or benchmark proves the gap?
4. Which artifact owns the semantics?
5. Which representation is authoritative?
6. Which identities survive, are created, or are tombstoned?
7. What remains failure-atomic?
8. Which compiler and runtime route proves success?
9. What is explicitly out of scope?
10. What is the exact stop condition?
11. Why is broader machinery unnecessary?
A hypothesis, roadmap bullet, aspiration, type name, crate idea, protocol sketch, or framework for
later is not a selected vertical. Characterize first. Prefer an existing focused test, example,
query, or benchmark. A temporary ignored probe is acceptable only when existing evidence cannot
answer the question; delete it afterward.

One implementation turn normally completes one dependency-closed vertical with one concrete
operation, one demonstrated need, one authority, one bounded producer-to-consumer path, one
end-to-end success path, one owning-boundary rejection path, explicit identity and atomicity
behavior, focused evidence, explicit non-goals, and an exact stop condition.

A prerequisite cleanup is allowed only when the current representation makes the selected operation
dishonest or impossible, removes a real contradiction, and remains inside the dependency closure. Do
not combine roadmap items because they touch the same file or because full verification is
expensive. Stop after the vertical passes. Record the next evidenced problem without implementing
it.

If evidence does not justify implementation, stop with an exact handoff. Do not build infrastructure
to avoid stopping.

## Semantic Authority and AI-Primary Interfaces

- Use typed operations, stable opaque identities, immutable snapshots, revision-checked
  transactions, structured diagnostics, deterministic ordering, revision-bound continuations,
  queryable incomplete states, explicit unsupported results, direct source-free compilation, and
  failure-atomic publication.
- Avoid heuristic rebinding, hidden resolution, implicit globals, unstable coordinates,
  natural-language correctness paths, stale caches, opaque retries, partial publication, and APIs
  that require callers to reconstruct private compiler state.
- Resolve names during import, then use identities. Post-import semantic edits do not reload or
  reparse source. Source-free programs compile without synthetic source.
- Do not render and reparse semantic edits, retain parallel mutable HIR and workspace authorities,
  retain stale derived forms in snapshots, reconnect references by spelling, or infer continuity
  from source text.
- Compiler-owned built-ins remain fixed context unless a normative contract makes them mutable
  entities.
- Expose exact entities, owners, kinds, types, effects, capabilities, dependencies, legal
  constructors, diagnostics, blockers, references, calls, ownership facts, and revisions through
  typed APIs.
- A transaction-local handle is not a public identity, and a public identity is not a
  transaction-local handle. Never accept one in the domain of the other.
- Names support presentation and lookup. After resolution, names do not replace identities.
- Do not require callers to reconstruct private indexes, ordinals, addresses, source spans, preorder
  positions, runtime tags, slots, registers, or machine layouts.
- Every public field has one exact semantic meaning. Do not expose ignored bounds, dead flags,
  advisory facts, or metadata that canonical validation does not consume.
- A legal-constructor query must be honest: do not advertise an edit the transaction route cannot
  validate, and mark provisional candidates when canonical validation remains required.
- Prefer compact identity-first queries followed by selective expansion. Pagination is deterministic
  and revision-bound. Projection is presentation, never authority, identity, or a transport
  substitute.

## Public Identity

- namespace-scoped, generation-aware, kind-checked, and owner-checked where applicable
- stable across unrelated edits and private compaction
- invalid after deletion while remaining valid in old snapshots
- reconciled explicitly for every survivor
- never inferred from names, content, hashes, preorder, spans, vector positions, private IDs,
  addresses, or projection output
Survivors keep identities. Deletion tombstones identities. Later same-name recreation receives fresh
generations. Replacement descendants receive new identities unless one explicit movement operation
proves continuity. Private relocation emits no semantic movement.

Rename changes presentation only. It does not create entities, rebind by text, alter runtime
identity, rewrite unrelated structure, or fabricate reference rewiring. Reject semantic no-ops
unless the accepted contract explicitly publishes them.

Declaration-local handles never enter published state, queries, projections, diagnostics, or diffs.
Reuse an existing local handle domain when its semantics match. Do not create a new public identity
domain for symmetry.

## Transactions and Incomplete States

Every transaction checks applicable base revision, namespace, generation, liveness, kind, owner,
visibility, exact type, exact capability, and operation preconditions.

Stage allocator state and every fallible result before publication. Failure publishes nothing and
consumes no public identity. It preserves the current snapshot, revision, allocator, generations,
free lists, diagnostics, blockers, attachments, provenance, continuations, and deterministic future
allocation.

Success publishes one immutable revision and one deterministic base-to-final semantic diff. Diffs
describe semantic changes, not private addresses, vector shifts, compaction, or compiler churn.

Reject unsupported overlap rather than adding a generalized edit planner. Deletion owns only defined
containment. Independent dependents do not disappear transitively. Validate deletion closure against
the final staged graph.

Incomplete snapshots are queryable values. Holes and unresolved references are semantic nodes.
Blockers are structured. Diagnostics derive from current state. Incomplete snapshots do not compile.

Do not install placeholder executable values, fallback targets, automatic ambiguity resolution,
hidden repair state, or partial artifacts.

## Types, Generics, Nominals, and Traits

Public types use stable semantic identities. Compiler-local types may use private identities.
Nominal identity is independent of shape and spelling. Binder names are presentation; stable binder
entities own published identity.

Reuse canonical generic resolution, assignability, substitution, trait validation, witness
derivation, ownership restrictions, and reference restrictions. Do not create a second solver,
substitution law, or witness path.

Semantic edits supply exact type arguments unless an accepted contract requires inference.
Unsupported generic construction rejects explicitly.

A type-shape change must inventory construction, matching, clone, destruction, equality, hashing,
display/debug, substitution, validation, dependency tracking, deletion, compaction, query,
projection, diagnostics, tests, external boundaries, and persistent boundaries.

User-controlled type depth must not consume unbounded native stack. Do not impose arbitrary
type-depth quotas.

Creation-only markers must not enter published `SemanticType`, HIR, SSA, bytecode, runtime values,
queries, projections, or diffs. Such a marker is legal only in the exact operation context that owns
its resolution.

## Compiler, Capabilities, Ownership, and Runtime

`compile_snapshot` is the sole semantic-snapshot compilation boundary. It rejects incompleteness
before HIR derivation, memory planning, SSA, bytecode, native lowering, or execution.

Complete HIR passes canonical consistency and ownership validation. Memory planning remains
authoritative. SSA is verified. Bytecode is validated. Workspace staging must not duplicate or
bypass compiler meaning.

Capabilities are explicit typed values. Imported and source-free entry points obey the same rules.
Operation signatures, effects, ownership, capability requirements, traps, divergence, and lowering
come from canonical operation contracts, not copied workspace whitelists.

Do not fabricate grants, suppress effects, copy affine values, suppress cleanup, weaken ownership
validation, or perform a host effect more than once. Moves, borrows, drops, cleanup, control
transfer, and failures remain exact.

The VM is the complete generic validated route. Baseline native execution is bounded specialization.
Eligibility, lowering, installation, or preparation may decline before entry, then the unchanged
validated program runs in the VM. After native entry, never retry in the VM.

Do not narrow valid behavior for native support, add public engine selection without a current
requirement, add automatic transition policy without evidence, or add a compilation cache without
measured reuse.

Unsafe code remains localized at genuine unsafe or FFI boundaries. State the safety argument,
validate FFI values, preserve W^X, and keep pre-entry installation atomic.

## Queries, Projections, Diffs, and Determinism

- Queries are revision-labelled, deterministic, stably ordered, paginated when large, compact by
  default, and selectively expandable.
- Continuations bind namespace, revision, query kind, filters, and ordering.
- Return compact identities first. Never expose compiler-dense IDs.
- Projection text never becomes authority or identity.
- Do not claim an incomplete result is complete.
- Diffs report semantic changes. Pure private compaction emits no semantic diff.
- Observable output must not depend on hash iteration, allocator accidents, private vector order, or
  unstable debug formatting.
- Sort by semantic keys when order is undefined. Preserve declaration and evaluation order when
  order is defined.

## Scale and Stack Safety

User-controlled depth must not consume unbounded native stack. Use explicit work stacks or another
proven bounded mechanism for expressions, patterns, types, values, dependency traversal, compaction,
projection, queries, validation, diffs, equality, cloning, display, and destruction.

Do not impose arbitrary semantic limits or turn benchmark sizes into language maxima. Use wide
identities and counts for user-scale data. Check conversion before host indexing.

A query or execution budget limits one response or execution boundary; it does not redefine semantic
validity. Compact native and external encodings are specialization or trust boundaries and must not
narrow the generic valid language.

## Performance and Boundary Classification

Before adding machinery, classify the boundary.

- **Persistent bytes:** require exact encoding, validation, corruption behavior, compatibility
  policy, and a current durable consumer.
- **Unsafe or FFI values:** require checked representation, safety invariants, and atomicity.
- **Machine-readable CLI output:** requires a deterministic schema, a real consumer, and consumer
  decoding tests.
- **Same-build in-process values:** normally require Rust types and constructor validation, not
  digests, registries, or serialization.
- **Closed vocabularies:** normally require a closed enum, exhaustive matches, and canonical
  metadata, not a global registry.
Begin with in-memory immutable or copy-on-write snapshots. Add persistence only for measured crash
recovery, retained scale, restart continuity, or durable consumption. Add collaboration only for
defined multi-writer semantics. Add a daemon only after local paths are complete enough to measure a
process boundary. The language SQLite capability is not workspace persistence.

Performance work requires a workload, baseline, suspected cost, structural proposal, correctness
oracle, and reversal condition. Measure the production path and relevant phases. Do not infer gains
from code size, allocation count, or one sample.

Do not optimize inactive paths, narrow accepted programs for specialization, add parallelism before
independent work is measured, add caches before reuse is measured, or use a warm service to hide
local recomputation.

Incrementality requires measured repeated work, exact dependencies, failure-safe publication, the
full path as oracle, and demonstrated reduction. Prefer a narrow proven fast path over a generalized
dependency engine. Cache presence and scheduling never change meaning; cache keys never become
public identity.

Record only retained reproducible measurements in `docs/performance.md`. Never fabricate or
overclaim.

## Architecture Restraint

Before adding an abstraction, identify its current producer, current consumer, owned invariant,
removed invalid state or repeated work, proving test or measurement, whether a local helper is
sufficient, whether it creates another authority or identity domain, whether it requires
serialization/versioning/process boundaries, whether it narrows the generic route, whether it
increases agent search space, and its deletion condition.

A new type is not automatically better modeling. A crate is not automatically modularity. A registry
is not authority. A digest is not integrity. A protocol is not agent usability. A cache is not
performance. A planner is not composability.

Prefer, in order:

1. delete obsolete code
2. reuse an existing identity or typed value
3. make one representation authoritative
4. move validation to its owner
5. replace repeated scans with one local index
6. replace user-depth recursion with an explicit work stack
7. simplify a data structure
8. add a narrow helper
9. add focused measurement
10. add cache, process, protocol, or persistence only after measured need
Do not add a generic tree editor for one edit, a visitor framework for one traversal, a planner for
one overlap, a registry for a closed enum, a serializer for a same-build value, a cache for
unmeasured work, a service for an in-process consumer, persistence for an editing test, or
collaboration state without writers.

A crate boundary must earn itself through unsafe/FFI isolation, an independently useful API, a
supported target boundary, measured compile isolation, low coupling, or a current product boundary.

A module owns one coherent responsibility. Large files are not automatically wrong; small files are
not automatically modular. Do not impose line limits. Split only when the selected vertical reveals
a stable responsibility, narrow interface, and lower search fan-out. Do not reorganize unrelated
code merely to make a task look architectural.

Prefer mature dependencies when they remove substantial machinery or risk. Keep local code when it
is smaller, clearer, safer, or measurably better. Do not add dependencies for trivial
transformations, procedural macros for small closed vocabularies, an async runtime without an async
product, or serialization for in-process values.

## Multi-Turn Workflow

### Orient

1. Record starting commit, branch, and `git status --short`.
2. Read applicable instructions and only the required normative/status sections.
3. Inspect recent relevant commits.
4. Search owning symbols before opening large files.
5. Inspect representative producers, consumers, and tests.
6. Run the smallest characterization.
7. Preserve unrelated work.
Keep a compact ignored task ledger containing operation, consumer, gap, authority, producer,
consumers, identities, atomicity, focused tests, non-goals, stop condition, and measurement
question. Do not commit it unless the task makes it a maintained artifact.

### Characterize

Prefer a focused existing test, executable example, exact query assertion, imported/source-free
equivalence case, malformed-input case at the owning boundary, deterministic work counter, or
retained benchmark. Do not build a general harness for a focused defect. Do not add measurement
without a stated complexity question.

### Decide

Make one coherent representation decision. Record rejected alternatives briefly. Stop exploring
after evidence selects one. Do not keep competing implementations or implement an adjacent roadmap
item while the same files are open.

### Implement

1. Update the authoritative representation.
2. Update its producer.
3. Update every active consumer.
4. Preserve or reconcile public identity explicitly.
5. Reuse canonical type, effect, ownership, capability, and generic validation.
6. Delete displaced alternatives.
7. Add focused success evidence.
8. Add focused rejection evidence.
9. Run focused verification.
10. Update maintained documentation.
11. Run the full boundary once.
12. Inspect the final diff.
13. Commit one cohesive change when permitted.

### Continue Across Turns

A turn may end after one complete verified vertical or one focused audit that disproves it. Do not
hand off a half-migrated public API, two active representations, disabled compatibility code, an
unverified speculative subsystem, or an intentionally failing branch.

A handoff names exact paths, symbols, invariants, observed failures, commands, and the next evidence
gate. Keep it compact enough that the next agent does not reread the repository. Durable progress
belongs in code, tests, specifications, status, architecture, performance evidence, or roadmap
ordering—not copied prompt prose.

## Errors

Reject invalid input at the owning boundary. Use structured errors when callers need facts. Do not
stringify and reparse typed errors, fabricate paths/spans/identities, swallow host failures, or
panic on user-controlled input.

## Tests

Tests protect intended semantics, not obsolete topology. Add the smallest focused evidence. Prefer
coherent table-driven scenarios with independent oracles over one test per trivial spelling.

- success and owning-boundary rejection
- namespace, generation, kind, owner, visibility, and revision checks
- atomicity and allocator rollback
- stable identity, tombstones, and old snapshots
- deterministic diffs, queries, and projections
- exact types, generics, effects, and capabilities
- ownership and cleanup
- source-free compilation with zero parser and source-loading work
- VM behavior and bounded native behavior
- stack safety, private compaction, and complexity shape
Do not weaken tests to make a redesign pass. Delete tests for obsolete APIs. Quiet-success CLI tests
assert both streams are empty. Machine-output tests decode output as consumers. Ignored stress tests
state why they are ignored, which invariant they prove, and how to run them. Executable source-free
features require production compilation and execution evidence.

Use this iteration ladder:

1. one narrow affected test
2. affected module tests
3. affected crate tests
4. one compiler check after a deliberate type-shape migration
5. fix warnings immediately
6. format after the representation stabilizes
7. full verification once
Capture long output once and inspect the causal region. Do not repeatedly dump logs or rerun
unchanged commands.

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

Run `cargo fmt --all` first when formatting is needed. Do not omit `--locked`, `--all-targets`,
`--all-features`, or the release build. Run Docker verification when available and relevant. Report
exact environment failures. Never claim unrun verification.

Inspect final status and diff for prompts, raw logs, caches, generated files, build artifacts,
temporary probes, and unrelated changes.

## Documentation

Documentation roles:

- **`docs/spec/`:** normative contracts.
- **`docs/status.md`:** implemented facts.
- **`docs/architecture.md`:** responsibilities, flow, ownership, and trust boundaries.
- **`docs/performance.md`:** retained measurements and reversal conditions.
- **`docs/roadmap.md`:** ordering and evidence gates.
- **`README.md`:** concise product entry point.
Do not present future architecture as current, duplicate authorities, or retain stale claims. A
completed item leaves roadmap `Now`. Do not invent speculative work merely to keep `Now` populated.
Add a decision record only for a non-obvious cross-cutting choice likely to be reopened. Do not
create permanent task summaries that immediately become stale.

## Git

Record the starting commit and inspect the worktree before editing. Keep generated artifacts and
prompts out of product commits. Use one cohesive commit per verified vertical when permitted. The
subject describes the semantic result, not the prompt.

Do not push, force-push, open a pull request, or alter remote state unless the user explicitly
requests it.

Before commit, inspect `git status --short`, `git diff --stat`, the substantive diff, and `git diff
--check`; confirm verification, documentation, and no unrelated staging. After commit, inspect final
status, record the ending commit, and report remaining worktree state.

## Agent Attention and API Cost

Treat context, tool calls, attention, and API spend as finite. Save cost through less search and
rework, never through weaker correctness or skipped final verification.

- Search before reading; use exact symbols and narrow line ranges.
- Use recent diffs to avoid rediscovering settled decisions.
- Do not reread this file repeatedly in one turn.
- Do not dump whole compiler files, generated fixtures, or full logs.
- Open the smallest producer, consumer, and test slices that answer the question.
- Build a migration inventory before editing a public enum or type shape.
- Batch mechanical representation updates.
- Use one compiler pass as a migration inventory.
- Run focused tests before workspace tests.
- Do not rerun commands with unchanged inputs.
- Stop generating alternatives after evidence selects one.
- Reduce future search cost through clearer authority, narrower interfaces, deleted duplication,
  exact names, focused tests, and coherent modules.
- Do not add token counters, prompt registries, agent registries, model-routing metadata, or a
  special service merely to reduce local search.
The lead agent owns design, integration, and final verification. Use subagents only for independent
bounded investigations or disjoint implementation. Give each one question, exact scope, required
evidence, stop condition, and compact output. Do not assign overlapping work or ask multiple agents
for competing architectures after evidence selects one.

## Completion and Stop Rules

Complete only when the selected operation works end to end, invalid input rejects deterministically,
failed transactions publish nothing and preserve allocation, identities follow continuity rules, old
snapshots remain valid, canonical validation still runs, effects occur exactly once, focused tests
protect the root cause, documentation matches the checkout, required verification passed or is
reported honestly, obsolete paths are deleted, and no speculative adjacent system appears.

Stop and narrow if the change introduces a second semantic authority, unjustified identity domain,
generic planner for one edit, unmeasured cache, consumerless protocol or serializer, daemon without
a measured need, persistence without a durable consumer, narrowed VM behavior, weakened atomicity,
invalid old snapshots, user-depth recursion, arbitrary validity quotas, unrelated expansion,
compatibility layers, or broad reorganization without measured benefit.

Usually correct by reusing an identity, reusing a canonical validator, localizing a helper, deleting
a redundant representation, deferring an unproven system, or stopping after a focused audit. Do not
solve uncertainty by building more infrastructure.

## Final Report

```text
Starting commit:
Ending commit:
Selected vertical:
Current consumer:
Characterization evidence:
Semantic result:
Representation decision:
Rejected alternatives:
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

Distinguish product failures from environment failures. State exact unrun verification. Report
evidence and decisions, not hidden reasoning or a tool transcript.
