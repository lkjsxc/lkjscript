# AGENTS.md

## Scope and precedence

This file applies to the entire repository.

Follow, in descending order of authority:

1. The user's current request.
2. This file.
3. Executable behavior, tests, and machine-readable interfaces in the current checkout.
4. The small active documentation set established by this file.
5. Historical documents and Git history.

The repository is in an architectural reset. Legacy status registries, digest markers,
platform revisions, capsule manifests, repository graphs, evidence ledgers, and old
handoff sequences are not higher authority than this file. Do not maintain obsolete
machinery merely because that machinery requires its own maintenance. Delete or replace
it when it obstructs the product direction below.

Never infer a requirement from an old document when it conflicts with the user's current
request or this file. Record uncertainty explicitly and inspect the implementation.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance
language and runtime platform.

The long-term source of truth should be a semantic program model that agents manipulate
directly. Text is an import, export, debugging, review, and interoperability projection;
it is not assumed to be the permanent authoritative representation.

Optimize for the best long-term architecture rather than compatibility with the current
prototype. Breaking changes, large deletions, crate consolidation, syntax replacement,
IR replacement, and storage replacement are permitted when they produce a simpler and
stronger system.

Correctness, security boundaries, and final performance matter. Process and documentation
machinery are valuable only when they materially improve those outcomes.

## Priority order

Unless the user gives a narrower priority, work in this order:

1. Remove accidental complexity and obsolete machinery.
2. Remove arbitrary language, compiler, repository, and representation ceilings.
3. Establish small, truthful, executable documentation authority.
4. Reduce the compiler and runtime to one coherent production path.
5. Build the semantic program model and direct agent operations.
6. Make compilation, incremental editing, startup, and execution fast on representative
   workloads.
7. Expand libraries, services, GUI, web, game, and distributed capabilities after the
   foundations are credible.

Choose the highest-priority coherent vertical slice that can be completed and verified.
Do not start lower-priority feature work while a higher-priority architectural blocker is
being preserved without evidence.

## Product principles

### One active architecture

Maintain one active source model, one active semantic pipeline, one active set of runtime
semantics, and one active roadmap.

Do not create a permanent `v2`, shadow compiler, compatibility parser, dual-write path,
parallel status system, or old/new runtime split. A temporary migration bridge is allowed
only inside one atomic change and must have an explicit deletion point. Prefer direct
cutovers because backward compatibility is not required.

A tiered runtime may exist as one product path when measurement justifies it. Tiers must
not become separate public language implementations that require every feature to be
reimplemented several times. A small reference executor may exist for semantic testing,
but it is not automatically a production engine or a feature-completeness obligation.

### Deletion before abstraction

When complexity comes from duplicated mechanisms, remove mechanisms before adding a new
framework over them.

Prefer:

- deleting obsolete code over adding adapters;
- merging artificial crates over adding cross-crate contracts;
- one ordinary data structure over multiple authenticated projections inside one trust
  boundary;
- direct tests over status markers;
- generated reference data over hand-copied registries;
- Git history over an active archive hierarchy;
- a measured generic path over many narrow special cases;
- a clear failure over silent fallback or partial publication.

Do not preserve code because it was expensive to build. Preserve it only when it remains
useful under the current direction.

### AI-primary, not AI-exclusive

Agents should be able to inspect, query, edit, validate, and compile programs through
stable semantic operations. Humans should still be able to review meaningful projections,
diffs, diagnostics, and decisions.

AI-primary does not mean adding model calls to hard correctness gates. The compiler core,
program model, transactions, queries, and validation must be deterministic and usable
offline. Models may propose operations; deterministic machinery decides whether those
operations are valid.

### Performance is empirical

Do not justify architecture with slogans. Establish baselines, profile representative
workloads, compare alternatives, and retain concise decisions.

Optimize end-to-end outcomes: cold and warm compilation, incremental recomputation,
startup, execution latency and throughput, memory use, code size, allocation, copying,
cache behavior, and agent interaction cost.

A microbenchmark can expose a mechanism, but it cannot by itself choose the architecture.

## Explicit non-goals

Do not use the following as roadmap authorities:

- Brainfuck compatibility, completion, or benchmark success;
- compatibility with current `.lkjscript` syntax or old artifacts;
- preservation of platform revision numbers or digest-marker ecosystems;
- mandatory evaluator, VM, baseline, and optimizing parity;
- repository topology rules based on file lines, file bytes, line width, directory width,
  directory depth, or arbitrary fan-out;
