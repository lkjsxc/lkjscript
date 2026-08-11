# AGENTS.md

## Scope

This file applies to the entire `lkjscript` repository.

It governs repository inspection, design, implementation, testing, measurement, documentation,
commit construction, optional publication, subagent use, and final engineering reports.

Use English for code, public APIs, diagnostics, tests, documentation, commit messages, measurement
records, and engineering reports unless an active task explicitly requires another language for a
user-facing artifact.

A more specific `AGENTS.md` may narrow procedure for its subtree, but it must not silently create a
second language definition, semantic authority, ownership model, compiler route, runtime route, or
documentation authority.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free,
high-performance language and implementation.

The project is not trying to place an AI wrapper around an ordinary text-first language.

The project is trying to provide a deterministic semantic programming substrate that agents can:

- discover without reading the entire repository;
- construct without manufacturing source text;
- edit through typed semantic operations;
- inspect through stable structured facts;
- leave incomplete without making incomplete state executable;
- check without entering program effects;
- compile directly from semantic authority;
- execute intentionally through one complete production route;
- review through compact deterministic projections and diffs; and
- verify with reproducible evidence.

A model may propose work.

Deterministic local machinery decides validity.

Model inference must not become part of parsing, type checking, ownership checking, validation,
optimization correctness, persistence correctness, runtime correctness, or any other trusted
boundary.

Do not optimize the language around one model, provider, tokenizer, context window, benchmark,
prompt style, orchestration product, or current API price.

## Priority order

Use this order when goals conflict:

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. Scale-safe representations and algorithms.
5. One syntax-independent mutable semantic authority.
6. One complete source-free edit, inspect, check, compile, and run route.
7. A small deterministic local workflow for agents and ordinary tools.
8. Representative evidence before caches, parallelism, incrementality, or services.
9. One complete generic production execution route.
10. Optional specialization that never weakens correctness.
11. Broader products only after their prerequisites are real.
12. Persistence, collaboration, daemonization, scheduling, and distribution only after measured
    demand.

A later platform idea is not permission to skip a prerequisite.

A benchmark may expose a defect.

It does not define the language.

## Authority and truth

Classify each claim before deciding which artifact owns it.

- The active task owns its objective and explicit acceptance criteria.
- This file owns repository-wide engineering procedure.
- Accepted files under `docs/spec/` own intended external semantics and target contracts.
- Executable code, tests, command definitions, schemas, manifests, and lock files own checkout
  behavior.
