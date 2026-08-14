# AGENTS.md

## Scope

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local implementation procedure.

A deeper file must not weaken repository-wide requirements for semantic authority,
identity, transaction atomicity, durability, determinism, safety, protocol strictness,
verification, evidence, or architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
documentation, benchmark labels, commit messages, and generated descriptions.

Preserve unrelated uncommitted work.

Never reset, clean, overwrite, or force-push work that you did not create.

Do not push, open a pull request, or change remote state unless the active user task
explicitly requests it.

## Operating Posture

`lkjscript` is past its initial architectural reset.

Do not perform another total rewrite merely because compatibility is unimportant.

The current semantic-graph daemon vertical is the baseline until concrete evidence
shows that a specific part should be replaced.

Backward compatibility is not a product requirement.

This permission exists to keep the active architecture singular and correct.

It is not permission to create churn without a dependency-closed replacement and
verification evidence.

When an incompatible replacement is cleaner:

- replace the old path directly;
- bump or replace the affected boundary version;
- delete the displaced implementation;
- delete tests that protect only displaced behavior;
- update the owning specification;
- retain no dual reader, dual writer, legacy mode, edition, compatibility namespace,
  adapter layer, or hidden fallback unless the active task explicitly requires one.

Git history is the archive for superseded implementation.

## Mission

Build `lkjscript` as an AI-primary programming system whose canonical program
representation is a closed, strongly typed, strongly constrained Semantic Program
Graph.

An external coding agent must be able to create, inspect, refine, validate, compile,
execute, package, debug, and maintain an `lkjscript` program without authoring,
round-tripping, or preserving source code.

The daemon is the one logical per-user control plane.

The daemon owns live workspaces, immutable snapshots, durable graph state, semantic
queries, compilation, runtime supervision, capability grants, resource policy,
caches when justified, and observability.

One logical daemon does not require one address space.

A worker process or runtime cell is permitted only when it enforces an actual
isolation, privilege, target, failure, or measured concurrency boundary.

Long-term runtime performance is a first-class objective.

Agent interaction cost is also a first-class objective.

Measure model-facing bytes, round trips, unnecessary expansions, failed edits,
rediscovery work, and elapsed time alongside compiler and runtime performance.

Never claim performance leadership without reproducible comparative evidence.

## Product Laws

The following laws are repository-wide defaults.

The active user may explicitly supersede them.

1. The Semantic Program Graph is the only mutable program authority.
2. Source text is not required to construct, revise, validate, compile, or run a
   program.
3. A textual, JSON, visual, or debug representation is a transport or projection,
   never a coequal program authority.
4. The compiler consumes an immutable semantic graph snapshot directly.
5. All persisted semantic nodes and relations obey a closed typed schema.
6. Arbitrary property bags are forbidden in canonical semantic state.
7. Arbitrary string-labelled graph edges are forbidden in canonical semantic state.
8. Untyped mutation is forbidden.
9. Every mutation is an ordered typed transaction.
10. Every successful committed transaction publishes exactly one immutable snapshot.
11. Every rejected transaction publishes nothing.
12. Every dry-run transaction publishes nothing.
13. Failed and dry-run transactions consume no persistent identities.
14. Stable node identity is independent of names, positions, hashes, compiler
    indexes, artifact offsets, and memory addresses.
15. Persistent node identities are never reused.
16. Names are presentation and lookup metadata, not identity.
17. Compiler indexes are private dense implementation details.
18. Content hashes are cache and integrity identities, not mutable semantic identity.
19. Derived types, bindings, effects, ownership facts, layouts, diagnostics,
    dependencies, and compiler IR are not mutable program authority.
20. Incomplete programs use explicit typed semantic nodes.
21. A published incomplete snapshot remains structurally valid and queryable.
22. A complete entry dependency closure is required before executable lowering.
23. The production compiler has one canonical executable IR path.
24. Execution tiers accelerate one semantic route rather than implement competing
    languages.
25. Optimization failure cannot redefine program validity.
26. The daemon is the sole live writer of durable workspace state.
27. Clients mutate workspaces through a typed daemon request.
28. Clients do not edit retained artifact bytes behind the daemon.
29. A `.lkjscript` file is a canonical semantic artifact, not source code.
30. AI output is an untrusted proposal.
31. Deterministic validators decide acceptance.
32. Host effects require explicit typed authority.
33. Ambient filesystem, network, process, clock, entropy, terminal, database, and
    device authority is forbidden.
34. No mandatory global stop-the-world tracing collector may be introduced.
35. User-controlled semantic depth must not consume unbounded native stack.
36. Operational quotas may protect a request, decoder, runtime, workspace, or host.
37. Operational quotas must not redefine which programs are semantically valid.
38. Arbitrary semantic node-count, depth, file-count, or repository-topology limits
    are forbidden.
