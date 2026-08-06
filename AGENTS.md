# AGENTS.md

## 1. Scope

This file applies to the entire repository.

All code, comments, diagnostics, documentation, tests, generated reference text, commit messages, and agent reports must be written in English unless a task explicitly requires another language for a user-facing artifact.

The repository is in an architectural reset. Backward compatibility is not a requirement. Large deletions, direct cutovers, syntax replacement, representation replacement, crate consolidation, runtime replacement, and temporary reduction of provisional language surface are permitted when they produce a simpler and stronger system.

Do not preserve an old mechanism because it is old, expensive, tested, documented, or widely referenced. Preserve it only when it remains part of the target product and earns its complexity.

## 2. Authority by dimension

There is no single linear authority order for every kind of claim. Different artifacts own different kinds of truth.

### 2.1 Task and procedure authority

For what to do now and how to work:

1. explicit current task instructions;
2. this `AGENTS.md`;
3. narrower repository-local instructions only when they do not conflict with the first two.

A current task may intentionally override existing architecture, tests, or documentation.

### 2.2 Intended product and semantic authority

The accepted normative specification owns intended externally visible language semantics and semantic-workspace behavior.

Until the normative specification exists, explicit current task instructions together with the mission and non-negotiable direction in this file define the target. Do not infer permanence from prototype code, tests, examples, or old prose merely because no replacement specification has been written yet.

Implementation, tests, and status prose may lag the specification. When they do, the mismatch is a documented implementation gap, not an automatic change to the intended semantics.

An accepted architecture decision may constrain implementation strategy. It does not silently amend the language specification. Update the owning specification in the same change when semantics change.

### 2.3 Actual checkout behavior authority

Executable code, tests, CLI definitions, schemas, and manifests own what the current checkout actually does.

`docs/status.md` or its active equivalent summarizes actual behavior and known gaps. It must not claim behavior that the checkout does not implement.

Cargo metadata owns actual workspace membership and dependency edges. Architecture prose explains the graph; it does not override it.

### 2.4 Performance authority

A reproducible benchmark harness, its workload definition, measurement protocol, and compact recorded baseline own performance evidence.

A slogan, microbenchmark, unrepeatable timing, or design document does not establish performance.

### 2.5 Planning authority

`docs/roadmap.md` owns ordering and intent only. It owns no implemented fact and no normative semantic rule.

### 2.6 Historical authority

Git history owns history.

Obsolete documents, old handoff files, archived plans, platform revisions, digest markers, public-fact registries, evidence dossiers, and past test expectations do not override the active specification, current task, or current checkout.

### 2.7 Conflict handling

When artifacts conflict:

1. identify the dimension of the claim;
2. consult the artifact that owns that dimension;
3. inspect executable evidence;
4. update or delete stale artifacts in the same change;
5. record a durable decision only when future reversal would otherwise repeat the same analysis.

Never resolve a conflict by inventing another registry that attempts to make all facts globally authoritative.

## 3. Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, high-performance language and runtime platform.

The long-term authoritative source is a typed semantic program workspace. Text is an import, export, review, debugging, and interoperability projection. It is not assumed to be the permanent source of semantic identity.

An AI agent should be able to:

- inspect semantic entities and relationships;
- query precise, revision-labelled context;
- operate on incomplete but meaningful programs;
- apply typed atomic edits;
- receive semantic diffs and diagnostics;
- compile complete semantic snapshots directly;
- use deterministic local validation for every correctness decision.

Humans must retain useful projections, reviewable semantic and textual diffs, understandable diagnostics, reproducible builds, and explicit architectural decisions.

Final performance matters. Optimize agent interaction, edit latency, compilation, startup, execution, memory, code size, allocation, copying, and cache behavior as one product.

## 4. Current priority order

Unless the current task narrows the scope, work in this order:

1. eradicate arbitrary validity-changing limits end to end;
2. separate language validity from untrusted host resource policy;
3. repair stack safety, representation widths, and poor scale complexity;
4. delete residual overengineering and obsolete authority machinery;
5. establish truthful documentation authority;
6. measure and select one production execution path;
7. make the semantic workspace the direct compiler input;
8. add measured incremental recomputation;
9. resume daemon, database, distributed, GUI, web, game, and broader platform work.

