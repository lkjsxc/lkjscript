# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow local implementation procedure, but it must not weaken
repository-wide requirements for semantic authority, identity, atomicity, durability,
determinism, safety, protocol strictness, verification, evidence, human-facing honesty, or
architectural restraint.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
documentation, benchmark labels, generated descriptions, commit messages, and handoff reports.

Preserve unrelated work. Never reset, clean, overwrite, or force-push work that you did not
create.

Do not push, open a pull request, merge, publish a release, or otherwise change remote state
unless the active user task explicitly requests it.

## Mission

Build `lkjscript` as an AI-primary programming system in which autonomous software agents,
rather than human source-code authors, construct and maintain programs.

Humans remain first-class users of the product at the level of intent, explanation,
governance, review, and operation. They are not expected to hand-author the canonical program
representation.

The canonical program representation is a closed, strongly typed, strongly constrained
Semantic Program Graph owned by one logical daemon.

An external coding agent must be able to discover the active schema, create a program, inspect
exact context, apply bounded typed changes, validate without publication, commit atomically,
query retained revisions, compile, execute, debug, and continue work after restart without
preserving or round-tripping source text.

Long-term runtime performance is a first-class objective.

Agent interaction cost is also a first-class objective.

Measure model-facing bytes, machine round trips, failed proposals, redundant expansion,
repeated discovery, repository search work, build latency, verification latency, and elapsed
task time alongside compiler and runtime performance.

Never claim runtime, build, token, cost, reliability, or usability leadership without
reproducible comparative evidence.

## Audience and Product Surfaces

The repository serves distinct audiences through distinct surfaces.

- `README.md` is a human-first product introduction.
- `docs/spec/` owns accepted normative semantics.
- `docs/architecture.md` explains current component responsibility and trust boundaries.
- `docs/status.md` states exactly what the current checkout implements and does not implement.
- `docs/performance.md` retains measurements, regressions, and reversal conditions.
- `docs/roadmap.md` states ordered evidence gates rather than promises.
- The generic machine interface and runtime-generated schema are the primary agent-facing
  product surfaces.
- This `AGENTS.md` governs repository work and is not product marketing or a user tutorial.

Do not collapse these audiences into one document.

In particular, do not turn the README into a test runner cheat sheet, a raw protocol dump, an
internal architecture memo, or a coding-agent operating contract.

## Operating Posture

The current source-free semantic-graph daemon vertical is the audited baseline, not an
untouchable monument.

The active user explicitly permits incompatible and sweeping change.

Backward compatibility is not a product requirement.

Use that freedom to keep one coherent architecture, not to create churn.

Do not preserve an old API, artifact, protocol, CLI command, schema, test, abstraction, or
documentation structure merely because it exists.

Do not perform another total rewrite merely because compatibility is unimportant.

Replace a subsystem when the replacement is dependency-closed, materially clearer or more
capable, and verified against the product laws.

When an incompatible replacement is selected:

- replace the active boundary directly;
- bump or replace the affected version or magic when the old bytes would otherwise be
  ambiguous;
- delete the displaced reader, writer, adapter, implementation, and tests that protect only
  displaced behavior;
- update the owning specification and status in the same milestone;
- retain no legacy mode, compatibility namespace, edition, dual reader, dual writer, hidden
  fallback, or silent migration path unless the active user explicitly requires one.

Git history is the archive for superseded implementation.

Newer active user instructions and newer verified repository state take precedence over older
prompts and assumptions.

## Product Laws

1. The Semantic Program Graph is the only mutable program authority.
2. Source text is not required to construct, inspect, revise, validate, compile, execute,
   package, or debug a program.
3. A textual, JSON, visual, diagnostic, or debug representation is a transport or projection,
   never a coequal program authority.
4. A structured authoring draft may be accepted as a typed transaction proposal, but it is
   never persisted as a second program representation.
5. The daemon is the sole live writer of durable workspace state.
6. One logical daemon authority does not require one address space.
7. A worker process or runtime cell is justified only by an actual isolation, privilege,
   target, failure, or measured concurrency boundary.
8. Every persisted semantic node and relation belongs to a closed typed schema.
9. Arbitrary property bags are forbidden in canonical semantic state.
10. Arbitrary string-labelled semantic edges are forbidden in canonical semantic state.
11. Runtime dialect registration and plugin-defined canonical node kinds are forbidden without
    multiple current consumers and an explicit trust model.
12. Unknown semantic kinds, fields, slots, tags, operations, and value forms reject.
13. Untyped mutation is forbidden.
14. Every mutation is an ordered typed transaction or an exact structured proposal
    deterministically expanded into one ordered typed transaction.
15. Every successful committed transaction publishes exactly one immutable snapshot.
16. Every rejected transaction publishes nothing.
17. Every validate-only transaction publishes nothing.
18. Rejected and validate-only transactions consume no persistent identities.
19. Stable semantic identity is independent of names, source positions, body ordinals, content
    hashes, dense compiler indexes, artifact offsets, and memory addresses.