- comprehensive proof or evidence dossiers for ordinary changes;
- a distributed program database before the local semantic model works;
- GUI, web framework, game engine, package network, or remote execution work that delays
  the compiler and source foundations;
- a new abstraction layer whose main purpose is to preserve an old abstraction layer.

Brainfuck material may remain only as low-cost historical or optional benchmark material.
It must not block changes, determine semantics, require special runtime mechanisms, or
appear in acceptance criteria. Delete it when its maintenance cost is nontrivial.

## Language validity and resource control

### Validity must not depend on arbitrary counts

A semantically valid program must not become invalid merely because it has more than a
project-selected number of:

- tokens;
- nested forms or nested types;
- children or arguments;
- declarations or top-level forms;
- fields, variants, parameters, patterns, or match arms;
- files, directories, imports, or directory entries;
- HIR expressions;
- SSA blocks, values, edges, or frame states;
- specialization instances;
- structural value nodes;
- repository nodes, documentation facts, or graph edges.

Do not “remove” a limit by raising its number, moving it to a different constant, renaming
it a safety maximum, or applying the same ceiling through a profile. Delete the semantic
restriction and repair the algorithm or representation that required it.

### Use four distinct concepts

Every remaining bound must belong to exactly one of these concepts:

1. **Semantic law.** A property required by the language, such as type correctness,
   capability access, ownership legality, or exhaustive matching. A semantic law is not a
   size quota.
2. **Unavoidable representation boundary.** A real boundary imposed by an integer width,
   operating-system API, file format, address space, or other external representation.
   Widen, segment, stream, or redesign before exposing it as an ordinary user limit.
3. **Host or request resource policy.** A configurable operational budget for untrusted or
   multi-tenant work, expressed in coarse resources such as bytes, memory, elapsed time,
   output, concurrency, or cancellation. It does not change language meaning.
4. **Internal implementation tuning or test geometry.** Private capacities, growth factors,
   thresholds, and fixture sizes. These may change without becoming public contracts.

Do not build another large taxonomy, positional ceiling table, or digest-bound profile
system around this classification.

### Trusted local behavior

Ordinary trusted local compilation should not use project-selected count quotas. It should
continue until success, cancellation, allocation failure, an unavoidable representation
boundary, or another real host failure.

Untrusted daemon and multi-tenant operations must accept explicit host policy. A program
that exhausts a low policy must be able to succeed under a higher policy without changing
its semantic validity or source representation.

Resource exhaustion must be typed, attributable, cancellation-safe, and failure-atomic.
Do not silently truncate programs, diagnostics, graphs, query results, generated code, or
serialized authority. Paginate or stream views whose output may be large.

### Scale-safe implementation

Replace recursive traversals that can follow user-controlled depth with explicit work
stacks or otherwise demonstrably stack-safe algorithms. Use checked arithmetic, fallible
allocation, and incremental or streaming processing where appropriate.

Do not preallocate a published maximum. Grow from observed need, reserve fallibly, and
measure peak memory.

Use wide or segmented identifiers when program scale can exceed a narrow index. Internal
compact IDs are welcome when overflow is checked and a scalable fallback exists; a narrow
ID must not become an unexplained language ceiling.

## Semantic program model

### Authority

The long-term authority is a typed semantic program model, not a text parse tree with more
metadata.

The model must represent program meaning independently of the current physical syntax,
formatting, comments, trivia, source spans, and file layout. Text-related data belongs to
projections and import/export attachments rather than semantic identity.

### Identity

Use stable logical identities for mutable program entities and edits. Names and source
locations are attributes, not identity.

Use content hashes selectively for immutable snapshots, immutable definitions, cached
artifacts, and interchange verification. Do not content-address every mutable node,
transaction, public fact, or prose fragment. Avoid cascades in which a small edit changes
unrelated identities.

### Incomplete programs are valid editor states

The model must eventually support typed holes, unresolved references, ambiguous choices,
conflict nodes, error nodes, and partially constructed declarations as first-class states.

The editor or agent service should preserve as much type, effect, ownership, and capability
information as can be determined. It must not require an agent to generate a complete text
file before receiving semantic feedback.

Incomplete program states are not executable releases, but they are valid transactional
workspace states.

### Operations

Prefer typed operations such as:

- create and delete entity;
- insert, move, and replace node;
- rename binding or declaration;
- set typed field or reference;
- rewire call or dependency;
- introduce, refine, and fill hole;
- apply a legal refactoring;
- resolve conflict;
- commit or abort transaction.