Do not begin a lower-priority feature while preserving a higher-priority blocker without evidence that the blocker is outside the current dependency-closed slice.

A completed narrow vertical is preferable to broad scaffolding.

## 5. Non-goals

The following are not product authorities or compatibility commitments:

- Brainfuck support, completion, or benchmark success;
- compatibility with current `.lkjscript` syntax;
- compatibility with old bytecode, prepared programs, package artifacts, protocol messages, manifests, caches, or serialized snapshots;
- preservation of platform revision numbers or digest ecosystems;
- mandatory evaluator, VM, baseline-JIT, and optimizing-JIT parity;
- source, compiler, runtime, or repository count quotas;
- file-length, line-width, directory-width, or directory-depth policies;
- a zero-dependency badge;
- a proof certificate for every internal compiler transformation;
- a distributed semantic database before the local semantic model works;
- speculative scheduler, NUMA, process-cell, GUI, web, or remote-execution work that delays the language foundation;
- a new abstraction whose main purpose is preserving an obsolete abstraction.

Delete Brainfuck material when it costs anything meaningful to maintain. Do not add Brainfuck acceptance criteria.

## 6. One active architecture

Maintain one active:

- semantic source model;
- compiler pipeline;
- language definition;
- production runtime architecture;
- documentation authority system;
- roadmap.

Do not create permanent:

- `v2` trees;
- shadow compilers;
- compatibility parsers;
- dual-write source authorities;
- parallel old/new runtimes;
- duplicate status systems;
- old-path feature flags without a scheduled deletion in the same coherent migration.

A temporary bridge is allowed only inside a direct cutover. Remove it before declaring the cutover complete.

A small reference executor may remain as a semantic oracle. It is not automatically a public engine or a feature-parity obligation.

## 7. Deletion before abstraction

When complexity comes from duplicated mechanisms, delete mechanisms before adding a framework over them.

Prefer:

- deleting obsolete code over adapting it;
- merging artificial crates over adding cross-crate contracts;
- one typed owner of a fact over several digested reconstructions;
- direct tests over status markers;
- generated reference output over hand-copied registries;
- Git history over an active archive hierarchy;
- a generic correct path over many incomplete special cases;
- a clear typed failure over truncation or silent fallback;
- a mature dependency over a large custom subsystem when evidence favors it.

Sunk cost is not evidence.

Do not move deleted architecture into `legacy/`, `archive/`, `compat/`, `v1/`, or an inactive feature. Delete it.

## 8. AI-primary design

AI-primary does not mean embedding model calls into the compiler.

The semantic workspace, compiler, transaction engine, query engine, validation, and runtime must be deterministic and usable offline. Models may propose operations. Deterministic machinery accepts or rejects them.

Optimize the agent interface for:

- semantic locality rather than lexical proximity;
- compact typed summaries;
- stable identities;
- batch operations;
- selective expansion;
- predictable schemas;
- explicit legal-action queries;
- low round-trip count;
- pagination and continuation;
- reproducible diagnostics;
- failure atomicity.

Do not optimize for AI by hiding semantics in an opaque binary blob. Non-text authority still requires inspectable projections and stable typed APIs.

Do not dump a whole repository into model context when a dependency, type, ownership, or impact slice can answer the request.

## 9. Semantic validity and resource control

### 9.1 Arbitrary counts are not semantic laws

A semantically valid program must not become invalid merely because it exceeds a project-selected count of:

- tokens;
- bytes in an ordinary trusted source unit;
- nested forms or nested types;
- children, arguments, parameters, or operands;
- declarations or top-level forms;
- fields, variants, patterns, or match arms;
- files, modules, imports, directories, or directory entries;
- HIR expressions, functions, entries, uses, loans, calls, obligations, witnesses, or type nodes;
- ownership states;
- SSA functions, blocks, edges, values, frame states, or metadata;
- specialization instances;
- structural types, layouts, representations, destinations, values, or dependencies;
- task-graph nodes or scheduler work units;
- diagnostics, query results, repository nodes, documentation facts, or graph edges.

