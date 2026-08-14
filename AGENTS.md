# AGENTS.md

## Scope

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local implementation procedure.
It must not weaken the repository-wide requirements in this file.

Use English for maintained code, tests, diagnostics, protocol fields, documentation,
commit messages, benchmark labels, and machine-readable output.

The active user task has authorized a complete architectural reset.
Backward compatibility with any implementation, source form, artifact, protocol,
package format, CLI, test, document, identifier scheme, or runtime behavior that
predates this reset is not a goal.

Preserve unrelated uncommitted work.
Never use destructive Git commands against work that you did not create.
Git history is the archive for the deleted implementation.
Do not preserve dead code in the active tree merely for reference.

## Mission

Build `lkjscript` as an AI-primary programming system whose canonical program
representation is a strongly typed and strongly constrained semantic graph.

An AI coding agent must be able to create, inspect, transform, validate, compile,
execute, package, debug, and maintain an `lkjscript` program without authoring or
round-tripping source code.

The live semantic graph is the sole program authority.
Textual source code is not a required input, storage form, edit authority, or
compiler boundary.

One logical per-user host daemon is the long-term control plane.
The daemon owns live workspaces, immutable snapshots, durable graph state,
semantic queries, compilation caches, executable caches, runtime instances,
capability grants, resource policy, and observability.

"One daemon" means one logical authority.
It does not require one address space.
Sandboxed workers, compiler workers, and runtime cells are permitted when they
preserve one control plane and one semantic authority.

Long-term performance is a first-class product objective.
The project must pursue world-leading edit latency, compilation latency, warm
startup, throughput, tail latency, memory density, and native runtime speed.
No world-leading claim may be made without reproducible comparative evidence.

## Product Laws

The following laws are non-negotiable unless the user explicitly supersedes them.

1. The semantic program graph is the only mutable program authority.
2. Source text is never required to construct, revise, compile, or run a program.
3. The compiler consumes a semantic graph snapshot directly.
4. All persisted graph nodes and edges obey a closed typed schema.
5. Arbitrary property bags, arbitrary string-labelled edges, and untyped graph
   mutation are forbidden in the canonical model.
6. Every accepted mutation is a typed transaction.
7. Every successful transaction publishes exactly one immutable snapshot.
8. Every failed transaction publishes nothing.
9. Failed transactions consume no persistent identifiers.
10. Stable node identity is independent of names, positions, ordering accidents,
    content hashes, dense compiler indexes, and memory addresses.
11. Persistent node identifiers are never reused.
12. Names are presentation and lookup metadata, not identity.
13. Derived types, bindings, effects, ownership facts, layouts, dependencies,
    diagnostics, and compiler IR are not duplicated source authority.
14. Incomplete programs are represented by typed semantic holes or other explicit
    incomplete nodes, not malformed graph state.
15. A snapshot may be incomplete but must always be structurally valid.
16. Incomplete snapshots are queryable.
17. Only complete snapshots may enter executable lowering.
18. The production compiler has one canonical executable IR path.
19. Execution tiers are accelerators over one semantic path, not separate language
    implementations.
20. Runtime or compiler optimization failure must not redefine program validity.
21. Resource policy limits one host boundary or execution, not language meaning.
22. The daemon is the sole live writer of durable workspace state.
23. Clients mutate programs through the daemon protocol, not by editing serialized
    artifacts behind the daemon.
24. `.lkjscript` denotes a semantic graph artifact, not a source-code file.
25. Old architecture is deleted rather than wrapped, migrated, versioned, or
    maintained in parallel.
26. No compatibility edition, legacy mode, dual reader, dual writer, or transitional
    source authority may be added.
27. No parser is required by the core product.
28. Optional textual or visual projections may exist only as derived, lossy or
    lossless views whose output is never automatically re-ingested as authority.
29. AI suggestions are untrusted proposals.
30. Deterministic validators, type checkers, effect checkers, ownership checkers,
    artifact validators, and runtime guards decide acceptance.