39. Observable order is explicit and deterministic.
40. Hash-table, allocator, thread, and filesystem enumeration order are not
    observable language semantics.
41. Public mutation responses are bounded by explicit response projections.
42. Optional unbounded detail belongs in revision-bound paginated queries.
43. A daemon must preflight the exact committed response before durable publication.
44. A successful commit must never become unreportable merely because a default
    response included an unbounded allocation map or semantic diff.
45. Idempotent retry returns the same accepted result or a structured conflict.
46. Durable state is acknowledged only after the authoritative commit is durable.
47. Corrupt or ambiguous durable state is rejected rather than heuristically repaired.
48. Protocol and artifact decoders reject unknown variants and trailing data.
49. The repository retains one active implementation for each product path.
50. The active repository remains small enough for an agent to search and understand.

## Authority Order

Use the following authority order.

1. The active user task.
2. This root `AGENTS.md`.
3. The active prompt named by the current user task.
4. Accepted normative files under `docs/spec/`.
5. Executable code and focused invariant tests.
6. Boundary schemas and generated machine descriptions derived from code.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, old prompts, old commits, issue prose, and historical documents.

A newer active prompt supersedes older prompts for campaign sequencing.

A prompt does not silently supersede this file or an accepted specification.

When a campaign deliberately changes semantics, update the owning specification in
the same verified milestone.

Prompts are execution artifacts.

Do not copy prompt prose into permanent documentation unless the fact has become an
accepted product contract.

When two active artifacts disagree, identify the fact owner and repair that owner.

Do not create a third authority to reconcile the disagreement.

## Current Architecture

The current product path is:

```text
strict client request
    -> local typed IPC
    -> daemon-owned workspace
    -> staged typed transaction
    -> full deterministic graph validation
    -> durable immutable snapshot publication
    -> revision-bound semantic query
    -> direct Semantic Program Graph lowering
    -> private verified Core IR
    -> interpreter
    -> typed result
```

Keep this path singular.

The current implementation deliberately uses one Rust package, one synchronous
daemon, immutable full snapshots, canonical full revision artifacts, full semantic
recomputation, and one interpreter route.

These are measured bootstrap baselines.

They are not permanent promises.

Do not replace them with a database, async runtime, query framework, cache graph,
custom allocator, native backend, or process topology before a current consumer and
measurement justify the replacement.

## Canonical Data Classes

Classify every maintained datum as exactly one of the following.

### Semantic State

Semantic state determines program meaning.

Examples include:

- workspace identity;
- stable node identity;
- node kind;
- typed attributes;
- typed containment;
- ordered body position where order is semantic;
- direct semantic references;
- operation payloads;
- explicit holes;
- selected package entry;
- persistent identity allocator state;
- tombstones;
- revision.

Semantic state belongs in immutable snapshots and canonical artifacts.

### Derived State

Derived state is recomputable from semantic state.

Examples include:

- type facts;
- binding facts;
- completeness blockers;
- incoming uses;
- dependency closure;
- legal constructors;
- diagnostics;
- semantic diffs;
- layouts;
- ownership facts;
- effect facts;
- Core IR;
- machine code;
- profiles;
- query indexes.

Derived state may be cached.

Derived state never becomes a second mutable source of program truth.

### Executable State

Executable state includes verified Core IR, interpreter values, runtime cells,
machine code, runtime handles, and active capability instances.

Executable state is not persisted in a semantic artifact unless a separate,
invalidatable cache format explicitly owns it.

### Presentation and Transport

Presentation and transport include names, compact JSON, CLI text, diagrams, debug
views, schema descriptions, request IDs, session aliases, and pagination cursors.

Presentation and transport may be lossy or lossless.

They never become semantic identity.

## Semantic Graph Schema

The canonical graph is not a generic property graph.

Each node has one closed `NodeKind`.

Each node kind has explicit:

- attributes;
- owner rules;
- child slots;
- child ordering;
- reference slots;
- operand slots;
- result slots;
- cardinality;
- allowed target kinds;
- completeness rules;
- local validation rules;
- history-continuity rules.

Unknown kinds, attributes, slots, operation codes, and value forms reject.

Do not preserve unknown semantic fields for hypothetical forward compatibility.

Language evolution replaces the closed schema deliberately.

Do not add runtime dialect registration, plugin-defined semantic nodes, arbitrary
schema extension, or open operation registries without multiple real consumers and
an explicit trust model.

One code-owned descriptor should expose stable schema facts to validators, legal
constructor queries, machine descriptions, and codecs where practical.

