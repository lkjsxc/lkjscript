# AGENTS.md

## Scope and language

This file applies to the entire repository and to every agent that inspects, changes, tests,
documents, commits, or reports on it.

Write code, comments, public APIs, diagnostics, tests, documentation, commit messages, and final
reports in English unless the active task explicitly requires another language for a user-facing
artifact.

The user authorizes autonomous technical judgment, incompatible changes, destructive
simplification, specification revision, representation replacement, file and crate
reorganization, and deletion of obsolete work.

Backward compatibility is not a project objective unless the active task explicitly restores it
for one named, current, externally consumed boundary.

Do not preserve old syntax, serialized bytes, command shapes, internal Rust APIs, module
layouts, package layouts, cache formats, compiler representations, runtime representations,
fixtures, or prose merely because they existed.

Historical requests, previous prompts, earlier assistant answers, old branches, and Git history
are context. They are not permanent architecture requirements.

Do not ask the user to choose among technical alternatives when the checkout, accepted
specifications, focused tests, measurements, or a reversible local assumption can decide.

Ask only when a genuinely external product requirement is missing and no safe assumption can
unblock the active task.

Do not destroy unrelated local work, external data, credentials, host state, or remote history.
Authorization to redesign this repository is not authorization to erase unrelated state.

The only permanently fixed property of the program file format is the `.lkjscript` extension.
Every other current notation, byte layout, grammar, schema, CLI, package, cache, compiler, and
runtime detail remains provisional unless an accepted specification explicitly fixes it.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free,
high-performance programming language and implementation.

AI-primary means that an agent can discover, construct, inspect, transform, validate, compile,
execute, test, compare, and review programs through deterministic, precise, compact, composable
interfaces.

AI-primary does not mean model inference inside the compiler, validator, optimizer, runtime,
storage layer, or correctness boundary.

AI-primary does not mean optimizing semantics for one model, tokenizer, context window,
provider, benchmark, or prompting style.

AI-primary does not mean making the representation opaque to humans or ordinary tools.

AI-primary does not mean multiplying protocols, schemas, descriptors, registries, services,
persistent stores, or agent-specific metadata.

A model may propose an operation. Deterministic implementation machinery decides whether that
operation is valid.

Humans and ordinary tools must retain deterministic inspection, diagnostics, validation,
semantic diffing, projection, and review.

The long-term product direction is:

- one syntax-independent mutable semantic program authority;
- first-class source-free construction and editing;
- stable logical identities independent of names, paths, spans, formatting, hashes, and
  compiler-dense indexes;
- explicit incomplete states instead of fabricated executable placeholders;
- typed, atomic, revision-checked semantic transactions;
- direct access to types, scope, effects, capabilities, ownership, dependencies, diagnostics,
  and legal next actions;
- deterministic ordering, pagination, projections, and semantic diffs;
- direct compilation from complete semantic state without rendering and reparsing;
- one complete generic production execution route;
- optional specialization that may decline only before effects;
- low agent round-trip count and actionable failures;
- human-reviewable deterministic projections;
- reproducible builds and execution meaning; and
- strong end-to-end startup, compilation, editing, querying, runtime, memory, allocation,
  copying, serialization, generated-code, and binary-size behavior.

## Priority order

Use this priority order unless current evidence establishes a more severe prerequisite:

1. correct and coherent language semantics;
2. memory safety, ownership, cleanup, failure atomicity, and deterministic behavior;
3. scale-safe compiler and representation algorithms without arbitrary validity quotas;
4. a complete local semantic workspace for construction, editing, querying, and direct
   compilation;
5. measured edit workloads and only then justified incremental recomputation;
6. one complete measured execution path and only then justified specialization;
7. self-hosting and broader platform products only after the lower layers are coherent; and
8. process boundaries, persistence, collaboration, daemonization, and distributed operation only
   after a present consumer and measurements justify them.

Do not skip a lower layer because a later platform idea is more exciting.

Do not optimize the language around Brainfuck, one benchmark, one demo, or one synthetic
workload. A benchmark may expose a defect. It does not define the language.

## Authority and truth