31. Host effects require explicit typed capabilities.
32. Ambient filesystem, network, process, database, clock, entropy, terminal, and
    device authority is forbidden.
33. No mandatory global stop-the-world tracing collector may be introduced.
34. Any isolated managed-memory region must have explicit semantics, boundaries,
    latency evidence, and a reason ownership or region storage is insufficient.
35. User-controlled depth must not consume unbounded native stack.
36. Arbitrary semantic count ceilings are forbidden.
37. Performance tuning values are implementation policy, not semantic validity.
38. Stable public identity and compact private identity are separate domains.
39. Cache keys and content hashes are never public mutable identity.
40. The repository must remain smaller, clearer, and easier for agents to search
    than the implementation it replaces.

## Authority Order

Use the following authority order.

1. The active user task.
2. This `AGENTS.md`.
3. Accepted normative files under `docs/spec/`.
4. Executable code and focused tests.
5. Protocol schemas, artifact schemas, manifests, and generated schema output.
6. `docs/status.md`.
7. `docs/architecture.md`.
8. `docs/performance.md`.
9. `docs/roadmap.md`.
10. `README.md`.
11. Comments, old prompts, old commits, issue prose, and historical documents.

The pre-reset implementation is historical evidence only.
Its behavior is not a compatibility contract.
Its names and abstractions do not receive presumption merely because they exist in
Git history.

When two active artifacts disagree, classify the fact and repair its owner.
Do not create a third authority to reconcile them.

## Repository Reset Rule

The reset must be a direct cutover.

Keep only:

- Git history;
- the repository identity;
- the Apache-2.0 license unless the user changes it;
- this `AGENTS.md`;
- the active reset prompt while it is needed;
- externally settled product laws in this file;
- new files deliberately created for the replacement architecture.

Delete or replace:

- the old Cargo workspace and crate graph;
- the old parser and source representation;
- old semantic workspace implementations;
- old HIR, SSA, bytecode, VM, native, JIT, host, package, contract, and app code;
- old generated fixtures and tests;
- old package and lock formats;
- old CLI commands and machine output;
- old documentation and authority machinery;
- old Docker and verification topology;
- old prompt archives once they no longer serve the active task;
- every compatibility shim for the deleted system.

Do not port old modules one by one.
Do not preserve old tests as hidden requirements.
Do not copy old code into `legacy`, `archive`, `v1`, `old`, or `compat`.
Git already provides history.

The first replacement commit must be buildable and testable.
Do not leave the repository as a deletion-only state.

## Canonical Terminology

Use these terms consistently.

### Semantic Program Graph

The canonical typed graph that represents packages, modules, declarations, types,
functions, regions, blocks, operations, values, holes, and semantic references.

Abbreviation in prose may be `SPG`.
Prefer the full term in public documentation and diagnostics.

### Workspace

A daemon-owned mutable history of immutable semantic graph snapshots.

### Snapshot

An immutable, revision-labelled, content-hashed semantic graph state.

### Node ID

A stable workspace-scoped identifier for one persistent semantic node.

A node ID is never reused.
A node ID is not a content hash.
A node ID is not a compiler index.

### Local Handle

A transaction-scoped symbolic reference to a node that the same transaction creates.

A local handle never appears in a published snapshot.

### Revision

The monotonically increasing publication sequence of one workspace.

### Snapshot Hash

The deterministic content hash of one canonical snapshot.

It is used for cache identity, export identity, and reproducibility.
It does not replace node identity.

### Graph Schema

The closed machine-enforced definition of node kinds, attribute types, edge slots,
cardinality, ordering, containment, reference rules, and local invariants.

### Derived Fact

A fact computed from a snapshot, including resolution, type, effect, ownership,
layout, dependency, diagnostic, liveness, specialization, or optimization data.

Derived facts may be cached.
They are never mutable source authority.

### Core IR

The compact verified executable representation derived from a complete snapshot.

Core IR is private compiler state.
It may be cached or serialized as a same-build executable cache.
It is not program authority.