Do not remove a limit by:

- raising it;
- changing its type;
- moving it to a later phase;
- renaming it;
- hiding it in a profile;
- labelling it a foundation or safety maximum;
- making the default profile larger;
- replacing a count with a deterministic work quota that rejects the same programs.

Delete the semantic restriction and repair the algorithm or representation.

### 9.2 Every remaining bound has one class

Every bound must be classified as exactly one of:

1. **Semantic law**
   Type correctness, effect legality, capability authority, ownership legality, exhaustive matching, valid control flow, or another rule about meaning. A semantic law is not a size quota.

2. **Unavoidable external representation boundary**
   A real operating-system, ABI, address-space, file-format, or third-party boundary. Widen, segment, stream, or redesign before exposing it to ordinary programs. Keep it local to the boundary.

3. **Untrusted host or request policy**
   Explicit operational limits over coarse resources such as input bytes, memory, output, elapsed time, cancellation, and concurrency. Policy controls a request; it does not redefine language validity.

4. **Private implementation tuning or test geometry**
   Initial capacities, growth factors, thresholds, benchmark sizes, and cache policy. These are not public contracts and must not silently reject valid programs.

Do not build another positional ceiling table, digest-bound profile system, or universal budget taxonomy around this classification.

### 9.3 Trusted local compilation

Trusted local compilation must not use finite project-selected count quotas.

It continues until:

- success;
- explicit cancellation;
- allocation failure;
- operating-system or I/O failure;
- a genuine external representation failure;
- another real host failure.

Telemetry may count work. Telemetry does not grant or deny semantic validity.

### 9.4 Untrusted requests

Daemon, multi-tenant, remote, and adversarial requests must carry explicit host policy.

Prefer one small policy containing only enforceable coarse resources. An unrestricted policy must be representable.

The same program must be able to exhaust a low policy and succeed unchanged under a higher or unrestricted policy.

Resource exhaustion must be:

- typed;
- attributable;
- cancellation-safe;
- failure-atomic;
- non-poisoning to caches and prior snapshots;
- distinct from a semantic error.

### 9.5 No truncation

Never silently truncate:

- source;
- declarations;
- diagnostics;
- semantic graphs;
- query results;
- diffs;
- generated code;
- serialization;
- artifacts;
- metrics claimed as complete.

Stream, paginate, provide continuation, return an explicit partial-result marker, or fail.

## 10. Scale-safe implementation

### 10.1 Stack safety

User-controlled depth must not consume the native call stack without a demonstrated safe bound imposed by an external interface.

Use explicit work stacks or otherwise stack-safe designs for:

- parsing;
- formatting and rendering;
- AST or semantic traversal;
- type traversal;
- ownership analysis;
- HIR and SSA lowering;
- verification;
- identity and hashing;
- diagnostics;
- serialization and deserialization;
- destruction of deeply nested structures.

Add deep generated tests.

### 10.2 Arithmetic and allocation

Use:

- checked arithmetic for lengths, offsets, sizes, and work estimates;
- fallible allocation and reservation where large user input is involved;
- incremental or streaming processing when full materialization is unnecessary;
- staged publication;
- prompt release of temporary memory.

Do not preallocate a published maximum.

Do not use saturating arithmetic to conceal overflow in a value that controls identity, indexing, allocation, admission, or correctness. Saturation is acceptable only for explicitly approximate telemetry.

### 10.3 Identifiers and encoded widths

Use wide or segmented identifiers for user-scale program structures.

Requirements:

- public semantic IDs are not raw vector positions;
- stable semantic identity is distinct from snapshot-local dense indexing;
- every narrowing conversion is checked;
- no `as u8`, `as u16`, or similar cast may silently wrap a user-scale value;
- malformed encoded operands fail before indexing;
- a compact encoding has a wide fallback;
- representation overflow is not reported as semantic invalidity.