Do not hand-maintain a second JSON schema that can drift from executable contracts.

Avoid abstraction machinery that is larger than the closed vocabulary it describes.

A closed enum plus static descriptors and exhaustive matches is usually preferred.

## Operation Contracts

Each operation constructor has one authoritative contract.

The contract defines:

- stable operation code;
- stable boundary tag;
- operand count;
- operand types or type rules;
- operand use modes;
- result count;
- result types or type rules;
- terminator status;
- completeness status;
- literal fields;
- nested region slots when applicable;
- capability requirements when applicable;
- effect classification when applicable.

Validators, queries, lowering, interpreters, codecs, and machine descriptions derive
from that contract.

Do not duplicate operation truth in unrelated match tables when an exhaustive helper
can own it.

Avoid heap allocation for static operation contract lookup.

Avoid allocating temporary operand vectors in graph-wide scans when a direct indexed
or iterator-style API is sufficient.

Do not build a generic visitor framework for one traversal.

## Identity

A workspace has one persistent node-ID domain.

A node ID contains or is unambiguously scoped by workspace identity.

Persistent allocation is monotonic.

Persistent IDs are staged during transaction preparation.

A rejected or dry-run transaction leaves the published allocation frontier unchanged.

Deletion tombstones identity.

Old retained snapshots preserve deleted nodes.

A later node never receives a tombstoned identity.

Rename preserves identity.

Scalar payload update preserves identity when the operation constructor and identity
contract remain the same.

A move preserves identity only when the typed operation explicitly defines semantic
continuity.

A replacement creates new identity unless a specific semantic refinement rule says
otherwise.

Builtins use an explicit builtin identity domain.

Do not encode builtins as workspace nodes selected by magic names.

## Typed Holes and Refinement

Typed holes are first-class semantic nodes.

A hole records an exact expected semantic contract.

A hole may be inspected, referenced, retained in old snapshots, and reported as a
completeness blocker.

A hole cannot enter executable lowering.

Filling a hole is a semantic refinement, not a generic operation mutation.

The default refinement rule is one-way:

```text
Hole(expected result contract)
    -> complete non-terminator operation with the same result contract
```

A successful refinement preserves the hole node ID, owner, body position, and
existing uses of its result.

A refinement may reference existing values and values created in the same
transaction.

Final graph validation still enforces scope, order, dominance, type, and ownership.

Refinement to another hole rejects.

Refinement to a terminator rejects.

Refinement with a different result contract rejects.

Refinement of an already complete operation rejects unless another explicit edit
operation owns that behavior.

Reverse refinement from a complete operation to a hole is not implicit.

History validation must recognize only the explicit allowed refinement transition.

Do not broaden identity-preserving constructor changes into a generic morph operation
without a concrete use case and exact continuity semantics.

## Transactions

Every mutation request names:

- workspace identity;
- base revision;
- commit or validate-only mode;
- optional idempotency key for committed requests;
- an ordered batch of typed operations;
- transaction-local handles for created nodes;
- an explicit bounded response projection.

A transaction stages:

- graph changes;
- identifier allocation;
- tombstones;
- validation results;
- deterministic semantic change facts;
- canonical artifact bytes;
- durable HEAD bytes;
- publication metadata;
- exact response bytes.

A successful commit performs this order:

1. validate workspace and base revision;
2. allocate local handles in staged state;
3. apply typed operations to staged state;
4. validate the final staged graph;
5. derive deterministic change summary and digest;
6. construct the bounded transaction receipt;
7. encode and preflight the exact response;
8. encode and preflight durable state;
9. durably publish the new revision and HEAD;
10. publish the in-memory snapshot;
11. return the preflighted receipt.

A rejected request changes none of those states.

Do not partially publish a batch.

Do not consume IDs on rejection.

Do not heuristically repair invalid requests.

Return structured rejection facts.

Validate-only mode uses the same semantic preparation path as commit mode.

Validate-only mode reports the predicted revision, hash, allocations requested by
the response projection, and deterministic change summary without publication.

An idempotency key is for a committed request.

Reject or precisely define idempotency on validate-only requests rather than leaving
ambiguous behavior.

## Transaction Receipts

The default transaction response is compact.

A receipt should contain only bounded facts such as:

- workspace;
- base revision;
- resulting or predicted revision;
- snapshot hash;
- publication status;
- total created-node count;
- explicitly requested local-handle bindings;
- total semantic-change count;
- deterministic semantic-change digest;
- completeness transition or blocker-count summary when useful.

Do not return the complete semantic diff by default.

Do not return every local-handle allocation by default.

The request may select a bounded set of handles whose stable IDs are required by the
client.

Reject duplicate or nonexistent requested handles before publication.