### Daemon

The one logical per-user host authority that owns workspace mutation and runtime
coordination.

### Runtime Cell

An isolated execution instance supervised by the daemon.

## Semantic Graph Schema

The canonical graph is not a generic property graph.

Each node has exactly one closed `NodeKind`.

Each node kind declares:

- required attributes;
- optional attributes;
- ordered child slots;
- unordered canonical-set slots when order is semantically irrelevant;
- reference slots;
- operand slots;
- result slots;
- region slots;
- cardinality;
- target node kinds;
- ownership of contained nodes;
- whether cycles are legal;
- local validation rules;
- completeness requirements;
- derived queries that apply.

Attributes use closed typed values.
Do not use `Map<String, Value>` as canonical semantic state.
Do not use unknown-field preservation.
Reject unknown node kinds, attributes, edge slots, and operation variants.

Containment edges form a forest or DAG according to explicit schema rules.
Reference edges may form cycles where recursion or mutual dependency requires them.
Value edges obey scope and dominance rules.
Region boundaries define visibility.

Every collection has one declared ordering rule.
Observable order must never depend on hash-table iteration or allocator order.

The initial language schema is deliberately closed.
Do not implement dialect registration, plugin-defined node kinds, or arbitrary
schema extension.
Language evolution changes the schema deliberately.

## Canonical Body Model

Function bodies use a structured region-and-value graph.

The canonical body model contains:

- regions;
- ordered blocks;
- block parameters;
- ordered operations;
- operation operands;
- operation results;
- nested regions;
- explicit terminators;
- direct references to definitions and values.

Value references use stable semantic node identity.
They do not use variable spelling, source coordinates, or preorder positions.

The graph may retain structured control operations such as conditional, loop, and
match regions.
The compiler may lower them to a private CFG.
Do not force agents to author low-level machine CFG details unless an accepted
feature requires irreducible control flow.

Operation order is explicit where evaluation order matters.
Pure operations may be optimized according to dependency and effect facts.
Do not introduce mandatory effect-token plumbing merely to imitate another IR.

Each operand has a schema-defined or signature-defined use mode.
Examples include read, copy, move, shared borrow, unique borrow, and capability use.
The ownership checker validates those modes.
Do not encode ownership semantics in names or comments.

## Identity

Use one stable persistent node-ID domain per workspace.

Prefer a simple design:

- a random or otherwise globally unique workspace identity;
- a monotonically increasing 64-bit or wider node counter;
- no persistent ID reuse;
- transaction-staged allocation;
- checked exhaustion behavior;
- deterministic test allocation.

A failed transaction leaves the next persistent ID unchanged.

Deletion removes a node from the current snapshot.
Old snapshots retain it.
The identifier is never assigned to another node.

Rename preserves identity.
Move preserves identity only when the operation explicitly defines semantic
continuity.
Replacement creates new identity unless the operation explicitly preserves the
same semantic entity.

Private compiler structures use dense indexes chosen for locality.
Never expose those indexes through public APIs, diagnostics, artifacts, diffs, or
runtime traces.

Builtins use one explicit builtin identity domain.
Do not fake builtins as workspace nodes with magic names.

## Transactions

Every mutation request includes:

- workspace identity;
- base revision;
- optional idempotency key;
- an ordered batch of typed operations;
- transaction-local handles for created nodes;
- requested validation level;
- requested response projection.

Transactions stage:

- graph changes;
- index changes;
- identifier allocation;
- diagnostics;
- derived invalidation;
- durable bytes;
- publication metadata.

A successful transaction:

1. validates the base revision;
2. resolves local handles;
3. validates every operation;
4. validates the final staged graph;
5. computes the deterministic diff;
6. prepares durable state;
7. durably commits;
8. publishes one new immutable snapshot;
9. returns the new revision and hash.

A failed transaction changes none of those states.

Do not partially publish a batch.
Do not consume IDs on failure.
Do not repair invalid requests heuristically.
Return structured rejection facts.