- `README.md` owns the product introduction and first successful use.
- `docs/status.md` summarizes current implementation and known gaps.
- `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust
  boundaries.
- `docs/performance.md` owns measurement method, retained workloads, compact evidence, selection
  criteria, and reversal conditions.
- `docs/roadmap.md` owns ordering and intent only.
- Sparse accepted files under `docs/decisions/` own durable rationale when one is justified.
- Git history owns superseded implementation and prose.

A target specification may lead implementation.

That difference is an implementation gap, not permission for silent contradiction.

When claims conflict:

1. identify the exact semantic or operational dimension;
2. inspect the artifact that owns that dimension;
3. inspect executable evidence for current behavior;
4. decide which artifact is stale or wrong;
5. update or delete stale material in the same coherent change; and
6. leave one intelligible authority.

Use explicit labels such as `Current`, `Target`, `Hypothesis`, `Measured`, `Historical`, `Unknown`,
and `Blocked` when unlabelled prose would mislead.

Do not create another authority system from:

- prompt archives;
- conversation transcripts;
- scratch plans;
- completion capsules;
- handoff documents;
- generated fact ledgers;
- duplicated status tables;
- global architecture revisions;
- closure registries;
- plan trees;
- semantic digests with no consumer; or
- copied manifests and dependency graphs.

Task prompts and subagent reports are transport, not durable repository authority.

## Autonomous judgment

Choose internal designs from:

- the actual checkout;
- accepted specifications;
- focused executable tests;
- measurements;
- concrete product requirements;
- current consumers;
- real failure boundaries; and
- reversible assumptions.

Ask the user only when a genuinely external product requirement is missing and no safe explicit
assumption can complete the active objective.

Do not ask the user to choose among internal implementation alternatives that the repository can
decide.

Incompatible changes are permitted.

This includes incompatible changes to:

- language semantics;
- syntax;
- source encoding;
- Rust APIs;
- command shapes;
- package formats;
- lock formats;
- cache formats;
- module paths;
- crate boundaries;
- compiler representations;
- runtime representations;
- tests;
- examples; and
- documentation.

Obsolete features, adapters, fixtures, aliases, feature flags, exports, dependencies, and prose may
be deleted.

Broad authority is permission to make the right change.

It is not an instruction to maximize scope.

Preserve unrelated tracked work, untracked work, credentials, host state, external data, and remote
history.

Repository redesign authority is not authority to erase unrelated state.

## Backward compatibility

Backward compatibility is not a project objective unless the active task identifies a currently
consumed external boundary that must remain compatible.

Do not retain old:

- syntax;
- serialized bytes;
- commands;
- Rust APIs;
- module paths;
- crate names;
- fixtures;
- examples;
- aliases;
- adapters;
- migrations;
- feature flags; or
- documentation

merely because they already exist.

When a better design requires a cutover:

1. update the owning specification when semantics change;
2. update every active producer;
3. update every active consumer;
4. replace active fixtures and examples;
5. replace tests that protect the old route;
6. delete the displaced implementation;
7. delete compatibility layers and migration scaffolding;
8. remove stale exports, dependencies, and prose; and
9. verify that exactly one active route remains.

Do not create permanent `legacy`, `v2`, `next`, `new`, edition, compatibility, or dual-write
architectures.

The `.lkjscript` extension is fixed.

Other notation, grammar, schemas, bytes, commands, package models, compiler forms, runtime forms,
and persistence choices remain replaceable unless accepted semantics explicitly fix them.

## Multi-turn engineering

Work as a sequence of coherent, dependency-closed verticals.

One turn should not attempt the whole roadmap.

For each turn:

1. inspect branch, worktree, upstream, and recent history;
2. read the relevant authority documents;
3. map producers, consumers, mutable authority, derived facts, ownership, trust boundaries, and
   failure paths;
4. identify one demonstrated defect, accepted gap, measured bottleneck, or explicit product result;
5. state a falsifiable hypothesis in temporary working state;
6. state completion criteria;
7. state a reversal condition;
8. state a stop condition;
9. implement the smallest root-cause correction that creates a complete product result;
10. delete displaced paths;
11. update the owning documentation;
12. run focused checks during iteration;
13. run the final relevant verification boundary once the final relevant inputs are stable;
14. commit cohesive changes when permitted;
15. publish only when explicitly requested;
16. report remaining risk; and
17. name the next highest-leverage problem without beginning it merely to appear ambitious.

Do not leave:

- two mutable semantic authorities;
- two active compiler routes;
- a half-cutover;
- disabled correctness checks;
- a hidden executable fallback;
- an unfinished required migration;
- stale prose presented as current;
- an unowned cache;
- a temporary compatibility layer; or
- a subagent branch whose required changes were not reviewed and integrated.

Do not turn incidental findings into unrelated rewrites.

## Evidence-first work selection

Begin from at least one of:

- a demonstrated defect;
- a focused failing test;
- an accepted specification gap;
- a current roadmap item;
- an explicit product requirement;
- a measured bottleneck;
- a blocking safety boundary; or
- a concrete maintenance burden with current consumers.

Characterize current behavior before choosing a mechanism.

Use the smallest useful:

- regression test;
- semantic comparison;
- deterministic work counter;
- phase timer;
- profile;
- allocation observation;
- output measurement;
- command trace; or
- representative benchmark.

Fix the dependency-closed root cause rather than the most visible symptom.

Prefer semantic simplification and deletion before new machinery.

A vague concern that a system may eventually become large is not evidence.

A hypothetical future consumer is not a current consumer.

A design diagram is not an implementation result.

A single noisy timing is orientation, not a performance claim.

## Anti-overengineering gate

Before adding an abstraction, write down in temporary working state:

- the present problem;
- the current producer;
- the current consumer;
- the owner of the new state;
- its lifetime;
- its invalidation rule;
- its failure behavior;
- the concrete benefit;
- why direct local code is insufficient;
- what duplicated work or invalid state it removes;
- the maintenance cost;
- the reversal condition; and
- the deletion condition.

An abstraction is justified when it does at least one of the following for current code:

- removes meaningful duplication;
- makes invalid state unrepresentable;
- isolates a real trust or unsafe boundary;
- exposes an independently useful API;
- enables a measured property;
- materially simplifies reasoning; or
- removes a larger and more fragile mechanism.

The mechanism must be smaller than the problem.

The mechanism must not duplicate authority.

Use this escalation order:

1. Delete unused work.
2. Simplify semantics.
3. Simplify representation.
4. Reuse an existing invariant.
5. Reuse the canonical validator.
6. Add a small local derived fact.
7. Improve traversal or layout.
8. Make an invalid state unrepresentable.
9. Add caching only after measured repeated work.
10. Add parallelism only after measured separable work.
11. Add incrementality only after measured recomputation dominates.
12. Add specialization only behind the complete generic route.
13. Add a process boundary only after process-boundary cost or isolation demand is measured.

Prefer explicit local code for one current use.

Extract a framework only after multiple current consumers prove shared semantics.

Do not add speculative:

- daemons;
- services;
- session brokers;
- journals;
- databases;
- CRDTs;
- schedulers;
- registries;
- plugin systems;
- rewrite DSLs;
- cache frameworks;
- incremental frameworks;
- proof ecosystems;
- network protocols;
- broad target matrices;
- deoptimization systems;
- PGO systems;
- self-hosting scaffolding;
- orchestration platforms; or
- persistent agent state.

Such machinery requires a demonstrated current boundary, an end-to-end consumer, measured need,
explicit ownership, acceptance criteria, failure behavior, and a reversal condition.

Do not create:

- a universal graph engine for one traversal;
- a generic recovery framework for one incomplete state;
- a rewrite framework for a few remaps;
- an event system for one synchronous result;
- a trait hierarchy to share two short functions;
- a general scheduler to run a few independent checks; or
- an agent protocol to pass a small evidence packet.

## One active architecture

Maintain one active:

- language definition;
- semantic authority;
- identity model;
- incomplete-state model;
- compiler input route;
- ownership model;
- generic production execution route;
- package model;
- verification contract; and
- documentation authority model.

A small independent evaluator may remain as a test oracle.

It is not automatically a second production engine.

Crate and module names have no authority by themselves.

Preserve, merge, split, rename, or delete components according to:

- cohesion;
- ownership;
- trust boundaries;
- unsafe boundaries;
- independently useful APIs;
- current consumers;
- coupling;
- compile isolation; and
- measured maintenance or build cost.

When architecture causes a defect, replace it.

Do not surround it with adapters, registries, synchronization bookkeeping, dual writes, or migration
scaffolding.

## Semantic authority

One syntax-independent semantic state owns mutable program meaning.

Semantic state must be able to exist without:

- source text;
- formatting;
- file paths;
- source spans;
- parser nodes;
- source hashes;
- compiler-dense indexes;
- rendered diagnostics; or
- a running service.

Source, files, comments, formatting, spans, and hashes may be:

- importer inputs;
- provenance;
- presentation attachments;
- cache keys;
- review views; or
- interoperability forms.

They are not semantic authority.

Do not use:

- dummy files;
- placeholder paths;
- fabricated hashes;
- synthetic declarations;
- fake entry points;
- reserved placeholder identities;
- hidden executable bodies; or
- render-and-reparse cycles

to satisfy semantic invariants.

Every derived representation needs:

- a current producer;
- a current consumer;
- a lifetime;
- an invalidation rule;
- an owner; and
- a deletion condition.

Dense IDs, vector positions, slots, offsets, register numbers, layout indexes, and traversal
coordinates remain private.

Compilation consumes one complete semantic snapshot directly.

Do not render and reparse source internally.

Do not serialize and deserialize an in-process typed value merely to manufacture another authority
token.

## Incomplete semantic state

Incomplete state is valid editing state.

Incomplete state is never executable state.

Represent missing, unresolved, ambiguous, conflicting, or recovered meaning explicitly, one concrete
lifecycle at a time.

For each incomplete state define:

- known facts;
- unknown facts;
- semantic identity;
- owner;
- context;
- expected type;
- actual type where known;
- effects;
- scope;
- diagnostic;
- completeness blocker;
- legal next actions;
- query view;
- projection;
- diff behavior;
- replacement;
- deletion;
- resolution;
- stale and foreign identity behavior;
- failure atomicity;
- old-snapshot behavior;
- compilation rejection; and
- independent downstream defense.

Preserve every sound fact.

Mark unknown facts explicitly.

Never lower incomplete state to:

- `unit`;
- zero;
- `false`;
- an empty value;
- a trap;
- a guessed declaration;
- the first candidate;
- an arbitrary candidate; or
- a hidden fallback.

Never retain the displaced executable expression behind an incomplete node.

Reject incomplete snapshots before:

- ownership planning;
- memory planning;
- SSA construction;
- bytecode lowering;
- native lowering;
- executable installation;
- VM entry; or
- host effects.

Do not build a universal incomplete-state framework before multiple current states prove shared
semantics and shared lifecycle.

A finite ambiguity is not automatically a new stored state.

First establish that existing unresolved state plus deterministic candidate queries cannot represent
the required authoring intent.

## Identity and revisions

Use opaque logical identity only where meaning must survive rename, movement, replacement boundaries,
or private compaction.

For each public identity define:

- namespace;
- allocator;
- kind;
- owner;
- uniqueness lifetime;
- generation validation;
- revision precondition where needed;
- continuity law;
- removal law;
- tombstone behavior;
- slot reuse behavior; and
- persistence lifetime, if persistence actually exists.

Names, paths, spans, formatting, source order, and hashes are not universal mutable identity.

Surviving identities remain stable across private relocation.

Old immutable snapshots remain valid.

Reject foreign namespace, stale revision, stale generation, wrong kind, and wrong owner before
publication.

A failed operation must not:

- publish a revision;
- consume stable identities;
- mutate allocator free lists;
- change future allocation order;
- change diagnostics;
- change blockers;
- change continuations; or
- mutate derived state.

Do not expose compiler-dense IDs.

Do not imply cross-process identity without a real cross-process lifetime and validation boundary.

## Transactions

Semantic edits are typed operations over identities.

One successful transaction publishes one coherent revision.

One failed transaction publishes nothing.

Validate as applicable:

- namespace;
- base revision;
- generation;
- identity kind;
- owner;
- operation shape;
- operation disjointness;
- preconditions;
- draft connectivity;
- acyclicity;
- single-parent structure;
- child uniqueness;
- lexical visibility;
- final dependency closure;
- type consistency;
- effect consistency;
- ownership legality;
- match usefulness;
- match exhaustiveness;
- cleanup correctness; and
- allocation failure.

When batching promises order independence, validate the intended final semantic graph rather than
edit-list order.

Containment-owned facts may cascade with their owner.

Independent dependents must not be silently deleted.

Transaction-local handles may exist before stable entities.

They must be:

- typed;
- scoped;
- validated;
- non-persistent; and
- impossible to confuse with stable identity.

Use one structured public model per concept unless input and output genuinely have different
semantics.

Do not add transactional machinery whose only purpose is to preserve obsolete routes.

## Public semantic APIs

Expose semantic meaning.

Do not expose parser nodes, private addresses, dense indexes, debug formatting, or display strings as
the only representation.

Machine-facing results must be:

- deterministic;
- revision-labelled;
- stably ordered;
- completeness-explicit;
- bounded or paginated;
- structurally typed; and
- honest about provisional versus canonically validated facts.

Never silently truncate.

Return compact headers and stable identities before expensive expansion.

Expose legal next actions where deterministic machinery already knows them.

Do not claim a candidate is legal when canonical ownership, effect, scope, or capability validation
has not run.

Label provisional candidates accurately.

Public recursive values must be stack-safe to:

- construct;
- clone;
- compare;
- hash when required;
- project;
- validate;
- convert;
- rewrite; and
- destroy.

## Types and generics

Generic declarations, substitutions, bounds, instantiations, and witnesses are semantic facts.

They are not parser decoration.

Source import and source-free editing converge on one exact instantiation and trait-selection path.

Inference is an authoring convenience.

Exact substitutions and compiler-derived witnesses are the semantic result.

Keep type identity stable and syntax-independent.

Use checked conversion at compact or host representation boundaries.

Do not add a general inference framework, higher-rank framework, or generic recovery framework before
a current language requirement needs it.

Do not impose arbitrary type-depth quotas.

## Ownership and memory

Ordinary execution is collector-free and non-tracing.

Do not add:

- tracing garbage collection;
- hidden language-visible reference counting;
- raw-pointer language surfaces;
- retain/release APIs;
- a general `free`;
- parallel GC and non-GC modes; or
- an ownership escape hatch that bypasses canonical checking.

Preserve exact laws for:

- move;
- shared borrow;
- mutable borrow;
- loan lifetime;
- cleanup;
- early return;
- trap;
- cancellation;
- allocation failure;
- resource failure;
- host-resource ownership; and
- teardown.

Prevent:

- leaks;
- double release;
- use after move;
- use after owner end;
- stale loan state;
- duplicated side effects; and
- stack-overflow destruction.

Unsafe code belongs in a narrow named mechanism with:

- explicit invariants;
- a safe-caller contract;
- focused malformed-input tests; and
- suitable Miri, sanitizer, fuzz, property, or differential coverage.

Do not spread unsafe code to avoid designing a typed boundary.

## Compilation and execution

Maintain one complete generic production execution route.

Optional specialization may decline only before program effects.

The unchanged generic route must remain available after a pre-entry decline.

Once specialized entry begins, its result or failure is final.

Never rerun effects through fallback.

Checking must not execute program effects.

Do not construct execution state in an effect-free check path when compilation can finish without it.

Validate fail-closed at real untrusted boundaries.

Inside one trusted typed synchronous pipeline, do not repeatedly:

- serialize;
- hash;
- reconstruct;
- clone;
- render;
- parse; or
- independently revalidate

the same value without a consumer boundary or measured need.

## Scale and resource policy

Language validity follows semantic laws.

It does not follow project-selected quotas.

Do not reject trusted valid programs because of arbitrary:

- byte counts;
- token counts;
- nesting depth;
- declaration counts;
- field counts;
- variant counts;
- parameter counts;
- local counts;
- function counts;
- file counts;
- module counts;
- IR-node counts;
- identity counts;
- value counts;
- diagnostic counts;
- handle counts; or
- work counts.

Do not disguise a limit by raising, widening, moving, renaming, or profiling it.

Use checked arithmetic and checked narrowing.

Use iterative traversal or a justified heap-backed work stack for user-controlled depth.

An untrusted product may impose explicit coarse:

- input;
- memory;
- output;
- time;
- cancellation;
- concurrency;
- handle;
- fuel; or
- artifact-size

policy.

Resource exhaustion is a typed host result.

It is not a semantic error.

Do not design detailed untrusted policy before an actual untrusted product exists.

## AI-facing local workflow

The smallest complete agent workflow is:

1. discover;
2. inspect;
3. edit;
4. query;
5. check;
6. review;
7. run intentionally; and
8. verify.

Prefer:

- executable examples;
- concise authoring documentation;
- focused symbol search;
- effect-free compile-only commands;
- structured diagnostics;
- one-shot semantic operations;
- deterministic projections;
- deterministic diffs;
- direct in-process semantic APIs; and
- quiet successful validation.

Add a daemon only after measurements show process startup, repeated import, or a real isolation
boundary dominates a representative workflow.

Agent use does not imply a:

- database;
- journal;
- session broker;
- scheduler;
- network protocol;
- CRDT;
- persistent semantic store;
- remote executor; or
- broad agent framework.

Command names, arguments, exit behavior, stdout, and stderr must be deterministic and tested.

Successful high-frequency validation is quiet by default.

A one-shot command must not pretend identities survive across invocations without a real lifetime.

## Lead-agent responsibility

One lead agent owns:

- the active objective;
- architectural judgment;
- branch and worktree awareness;
- task decomposition;
- write ownership;
- integration;
- final verification;
- commit construction; and
- the final report.

The lead agent may use subagents.

The lead agent must not outsource final architectural responsibility.

The lead agent must review evidence and code before integration.

A subagent recommendation is evidence, not authority.

## Subagent selection

Use subagents only when independent work can reduce critical-path reasoning or execution time.

Good subagent work includes:

- independent read-only repository mapping;
- focused specification comparison;
- focused test-gap analysis;
- focused performance measurement;
- isolated implementation with a narrow file boundary;
- independent invariant review; and
- independent final diff review.

Poor subagent work includes:

- multiple agents reading the same broad files;
- multiple agents solving the same design question;
- splitting one tightly coupled semantic edit among writers;
- asking agents to produce long generic summaries;
- delegating a decision that requires one coherent global model;
- spawning agents merely because they are available; and
- running several heavyweight verification suites on the same machine.

Do not select an arbitrary fixed number of subagents.

Select the smallest useful set from actual independent questions.

Stop adding subagents when coordination cost exceeds expected benefit.

## Subagent evidence packets

A read-only subagent returns a compact evidence packet containing:

- assigned question;
- conclusion;
- exact files and symbols inspected;
- relevant executable evidence;
- unresolved uncertainty;
- recommendation;
- reversal condition where applicable; and
- no unrelated redesign proposal.

A writing subagent additionally returns:

- isolated worktree or branch;
- owned files;
- commit identifier;
- exact tests run;
- command outcomes;
- assumptions;
- integration hazards; and
- remaining risk.

Do not return entire files, complete successful logs, broad repository summaries, or private
scratch reasoning when a concise evidence packet is sufficient.

The lead agent should reuse these packets rather than repeat the same search.

## Parallel work policy

Prefer parallel read-only discovery before parallel writing.

Parallel writing is allowed only when tasks are genuinely independent and integration boundaries are
explicit.

For parallel writers:

- use separate worktrees or equivalent isolated checkouts;
- assign non-overlapping file or component ownership;
- avoid shared generated outputs;
- avoid shared mutable external state;
- require cohesive commits;
- have the lead review every diff;
- integrate one change at a time;
- rerun affected tests after integration; and
- resolve architectural conflicts centrally.

Never allow two agents to edit the same file concurrently.

Never allow two agents to evolve the same semantic model independently.

Never use a shared dirty worktree as a multi-writer coordination mechanism.

Do not create an orchestration service, task database, protocol, or registry for repository-local
parallel work.

A small explicit ownership table in temporary working state is sufficient.

## Heavy-command coordination

Heavy commands compete for CPU, memory, disk bandwidth, linker resources, file locks, and caches.

Do not run multiple heavyweight Cargo commands concurrently in the same target directory unless an
experiment has established that the combination is correct and improves end-to-end wall time.

In particular, avoid concurrently running full forms of:

- `cargo clippy`;
- `cargo test`;
- release `cargo build`;
- LTO linking;
- Miri;
- sanitizer suites;
- fuzzing;
- container builds; and
- large generated stress tests

against one shared target directory.

Separate `CARGO_TARGET_DIR` values avoid file interference but duplicate compilation, storage, and
cache work.

Use them only when measurement shows net benefit.

Do not have subagents independently launch the full final verification boundary.

The lead schedules final heavyweight verification.

## Attention and API-cost discipline

Model context, tool output, developer attention, wall time, CPU time, memory, disk, CI minutes, and
API spend are engineering resources.

Search before opening large files.

Read focused ranges and diffs before full material.

Run the smallest command that can falsify the current hypothesis.

Prefer:

1. a focused test;
2. the affected test target;
3. the affected crate;
4. the workspace boundary;
5. the retained container or packaging boundary.

Do not repeat an identical successful command when no relevant input changed.

Do not dump:

- unchanged files;
- repository-wide diffs;
- generated HIR;
- generated SSA;
- full bytecode;
- machine code;
- massive JSON;
- complete projections;
- complete successful logs; or
- duplicate subagent reports

without a concrete consumer.

Never hide a non-zero status, diagnostic, sanitizer finding, fuzz failure, malformed output, or
environment error.

Use native quiet flags for successful commands.

When a native quiet mode is unavailable, capture the full log outside Git and surface the relevant
failure excerpt and log path.

Do not add a runner, broker, logging framework, cache, service, or output protocol merely to silence
commands.

When efficiency is an objective, measure as applicable:

- command count;
- model round trips;
- tool calls;
- stdout bytes;
- stderr bytes;
- output lines;
- duplicate diagnostics;
- wall time;
- CPU time;
- peak RSS;
- repeated compilation;
- repeated parsing;
- repeated serialization;
- repeated validation;
- cache misses;
- context required for the next decision; and
- critical-path duration.

Do not infer provider-token or billing savings from byte counts alone.

State API-cost claims as measured, estimated with explicit assumptions, or unknown.

## Performance

Profile before optimizing.

Measure the selected product path.

Before a comparison, state:

- hypothesis;
- equivalent semantics;
- workload;
- environment;
- build profile;
- cache state;
- sample protocol;
- selection criterion;
- reversal condition; and
- stop condition.

Use as appropriate:

- wall time;
- phase time;
- startup;
- time to first result;
- throughput;
- edit latency;
- query latency;
- compile latency;
- execution latency;
- memory;
- allocations;
- copied bytes;
- parsed bytes;
- serialized bytes;
- rendered bytes;
- output volume;
- command count;
- deterministic work counts;
- code size;
- binary size; and
- scale shape.

Prefer deterministic work counters when they answer the question better than noisy timing.

Generated scale tests establish correctness and complexity shape.

They do not establish application performance.

Keep raw samples outside Git.

Commit only compact reproducible evidence.

Do not turn developer-machine noise into a correctness gate.

Keep an optimization only when end-to-end benefit justifies:

- compile time;
- memory;
- code size;
- complexity;
- tests;
- dependency cost;
- portability cost; and
- maintenance.

Full recomputation may remain correct until representative edits justify incrementality.

Serial execution may remain correct until representative separable work justifies parallelism.

Remove temporary instrumentation without a continuing consumer.

Do not claim improvement without equivalent evidence.

## Verification throughput

Verification speed is a product property of the development workflow, but coverage and failure
visibility remain correctness properties.

Before changing verification topology:

1. measure the current critical path;
2. separate compilation from test execution where practical;
3. identify repeated compilation and repeated product execution;
4. identify CPU, memory, disk, linker, port, and shared-file constraints;
5. distinguish local developer latency from total CI compute;
6. predeclare a selection criterion;
7. predeclare a reversal condition; and
8. keep the current route when evidence does not justify change.

Candidate improvements may include:

- removing truly duplicate work;
- reusing one already-built product binary;
- narrowing focused iteration commands;
- isolating independent smoke tests;
- bounded parallel execution of isolated post-build smokes;
- CI job decomposition when latency benefit exceeds duplicate compilation;
- cache-key correction;
- test-target reorganization around real ownership;
- reducing unnecessary feature or target duplication; or
- adopting a mature test runner after measurement.

Do not assume that more parallel jobs are faster.

Parallel jobs may increase:

- repeated compilation;
- cache fragmentation;
- peak memory;
- disk contention;
- linker contention;
- CI minutes;
- cold-start overhead;
- log complexity; and
- flakiness.

Do not add `cargo-nextest`, `sccache`, a custom scheduler, a custom test protocol, or a persistent
runner without measured net benefit and a clear maintenance owner.

## Smoke-test parallelism

Independent post-build smoke tests may run concurrently only when each test has isolated:

- temporary directories;
- output files;
- network ports;
- database paths;
- process cleanup;
- compiler outputs;
- benchmark outputs; and
- environment variables.

A test using a fixed port, shared output path, shared database, shared target artifact, or global
process state is not independent until that state is parameterized or serialized.

A parallel smoke harness must:

- bound concurrency;
- capture each command's stdout and stderr separately;
- preserve every exit status;
- terminate and reap child processes;
- clean temporary state;
- report all failed tasks deterministically;
- avoid hiding a failure behind another process;
- remain understandable as a small local mechanism; and
- demonstrate end-to-end benefit.

Do not parallelize benchmark and correctness workloads together when resource competition makes the
measurement meaningless.

## Repository structure and dependencies

Organize by coherent responsibility.

Do not organize by arbitrary line counts, directory width, depth, symmetry, or visual uniformity.

A crate boundary needs at least one of:

- a real trust boundary;
- an unsafe boundary;
- an independently useful library;
- a supported target boundary;
- measured compile isolation;
- a low-coupling subsystem; or
- a product boundary with current consumers.

Merge crates that mainly exchange internal descriptors, re-exports, or adapters.

Remove:

- numbered shards;
- include-only facades;
- one-child ladders;
- artificial tiny modules;
- redundant models;
- conversion-only layers without a boundary; and
- empty architectural placeholders.

Split a large module only when the split establishes ownership and reduces change coupling.

Use mature dependencies when they remove substantial machinery or risk.

Keep local code when it is smaller, clearer, safer, easier to audit, or measurably better.

Do not add benchmark, logging, serialization, allocation, orchestration, or parallelism frameworks
when a small current-purpose mechanism is sufficient.

## Tests

Tests protect intended semantics and public invariants.

They do not protect accidental topology.

Cover as relevant:

- type behavior;
- generic behavior;
- trait selection;
- effects;
- capabilities;
- ownership;
- control flow;
- cleanup;
- completeness;
- identity;
- namespace;
- generation;
- revision;
- deletion;
- replacement;
- ordering;
- malformed input;
- stale identity;
- foreign identity;
- wrong kind;
- wrong owner;
- visibility;
- exactly-once effects;
- cancellation;
- resource failure;
- allocation failure;
- deep operations;
- checked boundaries;
- machine output;
- effect-free checking;
- integration;
- failure atomicity; and
- old-snapshot validity.

Add a focused regression test for each root cause.

Use generated fixtures for scale.

Keep fast defaults separate from ignored locked-release stress while exercising the same algorithm at
smaller scale.

Use differential, property, model, or test-only reference implementations when an independent oracle
is cheap and clear.

Delete tests that preserve:

- provisional syntax;
- old bytes;
- obsolete APIs;
- deleted machinery;
- arbitrary semantic limits;
- private topology; or
- accidental behavior.

Never weaken a test merely to make a redesign pass.

Convergence compares semantic outcomes, not only text.

Failure-atomicity tests verify prior snapshot and allocator state.

Stack-safety tests cover construction, transformation, and destruction on a small native stack.

Machine-output tests decode output as a consumer would.

Do not validate structured output only by substring.

Quiet-success tests assert both streams are empty.

No-effects checks use observable would-be effects.

Parallel tests must not depend on execution order or uncontrolled global state.

## Documentation

`README.md` owns product introduction and first successful use.

`docs/spec/` owns intended semantics and target contracts.

`docs/status.md` owns current implementation and known gaps.

`docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.