20. Persistent node identities are never reused.
21. Deletion tombstones identity.
22. Retained historical snapshots preserve historical identity and meaning.
23. Names are presentation and lookup metadata, not identity.
24. Content hashes are cache and integrity identities, not mutable semantic identity.
25. Dense compiler IDs are private derived implementation details.
26. Types, bindings, effects, ownership facts, layouts, diagnostics, dependencies, query
    indexes, Core IR, and machine code are derived state unless the accepted specification
    explicitly says otherwise.
27. Incomplete programs use explicit typed semantic nodes or exact missing-definition states.
28. A published incomplete snapshot remains structurally valid and queryable.
29. Only a complete selected-entry dependency closure may enter executable lowering.
30. The compiler consumes an immutable semantic snapshot directly.
31. The production compiler has one canonical executable IR route.
32. Execution tiers accelerate one semantic route rather than implement competing languages.
33. Optimization failure cannot redefine program validity.
34. AI output is an untrusted proposal.
35. Deterministic validators decide acceptance.
36. Structured authoring convenience must not weaken final graph validation.
37. Host effects require explicit typed authority.
38. Ambient filesystem, network, process, terminal, database, clock, entropy, device,
    environment, and foreign-memory authority is forbidden.
39. No mandatory global stop-the-world tracing collector may be introduced.
40. User-controlled semantic depth must not consume unbounded native stack.
41. Runtime calls, recursion, structured control, decoding, validation, querying, deletion,
    and compilation use explicit work structures where input depth can grow.
42. Operational quotas may protect one request, decoder, runtime, workspace, or host boundary.
43. Operational quotas must not redefine which programs are semantically valid.
44. Arbitrary semantic node-count, graph-depth, file-count, line-count, directory-fanout, or
    repository-topology limits are forbidden.
45. Observable order is explicit and deterministic.
46. Hash-table, allocator, thread, filesystem, process, and directory enumeration order are
    not language semantics.
47. Public mutation responses are bounded by explicit response projections.
48. Potentially large detail belongs in exact revision-bound paginated queries.
49. A daemon preflights the exact committed response before durable publication.
50. A valid commit must not become unreportable because an optional response attempted to
    include an unbounded allocation map, graph dump, or semantic diff.
51. Idempotent retry returns the same accepted compact result or a structured conflict.
52. Durable state is acknowledged only after the authoritative publication contract is
    satisfied.
53. Corrupt, old, ambiguous, or partially published durable state is rejected rather than
    heuristically repaired.
54. Protocol, JSON, artifact, HEAD, cache, and future native-image decoders reject unknown
    variants and trailing data.
55. The repository retains one active implementation for each product path.
56. The active repository remains searchable and understandable by a coding agent.
57. Human-facing documentation accurately explains the product without requiring readers to
    reverse-engineer tests or raw protocol payloads.
58. README simplicity must not be purchased by hiding material limitations.
59. Agent-interface compactness must not be purchased by cryptic ambiguity, weaker typing,
    silent defaults, or lossy identity.
60. Performance work follows representative evidence and preserves a simple correctness
    oracle.
61. Application work is used to discover missing language and system capabilities, not to
    justify speculative frameworks.

## Authority Order

1. The active user task.
2. This root `AGENTS.md`.
3. The active prompt explicitly selected by the current task.
4. Accepted normative files under `docs/spec/`.
5. Executable code and focused invariant tests.
6. Boundary schemas and machine descriptions derived from executable contracts.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, old commits, issues, discussions, and historical
    documents.

A newer active prompt supersedes older prompts for campaign sequencing.

A prompt does not silently supersede this file or an accepted specification.

When a campaign deliberately changes semantics, update the owning specification in the same
verified milestone.

Prompts are execution artifacts. Do not copy campaign prose into permanent documentation
unless the fact has become an accepted product contract.

When two active artifacts disagree, identify the fact owner and repair that owner. Do not
create a third authority to reconcile the disagreement.

## Fact Ownership

**Canonical graph and identity:** `docs/spec/semantic-graph.md` and executable
schema/validation code.

**Language types and operation semantics:** `docs/spec/language.md` and executable operation
contracts.

**Daemon and machine transport:** `docs/spec/protocol.md` and executable request/response
definitions.

**Component ownership and trust boundaries:** `docs/architecture.md`.

**Implemented state and exact limitations:** `docs/status.md`.

**Measurements and reversal conditions:** `docs/performance.md`.

**Ordered future gates:** `docs/roadmap.md`.

**Human product introduction:** `README.md`.

**Repository operating policy:** this file.

Do not maintain duplicate status catalogues, fact ledgers, digest registries, documentation
shards, or prompt archives as active authority.

## Human-First README Contract

Humans are expected to read the repository README even though humans are not expected to
author canonical lkjscript programs directly.

The README must answer, in ordinary technical language:

- What is lkjscript?
- Why does it exist?
- Who is it for?
- What is unusual about it?
- How does a human use it through an AI coding agent?
- How does an agent interact with the daemon?
- What is a `.lkjscript` file?
- What can the current implementation actually do?
- What can it not yet do?
- How can a reader try the current product path?
- Where are the specification, architecture, status, performance evidence, and roadmap?

The README opening must explain the product before presenting build, test, benchmark,
mutation-smoke, or repository-maintenance commands.