Support dry-run validation without publication.
Dry-run must use the same validators as commit.

Use optimistic revision checking.
Do not add CRDTs, conflict nodes, multi-writer merge machinery, or collaborative
editing until there are defined concurrent writers and accepted semantics.

## Incomplete Programs

Incomplete states are first-class and explicit.

Use typed holes for missing expressions, types, declarations, operands, regions,
or other schema-owned content where a partial state is useful.

A hole records only facts its owner can know, such as:

- expected type;
- expected effect bound;
- expected ownership mode;
- visible values;
- visible declarations;
- owning region;
- source-free semantic context;
- completeness blockers.

Do not store natural-language model guesses as semantic facts.

Unresolved references, when needed, are explicit constrained nodes.
They never masquerade as a valid resolved reference.

Every published partial graph remains structurally valid.
Diagnostics and legal-constructor queries must work on it.

Initial execution stops at the complete-snapshot gate.
Do not implement execution-through-holes until a concrete product need justifies
the semantic and runtime complexity.

## Types

Start with a small explicit type system and grow it through complete verticals.

Persistent type references use stable builtin or semantic node identities.

Derived type interning may use private canonical indexes.

At declaration boundaries, prefer explicit types.
At operation boundaries, derive types from operation schemas and called
signatures.
Do not add broad global inference before agent workflows demonstrate a need.

Nominal type identity is independent of structure and name.

Generic arguments are explicit in the first complete implementation.
Inference may later become a query that proposes exact arguments.
Inference must never silently alter persisted graph meaning.

There is no implicit numeric conversion.
There is no hidden null.
There is no unchecked cast.
There is no ambient exception mechanism.

Use explicit sum types for recoverable failure.
Runtime traps are structured and reserved for conditions the language contract
classifies as traps.

## Effects and Capabilities

Effects are deterministic derived facts.

Capabilities are explicit typed values or explicit entry grants.

A function signature declares or constrains its allowed effects.
The checker derives actual effects and rejects violations.

Host operations declare:

- required capability kind;
- operand and result types;
- ownership modes;
- deterministic effect classification;
- failure result;
- blocking behavior;
- cancellation behavior;
- resource accounting hooks.

Do not duplicate host-operation truth in workspace, compiler, daemon, and runtime
tables.
One canonical contract owns it and active consumers derive from it.

No ambient access is permitted.
Tests must prove denial without a grant and success with an exact grant.

## Ownership and Memory

The semantic graph must support memory safety without textual lifetime syntax.

The long-term default is:

- value semantics for ordinary immutable data;
- explicit affine semantics for external resources and mutable identity;
- compiler-derived moves, borrows, storage, and cleanup;
- no mandatory global tracing collector;
- explicit arenas or isolated managed regions only for use cases that require them.

Do not promise a complete memory model before it is specified and tested.

Before adding heap aggregates, define:

- value identity versus value semantics;
- copying and moving;
- sharing;
- mutation;
- cycles;
- destruction;
- failure cleanup;
- cancellation cleanup;
- FFI ownership;
- runtime layout;
- interpreter behavior;
- native behavior.

Do not let the first primitive execution slice accidentally define the final heap
model.

## Compiler

The compiler entry accepts:

- an immutable complete snapshot;
- one entry definition;
- target information;
- explicit capability grants;
- explicit optimization policy.

The canonical path is:

```text
Semantic Program Graph snapshot
    -> completeness and semantic validation
    -> compact dense Core IR
    -> Core IR verification
    -> generic execution or native lowering
```

Do not recreate the old source -> HIR -> SSA -> bytecode cross-product.

Core IR should be a compact typed CFG/register representation suitable for both a
stack-safe interpreter and native lowering.

The semantic graph remains high level enough for stable editing.
Core IR remains low level enough for efficient execution.

Every lowering is deterministic.
Every private ID is rebuilt from snapshot content.
Every verifier rejects malformed private IR before execution.