Breaking the current bytecode and serialized formats is allowed and expected when necessary.

### 10.4 Complexity

Do not solve poor complexity by rejecting input.

Look for:

- repeated global scans;
- nested linear lookups;
- cloned whole-program structures;
- duplicate serialization and hashing;
- quadratic CFG and dominance work;
- per-node heap allocation;
- repeated string names where interned or direct IDs are appropriate;
- duplicate witness and cleanup metadata;
- always constructing all runtime representations.

Profile before adding custom scheduling or parallelism.

## 11. Semantic workspace authority

### 11.1 Core model

The target authority is a typed semantic workspace snapshot, not a text parse tree with extra metadata.

Separate:

- semantic entities and owned nodes;
- reference and dependency edges;
- presentation and source attachments;
- derived analysis;
- compiled artifacts;
- cache state.

Use an ownership or containment tree where the language has ownership, with explicit graph edges for references and dependencies. Do not assume that a universal graph database is required.

### 11.2 Stable identity

Use stable logical identities for mutable entities, bindings, and nodes.

Identity must survive, where meaning permits:

- rename;
- movement;
- formatting;
- file regrouping;
- unrelated edits;
- projection changes.

Names, paths, spans, source order, and formatting are attributes.

Use generation, workspace namespace, revision preconditions, or equivalent checks to reject stale references.

Use content hashes selectively for immutable snapshots, immutable definitions, artifacts, cache keys, and transfer integrity. Do not content-address every mutable node or make small edits cascade through unrelated identities.

### 11.3 Incomplete programs

The workspace must eventually represent as first-class states:

- typed holes;
- untyped holes;
- unresolved references;
- ambiguous choices;
- type and effect mismatches;
- missing fields, arms, parameters, or declarations;
- import errors;
- conflict nodes;
- recovery nodes.

Preserve all sound type, binding, effect, capability, and ownership information available around an error.

Incomplete snapshots are valid editing states. They are not executable releases.

### 11.4 Transactions

Semantic edits are typed operations, not primarily text patches.

Useful operations include:

- create or delete entity;
- insert, move, or replace node;
- rename declaration or binding;
- set a type, field, reference, effect, or capability;
- rewire a call or dependency;
- introduce, refine, or fill a hole;
- apply a legal refactoring;
- resolve a conflict;
- commit or abort.

Every transaction must:

- name a base revision or equivalent precondition;
- validate operation shape and identities before publication;
- apply atomically;
- preserve the old snapshot on failure or cancellation;
- publish one new revision on success;
- return semantic diff, diagnostics, and invalidation information;
- support batching.

A text patch is an importer into a transaction, not the foundational edit representation.

### 11.5 Queries

Queries are deterministic and revision-labelled.

Provide useful semantic queries for:

- entity lookup;
- qualified resolution;
- definition and references;
- callers and callees;
- actual and expected type;
- typing context;
- effects and capabilities;
- ownership, movement, and borrowing;
- diagnostics;
- hole context;
- legal constructors and edits;
- dependencies and impact;
- search by name, type, operation, or capability;
- concise semantic slices.

Large result sets require stable order, filters, pagination, and continuation.

Return compact headers and IDs first. Allow selective expansion. Never claim a truncated result is complete.

### 11.6 Projections

Support multiple projections from the same semantic authority:

- concise human-readable text;
- verbose diagnostic text;
- structured debug or interchange form;
- semantic diff;
- conventional text diff;
- IDE and visual views;
- compiled artifacts.

The current line-oriented syntax is a replaceable importer and renderer. It is not a compatibility promise.

Formatting, comments, trivia, spans, and file placement do not own semantic identity.

### 11.7 Direct compilation

The compiler consumes a complete semantic snapshot directly.

The conceptual path is:

```text
semantic workspace snapshot
    -> name, type, effect, capability, and ownership analysis
    -> canonical typed core
    -> verified executable representation
    -> one production execution path
```

Text import constructs or updates the semantic model. It must not remain a privileged sibling compiler path.

Tests must prove that direct semantic compilation does not render and reparse text.