Do not place `cargo test` or a full verification matrix in the first conceptual section.

Do not use a giant inline JSON transaction as the primary explanation of lkjscript.

A concise conceptual example, diagram, or linked runnable example is preferred.

Any explanatory pseudocode must be labelled as explanatory pseudocode and must not be
presented as lkjscript source syntax.

The README must not imply that a source language, parser, public network service, sandbox,
native backend, package ecosystem, or production-ready platform exists when it does not.

The README may be aspirational only where aspiration is clearly separated from implemented
fact.

Developer verification commands belong near the end of the README or in a clearly linked
development section.

When the public product path changes, update the README in the same milestone.

README review is part of product acceptance, not optional polish.

## Agent-Facing Interface Contract

The primary author of canonical programs is an external coding agent.

The agent-facing surface should minimize semantic guesswork and accidental scaffolding.

- Expose a complete closed machine vocabulary.
- Derive schema descriptions from executable contracts.
- Use stable canonical IDs and lowercase machine names.
- Make compact summaries the default.
- Make exact expansion explicit.
- Support revision-bound batches.
- Support bounded structured mutation receipts.
- Return deterministic typed errors with exact targets and expected contracts.
- Provide repair context that is structurally selected, not prose-selected.
- Provide legal constructors and visible candidate values from exact semantics.
- Allow aggregate typed writes when they remove repeated storage scaffolding without creating
  a second authority.
- Do not require an agent to author private compiler indexes, predecessor lists, phi nodes,
  artifact offsets, checksums, or durability metadata.

A low-level node-by-node mutation API is not automatically agent-native merely because it is
typed JSON.

Before preserving or adding public boilerplate, measure the request bytes, response bytes,
round trips, failed proposals, and implementation complexity of a representative agent task.

Prefer one semantically meaningful structured proposal over many transport-level setup
operations when the expansion is deterministic and final validation remains authoritative.

Do not invent a custom compact syntax solely from intuition. Compare reliability, ambiguity,
parser cost, schema discoverability, and actual model-facing cost.

## Canonical Data Classes

Classify maintained data into exactly one ownership class.

### Semantic State

- workspace identity;
- stable node identity;
- node kind;
- typed attributes;
- typed ownership;
- ordered semantic child slots;
- direct semantic references;
- operation payloads;
- explicit holes and incomplete definitions;
- selected package entry;
- persistent allocation frontier;
- tombstones;
- revision.

Semantic state belongs in immutable snapshots and canonical artifacts.

### Derived State

- types inferred from canonical declarations;
- binding and scope facts;
- completeness blockers;
- incoming uses;
- dependency closure;
- legal constructors;
- repair contexts;
- semantic diffs;
- layouts;
- ownership and effect facts;
- Core IR;
- machine code;
- profiles;
- query indexes;
- caches.

Derived state may be recomputed or cached. It never becomes a second mutable source of truth.

### Executable State

- verified Core IR;
- interpreter frames and values;
- runtime cells;
- machine code;
- runtime handles;
- active capability instances.

Executable state is not embedded in the semantic artifact unless a separate invalidatable
cache format explicitly owns it.

### Presentation and Transport

- JSON and future compact projections;
- CLI text;
- README diagrams;
- debug views;
- schema descriptions;
- request IDs;
- session aliases;
- pagination cursors;
- structured authoring drafts.

Presentation and transport may be lossy or lossless. They never become semantic identity.

## Semantic Graph Schema

The canonical graph is not a generic property graph.

Each node has one closed kind with explicit attributes, owner rules, child slots, ordering,
reference slots, operand slots, result slots, cardinality, completeness rules, and
history-continuity rules.

Unknown schema elements reject.

Do not preserve unknown semantic fields for hypothetical forward compatibility.

Language evolution replaces the closed schema deliberately.

One code-owned contract should expose stable schema facts to validators, codecs, query
construction, legal-constructor results, machine descriptions, and lowering where practical.

Do not hand-maintain a second JSON schema that can drift from executable contracts.

Avoid abstraction machinery larger than the closed vocabulary it describes.

A closed enum, a small descriptor, and exhaustive matches are usually preferable to a registry
or framework.

## Operation Contracts

Each operation constructor has one authoritative contract.

- stable operation code and boundary tag;
- operand count or exact dynamic-arity rule;
- operand types and use modes;
- result count or exact dynamic-result rule;
- result types;
- literal fields;
- definition references;
- owned region roles;
- region parameter contracts;
- terminator status;
- completeness status;
- capability requirements;
- effect classification;
- lowering obligations;
- identity-preserving edit rules.

Validators, queries, codecs, lowering, interpreters, and schema descriptions must consume the
same facts.

Do not duplicate operation truth across unrelated match tables when one exhaustive helper can
own it.

Do not force a static descriptor model to pretend a call or structured region has fixed arity
when it does not.

Extend the contract model only as far as current constructors require.

Avoid heap allocation for static lookup and avoid temporary operand vectors in graph-wide
scans when indexed access is sufficient.

## Identity and History