The complete diff belongs in a revision-bound paginated query.

Optional response detail must not cause a valid default mutation to exceed the IPC
frame.

Persistent idempotency metadata stores the compact replayable receipt, not an
unbounded full diff.

## Semantic Diffs

A semantic diff is derived from two immutable snapshots.

Diff order is deterministic.

A transaction computes enough of the diff to produce its count and digest.

The full change list is queried separately by exact revisions.

A diff query is paginated.

A diff cursor is bound to:

- workspace;
- from revision;
- to revision;
- query kind;
- deterministic next position.

A cursor is transport state, not persistent semantic identity.

Do not persist a full diff merely to serve pagination while both snapshots remain
available.

## Queries

Queries are pure over one immutable revision unless their request explicitly names
two revisions.

Correct full recomputation is the oracle.

Do not add a query cache until repeated cost is measured and invalidation is exact.

The agent-facing query surface should include compact typed forms for:

- workspace summary;
- node summary;
- exact node record when explicitly requested;
- owner chain;
- body slice;
- outgoing references and dependencies;
- incoming value uses;
- incoming definition references;
- completeness blockers;
- semantic diff;
- legal constructors;
- repair context;
- compile explanation;
- runtime trace when implemented.

Compact summaries are the default.

Exact expansion is opt-in.

Rename ambiguous fields.

For example, distinguish outgoing reference count from incoming use count and
completeness blocker count from general diagnostic count.

Every query result has deterministic ordering.

Every collection query has explicit pagination.

Every page reports whether more results exist and the exact next cursor when one
exists.

Page limits and batch limits are operational policy.

They do not limit semantic program size.

## Query Batches

Independent read queries may be submitted in one revision-bound batch.

All queries in the batch observe the same immutable snapshot.

Each query carries a client query ID or retains exact request order.

Read-only batch items may return independent structured outcomes.

One invalid read item need not discard unrelated valid read results.

Do not mix mutation and query execution in one atomic request.

Dependent mutations belong in one transaction.

A query batch has a bounded query count and bounded aggregate result budget.

Compact item structures and page policies must make legal responses encodable.

Add tests that exercise the maximum legal batch shape.

## Incoming Uses and Dependencies

Do not collapse all references to bare target node IDs.

A reference fact identifies:

- source node;
- target node or semantic value;
- relation kind;
- slot or operand index;
- output index when the target is an operation result;
- owning function or block when useful for compact context.

Containment and semantic reference are distinct.

A value use targets a semantic value, not merely its producer node.

The first implementation may scan the full snapshot.

Do not add a reverse-reference index until representative queries show repeated cost.

When an index is later added, full scanning remains the correctness oracle.

## Body Slices

Agents should not need a complete function dump to inspect one operation.

A body-slice query is revision-bound and block-specific or otherwise structurally
unambiguous.

It returns compact operation summaries in semantic order.

Each summary includes only currently useful facts, such as:

- stable operation node ID;
- ordinal within the block;
- operation code;
- result types;
- operands;
- completeness;
- terminator status where applicable.

A body cursor may use a deterministic ordinal because the queried revision is
immutable.

Do not expose private Core IR indexes as semantic body positions.

## Legal Constructors

A legal-constructor query returns exact schema-derived operations that can satisfy a
hole or operand contract.

It is not a language-model suggestion service.

It does not rank by learned likelihood.

For each constructor, return structured facts such as:

- stable operation code;
- result contract;
- operand contracts;
- operand use modes;
- required literal fields;
- completeness;
- terminator status;
- capability requirements when applicable.

Filter constructors by the exact target contract.

Return visible candidate values separately and deterministically.

Visibility derives from function scope, block order, dominance, and future ownership
rules.

Do not persist candidate lists.

## Repair Context

A repair-context query is a deterministic composition of semantic facts for one
explicit target.

Initial repair targets are:

- a typed expression hole;
- a specific operation operand.

A repair context should contain enough information for a simple repair without a
whole-workspace dump.

Useful fields include:

- target identity;
- exact expected type and use mode;
- current value when applicable;
- owner chain;
- function signature;
- block and ordinal;
- bounded surrounding body slice;
- bounded visible values;
- bounded incoming uses;
- legal constructors;
- relevant completeness blockers.

Selection is structural and deterministic.

Do not use an LLM inside the daemon to choose correctness-critical context.

Do not include arbitrary natural-language summaries in the canonical response.

A presentation layer may attach prose after the typed facts are produced.

## Protocol

The daemon protocol is closed, typed, and explicitly versioned.

One code definition owns each request and response variant.

Stable tags belong near the enum or contract they identify.

Do not scatter raw numeric tag truth across codecs.