The generic interpreter is the complete semantic route during bootstrap.
Native tiers are accelerators.
A native decline occurs before entry and leaves the unchanged verified Core IR for
generic execution.
After native entry begins, do not retry effects in another engine.

## Runtime and Native Code

Use one runtime ABI for generic and native execution.

Runtime values are statically typed by Core IR.
Avoid universal boxed values when static layout is available.
Choose representations from measurements and exact semantics.

Initial native work should evaluate a mature backend such as Cranelift before
building a custom assembler or integrating a heavy optimizing framework.

A future top tier may use a different backend only when representative evidence
shows that the added compile time, maintenance, and dependency cost buys material
product value.

JIT installation must preserve W^X.
Executable memory is writable only before sealing and executable only after
sealing.
Relocations, entry signatures, stack maps, unwind metadata, and runtime-call
targets are validated before publication.

Native code cache keys include every semantic and target fact that affects code.
A cache hit is an optimization.
A cache miss or invalid entry falls back safely.
Cache contents never define program validity.

## Daemon

The daemon is the sole writer of live workspace state.

Initial target:

- Linux x86-64;
- one per-user daemon;
- local authenticated IPC;
- foreground mode for tests;
- explicit state directory;
- deterministic startup and shutdown;
- one workspace writer lock;
- immutable snapshot reads;
- bounded request parsing;
- structured protocol errors.

The CLI is a client.
It must not contain a second semantic implementation.

The daemon may internally call the semantic kernel as a library.
Tests may call internal library APIs for focused verification.
Public mutation still belongs to the daemon protocol.

The daemon owns:

- workspace registry;
- current snapshots;
- durable commit;
- query service;
- compiler service;
- runtime supervision;
- capability grants;
- resource policy;
- cache lifecycle;
- metrics;
- protocol version.

Do not add a distributed cluster, remote daemon, plugin marketplace, generalized
scheduler, or service mesh during bootstrap.

## Persistence and `.lkjscript`

A `.lkjscript` artifact stores a semantic graph snapshot or package.
It is not source code.

The persistent format has:

- an explicit magic value;
- an explicit schema version;
- an explicit encoding version;
- deterministic canonical ordering;
- checked lengths and indexes;
- a content hash;
- corruption detection;
- bounded decoding;
- no Rust-memory-layout dependency;
- no pointer values;
- no private compiler indexes;
- no cached derived facts unless a separately invalidatable cache section owns them.

Until a stable format is explicitly declared, incompatible replacement is allowed.
Version fields prevent ambiguity; they do not promise compatibility.

The first durable implementation should favor obvious crash correctness over an
unproven database architecture.
An atomic snapshot replacement is acceptable.
Introduce a journal, embedded database, or chunk store only after measured rewrite
cost or recovery requirements justify it.

The daemon must not acknowledge a durable mutation before durable commit succeeds.

## Protocol

The protocol is typed and versioned.

One Rust or schema definition owns each request and response.
Machine-readable schema output is generated from that authority.
Do not maintain a parallel hand-written protocol catalog.

Requests reject unknown variants and invalid fields.

Responses use:

- stable error codes;
- exact node IDs;
- exact revisions;
- exact snapshot hashes;
- deterministic ordering;
- explicit truncation;
- revision-bound continuations;
- compact summaries by default;
- selective expansion.

Natural language may be attached as explanation.
It never replaces structured facts.

Support transaction-local handles so an agent can create a mutually dependent
subgraph in one request.

Support idempotency where clients may retry after transport uncertainty.
Idempotency does not bypass base-revision checks.

## Agent Interface and API Cost

Treat model tokens, context, latency, tool calls, and retries as product resources.

Reduce cost through semantics, not hidden heuristics.

Provide compact queries such as:

- workspace summary;
- node summary;
- node expansion;
- definition signature;
- body slice;
- incoming references;
- outgoing references;
- callers;
- callees;
- type facts;
- effect facts;
- ownership facts;
- diagnostics;
- completeness blockers;
- legal constructors;
- semantic diff;
- compile explanation;
- runtime trace.