- A workspace owns one persistent node-ID domain.
- A node ID is unambiguously scoped by workspace identity.
- Persistent allocation is monotonic.
- IDs are staged during transaction preparation.
- Rejected and validate-only transactions leave the published allocation frontier unchanged.
- Deletion tombstones every deleted ID.
- Old snapshots preserve deleted nodes.
- A later node never receives a tombstoned identity.
- Rename preserves identity.
- A scalar payload or operand update preserves identity when the constructor and continuity
  contract remain the same.
- A move preserves identity only when an explicit typed edit defines semantic continuity.
- A replacement creates new identity unless a specific refinement rule says otherwise.
- Structured authoring expansion assigns deterministic staged identities in one documented
  order.
- Changing the structure of a draft must not silently rebind a previously named persistent
  node.
- Builtins use an explicit builtin identity domain.
- Do not select builtins by magic names.
- Compiler function, block, and value IDs are dense private identities and never escape as
  semantic IDs.

History validation recognizes only explicit continuity rules.

Do not generalize one valid constructor transition into an unrestricted morph operation.

## Typed Holes and Refinement

Typed holes are first-class semantic nodes.

A hole records an exact expected contract, remains queryable, and blocks executable lowering
only in the selected dependency closure.

Filling a hole is a semantic refinement.

The default scalar refinement is one-way from a hole to a complete non-terminator operation
with the same result contract.

Successful refinement preserves the hole Node ID, owner, body position, and existing uses.

The replacement may reference existing values and transaction-local values.

Final validation still enforces scope, dominance, type, ownership, region role, and
result-index rules.

Refinement to another hole, a terminator, a different result contract, or from an already
complete operation rejects unless a new accepted specification defines a narrower transition.

When structured operations gain regions, define whether refinement allocates owned regions and
how failure rolls back every staged identity.

Do not create a generic graph rewrite language to implement one refinement.

## Transactions

Every mutation request names the workspace, exact base revision, commit or validate-only mode,
optional committed-request idempotency key, an ordered closed mutation batch, and an explicit
bounded response projection.

A transaction stages graph changes, identity allocation, tombstones, validation results,
change facts, artifact bytes, HEAD bytes, publication metadata, and exact response bytes.

A successful commit follows this logical order:

1. validate workspace, revision, mode, idempotency, and response projection;
2. allocate every declared transaction-local identity in staged state;
3. expand any structured proposal deterministically into canonical typed edits;
4. apply the ordered edits to staged state;
5. validate the final canonical graph;
6. validate history continuity;
7. derive deterministic change count and digest;
8. construct the bounded receipt;
9. encode and preflight the exact response;
10. encode and preflight authoritative durable state;
11. durably publish the new revision and HEAD;
12. publish the in-memory snapshot;
13. return the preflighted receipt.

A rejected request changes none of those states.

Do not partially publish a batch.

Do not consume IDs on rejection.

Do not heuristically repair an invalid request.

Return structured rejection facts.

Validate-only uses the same semantic preparation and byte-preflight path as commit, then
publishes nothing.

Idempotency is for committed requests unless an accepted protocol defines another exact
meaning.

## Structured Authoring Drafts

A structured authoring draft is permitted when it removes repeated canonical scaffolding from
agent requests.

A draft is a closed typed transaction payload, not source code, an AST authority, or a
persisted parallel program.

A draft may describe a function signature, a structured region, ordered operations, nested
regions, local bindings, and a terminator.

The daemon or transaction layer expands it deterministically into the same canonical nodes
that fine-grained edits would produce.

The final graph validator remains authoritative.

Draft-local names or handles never become persistent identity.

The expansion order, returned bindings, rejection behavior, and identity allocation must be
tested.

Do not retain two equally preferred public creation paths indefinitely.

When a structured path clearly supersedes low-level public scaffolding, remove or privatize
the displaced path.

Retain fine-grained semantic edits only where they serve real maintenance workflows such as
hole refinement, operand replacement, insertion, deletion, rename, or body replacement.

Do not introduce a general-purpose macro language, template engine, rewrite calculus, or
parser without multiple current consumers.

## Receipts and Semantic Diffs

The default mutation receipt is compact and explicitly bounded.

- workspace;
- base revision;
- resulting or predicted revision;
- snapshot hash;
- publication status;
- total created-node count;
- only explicitly selected local-handle bindings;
- total semantic-change count;
- deterministic semantic-change digest;
- bounded completeness transition facts when useful.

Do not return a complete semantic diff or every allocation by default.

Reject duplicate, undeclared, or excessive requested bindings before publication.

A full semantic diff is derived from exact retained revisions.

Diff order is deterministic.

Diff queries are paginated and bind every cursor to workspace, revisions, query purpose, and
deterministic position.

Do not persist a full diff solely to serve pagination while the snapshots remain available.

## Queries

Queries are pure over one immutable revision unless the request explicitly names two
revisions.

Correct full recomputation is the oracle.

Do not add a cache or reverse index until representative repeated cost is measured and
invalidation is exact.

- workspace summary;
- node summary;
- exact node record on explicit request;
- owner chain;
- region and body slices;
- outgoing dependencies;
- incoming value uses;
- incoming definition references;
- completeness blockers;
- semantic diff;
- legal constructors;
- visible values and definitions;
- repair context;
- compile explanation;
- runtime trace when implemented.