### 11.8 Persistence and collaboration

Begin with in-memory immutable or copy-on-write snapshots.

Add a transaction log, embedded database, binary snapshot, CRDT, or distributed store only after measurements establish a need for crash recovery, retained scale, concurrent writers, or collaboration.

Do not begin the semantic reset by building a distributed database.

## 12. Compiler and IR architecture

### 12.1 Clear responsibilities

Each representation boundary must have one clear responsibility.

Do not independently reproduce the same semantic facts in source records, HIR witnesses, memory descriptors, SSA witnesses, bytecode metadata, contract registries, prepared descriptors, and runtime tables unless each copy serves a real boundary or measured backend need.

Prefer typed construction that makes invalid internal states difficult to express.

### 12.2 Verification boundaries

Keep rigorous verification at:

- untrusted text or semantic-operation input;
- untrusted serialized workspace or artifact input;
- package and path boundaries;
- process and daemon messages;
- capability grants;
- persisted data;
- executable IR or bytecode loaded from outside the process;
- relocation and executable-memory installation;
- FFI, SQLite, and operating-system interfaces.

Within one synchronous trusted compiler pipeline, do not repeatedly hash, serialize, reconstruct, and independently verify the same data without a demonstrated threat or bug model.

### 12.3 Generic validity path

Every valid supported program needs a generic compilation and execution path.

Optimization and specialization may improve it. They must not decide whether the program is valid.

Unsupported specialization falls back to generic lowering. It does not become a count error.

### 12.4 Optimizations

Add or keep an optimization when:

- profiling identifies a material bottleneck;
- the transformation has a clear semantic contract;
- validation or differential testing is adequate;
- end-to-end benefit exceeds compile-time, memory, code-size, and maintenance cost.

A proof-producing optimizer must justify its proof and reconstruction cost at a real trust boundary or by demonstrated reliability value.

Freeze or delete optimizing machinery that lacks representative benefit.

### 12.5 Representation count

Do not add IR levels merely to make the architecture look sophisticated.

Add a representation when it enables a distinct invariant, transformation class, target, or performance property that cannot be owned cleanly elsewhere.

Remove a representation when it mainly mirrors another and forces repeated feature implementation.

## 13. Runtime architecture

### 13.1 One product path

Select one coherent production execution architecture by measurement.

A tiered runtime may be one product path when tiers are internal policy and share semantics. It must not become several public language implementations.

Public CLI engine selection is diagnostic tooling at most, not a compatibility commitment.

### 13.2 Reference executor

A small interpreter or evaluator may remain for:

- semantic differential testing;
- compile-time evaluation where justified;
- debugging;
- tiny cold tasks when measured.

It does not require complete product feature parity unless it is part of the selected production architecture.

### 13.3 Backend selection

Compare the custom backend with mature alternatives such as Cranelift and, only when justified, LLVM.

Measure:

- compile latency;
- time to first result;
- steady-state execution;
- memory;
- generated code size;
- release binary size;
- supported targets;
- safety;
- maintenance;
- integration complexity.

Do not retain custom code because it is custom. Do not adopt a dependency because it is popular.

### 13.4 Avoid unconditional representation work

Do not build bytecode, SSA snapshots, optimization certificates, native objects, and metadata tables on every execution when the selected path does not need them.

Construct representations lazily or not at all according to the selected architecture.

### 13.5 Runtime resource policy

Execution time, memory, output, concurrency, and cancellation are host policy.

They are not language validity.

Long-running or nonterminating programs require cancellation and isolation at untrusted boundaries, not a hidden source-shape quota.

## 14. Memory direction

The current product direction is collector-free ordinary execution and non-tracing memory management.

Do not introduce a tracing collector as a convenience fallback unless the user explicitly changes direction.

Ordinary source should not expose:

- named implementation lifetimes;
- retain and release;
- a general `free`;
- raw pointers;
- memory-engine selection;
- backend-specific representation controls.

Prefer a small coherent combination of:

- immediate and static values;
- unique or affine ownership;
- lexical or invocation regions;
- arenas for phase-local data;
- coarse immutable sharing;
- compiler-inserted cleanup.