Default output returns identities and compact facts.
Expansion is opt-in.

Use revision-bound aliases or dictionaries only as protocol compression.
Never let a session alias become persistent identity.

Allow an agent to request a deterministic context pack for a hole or failing node.
The pack must be selected from type, binding, dependency, and ownership facts.
Do not use an LLM inside the daemon to choose correctness-critical context.

Batch independent reads.
Batch dependent writes in one transaction.
Avoid one request per graph edge when a typed aggregate operation can express the
same invariant safely.

Do not add prompt registries, model routing, token billing, or agent orchestration
to the language daemon merely to claim AI integration.

## Queries and Incrementality

Correct full recomputation is the oracle.

Initial implementations may recompute derived facts.

Incremental computation is added when:

- repeated work is measured;
- query boundaries are pure;
- dependencies are exact;
- invalidation is tested;
- cache failure cannot change semantics;
- full recomputation remains available as an oracle.

A Salsa-like or rustc-like query DAG is a candidate, not a requirement.

Query keys bind the snapshot and semantic input.
Cross-snapshot reuse requires proof that the relevant input fingerprint is
unchanged.

Never persist mutable compiler caches inside canonical graph state.

## Packages and Dependencies

Packages and modules are graph entities, not directories.

File paths are not semantic identity.

An immutable exported package is content-addressed as a canonical package graph.
Live workspace nodes retain stable workspace identity.

Dependencies resolve to exact package identities.
Resolution output is stored as semantic package metadata, not a source lockfile.

Do not design a registry, federation protocol, signing hierarchy, or global package
network before local packages and exact dependency closure work end to end.

## Security

Treat protocol bytes, artifacts, host calls, FFI values, and native code as trust
boundaries.

Use OS peer identity and state-directory permissions for local daemon access.

Validate all untrusted lengths, counts, indexes, discriminants, IDs, revisions,
hashes, and capability references.

Unsafe code is permitted only at a genuine boundary.
Each unsafe block has a local safety argument.
Keep unsafe code physically isolated when practical.

Fuzz:

- artifact decoding;
- protocol decoding;
- graph transaction validation;
- Core IR verification;
- native lowering boundaries;
- capability dispatch.

Never claim sandboxing that is not implemented.
A supervised worker process is preferred when native execution cannot be safely
contained in the daemon address space.

## Performance

Measure at least:

- daemon cold start;
- daemon warm request latency;
- workspace open;
- snapshot load;
- transaction validation;
- durable commit;
- compact query latency;
- type/effect/ownership recomputation;
- Core IR lowering;
- interpreter startup;
- interpreter throughput;
- native compile latency;
- native warm execution;
- code cache hit;
- code cache memory;
- per-workspace memory;
- per-runtime-cell memory;
- throughput under multiple apps;
- p50, p95, and p99 latency where concurrency exists.

Use representative applications, not one toy loop.

Keep a small bootstrap microbenchmark set.
Add macrobenchmarks only when the language can express them honestly.

Comparisons must record:

- hardware;
- operating system;
- toolchain;
- target;
- commit;
- build mode;
- warmup;
- sample count;
- input;
- output oracle;
- median;
- dispersion or tails;
- memory;
- compiler and runtime phases.

Do not tune for a benchmark by narrowing valid programs.

## Architecture Restraint

Before adding an abstraction, answer:

1. Which current producer creates it?
2. Which current consumer needs it?
3. Which invalid state does it remove?
4. Which repeated work does it remove?
5. Which test or measurement proves that need?
6. Could a local function or enum suffice?
7. Does it create another authority?
8. Does it create another identity domain?
9. Does it require serialization?
10. Does it require a process boundary?
11. Does it increase agent search cost?
12. What is its deletion condition?

Prefer, in order:

1. delete dead code;
2. use a closed enum;
3. use one authoritative struct;
4. use a local helper;
5. use a sorted vector;
6. use an explicit work stack;
7. use one narrow index;
8. use one measured cache;
9. add a process only for isolation or measured concurrency;
10. add a general framework only after multiple real consumers exist.