Compact summaries are the default.

Exact expansion is opt-in.

Every observable collection has deterministic ordering and explicit pagination.

Page and batch limits protect transport and computation boundaries; they do not limit semantic
program size.

Independent read items may share one revision-bound batch and may report independent
structured outcomes.

Do not mix mutation and read execution into one ambiguous atomic request.

## Repair Context

Repair context is a deterministic composition of typed semantic facts for one explicit target.

- target identity and kind;
- exact expected type, region role, and use mode;
- current value or constructor when applicable;
- owner chain;
- function signature;
- region and block position;
- bounded surrounding body slice;
- bounded visible values and callable definitions;
- bounded incoming uses;
- legal constructors;
- relevant completeness blockers;
- nested structured-control context when applicable.

Selection is structural and deterministic.

Do not invoke an LLM inside the daemon to choose correctness-critical context.

Do not put arbitrary natural-language summaries in canonical machine responses.

A presentation layer may add prose after typed facts are produced.

## Protocol and Machine Projection

The daemon protocol is closed, typed, bounded, and explicitly versioned.

One code definition owns each request and response variant.

Stable tags belong near the enum or contract they identify.

Unknown versions, fields, variants, tags, and trailing bytes reject.

Every length, count, index, ID, revision, hash, cursor, discriminant, recursion depth, and
aggregate budget is checked.

A protocol replacement is direct. Do not retain a legacy decoder when compatibility is not
requested.

Binary local IPC is transport only and is not a semantic artifact.

The generic JSON CLI is also transport only.

Machine mode defaults to compact output; pretty output is explicit.

Machine stdout contains exactly one structured response.

Process diagnostics belong on stderr.

Unknown JSON fields and variants reject.

Trailing JSON data rejects.

Input bytes and nesting are bounded at the CLI boundary.

Do not accept arbitrary `serde_json::Value` property bags inside semantic requests.

Use closed DTOs or direct closed protocol types with exhaustive conversion.

IDs have one canonical machine representation.

Machine enum names are stable lowercase names, never Rust debug output.

Every JSON variant has round-trip, unknown-field, malformed-domain, and trailing-input tests.

A schema or descriptor command derives from executable contracts.

Do not add a large schema-generation framework when a small code-owned descriptor is
sufficient.

## Persistence

Each retained revision is immutable.

HEAD identifies the committed revision and hash plus bounded non-semantic publication and
idempotency metadata.

HEAD must remain independently bounded.

Do not store a full graph diff, graph dump, request body, or allocation map in HEAD.

Persistent format changes use a new unambiguous magic, version, or schema identity.

When compatibility is not required, reject old bytes rather than adding a migration reader.

- fixed magic;
- explicit format version;
- explicit semantic schema identity;
- canonical ordering;
- fixed endianness;
- checked counts and lengths;
- corruption detection;
- deterministic content hash;
- strict trailing-byte policy;
- bounded defensive decoding;
- no Rust memory-layout dependency;
- no pointer values;
- no private compiler IDs;
- no mutable cache truth.

A durable commit is acknowledged only after the authoritative directory and HEAD transitions
satisfy the documented crash contract.

Failure injection covers every publication step.

If the commit outcome becomes genuinely unknowable, stop the daemon rather than continuing
with ambiguous authority.

Full snapshot rewrite and full retained history remain acceptable until representative
measurements justify a journal, chunk store, pruning policy, or database.

Do not optimize persistence preemptively.

## Compiler and Core IR

The compiler accepts an immutable complete snapshot, an entry definition, ordered invocation
arguments when supported, target information when relevant, explicit capability grants when
relevant, and explicit optimization policy when relevant.

```text
Semantic Program Graph snapshot
    -> completeness and semantic validation
    -> deterministic dependency closure
    -> compact dense private Core IR
    -> Core IR verification
    -> interpreter or later native lowering
```

Core IR is private derived state.

Core IR is not serialized into semantic artifacts.

Core IR uses dense locality-oriented IDs that never escape as public semantic identity.

Every lowering is deterministic.

Every verifier rejects malformed IR before execution.

The interpreter remains the complete semantic oracle during bootstrap.

A future native tier is differential-tested against the interpreter.

Agents author structured semantic regions, not predecessor lists, branch targets, phi nodes,
dense values, or machine blocks.

Structured control may lower to a private CFG with block parameters.

Keep one executable route even when multiple execution tiers exist.

## Runtime

Runtime traps and resource exhaustion are structured and distinguishable.

Recoverable domain failure uses explicit result types when the language supports them.

Checked arithmetic follows the accepted language specification.

Execution of calls, recursion, branches, and loops must not recurse through the Rust call
stack based on user program depth.

Use explicit interpreter frames, block state, work queues, and fuel or another exact
operational budget.

A budget failure is an operational result and must not corrupt semantic or durable state.

Do not silently re-execute an effectful program in another engine after execution begins.

A native tier may decline before entry and fall back to the unchanged verified Core IR route.

Runtime values use static layouts where semantics permit.

Avoid universal boxing without measured need.