Operations must carry a base revision or equivalent precondition, be failure-atomic, and
return a semantic diff plus diagnostics. Avoid making textual patches the primary edit API.

### Queries

Provide deterministic queries for definitions, references, callers, callees, types,
effects, capabilities, ownership, diagnostics, holes, legal actions, dependencies, and
small context slices.

Queries over large results must support pagination, continuation, filtering, and stable
ordering. Do not require a complete repository graph to answer every local question.

### Projections

Support multiple projections from the same model:

- concise human-readable text;
- verbose diagnostic text;
- structured debug and interchange forms;
- semantic diffs;
- IDE or visual views;
- compiled artifacts.

The current line-oriented syntax may be retained temporarily as a bootstrap importer and
renderer. It is not a compatibility promise and must not constrain the model schema.

### Persistence and incrementality

Start with the smallest persistence design that proves the semantics: an in-memory model
and deterministic snapshot are sufficient for the first vertical slice.

Add a transaction log, embedded database, or binary snapshot only when scale, crash
recovery, or concurrency measurements justify it. Do not begin with a distributed store.

Incremental computation should be query-driven and dependency-aware. Evaluate existing
incremental-computation libraries as well as a small custom design. Select by correctness,
maintenance cost, memory behavior, invalidation precision, and measured latency, not by a
zero-dependency preference.

## Compiler architecture

### Preferred shape

Converge toward this conceptual flow:

```text
semantic program snapshot
    -> name/type/effect/ownership analysis
    -> canonical typed core IR
    -> verified executable IR
    -> one production execution path
```

Each boundary must have a clear responsibility. Do not independently reconstruct and hash
the same facts at every layer inside one trusted process.

The compiler should consume semantic model data directly. Text parsing should become an
adapter that constructs or updates the model, not a privileged sibling authority.

### Verification

Keep verification at genuine trust boundaries:

- untrusted source or semantic transaction input;
- untrusted serialized IR or artifact input;
- executable-memory installation and relocation;
- process or daemon protocol boundaries;
- capability and path boundaries;
- unsafe host interfaces.

Inside one trusted pipeline, prefer typed construction that makes invalid states hard to
represent. Do not duplicate a producer with several “independent” reconstructors unless a
real adversarial boundary or demonstrated bug class justifies the cost.

### Execution engines

End the default rule that every language feature must be implemented independently in an
evaluator, bytecode VM, baseline native tier, and optimizing native tier.

Measure and choose a coherent production path. Possible outcomes include a VM with a
measured native tier, direct native execution with a small reference executor, or another
simpler design. Do not preselect an outcome solely to preserve existing code.

A generic execution path must handle ordinary valid programs. Optimization and
specialization may improve it but must not turn unsupported specialization into a language
rejection. Use heuristics, caching, and generic fallbacks rather than fixed specialization
count ceilings.

Freeze or remove an optimizing tier that lacks representative benefit or creates a large
feature cross-product. Optimization work resumes when profiling identifies a material
bottleneck and a measurable candidate.

### Memory direction

Keep ordinary source free of lifetime names, retain/release operations, general `free`, raw
pointers, and memory-engine selection.

The current product direction remains collector-free and non-tracing. Do not add a tracing
fallback to avoid solving ownership, region, or representation problems unless the user
explicitly changes direction.

Prefer a small combination of static values, unique ownership, invocation or lexical
regions, arenas, and coarse immutable sharing. Minimize per-node metadata, repeated witness
records, owner copies, and cleanup plans. Cleanup authority should be established once and
consumed consistently.

Memory safety, deterministic cleanup where promised, and failure atomicity are mandatory.
The internal mechanism may change radically when evidence supports a better design.

## Repository architecture

### Organize by semantic cohesion

Files and directories may be as large or wide as their coherent responsibility requires.
Split when it improves ownership, testing, navigation, compilation, or retrieval. Do not
split to satisfy numeric topology policy.

Remove mechanically numbered shards, `impl_XX` fragments, one-child directory ladders,
and facade files that exist only because of old structural limits. Recombine related code
before choosing new natural boundaries.

Avoid a monolith, but do not confuse many crates with modularity. A crate boundary is
justified by at least one of:

- a real trust boundary;
- an independently useful library API;
- a materially different platform or build target;
- meaningful compile-time isolation;
- ownership by a distinct subsystem with low coupling.

Merge crates whose primary purpose is to exchange internal contracts, digests, witnesses,
or re-exports with each other.