`docs/performance.md` owns method, workloads, compact evidence, selection decisions, and reversal
conditions.

`docs/roadmap.md` contains only `Now`, `Next`, and `Later`.

`docs/decisions/` contains sparse durable decisions.

Update the owning document and delete stale text in the same change.

Do not add:

- digests;
- global revisions;
- fact shards;
- copied tables;
- transcripts;
- handoffs;
- prompt archives;
- completion capsules;
- duplicate roadmaps; or
- raw measurement logs.

Write a decision record only when a choice is:

- durable;
- non-obvious;
- expensive to rediscover; and
- governed by a meaningful reversal condition.

Do not describe:

- target as current;
- hypothesis as measurement;
- private relocation as public movement;
- planned systems as supported;
- developer observation as a guarantee; or
- one host result as a portable property.

Examples must use active APIs.

Mechanically check examples where practical.

## Git and publication

Inspect worktree and branch state before editing.

Preserve unrelated tracked and untracked work.

Do not use destructive reset, checkout, clean, history rewrite, or force push against work you did not
create.

Commit only cohesive repository changes.

Exclude:

- task prompts;
- raw logs;
- raw samples;
- scratch plans;
- credentials;
- generated temporary files;
- subagent evidence packets; and
- unrelated work.

Use a commit message naming the semantic, architectural, or measured workflow result.