## Effects, Capabilities, and Ownership

Pure structured semantics precede host effects.

Effects are deterministic derived facts.

Capabilities are explicit typed values or explicit invocation grants.

A host operation contract owns capability kind, operands, results, effects, ownership modes,
failure behavior, cancellation, blocking, and resource accounting.

Do not duplicate host-operation truth across graph, compiler, daemon, and runtime.

The long-term memory direction is value semantics for ordinary immutable data, affine
semantics for external resources and mutable identity, compiler-derived moves and cleanup, no
mandatory global tracing collector, and explicit managed regions only where their use case and
latency are defined.

Do not let the scalar or pure bootstrap accidentally define the final heap model.

Do not add effects, ownership, FFI, native code, or a managed heap as collateral to an
unrelated structured-control milestone.

## Security and Trust Boundaries

- binary IPC bytes;
- JSON CLI bytes;
- structured authoring drafts;
- semantic artifact bytes;
- HEAD and durable directory contents;
- pagination cursors;
- host-operation inputs;
- capability references;
- FFI values;
- native code and relocation metadata;
- cache files;
- runtime arguments.

Use filesystem permissions and OS peer identity for the local daemon boundary.

Do not claim sandboxing that is not implemented.

Prefer a supervised worker when native or foreign execution cannot be safely contained in the
daemon address space.

Malformed input may reject. It must not panic, corrupt state, allocate without bound, hang,
consume unbounded native stack, or publish a partial mutation.

Fuzz or deterministically mutate decoders, transaction validation, Core IR verification,
capability dispatch, and native boundaries as they exist.

Retain failing seeds or minimized corpora discovered by generated tests.

## Resource Policy

A resource policy protects one named operational boundary.

- maximum IPC frame bytes;
- maximum CLI input bytes and nesting;
- maximum page size;
- maximum batch query count;
- maximum aggregate query items or output bytes;
- maximum returned bindings;
- maximum artifact decoder bytes;
- maximum runtime fuel or deadline;
- maximum runtime frames;
- maximum structured-draft nesting or bytes;
- maximum one-response output bytes.

Policy constants belong near the boundary and have focused tests.

Do not scatter magic limits.

Do not describe an operational policy as a language maximum.

## Determinism

- transaction acceptance or rejection;
- assigned persistent IDs;
- structured-draft expansion;
- semantic diff order;
- change digest;
- query order;
- legal-constructor order;
- visible-value and visible-definition order;
- repair-context selection;
- artifact bytes;
- snapshot hash;
- dependency closure;
- Core IR lowering;
- interpreter result;
- machine collection order where exposed.

Use ordered collections or explicit sorting at every observable boundary.

Add tests that vary insertion and hash-map order where possible.

## Performance Evidence

Measure before replacing a bootstrap baseline.

- fresh debug and release build time;
- fresh and cached full verification time;
- dependency count and fresh target size;
- release binary size;
- daemon cold start;
- daemon restart with retained workspaces;
- workspace creation;
- transaction preparation;
- durable commit;
- query latency and response bytes;
- structured-draft expansion cost;
- agent request count and bytes;
- repair-context latency and bytes;
- semantic diff latency;
- artifact size;
- per-workspace memory;
- dependency-closure construction;
- Core IR lowering and verification;
- interpreter startup and throughput;
- call, branch, loop, and recursion workloads;
- native compile and execution when introduced.

Record hardware, OS, toolchain, commit, build mode, warmup, samples, workload, output oracle,
median, tails, and memory when available.

Do not tune one toy benchmark by narrowing valid semantics.

Label microbenchmarks as microbenchmarks.

Do not claim model-token savings from byte counts alone.

Record actual model tokens only when real telemetry exists.

Byte count, round trips, rejected proposals, and elapsed time remain valid direct evidence.

## Agent Cost Discipline

- request bytes;
- response bytes;
- daemon round trips;
- CLI process launches;
- repository files opened;
- large files repeatedly reread;
- full graph scans;
- failed mutations;
- unnecessary local-handle bindings;
- rediscovery after restart;
- model context;
- coding-agent API calls;
- verification commands and rebuilds.

Reduce these costs through exact semantics, compact defaults, selected bindings, batch reads,
aggregate typed writes, stable IDs, repair contexts, legal constructors, deterministic errors,
revision-bound pagination, one fact owner, small active documentation, and focused checks.

Do not reduce cost by weakening validation, omitting rejection tests, hiding failures, or
skipping final verification.

Do not add model routing, prompt registries, token billing, or agent orchestration to the
daemon merely to claim AI integration.

Use the coding agent itself efficiently: search exact symbols first, run focused tests during
implementation, and run the full boundary once after the milestone is coherent.

## Application-Driven Development

Use small representative applications to expose missing semantics and interface friction.

A representative application must exercise the real daemon, public machine interface,
persistence, compiler, verifier, and interpreter.

Do not satisfy application acceptance only through private Rust constructors.

Choose an application whose required features belong to the active gate.

Do not introduce a web framework, GUI system, package manager, database layer, scheduler, or
general standard library to support one bootstrap example.

Retain the example only when it remains a useful product-path oracle.