Use `docs/authority.md` for ownership by claim dimension.

In practical order, authority is:

1. the active task for the current objective;
2. this file for repository-wide engineering procedure;
3. accepted files under `docs/spec/` for intended language and workspace contracts;
4. code, tests, manifests, schemas, and command definitions for behavior in the checkout;
5. `docs/status.md` for current implementation and known gaps;
6. `docs/architecture.md` for current responsibilities, data flow, ownership, and trust
   boundaries;
7. `docs/performance.md` for measurement method and compact reproducible evidence;
8. `docs/roadmap.md` for planned ordering only;
9. sparse accepted decisions for durable rationale; and
10. Git history for superseded implementation and prose.

A roadmap item is not an architectural commitment. A task prompt is not a durable authority
artifact. A previous assistant answer is not a specification.

When claims conflict, classify the claim, inspect its owning artifact and executable evidence,
then update or delete stale material in the same change.

Use labels such as **Current**, **Target**, **Hypothesis**, **Historical**, **Unknown**, and
**Blocked** when ambiguity would otherwise remain.

Do not create another authority layer from prompts, planning trees, global revisions, prose
digests, registries, evidence ledgers, generated inventories, closure graphs, checkpoints,
handoffs, or completion capsules.

Specifications are revisable. When a better design changes intended semantics, update the owning
specification and perform one direct implementation cutover.

Do not silently contradict the specification. Do not retain an obsolete compatibility path.

## One active architecture

Maintain one active language definition, mutable semantic authority, compiler path, generic
production execution route, ownership model, package model, documentation authority model, and
implementation for each current product boundary.

Do not create editions, permanent `v2` systems, `next` trees, legacy modes, compatibility
layers, or parallel canonical representations.

When replacing a mechanism, prefer a dependency-closed cutover. Delete displaced
implementations, adapters, aliases, feature flags, compatibility tests, stale documentation,
dead dependencies, and obsolete data paths in the same coherent change.

Git history is the migration record for discarded experimental work.

A small independent evaluator, model, or reference implementation may remain when it is a useful
test oracle. It is not automatically a second production engine.

Names and current crate boundaries have no authority. Preserve, merge, split, rename, or delete
components according to cohesion, safety, real platform ownership, independently useful APIs,
measured compile isolation, coupling, and current consumers.

When the current architecture causes the defect, replace it. Do not hide it behind bookkeeping.

## Representation discipline

The authoritative semantic state must be able to exist without source text, files, formatting,
paths, spans, parser nodes, source hashes, or compiler-dense indexes.

Those are optional importer, presentation, provenance, cache, or trust-boundary attachments.

Text import may create semantic state. Compilation, semantic editing, querying, validation, and
correctness checks must not require rendering and reparsing text.

Do not satisfy an invariant with fake semantic data.

- dummy source files;
- placeholder paths;
- fabricated hashes;
- hidden valid bodies beneath holes;
- synthetic entry points;
- fake declarations;
- reserved semantic identities;
- fallback executable meaning behind incomplete state; or
- compiler-dense values presented as stable public identities.

Correct the representation or boundary instead.

Use an honest source-optional origin model. A source-backed fact may name source provenance. A
source-free fact remains source-free. Never smuggle source-free meaning through a dummy source
identity.

One representation owns mutable semantic facts. Other representations may be derived for
analysis, compilation, execution, projection, caching, persistence, or transport.

Every derived representation must have a current producer, current consumer, lifetime,
invalidation rule, and deletion condition. Derived representations are not coequal mutable
authorities.

Public stable identity does not require every internal object to be persistent. Dense compiler
IDs, vector positions, physical slots, code offsets, and layout indexes should be derived,
compact, and replaceable.

Preserve logical identity explicitly when meaning survives. Tombstone it when meaning does not
survive.

Names, paths, spans, formatting, semantically irrelevant order, and content hashes are not
universal mutable identities.

Use opaque logical identities with namespace, generation, and revision defenses where stable
editable identity is required.

Do not add a second identity merely to avoid understanding an existing one.

Before adding an identity, state:

- the semantic object it identifies;
- the lifetime in which it is unique;
- whether it survives rename, movement, compaction, serialization, and recreation;
- who allocates it;
- who validates it;
- what tombstones it;
- how stale and foreign values fail; and
- why an existing identity cannot serve.

### Public semantic APIs

Public semantic APIs must expose semantic meaning rather than parser nodes, display strings,
private addresses, dense indexes, or unvalidated internal compiler objects.

Use one exact structured public model for one semantic concept unless input and output genuinely
have different semantics.

Do not proliferate `Input`, `View`, `Ref`, `Descriptor`, `Resolved`, and `Wire` variants merely
because conversion code is convenient.

When one structured value can serve multiple positions, validate the position-specific rules at
the boundary.

A display string may accompany structured data for diagnostics. It must not be the only
machine-readable meaning when the meaning is implemented.

A public recursive or deeply nested value must be safe to clone, compare, hash where required,
project, validate, convert, and destroy without consuming unbounded native stack.

Do not impose a nesting quota to compensate for a recursive implementation.

Stable nominal references in public types use stable semantic identity. They do not use
declaration names or compiler-dense IDs.

Builtin semantic constructors without workspace entities must have an explicit builtin identity.
Do not fabricate workspace entities for them.

Unsupported semantic forms must be explicit and narrow. Do not collapse implemented structure
into opaque display text.

### Incomplete states

Incomplete semantic state is valid editing state.

Missing declarations, bodies, expressions, references, choices, and conflict resolutions must be
explicit blockers, holes, or recovery facts.

Do not retain executable fallback meaning behind an incomplete node.

Compilation of an incomplete snapshot must stop before ownership, memory planning, SSA,
bytecode, native code, or execution work.

The `.lkjscript` encoding is deliberately open. Do not redesign it without a current storage,
transfer, integrity, startup, interoperability, or tooling requirement.

An opaque or binary primary representation must still provide deterministic first-party
inspection, validation, querying, editing, semantic diffing, projection, malformed-data
handling, and failure-atomic publication.

Source-free authoring does not imply a database, journal, daemon, protocol, distributed store,
CRDT, or collaboration system. Complete and measure the in-process semantic workflow first.

## Breaking changes and direct cutovers

Assume breaking changes are permitted.

Do not spend implementation effort preserving provisional behavior unless the active task names
a real external consumer that requires it.

When a public or internal contract is wrong:

1. identify the intended replacement;
2. update the owning specification;
3. update every active producer and consumer;
4. migrate or replace active fixtures;
5. delete the old contract;
6. delete compatibility code and compatibility tests;
7. remove stale exports, dependencies, and documentation; and
8. verify that one active route remains.

Do not deprecate an experiment indefinitely.

Do not leave aliases that preserve two names for one concept without a current migration
consumer.

Do not preserve serialized bytes that no accepted storage contract has promised.

Do not preserve source syntax merely to avoid updating fixtures.

Do not preserve crate boundaries merely to avoid updating imports.

A direct cutover may be large when the dependency closure is genuinely large. It must still be
coherent, reviewable, and testable.

## Decision discipline

Start from a demonstrated current defect or an explicit current product requirement.

Correct the dependency-closed root cause, not one visible symptom.

Before adding an abstraction, identify:

- the concrete current problem;
- its producer and consumer;
- the authority it owns or derives;
- its lifetime and invalidation behavior;
- its failure behavior;
- its measured or structural benefit;
- why local code is insufficient; and
- the condition under which the abstraction should be deleted.

A new abstraction should perform concrete work now.

It should remove duplication or repeated work, make an important invalid state unrepresentable,
isolate a real boundary, expose an independently useful current API, enable a measured property,
or materially simplify reasoning and testing.

It must be smaller than the problem it replaces. It must not duplicate authority.

Otherwise keep the logic local or do not add it.

Prefer, in order:

1. delete unused work;
2. simplify semantics or representation;
3. reuse an existing invariant;
4. replace repeated discovery with one local precomputed fact;
5. improve data layout and traversal;
6. make invalid states unrepresentable;
7. add caching only after repeated equivalent work is measured;
8. add parallelism only when remaining work is large and separable; and
9. add target-specific specialization only behind a complete generic route and measured need.