A multi-part task may use multiple cohesive commits when each commit leaves the repository coherent.

Do not split commits merely by file type.

Push only when explicitly requested.

Never force push for convenience.

After a requested push, verify:

- local branch;
- tracking branch;
- pushed commit; and
- remote result.

If publication fails, preserve the verified local commit and report the exact failure.

## Verification strategy

During iteration, run the smallest focused command that can disprove the change.

Escalate verification only after focused evidence passes.

Do not repeatedly run the full boundary after unchanged inputs.

### Tier 0: inspection

Use:

- focused search;
- dependency inspection;
- diff inspection;
- static reasoning;
- existing measurements; and
- current test inventory.

Tier 0 never substitutes for executable evidence when behavior changes.

### Tier 1: focused verification

Run the smallest relevant:

- unit test;
- integration test;
- generated fixture;
- compile-only check;
- source-free convergence test;
- machine-output test;
- property test; or
- smoke test.

### Tier 2: affected component verification

Run the affected crate, binary, feature, or integration target with the production feature shape
needed by the change.

### Tier 3: native repository boundary

At the final relevant state, normally run:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Run these commands against the final relevant inputs.

Do not repeat them merely because documentation changed after a code-complete passing state, unless
that documentation is compiled or checked by the command.

### Tier 4: retained container and product boundary