Do not add:

- a generic property graph;
- a general constraint solver;
- an open dialect system;
- a visitor framework for one traversal;
- a registry for a closed vocabulary;
- a serializer for same-build in-process values;
- a database before durable state requires one;
- a scheduler before multiple runnable cells exist;
- a cache before reuse is measured;
- a custom JIT backend before mature backends are measured;
- formal proof infrastructure without a concrete high-value theorem;
- a second source or IR authority;
- an abstraction named for a hypothetical future consumer.

## Repository Shape

Begin with one Rust package unless a boundary earns a split.

A crate boundary must provide at least one of:

- unsafe or FFI isolation;
- an independently useful stable API;
- a target-specific build boundary;
- a process binary with a narrow protocol;
- measured compile-time isolation;
- a dependency boundary that materially reduces builds.

Do not recreate the old many-crate topology by habit.

Modules split on semantic ownership.
No line-count, byte-count, directory-fanout, or depth rule defines validity.

Large coherent files are acceptable.
Tiny façade files are not automatically desirable.

The active documentation set should remain small:

- `docs/spec/semantic-graph.md`;
- `docs/spec/language.md`;
- `docs/spec/protocol.md`;
- `docs/architecture.md`;
- `docs/status.md`;
- `docs/performance.md`;
- `docs/roadmap.md`.

Add another maintained document only when one of those roles cannot own the fact.

## Implementation Language

Use stable Rust as the bootstrap implementation unless measured evidence shows a
specific blocker.

The reset does not require changing implementation language.

Prefer safe Rust.
Allow unsafe Rust only at explicit OS, FFI, executable-memory, or validated
zero-copy boundaries.

Do not begin self-hosting until the graph model, compiler, runtime, package model,
and daemon are mature enough that self-hosting reduces rather than multiplies
risk.

## Testing

Use focused tests for each invariant.

Required categories include:

- graph schema acceptance;
- graph schema rejection;
- unknown kind rejection;
- unknown field rejection;
- containment validation;
- scope validation;
- dominance validation;
- type validation;
- effect validation;
- ownership validation;
- hole behavior;
- incomplete snapshot queries;
- complete snapshot gate;
- stable IDs;
- no ID reuse;
- rename identity preservation;
- deletion behavior;
- old snapshot validity;
- transaction atomicity;
- allocator rollback;
- revision conflict;
- idempotent retry;
- deterministic diff;
- deterministic hash;
- deterministic artifact bytes;
- corrupt artifact rejection;
- durable commit failure;
- daemon restart;
- protocol framing;
- protocol unknown variant rejection;
- compact query pagination;
- direct graph compilation;
- source-free execution;
- Core IR verification;
- interpreter semantics;
- native pre-entry decline;
- no retry after native entry;
- capability denial;
- capability success;
- cancellation cleanup;
- resource cleanup;
- stack safety;
- fuzz regression.

Use property tests where many graph shapes share one invariant.
Use differential tests between generic and native execution.
Use mutation or malformed-input tests at trust boundaries.
Do not retain tests for deleted architecture.

## Verification

The exact command set belongs to the current repository and may evolve.

At minimum, before a completed change:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run focused tests before full verification.

When daemon integration exists, run the real daemon/client end-to-end test.

When artifact or protocol fuzz targets exist, run the bounded local smoke target
required by `docs/status.md`.

Never claim an unrun command.
Separate product failure from environment failure.

## Workflow

### Orient

1. Record branch, starting commit, and `git status --short`.
2. Read this file once.
3. Read only the current status and the relevant specification section.
4. Search exact symbols before opening large files.
5. Inspect recent relevant commits.
6. Preserve unrelated work.
7. Identify one dependency-closed milestone.

### Decide

State privately:

- the operation being improved;
- the current consumer;
- the authoritative representation;
- the invariants;
- the rejection path;
- the evidence;
- the non-goals;
- the stop condition.