A suspicious asymmetry is a reason to investigate. It is not by itself permission for a
repository-wide redesign.

Do not refactor unrelated code for symmetry, aesthetics, novelty, or theoretical completeness.

When two representations differ, determine whether the difference is semantic, historical, or
accidental before unifying them.

Prefer a small explicit mechanism over a generic framework when only one current use exists.

Prefer deletion over documentation of machinery that has no current consumer.

## Anti-overengineering

Do not build speculative:

- daemons, services, sessions, or process boundaries;
- persistence layers, journals, databases, distributed stores, or CRDTs;
- schedulers, resource topologies, process-cell systems, or custom allocators;
- universal registries, descriptor systems, plugin frameworks, or generic rewrite DSLs;
- general incremental-computation or cache frameworks;
- proof, certificate, witness, or evidence ecosystems without a current verifier and consumer;
- wire protocols or broad target matrices;
- multi-tier JIT policy, deoptimization, or PGO machinery;
- GUI, web, game, database, or cloud platforms before the language path supports them; or
- self-hosting scaffolding that makes the current compiler harder to improve.

A current task may introduce one of these only with a demonstrated present boundary, a present
end-to-end consumer, measured need, explicit acceptance criteria, explicit ownership and failure
behavior, and a reversal or deletion condition.

Do not solve complexity by adding bookkeeping around it.

First remove repeated scans, reconstruction, duplicated facts, unnecessary boundaries, and dead
work.

Do not create a universal graph engine to perform one explicit dependency walk.

Do not create a generic rewrite framework to remap a few concrete identity kinds.

Do not create an event system to report one synchronous transaction result.

Do not create a cache to avoid work that has not been measured.

Do not create a protocol because an in-process API exists.

Do not create a daemon because multiple future tools might someday exist.

Do not introduce an interner merely because types are recursive.

Do not add an arena merely to avoid writing one bounded conversion.

Do not add a trait hierarchy merely to share two short functions.

Do not impose numeric file-length, directory-width, directory-depth, module-count, plan-count,
or repository-shape rules.

Split and merge by cohesion, ownership, retrieval quality, testability, compile isolation, and
real boundaries.

Do not reorganize unrelated code for symmetry or aesthetics.

Temporary planning belongs in agent working state. Do not commit planning hierarchies or
bureaucracy to prove that the project avoided bureaucracy.

## Work selection and multi-turn execution

At the start of a substantial task:

1. inspect the branch, worktree, upstream state, and recent history without destroying unrelated
   changes;
2. read the authority documents relevant to the task;
3. trace producers, consumers, mutable authority, derived representations, ownership, trust
   boundaries, and failure paths;
4. characterize current behavior with focused tests or measurements;
5. identify the highest-leverage dependency-closed problem;
6. state a falsifiable hypothesis, completion criteria, reversal condition, and stop condition
   in temporary working state;
7. implement the smallest coherent correction that removes the root cause;
8. delete the displaced path and stale claims;
9. add focused tests and update the owning documentation;
10. run final verification after the final relevant change;
11. commit cohesive changes when the task requests or permits a commit; and
12. verify publication state when publication is requested.

The active task owns the objective, not an unverified proposed mechanism.

Change course when executable evidence invalidates the suggested implementation.

If a more severe correctness, safety, or authority defect blocks the selected vertical, fix that
dependency as part of the same vertical.

Do not use incidental findings as permission for an unrelated rewrite.

Every turn must leave the repository coherent, documented, tested, and usable.

Do not leave:

- two active architectures;
- a half-cutover;
- disabled correctness checks;
- a required unfinished migration;
- stale prose presented as current;
- a compatibility layer hiding an incomplete replacement; or
- dependence on a scratch artifact.

Prefer a smaller complete vertical over a larger half-implemented program.

Complete one coherent vertical. Update the roadmap. Identify the next problem. Stop.

Do not begin the next roadmap item merely because time remains.

Multi-turn progress is expected. The project does not need to decide every future
representation, runtime tier, platform, storage format, or service boundary in the current turn.