Establish cleanup authority once and consume it consistently.

Minimize:

- per-node metadata;
- witness tables;
- duplicate owner/place maps;
- cleanup-plan duplication;
- reference counting on fine-grained compiler structures;
- copying between nearly identical representations.

Memory safety, deterministic cleanup where promised, cancellation safety, and failure atomicity are mandatory. Internal mechanisms may change radically.

## 15. Repository architecture

### 15.1 Organize by cohesion

Files and directories may be as large or wide as coherent responsibility requires.

Split when it improves:

- ownership;
- testing;
- navigation;
- compilation isolation;
- retrieval;
- platform separation.

Do not split to satisfy numeric topology policy.

Remove:

- numbered `impl_XX` and `helpers_XX` shards;
- one-child directory ladders;
- facade files that only include fragments;
- artificial file boundaries inherited from old line or directory rules.

Recombine first, then choose natural boundaries.

### 15.2 Crate boundaries

A crate boundary is justified by at least one of:

- a real trust or unsafe boundary;
- an independently useful library API;
- a materially different build target or platform;
- measured compile-time isolation;
- a distinct low-coupling subsystem.

Many crates do not automatically mean modularity.

Merge crates whose primary purpose is exchanging internal contracts, digests, witnesses, or re-exports.

### 15.3 Current reset hotspots

At the beginning of the reset, known high-priority areas included:

- `crates/lkjscript-core/src/limits.rs`;
- `crates/lkjscript-core/src/profile/`;
- `crates/lkjscript-core/src/budget/`;
- `crates/lkjscript-compiler/src/budget/`;
- source parser and scale tests;
- ownership analysis;
- `crates/lkjscript-compiler/src/hir/memory_plan/`;
- SSA verification and structural witness limits;
- bytecode operand and index widths;
- proof-oriented optimization and scheduled discovery;
- the syntax-shaped Semantic Source schema;
- internal contract digests and prepared identities;
- runtime engine multiplication;
- speculative scheduler, process-cell, database, and daemon surface.

Verify current paths. When a hotspot is resolved, delete stale references to it rather than marking a permanent checklist complete.

### 15.4 Dependencies

Third-party dependencies are allowed.

Evaluate:

- maintenance;
- security;
- portability;
- compile time;
- binary size;
- runtime performance;
- API stability;
- replacement cost.

Prefer a mature dependency when it removes substantial custom machinery or risk. Prefer owned code for a small performance-critical mechanism when measurement supports it.

A zero-dependency claim is not a goal.

### 15.5 Unsafe code

Keep unsafe code in narrow named mechanism boundaries with:

- a documented safe caller contract;
- explicit invariants;
- focused tests;
- malformed-input coverage;
- sanitizer, Miri, fuzz, or property testing where useful.

Do not spread unsafe code to save unmeasured time.

## 16. Documentation authority

### 16.1 Active set

Converge toward a small non-overlapping set:

- `README.md` — identity, prerequisites, build, first successful use, links;
- `docs/authority.md` — authority dimensions and conflict rules;
- `docs/spec/language.md` — intended normative language semantics;
- `docs/spec/workspace.md` — semantic workspace and agent-editing contract;
- `docs/status.md` — actual implemented capability and known gaps;
- `docs/architecture.md` — current topology, data flow, ownership, and trust boundaries;
- `docs/performance.md` — benchmark methodology, compact baselines, active performance decisions;
- `docs/roadmap.md` — `Now`, `Next`, and `Later`;
- `docs/decisions/` — sparse durable decisions and supersessions.

Exact names may change. Roles may not overlap.

### 16.2 Document labels

Every substantive document must make clear whether a statement is:

- normative;
- currently implemented;
- target architecture;
- experimental;
- historical;
- planned.

Do not mix these categories without labels.

### 16.3 Executable facts

Use executable sources for exhaustive facts:

- Cargo metadata for crate graph;
- CLI definitions and tests for commands;
- schemas and types for wire structure;
- compiler tests for accepted and rejected semantics;
- benchmark harnesses for measurement;
- generated references for operation, opcode, capability, diagnostic, or schema tables.