Prefer one honest end-to-end application over many disconnected demo commands.

## Architecture Restraint

Before adding an abstraction, answer:

1. Which current producer creates it?
2. Which current consumer needs it?
3. Which invalid state does it remove?
4. Which repeated work does it remove?
5. Which test or measurement proves the need?
6. Could a closed enum, direct struct, or local helper suffice?
7. Does it create a second authority?
8. Does it create another identity domain?
9. Does it require serialization?
10. Does it require a process boundary?
11. Does it increase agent search or request cost?
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
- an open dialect or plugin system;
- a general constraint solver;
- a visitor framework for one traversal;
- a registry for a closed vocabulary;
- a serializer for same-build private values;
- a database before persistence pressure is measured;
- an async runtime before concurrency pressure exists;
- a scheduler before multiple runnable cells exist;
- a cache before reuse is measured;
- a reverse index before scans are measured;
- a custom JIT backend before mature backends are evaluated;
- formal proof infrastructure without a concrete high-value theorem;
- a second source, graph, or IR authority;
- an abstraction named only for a hypothetical future consumer;
- a documentation framework whose only output is more documentation machinery.

## Dependencies

Every dependency has a current named consumer.

Record why it is preferable to a small local implementation.

- transitive dependency count;
- fresh build cost;
- binary size;
- maintenance activity;
- trust boundary;
- unsafe code;
- platform impact;
- feature configuration;
- license.

A strict machine JSON projection may justify `serde` and `serde_json`.

Do not add a schema framework, parser generator, async runtime, database, compiler backend,
tokenization library, or benchmarking framework as collateral to a smaller milestone.

Use exact resolved versions through `Cargo.lock`.

Do not change unrelated dependencies.

## Repository Shape

Use one Rust package until a boundary earns a crate split.

- unsafe or FFI isolation;
- independently useful stable API;
- target-specific build isolation;
- a process binary with a narrow protocol;
- measured compile-time isolation;
- material dependency isolation.

Do not recreate a many-crate compiler topology by habit.

Split modules on semantic ownership.

No line-count, byte-count, fanout, or nesting rule defines repository validity.

Large coherent files are acceptable.

Tiny facade files are not automatically desirable.

Delete superseded code in the same milestone.

Do not leave `old`, `legacy`, `compat`, `v1`, disabled duplicate paths, or commented-out
replacements.

## Documentation

Keep the maintained documentation set small and role-specific.

- `docs/spec/semantic-graph.md` owns canonical graph, identity, transaction, and artifact
  semantics;
- `docs/spec/language.md` owns accepted language types, operations, control, and execution
  semantics;
- `docs/spec/protocol.md` owns public daemon and machine transport behavior;
- `docs/architecture.md` owns current component responsibility and trust boundaries;
- `docs/status.md` owns exact implemented state and limitations;
- `docs/performance.md` owns retained measurements and reversal conditions;
- `docs/roadmap.md` owns ordered evidence gates;
- `README.md` is the human-first product entry point.

Add another maintained document only when none of these roles can own the fact.

Specifications state accepted semantics.

Status states what the current checkout implements.

Architecture states ownership and boundaries.

Performance states measurements.

Roadmap states future gates.

README explains the product to humans.

Do not copy the same catalogue across documents.

Generated machine descriptions derive from code and are emitted on demand unless a real
external consumer requires a retained copy.

## Testing

- schema acceptance and exhaustive descriptor coverage
- schema rejection and unknown-tag rejection
- unknown JSON field and trailing-input rejection
- containment and owner-role validation
- reference-slot validation
- scope, visibility, and dominance validation
- type validation
- function signature and argument validation
- region-parameter validation
- structured-region ownership validation
- terminator-role validation
- hole completeness and refinement
- invalid refinement rejection
- old snapshot preservation after refinement
- stable identity
- no identity reuse
- allocator rollback
- structured-draft deterministic allocation
- structured-draft rejection rollback
- rename continuity
- deletion blocking and tombstones
- stale revision rejection
- idempotent retry and conflict
- compact receipt bounds
- selected allocation binding
- transaction response preflight
- deterministic diff count and digest
- diff pagination
- query pagination and cursor binding
- batch-query ordering and independent errors
- incoming-use precision
- outgoing-definition dependency precision
- body and nested-region slice order
- visible value and definition correctness
- legal-constructor correctness
- repair-context sufficiency
- artifact determinism
- artifact corruption and old-version rejection
- durable failure atomicity
- daemon restart and competing-writer rejection
- protocol framing and correlation
- real client and daemon execution
- direct graph compilation
- Core IR verification
- interpreter semantics
- explicit call-stack safety
- loop and branch semantics
- runtime fuel or budget exhaustion
- generated transaction or program sequences
- deterministic malformed-boundary mutation
- representative application end-to-end behavior

Use model-based or generated sequences where many operation orders share one invariant.

Retain a failing seed or minimized corpus when a generated test finds a defect.

Use the real daemon and public machine client for the principal end-to-end vertical.

Do not satisfy acceptance only through private Rust APIs.

## Verification

Run focused checks during implementation.