Do not ask the user to choose internal details that tests and measurements can
decide.

### Implement

1. Change the authority.
2. Change its producer.
3. Change every active consumer.
4. Delete displaced code.
5. Add focused success evidence.
6. Add focused rejection evidence.
7. Run focused verification.
8. Update current documentation.
9. Run full verification once.
10. Inspect the final diff.

### Continue

A turn ends only at a buildable, testable boundary.

Do not leave:

- two semantic authorities;
- two artifact formats;
- two compiler paths for the same tier;
- a half-migrated protocol;
- disabled legacy code;
- an intentionally failing branch;
- an undocumented durable format change.

A handoff names exact paths, symbols, commands, observed failures, and the next
acceptance gate.
Keep it concise.

## Multi-Agent Use

The lead agent owns architecture, integration, and final verification.

Use subagents only for independent bounded questions or disjoint implementation
areas.

Give each subagent:

- one question;
- exact paths;
- required evidence;
- explicit non-goals;
- a stop condition;
- a compact output format.

Do not ask multiple agents for competing architectures after evidence selects one.
Do not let subagents independently create protocol, identity, or schema authority.

## Documentation

Specifications state accepted semantics.

Status states what the checkout currently implements.

Architecture states responsibilities and trust boundaries.

Performance states retained measurements and reversal conditions.

Roadmap states ordered milestones and their evidence gates.

README is a concise entry point.

Do not copy the same table across documents.
Do not maintain public-fact shards, digest registries, status capsules, platform
revision ledgers, or repository graph authority.

Generated protocol and schema documentation derives from code.
Do not hand-copy every enum into prose.

## Git

Inspect the worktree before edits.

Use explicit deletion.
Do not use `git reset --hard`, `git clean -fd`, or force push.

The user has authorized incompatible changes.
That does not authorize destruction of unrelated uncommitted work.

Prefer one cohesive commit per verified milestone when commits are permitted.

Do not push or open a pull request unless explicitly requested.

The active prompt is an execution artifact.
Do not treat old prompts as normative after this file and accepted specifications
own their decisions.

## Cost Discipline

Save coding-agent API cost through:

- compact repository structure;
- exact symbol search;
- small active documentation;
- one authority per fact;
- generated schemas;
- focused tests;
- deterministic errors;
- batch graph operations;
- compact query defaults;
- selective expansion;
- semantic diffs;
- deletion of stale code;
- no compatibility work;
- no repeated full-test runs;
- no repeated log dumps;
- no speculative alternatives after evidence selects one.

Do not save cost by weakening correctness, skipping validation, or omitting final
verification.

## Stop Conditions

Stop and narrow the change if it introduces:

- source text as a required authority;
- a second mutable graph;
- a second ID domain without necessity;
- persistent ID reuse;
- content hashes as mutable identity;
- arbitrary graph fields;
- arbitrary semantic count limits;
- a generic solver for local rules;
- a plugin dialect before a consumer;
- a database before measured need;
- a daemon subsystem unrelated to the current milestone;
- a cache without measured reuse;
- a JIT tier without a workload;
- a global tracing collector;
- ambient capabilities;
- compatibility layers;
- old-code preservation;
- broad documentation machinery;
- repository fragmentation;
- unverifiable performance claims.

Usually correct by deleting, narrowing, using one closed enum, using one stable ID
domain, keeping derived facts private, or postponing a subsystem until its consumer
exists.

## Completion Report

Use this shape:

```text
Starting commit:
Ending commit:
Milestone:
Authority changed:
Old architecture deleted:
Semantic graph result:
Identity result:
Transaction result:
Persistence result:
Daemon result:
Compiler result:
Runtime result:
Agent-interface result:
Performance evidence:
Focused tests:
Full verification:
Environment limitations:
Documentation:
Compatibility breaks:
Remaining gaps:
Next acceptance gate:
Worktree state:
```

Report observable evidence and decisions.
Do not report hidden chain-of-thought.