Preserve future options by keeping current mechanisms small, explicit, tested, and replaceable.

Do not commit task prompts, checkpoints, transcript summaries, copied context, handoff files,
completion capsules, or prompt archives.

If a task prompt was placed inside the repository for transport, remove it from the intended
commit.

## Semantic editing and dependency rules

Transactions publish one coherent final state or publish nothing.

Validate revision and namespace before mutation.

Failed transactions must not consume stable identities, change allocator state, mutate the
current snapshot, poison caches, or partially publish derived state.

Interpret multi-edit transactions by their intended final semantic state where the contract
promises order independence.

Do not let edit list order accidentally decide whether a dependency-closed transaction is valid.

Distinguish containment from dependency.

Deleting a container may delete facts whose semantic existence is owned by that container.

Do not silently delete independent declarations merely because they depend on the deleted
object.

When a dependent object cannot survive, require its explicit deletion, define an explicit narrow
ownership rule, or reject the transaction with an actionable deterministic blocker.

Do not generalize one narrow deletion rule into implicit transitive deletion of arbitrary
declarations.

A private dense relocation is not a public semantic movement.

Do not report survivor compaction as deletion and recreation.

Stable public identities survive private compaction when semantic meaning survives.

Old immutable snapshots remain internally valid after later revisions.

A stale identity in the current snapshot must not corrupt or alias a recreated object.

Foreign, stale, wrong-kind, wrong-owner, duplicate, missing, and invisible identities must fail
deterministically at the semantic boundary.

Derive dependency indexes from semantic authority. Do not maintain a second mutable dependency
truth.

When a transaction changes only a type argument, witness, bound, or other non-name semantic
fact, its diff and projection must still make the change reviewable.

## Semantics, safety, and resources

Language validity is determined by semantic laws, not project-selected size quotas.

Do not reject an otherwise valid trusted program because it exceeds an arbitrary number of
source bytes, tokens, nesting levels, declarations, fields, variants, parameters, arguments,
locals, functions, files, modules, blocks, IR nodes, identities, values, diagnostics, handles,
or analysis steps.

Do not disguise a semantic limit by raising it, widening an integer, moving it to another phase,
renaming it, or calling it a safety profile.

Use checked arithmetic and checked narrowing for sizes, offsets, identities, code locations,
handles, and indexes.

User-controlled depth must not consume unbounded native stack.

Use iterative traversal or a justified heap-backed work stack.

Never silently truncate a complete result. Paginate, stream, return an explicit partial result,
or fail.

An untrusted product may impose explicit coarse host-resource policy for input, memory, output,
elapsed time, cancellation, and concurrency.

Resource exhaustion is a typed host result, not a semantic error.

Do not design detailed untrusted policy before such a product exists.

Follow the accepted ownership specification.

Ordinary execution is collector-free and non-tracing.

Do not introduce tracing collection, language-visible hidden reference counting, raw-pointer
language surfaces, retain/release, or general `free` merely to simplify implementation.

A memory-semantic change requires a specification change and one complete cutover, not parallel
GC and non-GC modes.

Preserve exact move and borrow laws, deterministic cleanup where promised, cleanup on normal,
trap, error, cancellation, allocation failure, and early-exit paths, no double release,
stack-safe destruction, explicit host-resource ownership, and failure-atomic publication.

Maintain one complete generic production execution route.

An optional native or specialized path may decline only before effects and must leave the
generic route intact.

After specialized entry, its result or failure is final. Never re-execute effects through
fallback.

Validate fail-closed at real untrusted boundaries.

Inside one synchronous trusted typed pipeline, validated values carry authority.

Do not repeatedly serialize, hash, reconstruct, or independently revalidate them without a real
boundary consumer.

Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented
safe-caller contract, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or
property coverage.

Given the same semantic snapshot, target, options, inputs, and capabilities, scheduling,
allocation addresses, hash-table state, profile state, and cache state must not change completed
meaning or deterministic diagnostic selection.

## Generic semantics

Generic declarations, instantiations, substitutions, bounds, and witnesses are semantic facts,
not parser decorations.