Unknown versions, variants, fields, tags, and trailing bytes reject.

Every length, count, index, ID, revision, hash, cursor, and discriminant is checked.

The protocol uses bounded frames.

Boundary limits are centralized and tested.

A protocol replacement is direct.

Do not retain a legacy decoder when compatibility is not requested.

The daemon may use a compact binary local IPC format.

The binary format is transport only.

It is not a semantic artifact.

## Machine-Facing CLI Projection

A generic strict machine-facing CLI projection is permitted and expected when it
materially improves external agent access.

A JSON request is a typed transport projection.

It is not source code and is never persisted as program authority.

The generic CLI should support the complete public request vocabulary rather than a
set of demo-only commands.

Machine mode defaults to compact output.

Pretty output is opt-in presentation.

Machine stdout contains exactly one structured response.

Process diagnostics belong on stderr.

Unknown JSON fields and variants reject.

Trailing JSON data rejects.

Input bytes and nesting are bounded at the CLI boundary.

Do not accept arbitrary `JSON Value` property bags inside semantic requests.

Use closed DTOs or direct closed protocol types with exhaustive conversion.

IDs have one canonical machine representation.

Enum names use stable lowercase machine names rather than Rust debug output.

Every JSON variant has round-trip and rejection tests.

A machine schema or descriptor command derives from executable contract metadata.

Do not add a large schema-generation dependency when a small code-owned descriptor
is sufficient.

Record the build-time and dependency cost of any new serialization dependency.

## Persistence

Each retained revision is immutable.

HEAD identifies the committed revision and hash plus bounded non-semantic commit
metadata.

HEAD metadata must remain small and independently bounded.

Do not store an unbounded transaction diff or full allocation map in HEAD.

Persistent format changes use a new unambiguous magic, version, or schema identity.

When compatibility is not required, reject the old format rather than implementing a
migration reader.

Canonical artifacts have:

- fixed magic;
- explicit format version;
- explicit semantic schema identity;
- canonical ordering;
- fixed integer endianness;
- checked counts and lengths;
- corruption detection;
- deterministic content hash;
- strict trailing-byte policy;
- bounded defensive decoding;
- no Rust memory-layout dependency;
- no pointer values;
- no private compiler IDs;
- no mutable cache truth.

A durable commit is acknowledged only after the authoritative directory and HEAD
transitions satisfy the documented crash contract.

Failure injection covers each publication step.

If commit outcome becomes genuinely unknowable, stop the daemon rather than
continuing with ambiguous authority.

Full snapshot rewrite remains acceptable until measured workloads justify a journal,
chunk store, or database.

Full retained history remains acceptable until measured storage or startup cost
justifies pruning.

Do not optimize persistence preemptively.

## Idempotency

Idempotency is exact request replay protection.

A key is bound to a deterministic request fingerprint and compact receipt.

An exact retry returns the retained receipt without reapplying mutation.

Reuse of the key for different request semantics or response projection rejects.

An unseen request still obeys current base-revision rules.

Persisted idempotency data is validated against retained snapshots on restart.

If future history pruning removes a required revision, remove or replace the affected
idempotency record atomically.

Do not retain arbitrary request bodies merely to implement replay.

## Compiler and Core IR

The compiler accepts:

- an immutable complete snapshot;
- one entry definition;
- target information when relevant;
- explicit capability grants when relevant;
- explicit optimization policy when relevant.

The canonical route is:

```text
Semantic Program Graph snapshot
    -> completeness and semantic validation
    -> compact dense Core IR
    -> Core IR verification
    -> interpreter or native lowering
```

Core IR is private derived state.

Core IR is not serialized into semantic artifacts.

Core IR uses dense locality-oriented IDs that never escape as public semantic
identity.

Every lowering is deterministic.

Every verifier rejects malformed IR before execution.

The interpreter remains the complete semantic oracle during bootstrap.

A future native tier must be differential-tested against it.

Do not add calls, branches, loops, aggregates, generics, effects, ownership, native
code, or caches in a campaign whose acceptance gate is agent repair ergonomics unless
the active prompt explicitly includes and orders that work.

## Runtime

Runtime traps are structured.

Recoverable domain failure uses explicit result types when the language supports
them.

Checked arithmetic follows the accepted language specification.

Do not catch an optimization or native failure by silently re-executing effects in a
second engine after execution begins.

A native tier may decline before entry and fall back to the unchanged verified Core
IR route.

Runtime values use static layouts where semantics permit.

Avoid universal boxing without measured need.

## Effects, Capabilities, and Ownership

Effects are deterministic derived facts.

Capabilities are explicit typed values or explicit entry grants.

No ambient host access is permitted.