### Dependencies

Third-party dependencies are permitted. Evaluate maintenance, security, portability,
binary size, compile time, API stability, and runtime performance.

Prefer a mature dependency when it removes substantial custom machinery or risk. Prefer
owned code for a small performance-critical mechanism when measurement supports it.

Do not reject a dependency merely to preserve a zero-dependency claim. Do not add a large
dependency without demonstrating its role.

### Unsafe code

Keep unsafe code at narrow, named mechanism boundaries with safe caller contracts and
focused tests. Use platform and standard-library facilities before custom unsafe code.

Do not spread unsafe code to save small amounts of time before profiling. Where unsafe code
is performance-critical, compare it against a safe implementation and retain the reason.

## Documentation authority

### Active documentation set

Converge toward a small set such as:

- `README.md`: product identity, build, and first successful use;
- `docs/current.md`: concise implemented capabilities and known gaps;
- `docs/architecture.md`: active architecture and trust boundaries;
- `docs/language.md`: current semantic language surface;
- `docs/source-model.md`: semantic program model and editing contract;
- `docs/roadmap.md`: ordered `Now`, `Next`, and `Later` work;
- `docs/decisions/`: only durable decisions whose rationale remains useful.

Exact names may change, but the roles must remain few and non-overlapping.

### Authority order

Use executable sources for facts whenever possible:

- Cargo metadata owns the crate graph;
- CLI code and tests own commands;
- schemas and types own wire structure;
- compiler tests own accepted and rejected language behavior;
- benchmark harnesses own measurement protocols;
- generated references own exhaustive tables.

Prose explains intent and consequences. It must not copy large registries that can drift.

### Remove documentation bureaucracy

Do not introduce or maintain:

- digest markers embedded in prose;
- platform revisions that must change for unrelated edits;
- hand-authored public-fact shards;
- status closure graphs;
- capsule manifests for ordinary directories;
- evidence records for every narrow implementation commit;
- architecture graphs that duplicate Cargo without generation;
- agent checkpoints committed as product authority.

Documentation checks should validate links, generated references, code examples, and a
small number of explicit invariants. They must not claim to prove arbitrary surrounding
prose.

Execute documentation examples when practical. Keep raw benchmark output and transient
audits in CI artifacts or `target/`; commit only compact baselines or decisions that serve
future engineering.

Historical material belongs in Git history. Retain a historical file only when it is
actively useful and clearly separated from current guidance.

## Performance program

### Representative workloads

Maintain a compact suite covering:

- programs substantially beyond every former source and IR boundary;
- deep and wide semantic structures;
- large declaration, product, enum, pattern, and generic workloads;
- full and incremental compilation;
- single-node and small-transaction edits;
- name, type, effect, ownership, and exhaustiveness analysis;
- numeric loops and function calls;
- byte and streaming I/O;
- aggregate construction, traversal, and cleanup;
- allocation and region-heavy behavior;
- warm daemon reuse and multiple applications;
- at least one service-like workload.

Brainfuck is not a required workload.

### Metrics

Measure, as relevant:

- cold and warm wall time;
- incremental p50, p95, and p99 latency;
- peak and retained memory;
- allocation count and allocated bytes;
- copying and serialization bytes;
- generated code and metadata size;
- runtime throughput and tail latency;
- startup and time to useful execution;
- cache hit rate and invalidation breadth;
- semantic edit success rate;
- agent tokens, tool calls, invalid operations, and repair cycles.

Keep benchmark configuration and environment visible. Do not discard unfavorable samples
or move acceptance thresholds after observing results.

### Optimization discipline

Profile before optimizing. Prefer algorithmic and data-layout improvements before unsafe
micro-optimization.

Use contiguous or cache-conscious representations in hot paths where measurement supports
them. Avoid forcing every semantic structure into a flat representation when segmentation
or streaming scales better.

A regression gate should protect a demonstrated property, not freeze the prototype's
architecture. Update or remove a benchmark when its workload no longer represents the
product, while retaining the decision rationale.

## Testing and validation

### Test behavior, not bureaucracy

Tests should establish semantic behavior, safety properties, scale behavior, failure
atomicity, and performance-relevant invariants.

When removing a former ceiling, add:

1. a fast test just beyond the old boundary;
2. a substantially larger generated case in a scale suite;
3. an overflow, allocation-failure, quota, or cancellation test where relevant;
4. a check that low operational policy exhausts while higher or unrestricted local policy
   accepts the same semantic program.