One exact instantiation path should validate source-imported and source-free calls whenever
their semantics are equivalent.

Do not maintain separate source and workspace trait solvers that can disagree.

Inference is an authoring convenience. Exact validated substitutions and witnesses are the
semantic result.

Public semantic APIs must not expose compiler-dense implementation IDs or type-parameter names
as stable identity.

The compiler, not the caller, derives trait witnesses and implementation selection.

A valid generic call must use the complete generic production route when specialization is
unavailable.

Do not introduce a general higher-rank type framework, implicit inference engine, or generic
rewrite system before a current language requirement needs it.

If a current narrow generic restriction remains, expose one deterministic, consistent error
across source and source-free paths.

## Performance and evidence

Profile before optimizing.

Measure the selected product path rather than a detached surrogate.

Relevant evidence may include:

- process wall time and phase time;
- startup and throughput;
- edit and query latency;
- peak and retained memory;
- allocations and allocated bytes;
- bytes copied, parsed, rendered, serialized, hashed, or generated;
- repeated traversal counts;
- scale behavior;
- generated-code size; and
- binary size.

Before a comparison, state:

- the hypothesis;
- equivalent semantics;
- workload;
- environment;
- build and cache state;
- sample protocol;
- selection criterion; and
- reversal condition.

Prefer deterministic structural work counters over noisy timings when they answer the question.

Generated scale tests establish correctness and complexity shape. They are not substitutes for
representative application benchmarks.

Keep raw samples outside Git. Commit only compact reproducible evidence.

Do not turn noisy developer-machine timing into a correctness gate.

An optimization remains only when its end-to-end benefit justifies compile time, memory, code
size, complexity, test burden, and maintenance.

Do not add a validity quota to hide a performance defect.

Do not add a framework to avoid one local scan.

Full recomputation may remain the correct simple implementation until representative edit
workloads demonstrate that incremental machinery would pay for its complexity.

Do not add a second maximum-scale full-pipeline fixture when a smaller default test plus a
targeted ignored stress test establishes the same invariant.

Do not claim latency, memory, allocation, or throughput improvement without equivalent
measurements.

## Structure and dependencies

Organize code by coherent responsibility, not counts or symmetry.

A crate boundary should represent a real trust or unsafe boundary, an independently useful
library, a supported target, measured compile isolation, or a low-coupling subsystem.

Merge crates that mainly exchange internal descriptors, witnesses, re-exports, or compatibility
adapters.

Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny
modules, redundant models, and conversion layers when recombination improves comprehension.

Split a very large module only when the split establishes coherent ownership and reduces change
coupling. Do not shard it by line number.

Do not reorganize unrelated code for aesthetics.

Use mature dependencies when they remove substantial custom machinery or risk.

Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better.

Before adding a descriptor, registry, witness, identity, plan, contract, cache, conversion
layer, arena, or interner, identify its current producer, consumer, lifetime, boundary,
invalidation rule, and deletion condition.

Do not add a dependency for functionality that is smaller and clearer as local code.

Do not duplicate an internal representation in a public wrapper unless the wrapper removes
private identity, enforces a real boundary, or provides independently useful semantics.

## Tests

Tests should protect intended semantics and public invariants, not accidental topology.

Cover, as relevant:

- type, generic, trait, effect, capability, ownership, control-flow, and cleanup laws;
- completeness and explicit incomplete states;
- stable identity, namespaces, generations, revisions, deletion, and deterministic ordering;
- malformed input and stale, foreign, wrong-kind, wrong-owner, duplicate, and missing
  identities;
- transaction and artifact failure atomicity;
- exactly-once effects and generic/specialized equivalence;
- cancellation, resource failure, host errors, and cleanup;
- deep input, deep destruction, scale behavior, and checked representation boundaries; and
- real product integration.

Add a focused regression test for each root cause.

Use generated fixtures for scale.

Keep fast default tests separate from ignored locked-release stress geometry while exercising
the same algorithm at smaller sizes.

Use differential, property, model, or test-only reference implementations when an independent
oracle is cheap and useful.