Prose explains intent, rationale, and consequences.

### 16.4 Documentation checks

Check:

- internal links;
- stale paths;
- generated references;
- executable examples;
- code snippets where practical;
- ownership of claims.

Do not build a documentation system that hashes arbitrary prose and claims the hash proves correctness.

### 16.5 Prohibited documentation bureaucracy

Do not introduce or maintain:

- digest markers embedded in prose;
- global platform revisions for unrelated changes;
- hand-authored public-fact shards;
- status closure graphs;
- capsule manifests;
- evidence records for every implementation commit;
- architecture graphs manually duplicating Cargo;
- committed agent checkpoints or scratch plans as product authority;
- an active archive of superseded plans.

Use Git history.

### 16.6 Decisions

Write an ADR only when the choice is durable, non-obvious, and expensive to rediscover.

An ADR includes:

- context;
- options considered;
- evidence;
- decision;
- consequences;
- reversal condition;
- status: proposed, accepted, or superseded.

An ADR does not duplicate current status, specification, or implementation inventory.

## 17. Performance discipline

### 17.1 Measure whole outcomes

Measure:

- agent query and transaction latency;
- request and response bytes;
- round trips;
- cold and warm load;
- from-scratch and incremental analysis;
- executable lowering;
- time to first result;
- steady-state execution;
- peak resident memory;
- allocation count and bytes;
- copies;
- generated code size;
- release binary size;
- cache hit rate and retained memory;
- failure and cancellation paths.

### 17.2 Representative workloads

Use a matrix covering:

- small one-shot programs;
- many small functions;
- large functions and CFGs;
- arithmetic and branches;
- calls;
- products, enums, and matching;
- bytes, strings, lists, and allocation;
- ownership and cleanup;
- errors and early exits;
- host boundaries;
- generated scale;
- realistic applications as the language supports them.

Brainfuck is not required.

### 17.3 Experimental method

Before measuring:

- state the hypothesis;
- define the workload;
- define selection criteria;
- define reversal conditions.

During measurement:

- use release builds;
- record machine and compiler metadata;
- use repeated runs;
- report median and tail behavior;
- isolate cold and warm cases;
- avoid comparing paths with different semantics.

After measurement:

- keep the harness;
- store raw data in `target/` or CI artifacts;
- commit only compact baselines and decisions;
- remove losing architecture instead of maintaining permanent comparison parity.

### 17.4 Regression policy

Use noise-aware thresholds. Do not turn one noisy machine result into a hard semantic gate.

A performance regression may be accepted when it buys a larger correctness or architectural simplification, but the trade must be explicit and revisited.

## 18. Failure atomicity and determinism

### 18.1 Failure atomicity

The following require staged construction and a single publication point:

- semantic transactions;
- compilation cache updates;
- snapshot persistence;
- package or artifact publication;
- executable-memory registration;
- runtime state replacement;
- database updates;
- control-store updates.

On validation failure, cancellation, timeout, allocation failure, I/O failure, or backend failure, preserve the prior published state.

### 18.2 Determinism

Given the same semantic snapshot, explicit target, options, and capabilities:

- analysis has stable meaning;
- diagnostics and diffs have stable order;
- serialization is deterministic where claimed;
- parallel scheduling does not alter meaning;
- cache state does not alter meaning;
- selected runtime tiers do not alter semantics.

A deadline may determine whether a request completes. It may not change the result of a completed request.

## 19. Security boundaries

Do not weaken genuine boundaries while removing internal ceremony.

Retain or improve:

- path containment;
- symlink policy;
- exact import resolution;
- capability checks;
- package and artifact validation;
- process framing and authorization;
- persisted-data validation;
- W^X executable memory;
- relocation checks;
- generated-entry validation;
- FFI contracts;
- SQLite transaction safety;
- operating-system error handling.

Validate once at entry, then use typed internal data.

## 20. Change protocol

For every substantial task:

1. inspect current instructions, worktree, branch, and recent history;
2. read active authority documents and relevant code;
3. identify the highest-priority dependency-closed problem;
4. classify affected bounds and validation;
5. state a testable hypothesis;
6. run focused baseline tests;
7. implement the simplest direct cutover;
8. delete the old path and obsolete tests in the same change;
9. add positive scale, semantic, failure, and boundary tests;
10. profile when the change is performance-sensitive;
11. update owning documentation;
12. run the complete relevant verification;
13. inspect the diff for duplicated architecture, stale paths, unchecked narrowing, and accidental compatibility;
14. commit cohesively;
15. report exact evidence and remaining risk.

Do not spend the whole task planning. Do not start implementation before understanding the dependency chain. Balance both requirements by moving quickly from a bounded audit into a complete vertical.

## 21. Tests

Tests own semantic laws and boundary invariants, not prototype accidents.

Preserve or add tests for:

- type, effect, capability, ownership, and control semantics;
- path and protocol boundaries;
- malformed artifacts;
- executable-memory safety;
- failure atomicity;
- cancellation;
- deterministic diagnostics and diffs;
- stable identities;
- stale revisions;
- direct semantic compilation;
- positive scale beyond former boundaries;
- differential execution;
- selected runtime behavior.

Delete or replace tests that canonize:

- arbitrary counts;
- old syntax compatibility;
- old artifact bytes;
- obsolete engine parity;
- platform revision rituals;
- digest-marker ecosystems;
- repository shape rules.

Generated fixtures are preferred for scale. Keep quick CI and opt-in stress geometry distinct.

## 22. Verification

Use focused commands during development.

Before completion, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo build --workspace --release --locked
```

Run retained Docker verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run additional relevant tools at boundaries:

- generated scale suites;
- differential tests;
- fuzzing;
- Miri;
- ASan, LSan, and TSan;
- property tests;
- malformed decoder tests;
- cancellation and allocation-failure tests;
- release benchmarks;
- documentation link and example checks.

Never claim a command passed unless it was run after the final relevant change.

If a command fails for an environmental reason, report the exact command, output category, and unaffected evidence. Do not silently omit it.

## 23. Definition of done

A change is done only when:

- it solves the dependency-closed problem, not one symptom;
- the old architecture is deleted rather than preserved beside the new;
- semantic validity is not replaced by another arbitrary quota;
- every remaining bound has a justified class;
- user-scale narrowing is checked or redesigned;
- failure leaves no partial publication;
- focused positive and negative tests pass;
- active documentation is truthful;
- verification has been run;
- the final report distinguishes implemented, measured, untested, and planned work.

For limit-removal work, completion additionally requires positive execution beyond former boundaries.

For semantic-workspace work, completion additionally requires direct lowering without a text round trip.

For runtime-selection work, completion additionally requires representative measurements and deletion or demotion of losing product paths.

## 24. Prohibited patterns

Do not add:

- “temporary” arbitrary maxima without a deletion in the same change;
- public compiler profiles listing internal node categories;
- unchecked integer narrowing;
- silent truncation;
- recursive user-depth traversal without stack-safety evidence;
- count rejection as protection against poor complexity;
- universal authority hashes inside one trusted process;
- duplicate producer and verifier models without a real boundary;
- compatibility layers by default;
- `v2` architecture trees;
- feature parity matrices for discarded engines;
- numbered implementation shards;
- speculative scheduler infrastructure without profiles;
- source-spans or names as semantic identity;
- full-repository agent context when a semantic slice is available;
- model calls in correctness gates;
- raw benchmark dumps or agent scratch state as permanent documentation.

## 25. Agent report

A substantive final report must state:

- starting and ending commits;
- worktree and branch status;
- architectural result;
- deleted and merged components;
- former limits crossed;
- remaining bounds and their classification;
- tests and benchmarks run;
- performance results and environment;
- documentation authority changes;
- exact verification results;
- known risks and untested boundaries;
- next highest-priority dependency-closed work.

Do not report planned architecture as implemented. Do not conceal fallback, skipped validation, or unsupported scale.