Fixture sizes are not new public limits.

### Differential testing

Use differential testing when implementations are genuinely independent and the comparison
has high defect-finding value. Do not keep several production engines solely to preserve a
differential oracle.

Property tests, fuzzing, sanitizers, Miri, model-based tests, and generated programs are
valuable when they target risky boundaries. Run focused checks during development and the
broad applicable suite before publication.

### Canonical local checks

During the reset, no legacy all-in-one command is automatically canonical. Establish a
small transparent command set based on the current architecture, normally including:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

Add focused runtime, sanitizer, fuzz, documentation-example, and benchmark checks when the
changed boundary warrants them. If an old `xtask` gate fails only because obsolete
structure or status machinery was intentionally removed, replace the gate rather than
repairing the obsolete machinery.

Report exactly what ran, what failed, what was unavailable, and what remains untested.

## Working method

### Start from reality

At the start of each substantial task:

1. Read this file and the user's request.
2. Inspect `git status`, the current commit, Cargo metadata, active docs, and relevant code.
3. Search for all definitions, consumers, tests, documentation, and generated artifacts of
   the concept being changed.
4. Reproduce the important current behavior or failure.
5. Establish a focused baseline if performance or scale is involved.
6. Select one coherent vertical slice and define its deletion and acceptance criteria.

Do not assume handoff prose is current.

### Plan enough, then change the system

Use planning to expose dependencies and deletion opportunities. Do not spend an entire run
building a planning hierarchy while leaving the architecture unchanged.

For a large objective, maintain a concise ordered roadmap and implement the highest-value
coherent slice now. The repository should finish every run in a buildable, understandable
state even when the overall reset is incomplete.

### Make changes end to end

A coherent change includes every layer that still legitimately owns the behavior. It does
not include obsolete layers merely because they once claimed ownership.

Remove dead tests, documentation, registries, feature flags, aliases, and generated files in
the same cut. Avoid commented-out code and permanent migration scaffolding.

### Review continuously

After implementation:

- inspect the diff for accidental compatibility paths and duplicated authority;
- search for old symbols, numbers, statuses, and terminology;
- compare code and documentation claims;
- run focused and broad validation;
- inspect binary size, compile time, or runtime metrics when materially affected;
- check that the change reduced or at least did not gratuitously increase conceptual
  complexity.

### Git discipline

Respect concurrent user work. Do not discard unrelated modifications. Do not force-push,
rewrite public history, or use destructive cleanup without explicit instruction.

Make coherent commits with descriptive messages. A commit may be large when it performs an
atomic architectural cutover. Avoid sequences of authority-only commits that exist solely
to synchronize digest machinery.

Push only when the user's workflow requests it or existing task context clearly authorizes
it. Report the exact commit and branch state when publication occurs.

## Definition of done

A change is done when all applicable statements are true:

- the intended behavior exists in the active architecture;
- obsolete paths and compatibility mechanisms are removed;
- arbitrary count ceilings in scope are gone rather than enlarged;
- user-controlled traversals are scale-safe;
- errors are typed and failure-atomic;
- the production path and any retained reference path agree where required;
- documentation is concise, current, and does not duplicate machine authority;
- focused tests cover success, failure, old-boundary regression, and scale;
- representative performance was measured when architecture or hot paths changed;
- applicable broad checks pass;
- untested boundaries and remaining risks are explicit;
- the next step follows from the active roadmap rather than an obsolete handoff.

## Prohibited patterns

Do not:

- replace one unexplained limit with a larger unexplained limit;
- make trusted local compilation subject to tiny fixed count profiles;
- convert maintainability preferences into language validity rules;
- split code by line count, directory width, or numbered shard policy;
- add a second canonical source representation;
- mirror provisional text syntax in the permanent semantic schema;
- use source spans, names, formatting, or content hashes as the sole identity of mutable
  entities;
- silently truncate a graph, query, diagnostic set, output, or artifact;
- claim a fallback path was not used without testing it;
- require every feature in every experimental engine;
- add an optimization-specific language rejection instead of a generic path;
- duplicate facts across HIR, SSA, bytecode, runtime, and documentation without a trust
  boundary;
- maintain digest and status machinery solely to certify itself;
- add compatibility aliases, editions, decoders, or dual formats without explicit user
  direction;
- let a toy workload determine language architecture;
- start broad product features before the source, compiler, and runtime foundations are
  coherent;
- stop at analysis when a safe, high-value implementation slice can be completed.