Run the retained container verification when available and when the change can affect:

- dependency installation;
- Docker inputs;
- packaging;
- release compilation;
- compiler behavior;
- executable behavior;
- VM behavior;
- native behavior;
- host capabilities;
- examples;
- smoke scripts;
- system libraries; or
- publication confidence.

The retained command is:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

A documentation-only change need not rebuild the container.

A narrowly isolated internal change may omit Tier 4 only when the lead records concrete dependency
and risk reasoning, native end-to-end evidence passes, and no changed input reaches the retained
container result.

Do not omit Tier 4 merely because it is slow.

Improve or narrow it only through an equivalent measured verification design.

### Additional verification

Run additional relevant:

- locked-release stress;
- differential tests;
- property tests;
- small-stack tests;
- deep-input tests;
- malformed-boundary tests;
- cancellation tests;
- allocation-failure tests;
- Miri;
- sanitizers;
- fuzzers;
- benchmarks;
- documentation checks; and
- machine-output checks.

If the environment blocks a command, report:

- exact command;
- failure category;
- relevant output;
- whether the change caused it;
- successful remaining evidence; and
- unverified risk.

Never claim a command passed when it did not complete successfully.

## Final report

Report:

- completed objective;
- demonstrated root cause or evidence gate;
- principal design;
- replaced or deleted paths;
- focused tests;
- convergence evidence;
- measurements when relevant;
- output-volume evidence only when measured;
- exact verification commands and outcomes;
- environment-limited checks;
- remaining risk;
- documentation state;
- commit state;
- publication state;
- subagent use and integration result, when subagents materially contributed; and
- the next highest-leverage problem.

Explain why work stopped before beginning the next problem.

Keep the report factual and compact.

Do not reproduce the task prompt.

Do not paste complete successful logs.

Do not claim future work is implemented.