Run the full boundary once after the milestone is coherent.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run the real daemon/client integration test when protocol, persistence, query, compiler,
runtime, CLI, structured authoring, or README runnable examples change.

Run the bounded deterministic fuzz or mutation smoke required by `docs/status.md` when a trust
boundary changes.

Never claim an unrun command.

Distinguish product failure from environment failure.

Record exact failed commands and relevant output without dumping unrelated logs.

Do not repeatedly run the complete expensive boundary after every small edit.

Use the smallest focused test that can invalidate the current hypothesis, then one full final
boundary.

## Workflow

### Orient

1. Record branch, starting commit, and `git status --short`.
2. Read this file once.
3. Read `README.md` as a human product surface.
4. Read `docs/status.md`.
5. Read only the specification sections relevant to the active milestone.
6. Search exact symbols before opening large files.
7. Inspect recent relevant commits newer than the prompt audit.
8. Preserve unrelated work.
9. State one dependency-closed acceptance gate.

### Decide

Before editing, identify:

- the human-visible or agent-visible operation being improved;
- the semantic authority;
- the current producer;
- every active consumer;
- identity and history rules;
- success and rejection behavior;
- durability behavior;
- response bounds;
- security boundary;
- representative evidence;
- non-goals;
- stop condition.

Do not ask the user to choose internal details that coherent semantics, tests, measurements,
or the active product laws can decide.

### Implement

1. Change the authoritative type or contract.
2. Change every active producer.
3. Change every active consumer.
4. Delete displaced code and obsolete tests.
5. Add success evidence.
6. Add rejection evidence.
7. Add boundary, restart, and corruption evidence when applicable.
8. Run focused checks.
9. Update owning specifications, status, architecture, performance, roadmap, and README as
   applicable.
10. Run the full verification boundary once.
11. Inspect the final diff and worktree.

### End a Turn

End only at a buildable, testable, dependency-closed boundary.

- Do not leave two semantic authorities.
- Do not leave two active protocol or artifact versions.
- Do not leave two preferred public authoring paths.
- Do not leave two compiler paths for one tier.
- Do not leave a half-migrated request vocabulary.
- Do not leave disabled legacy code.
- Do not leave an intentionally failing branch.
- Do not leave an undocumented durable format change.
- Do not leave a committed response that was not preflighted.
- Do not leave README claims ahead of implementation.
- Do not end with prompt-only design when implementation was requested and a coherent vertical
  was achievable.

A handoff names exact paths, symbols, commands, observed failures, unresolved decisions, and
the next acceptance gate.

Keep the handoff compact enough for the next agent turn to consume without rereading the whole
repository.

## Multi-Agent Use

The lead agent owns architecture, integration, documentation truth, and final verification.

Use subagents for independent bounded questions or disjoint implementation areas.

- one exact question;
- exact paths;
- required evidence;
- explicit non-goals;
- a stop condition;
- a compact output format.

Do not ask multiple subagents to invent competing architectures after evidence has selected
one.

Do not let a subagent independently create protocol, schema, identity, persistence, README
truth, or compiler authority.

Review every subagent result against the actual checkout before integration.

## Git

Inspect the worktree before edits.

Use explicit targeted deletion.

Do not use `git reset --hard`.

Do not use `git clean -fd`.

Do not force push.

Incompatible-change permission does not authorize destruction of unrelated work.

Prefer one cohesive commit per verified milestone when commits are permitted.

Do not create empty planning commits.

Do not push or open a pull request unless explicitly requested.

## Stop Conditions

- source text becomes required program authority
- a second mutable semantic graph or AST becomes authoritative
- a structured draft is persisted as a competing program representation
- arbitrary semantic properties or string-labelled edges enter canonical state
- persistent IDs are reused
- hashes or names become mutable identity
- generic identity-preserving constructor morphing appears without exact semantics
- default mutation responses become unbounded
- full diffs or request bodies enter durable HEAD metadata
- unknown semantic fields are preserved
- graph-wide query results become unpaginated
- a cache appears without measured reuse
- a reverse index appears without measured scan cost
- a database appears without measured persistence pressure
- async concurrency appears without concurrent workload pressure
- a native tier appears without representative language workloads
- a global tracing collector becomes mandatory
- ambient host authority appears
- a compatibility bridge or dual version remains
- a generic framework has only one consumer
- README becomes a build/test preamble or raw protocol dump
- documentation machinery multiplies without fact ownership
- repository fragmentation increases agent search cost
- performance claims lack reproducible evidence
- token claims lack model telemetry
- an example app pulls unrelated platform subsystems into the active gate

Usually correct by deleting, narrowing, using one closed enum, using one explicit transaction,
adding one structured proposal, returning a compact receipt, moving detail to a paginated
query, or postponing the subsystem until its consumer exists.

## Completion Report

```text
Starting commit:
Ending commit:
Milestone:
Human-facing outcome:
Agent-facing outcome:
Authority changed:
Compatibility breaks:
Schema result:
Identity and history result:
Structured-authoring result:
Transaction result:
Receipt-bound result:
Query and repair result:
Protocol and machine-interface result:
Artifact and persistence result:
Compiler and Core IR result:
Runtime result:
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