Delete tests whose purpose is preserving provisional syntax, old serialized bytes, obsolete
APIs, deleted machinery, arbitrary limits, internal topology, or accidental details.

Replace them with tests of intended invariants.

Never weaken a test merely to make a redesign pass.

Do not assert private dense indexes as public semantics except in focused internal compaction
tests.

Do assert that stable public identities and completed meaning survive private relocation.

A convergence test must compare semantic outcomes, not merely matching output text.

A failure-atomicity test must verify the prior snapshot and identity allocator remain unchanged.

A stack-safety test must exercise construction, relevant transformations, and destruction on a
deliberately small stack when those phases are user-depth dependent.

## Documentation

Keep active documentation small, non-overlapping, and truthful:

- `README.md`: product introduction and first successful use;
- `docs/spec/`: intended external semantics and target contracts;
- `docs/status.md`: current implementation and known gaps;
- `docs/architecture.md`: current responsibilities, data flow, ownership, and trust boundaries;
- `docs/performance.md`: method, reproducible workloads, compact evidence, and reversal
  conditions;
- `docs/roadmap.md`: only `Now`, `Next`, and `Later`; and
- `docs/decisions/`: sparse durable decisions.

Update the owning document and delete stale text in the same change.

Do not add prose digests, global revisions, fact shards, generated inventories, copied tables,
per-commit evidence records, transcripts, handoffs, prompt archives, completion capsules, or
duplicate roadmaps.

Write a decision record only when a choice is durable, non-obvious, expensive to rediscover, and
has a meaningful reversal condition.

Do not describe a target architecture as current, a hypothesis as measurement, private
implementation movement as a public feature, a planned subsystem as supported, or a
developer-machine observation as a product guarantee.

Documentation examples must use the active API and active semantics. Delete examples that
preserve a displaced contract.

## Git, worktree, and publication discipline

Inspect worktree and branch state before editing.

Preserve unrelated tracked and untracked work.

Do not use destructive reset, checkout, clean, history rewrite, or force push against work you
did not create.

Commit only coherent repository changes.

Do not include prompts, raw benchmark output, temporary plans, generated scratch files,
credentials, or unrelated local changes.

Use a commit message that names the semantic or architectural result rather than implementation
churn.

Push only when the active task requests publication.

Never force push merely to make publication convenient.

After a requested push, verify the local branch, remote tracking branch, and pushed commit.

If publication fails, preserve the verified local commit and report the exact failure.

## Verification

After the final relevant change, run at least:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

Run retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run additional focused release stress, differential or property tests, small-stack and
deep-input tests, malformed-boundary tests, cancellation or allocation-failure tests, Miri,
sanitizer, fuzz, benchmark, documentation, and example checks when relevant.

If the environment prevents a command, report the exact command, failure category, relevant
output, and successful evidence that remains.

Do not silently substitute a weaker command.

Do not claim a command passed unless it ran after the final relevant change.

Do not treat a test that passed before the final change as final verification.

When an expensive ignored fixture is relevant but infeasible in the environment, run the same
algorithm at the largest practical smaller geometry and report the omitted exact command.

## Definition of done

A change is complete only when:

- it removes the dependency-closed root cause rather than one symptom;
- the active architecture is singular and displaced paths are gone;
- semantics and real safety boundaries are preserved or intentionally updated in the owning
  specification;
- no fake semantic data, retained dead authority, or arbitrary validity limit substitutes for a
  representation or algorithmic correction;
- failure cannot partially publish state, consume stable identities, poison an earlier snapshot,
  or duplicate effects;
- focused tests cover the changed invariant and important failure paths;
- performance claims have reproducible equivalent evidence;
- active documentation describes the checkout truthfully;
- final relevant verification ran after the final change;
- intended changes are committed and branch or upstream state is verified when the task
  requested it; and
- the repository is coherent for the next independent turn.

The final report must separate:

- implemented work;
- semantic and architectural changes;
- important deletions;
- measurements;
- tests and exact commands;
- commit and push state;
- untested paths;
- remaining risks;
- deliberately deferred work; and
- the next highest-leverage problem.

Stop after one coherent vertical.