A host operation contract owns:

- capability kind;
- operands;
- results;
- effects;
- ownership modes;
- failure behavior;
- cancellation behavior;
- blocking behavior;
- resource accounting hooks.

Do not duplicate host-operation truth across graph, compiler, daemon, and runtime.

The long-term memory direction is:

- value semantics for ordinary immutable data;
- affine semantics for external resources and mutable identity;
- compiler-derived moves, borrows, storage, and cleanup;
- no mandatory global tracing collector;
- explicit isolated managed regions only when their use case and latency are defined.

Do not let the scalar bootstrap accidentally define the final heap model.

## Security and Trust Boundaries

Treat the following as untrusted boundaries:

- binary IPC bytes;
- JSON CLI bytes;
- semantic artifact bytes;
- durable directory contents;
- host-operation inputs;
- FFI values;
- native code and relocation metadata;
- capability references;
- pagination cursors.

Use filesystem permissions and OS peer identity for local daemon access.

Do not claim sandboxing that is not implemented.

Prefer a supervised process when native or foreign execution cannot be safely
contained in the daemon address space.

Fuzz or mutation-test:

- artifact decoding;
- protocol decoding;
- JSON projection decoding;
- graph transaction validation;
- Core IR verification;
- capability dispatch when introduced;
- native lowering boundaries when introduced.

A malformed input may reject.

It must not panic, corrupt state, allocate without bound, hang, or publish a partial
mutation.

## Resource Policy

A resource policy protects one operational boundary.

Each policy error names the boundary it protects.

Examples include:

- maximum IPC frame bytes;
- maximum CLI input bytes;
- maximum page size;
- maximum batch query count;
- maximum aggregate query items;
- maximum artifact decoder bytes;
- maximum runtime fuel or deadline;
- maximum retained response bindings.

Policy constants belong near the boundary and have focused tests.

Do not scatter magic limits.

Do not describe an operational limit as a language maximum.

## Determinism

The following must be deterministic for the same semantic input and build contract:

- accepted or rejected transaction result;
- assigned persistent IDs;
- semantic diff order;
- change digest;
- query order;
- legal constructor order;
- visible value order;
- context-pack selection;
- artifact bytes;
- snapshot hash;
- Core IR lowering;
- interpreter result;
- machine JSON field and collection order where the format exposes order.

Use ordered collections or explicit sorting at observable boundaries.

Add determinism tests that vary insertion order where possible.

## Performance

Measure before replacing a bootstrap baseline.

Relevant measurements include:

- fresh debug and release build time;
- full verification time;
- dependency count;
- daemon cold start;
- daemon restart with retained workspaces;
- workspace creation;
- transaction preparation;
- durable commit;
- query latency;
- query response bytes;
- request count;
- repair-context latency and bytes;
- semantic diff latency;
- artifact size;
- per-workspace memory;
- Core IR lowering;
- interpreter startup and throughput;
- native compile and execution when introduced.

Record hardware, OS, toolchain, commit, build mode, warmup, samples, input, output
oracle, median, tails, and memory when applicable.

Do not tune one toy benchmark by narrowing valid semantics.

Keep microbenchmarks labelled as microbenchmarks.

Do not claim model-token savings from byte counts alone.

Record actual model tokens only when real telemetry exists.

Byte count, round trips, and elapsed time remain valid direct evidence.

## Agent Cost Discipline

Treat the following as product resources:

- request bytes;
- response bytes;
- daemon round trips;
- CLI invocations;
- repository files opened;
- repeated full scans;
- failed mutations;
- rediscovery after restart;
- model context;
- coding-agent API calls.

Reduce these costs through exact semantics.

Preferred mechanisms include:

- compact default receipts;
- selected handle bindings;
- batch reads;
- aggregate typed writes;
- stable IDs;
- repair contexts;
- legal constructors;
- semantic diffs;
- deterministic errors;
- revision-bound pagination;
- one fact owner;
- small active documentation;
- generated contract descriptions;
- focused tests before one full verification run.

Do not reduce cost by weakening validation, omitting rejection tests, hiding failures,
or skipping final verification.

Do not add model routing, prompt registries, token billing, or agent orchestration to
the language daemon merely to claim AI integration.

## Architecture Restraint

Before adding an abstraction, answer:

1. Which current producer creates it?
2. Which current consumer needs it?
3. Which invalid state does it remove?
4. Which repeated work does it remove?
5. Which test or measurement proves the need?
6. Could a closed enum or local helper suffice?
7. Does it create a second authority?
8. Does it create another identity domain?
9. Does it require serialization?
10. Does it require a process boundary?
11. Does it increase agent search cost?
12. What evidence would cause its deletion?

Prefer, in order:

1. delete dead code;
2. use a closed enum;
3. use one authoritative struct;
4. use a local function;
5. use a static descriptor;
6. use a sorted vector;
7. use an explicit work stack;
8. use one narrow measured index;
9. use one measured cache;
10. add a process for a real boundary;
11. add a general framework only after multiple real consumers exist.

Do not add without evidence:

- a generic property graph;
- an open dialect system;
- a general constraint solver;
- a visitor framework for one traversal;
- a registry for a closed vocabulary;
- a serializer for same-build private values;
- a database before durable rewrite cost requires one;
- an async runtime before concurrency requires one;
- a scheduler before multiple runnable cells exist;
- a cache before reuse is measured;
- a reverse-reference index before scans are measured;
- a custom JIT backend before mature backends are measured;
- formal proof infrastructure without a concrete high-value theorem;
- a second source or IR authority;
- an abstraction named only for a hypothetical future consumer.

## Dependencies

Every dependency has a current named consumer.

Record why it is preferable to a small local implementation.

Consider:

- transitive dependency count;
- fresh build cost;
- binary size;
- maintenance activity;
- trust boundary;
- unsafe code;
- platform impact;
- feature configuration;
- license.

A standard machine JSON projection may justify `serde` and `serde_json` if the
measured agent-interface value exceeds their build and dependency cost.

Do not add a schema framework, parser generator, async runtime, database, or compiler
backend as collateral to a smaller milestone.

Use exact versions through `Cargo.lock`.

Do not change dependencies unrelated to the active milestone.

## Repository Shape

Use one Rust package until a boundary earns a crate split.

A crate boundary must provide at least one of:

- unsafe or FFI isolation;
- independently useful stable API;
- target-specific build isolation;
- process binary with a narrow protocol;
- measured compile-time isolation;
- materially useful dependency isolation.

Do not recreate a many-crate compiler topology by habit.

Modules split on semantic ownership.

No line-count, byte-count, directory-fanout, or nesting rule defines repository
validity.

Large coherent files are acceptable.

Tiny facade files are not automatically desirable.

Delete superseded code in the same milestone.

Do not leave `old`, `legacy`, `compat`, `v1`, or disabled duplicate paths.

## Documentation

Keep the maintained documentation set small.

The default roles are:

- `docs/spec/semantic-graph.md` owns canonical graph, identity, transaction, and
  artifact semantics;
- `docs/spec/language.md` owns accepted language types, operations, and execution
  semantics;
- `docs/spec/protocol.md` owns public daemon and machine transport behavior;
- `docs/architecture.md` owns component responsibility and trust boundaries;
- `docs/status.md` owns exact implemented state and limitations;
- `docs/performance.md` owns retained measurements and reversal conditions;
- `docs/roadmap.md` owns ordered evidence gates;
- `README.md` is a concise entry point.

Add another maintained document only when none of these roles can own the fact.

Specifications state accepted semantics.

Status states what the current checkout implements.

Architecture states ownership and boundaries.

Performance states measurements.

Roadmap states future gates.

Do not copy the same catalogue across documents.

Do not maintain digest registries, status shards, public-fact ledgers, or prompt
archives as semantic authority.

Generated machine descriptions derive from code and may be emitted on demand.

Do not commit generated copies unless an external consumer requires a retained file.

## Testing

Use focused tests for each invariant.

Required categories include, as applicable:

- schema acceptance;
- schema rejection;
- unknown tag rejection;
- unknown JSON field rejection;
- containment validation;
- reference-slot validation;
- scope and dominance validation;
- type validation;
- hole completeness;
- stable hole refinement;
- invalid refinement rejection;
- old snapshot preservation after refinement;
- stable identity;
- no identity reuse;
- allocator rollback;
- rename continuity;
- deletion blocking and tombstones;
- stale revision rejection;
- idempotent retry;
- idempotency conflict;
- compact receipt bounds;
- selected allocation binding;
- transaction response preflight;
- deterministic diff count and digest;
- diff pagination;
- query pagination;
- cursor validation;
- batch-query ordering;
- independent query errors;
- incoming-use precision;
- body-slice order;
- legal-constructor correctness;
- repair-context sufficiency;
- JSON round trip;
- JSON unknown and trailing input rejection;
- artifact determinism;
- artifact corruption rejection;
- durable failure atomicity;
- daemon restart;
- protocol framing;
- real client and daemon execution;
- direct graph compilation;
- Core IR verification;
- interpreter semantics;
- stack safety;
- deterministic mutation or fuzz smoke.

Use model-based or generated transaction sequences where many operation orders share
one invariant.

Retain a failing seed or minimized byte corpus when a generated test finds a defect.

Use the real daemon and generic machine client for the principal end-to-end vertical.

Do not satisfy the acceptance gate only through private Rust APIs.

## Verification

Run focused checks during implementation.

Run the full boundary once after the milestone is coherent.

The current minimum full verification is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run the real daemon/client integration test when protocol, persistence, query,
compiler, runtime, or CLI code changes.

Run the bounded deterministic fuzz or mutation smoke command required by
`docs/status.md` when a trust boundary changes.

Never claim an unrun command.

Distinguish product failure from environment failure.

Record exact failed commands and relevant output without dumping unrelated logs.

## Workflow

### Orient

1. Record branch, starting commit, and `git status --short`.
2. Read this file once.
3. Read `docs/status.md`.
4. Read only the specification section relevant to the active milestone.
5. Search exact symbols before opening large files.
6. Inspect recent relevant commits.
7. Preserve unrelated work.
8. State one dependency-closed acceptance gate.

### Decide

Before editing, identify:

- the user-visible operation being improved;
- the semantic authority;
- the current producer;
- every active consumer;
- invariants;
- rejection behavior;
- durability behavior;
- response bound;
- evidence;
- non-goals;
- stop condition.

Do not ask the user to choose internal details that tests, measurements, or coherent
semantics can decide.

### Implement

1. Change the authoritative type or contract.
2. Change every active producer.
3. Change every active consumer.
4. Delete displaced code.
5. Add success evidence.
6. Add rejection evidence.
7. Add boundary and restart evidence when applicable.
8. Run focused checks.
9. Update owning specifications and status.
10. Run the full verification boundary once.
11. Inspect the final diff and worktree.

### End a Turn

End only at a buildable, testable boundary.

Do not leave:

- two semantic authorities;
- two active protocol versions;
- two HEAD formats;
- two compiler paths for one tier;
- a half-migrated request vocabulary;
- disabled legacy code;
- an intentionally failing branch;
- an undocumented durable format change;
- a committed response that was not preflighted;
- a prompt-only design with no product progress when implementation was requested.

A handoff names exact paths, symbols, commands, observed failures, and the next
acceptance gate.

Keep the handoff concise.

## Multi-Agent Use

The lead agent owns architecture, integration, and final verification.

Use subagents for independent bounded questions or disjoint implementation areas.

Give each subagent:

- one exact question;
- exact paths;
- required evidence;
- explicit non-goals;
- a stop condition;
- a compact output format.

Do not ask multiple subagents to invent competing architectures after evidence has
selected one.

Do not let a subagent independently create protocol, schema, identity, or persistence
authority.

Review subagent output against the actual checkout before integrating it.

## Git

Inspect the worktree before edits.

Use explicit targeted deletion.

Do not use `git reset --hard`.

Do not use `git clean -fd`.

Do not force push.

Incompatible change permission does not authorize destruction of unrelated work.

Prefer one cohesive commit per verified milestone when commits are permitted.

Do not create empty planning commits.

Do not push or open a pull request unless explicitly requested.

## Stop Conditions

Stop and narrow the change if it introduces:

- source text as required program authority;
- a second mutable semantic graph;
- arbitrary semantic properties or edge labels;
- persistent ID reuse;
- hashes as mutable identity;
- generic identity-preserving operation morphing without exact semantics;
- unbounded default transaction responses;
- full diffs in durable HEAD metadata;
- unknown-field preservation in semantic requests;
- unpaginated graph-wide query results;
- a cache without measured reuse;
- a reverse index without measured scan cost;
- a database without measured persistence pressure;
- async concurrency without concurrent workload pressure;
- a native tier without representative language workloads;
- a global tracing collector;
- ambient host authority;
- a compatibility bridge;
- a generic framework with one consumer;
- broad documentation machinery;
- repository fragmentation;
- unverifiable performance claims;
- model-token claims without model telemetry.

Usually correct by deleting, narrowing, using one closed enum, using one explicit
transaction, returning a compact receipt, moving detail to a paginated query, or
postponing the subsystem until its consumer exists.

## Completion Report

Use this shape for substantial milestones:

```text
Starting commit:
Ending commit:
Milestone:
Authority changed:
Compatibility breaks:
Schema result:
Identity result:
Hole-refinement result:
Transaction result:
Receipt-bound result:
Query result:
Machine-interface result:
Persistence result:
Compiler result:
Runtime result:
Agent-cost evidence:
Performance evidence:
Focused tests:
Generated or fuzz evidence:
Full verification:
Environment limitations:
Documentation:
Deleted code:
Remaining gaps:
Next acceptance gate:
Worktree state:
```

Report observable evidence and decisions.

Do not report hidden chain-of-thought.
