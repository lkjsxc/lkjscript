# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository. A deeper `AGENTS.md` may add procedures
for a genuine ownership boundary, but it may not weaken any applicable rule in this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, revision metadata, and
handoffs.

## Mission

Build `lkjscript` as a meaning-first, agent-native semantic software platform.

The primary editable authority for an lkjscript program is its validated typed meaning graph and
immutable development history. Coding agents and humans should be able to create, inspect, change,
validate, test, build, run, package, diagnose, and evolve useful software through the public
lkjscript CLI without reconstructing the graph through a custom Python, shell, Rust, or generated
source builder.

Humans remain first-class for intent, product judgment, governance, security policy, explanation,
review, operations, and acceptance. Coding agents are first-class program authors and maintainers.

Optimize jointly for:

- semantic correctness;
- complete useful applications;
- direct CLI-native authoring;
- compact exact agent interactions;
- weak-model success;
- low correction depth and provider cost;
- automatic immutable development history;
- deterministic builds and artifacts;
- explicit authority and recoverable operation;
- bounded resource use;
- maintainable ownership;
- independently checkable evidence; and
- long-term performance.

Do not optimize for novelty, feature count, benchmark theater, syntax fashion, roadmap inertia,
sunk cost, compatibility with superseded repository states, or preservation of an implementation
merely because it was difficult to create.

## North star

A coding agent should be able to enter an ordinary project directory and, through public
`lkjscript` commands:

1. discover the exact semantic project and current revision;
2. request only the relevant typed context;
3. propose one bounded semantic change;
4. validate it through the same owner that would publish it;
5. publish exactly one immutable revision and exact revision record;
6. inspect the resulting semantic diff and history;
7. build, test, and run a named target derived from that revision;
8. recover after interruption or stale state without silent retry; and
9. continue development without reading or regenerating an enormous construction script.

The meaning graph is the center. Text documents, command streams, JSON, rendered source-like views,
TUI views, generated bindings, compiled forms, artifacts, caches, and Git diffs are interfaces,
proposals, distribution objects, or derived views according to their exact contracts. None silently
becomes a second editable source of truth.

A rich TUI editor with an explorer is a long-term representative application, not permission to add
speculative terminal, filesystem, event-loop, collection, or text machinery without a complete
consumer and exact authority model.

## Authority and precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. The accepted semantic development repository and its exact revision records.
7. Generated descriptions derived from one executable owner.
8. `docs/status.md`.
9. `docs/architecture.md`.
10. Current structured evidence and `docs/performance.md`.
11. `docs/roadmap.md`.
12. `README.md`.
13. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and
    historical documents.

Newer verified checkout state outranks older plans and remembered repository state.

A campaign prompt owns one campaign's objective, hypotheses, gates, and handoff. It does not become
permanent semantic authority.

An old prompt is historical evidence unless the active task explicitly selects it.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

Do not let generated documentation, a checked-in artifact, a test fixture, or a Git commit message
silently outrank the semantic owner that produced it.

## Repository safety

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -1 --oneline
find .. -name AGENTS.md -print
```

Read every applicable instruction file.

Preserve unrelated modified and untracked work.

Reading in-scope files, editing in-scope files, and running non-destructive validation are authorized
for implementation tasks unless the active task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state without authorization for that action.

Repository permissions are not user authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, raw
provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
unclear licensing.

Keep scratch state, destructive experiments, unsanitized measurements, downloaded research, and
losing prototypes outside the repository unless a retained artifact has a named consumer.

Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
irreproducible observations explicitly.

## Backward compatibility

Backward compatibility is absent unless the active user explicitly requires it.

Use incompatible-change freedom to converge on one coherent design.

After cutover, delete old readers, writers, aliases, fallbacks, compatibility tests, migration-only
code, stale examples, dormant flags, duplicate protocols, and superseded documentation.

Do not introduce editions, dual success paths, hidden fallback, automatic old-format adoption, or
silent migration as insurance.

A direct break still requires a complete replacement, exact rejection of predecessors, focused
negative tests, updated documentation, and a verified current application.

Incompatible-change freedom is not permission for an unverified rewrite.

## Meaning graph as development authority

Each accepted workspace revision has one authoritative typed meaning graph.

The graph may include program declarations, function bodies, tests, build targets, release
projections, application composition declarations, interface-role declarations, and other
development meaning only when their authority domain and consumer are explicit.

Names, formatting, source positions, file paths, command order, generated indexes, and renderings are
not semantic identity.

A human-readable document is a proposal or view. It may be convenient and editable, but accepted
meaning never depends on preserving, reparsing, or diffing its formatting.

A command stream is a proposal and may be retained as an exact revision recipe or audit fact. It is
not the current program authority.

A checked-in application artifact may be immutable distribution authority under its specification.
It is not an acceptable replacement for a maintained development repository when the application is
still developed in this repository.

A custom program that reconstructs the graph is not maintained semantic source. It is a temporary
migration tool, a test generator, or a losing duplicate authority and must not remain as the normal
development path.

Every first-party maintained lkjscript application must have a public-CLI-native path from its
tracked semantic development repository to its validated release and application artifacts.

## Semantic development repository

A semantic development repository owns one workspace continuity and its immutable accepted
development history.

It must expose:

- exact workspace identity;
- one selected current revision;
- immutable revision objects or artifacts;
- canonical revision records;
- exact parent continuity;
- durable entity allocation and tombstones;
- semantic diff facts;
- named build targets;
- validation, history, diagnosis, backup, and reconstruction;
- strict project discovery; and
- direct rejection of foreign, malformed, stale, incomplete, or excessive state.

Git and the semantic development repository are distinct domains.

Git owns collaboration and distribution of repository files. The semantic repository owns the
accepted evolution of lkjscript meaning. Neither is inferred from the other.

Do not require agents to reverse-engineer semantic history from binary Git diffs.

Do not use a Git commit hash as workspace identity, semantic entity identity, revision identity,
release identity, application identity, or authorization.

Tracked first-party semantic repositories must be portable, bounded, deterministic under their
documented trust model, and usable from a fresh checkout through public commands.

A project path is a locator. It is not semantic identity.

## Automatic development history

Every successful public semantic development mutation must publish exactly one immutable workspace
revision and one canonical revision record.

Validation-only and rejected operations publish no revision, record, durable identity, target
artifact, or HEAD change.

A revision record must bind at least:

- workspace identity;
- exact base and result revisions;
- parent and result snapshot facts;
- canonical accepted change-set digest;
- exact semantic diff digest and completeness facts;
- created, deleted, replaced, and modified durable entities;
- function-body change facts without inventing cross-revision local identity;
- build-target changes;
- publication outcome; and
- the exact contract version used to interpret the record.

Optional intent, actor label, tool label, or observed time is untrusted metadata. It must be bounded,
validated, clearly separated from semantic meaning, and irrelevant to release or application
identity unless a future specification deliberately says otherwise.

Do not store hidden chain of thought, provider transcripts, raw prompts, or secrets as revision
metadata.

A normal accepted semantic development command should record itself without requiring a manually
written commit message.

History is append-only. Revert publishes a new validated revision. It does not rewrite or delete
accepted history.

Branching, merging, rebasing, distributed synchronization, and conflict-free replication require
current consumers and exact semantics. Do not copy Git features speculatively.

## Public semantic CLI

The public `lkjscript` CLI is the primary development interface.

Raw Rust constructors, private library calls, test-only builders, custom generators, and direct
store mutation are not acceptable first-party authoring paths.

The CLI must support ordinary human and coding-agent use.

Human mode must provide deterministic bounded help, status, inspection, history, diff, validation,
build, test, run, and actionable errors.

Machine mode must provide one strict versioned typed contract with exact framing, request
correlation, stable error codes, explicit omissions, and no progress contamination.

A caller-owned foreground session may reuse validated state and local handles. It is not authority, a
daemon, a queue, or a scheduler.

The normal CLI should discover a project from the current directory or an explicit path. Users should
not need to supply workspace IDs, current revisions, schema digests, or absolute internal state paths
for ordinary work.

Convenience never weakens exactness:

- project discovery resolves to one exact workspace;
- reads report the exact selected revision;
- mutations bind an exact expected revision;
- stale state rejects;
- no mutation is silently retried;
- selectors reject ambiguity;
- session-local handles never persist as semantic identity; and
- all accepted changes still pass the same typed validator.

Relative paths are normal public input. Resolve them against an explicit documented base, canonicalize
and revalidate authority boundaries, reject unsafe traversal and symlink substitution, and keep paths
out of semantic identity.

The CLI contract should be generated from or mechanically checked against one typed executable owner.

Do not make routine agents request a global schema dump. Provide compact orientation, targeted
context, on-demand expansion, and exact unchanged/delta responses.

## CLI-native change model

A semantic change may be submitted as one complete high-level command, one bounded atomic change
bundle, or one request in a foreground session.

The change model must support multi-entity atomicity when a valid transformation cannot be expressed
as independently valid intermediate revisions.

Draft state is not accepted meaning.

If an interactive draft or change session exists, define its owner, lifetime, bounds, crash behavior,
identity domain, validation points, and whether it is retained. Do not let a draft become hidden
authority.

Use exact base-bound selectors. A convenient name or path may resolve only when unambiguous in the
selected revision.

Function-local operation references remain revision- and function-bound. A patch may target an exact
local item in one base revision without granting it durable continuity.

Prefer bounded subgraph edits and declaration-level operations over resending an entire large
application when they materially reduce context and correction risk.

Retain whole-function replacement as a simple oracle and escape hatch when it is the clearer exact
operation.

Validate-only and commit must share parsing, normalization, semantic validation, artifact
preflight, response preflight, and relevant resource checks.

## Build targets and deterministic derivation

Build configuration for maintained lkjscript software belongs in an exact versioned target graph, not
in a custom Python dictionary, shell command sequence, Cargo build script, or undocumented set of
manual arguments.

A target declaration may describe:

- reusable release projection;
- exact package root;
- exports;
- exact dependencies and imports;
- immutable tests;
- application composition;
- stateful or pure profile roles;
- query and response roles;
- host-interface requirements;
- application cases;
- generated binding views; and
- product packaging checks.

Target names are lookup metadata. Target identity and references use the exact graph domain.

A build selects one exact workspace revision and exact dependency artifacts.

No target resolves `latest`, a mutable registry coordinate, or an unverified path at the semantic
boundary.

Release and application artifacts remain separate authority domains. Development target declarations
do not silently become runtime grants, instance state, deployment authority, or executable identity.

Generated bindings are derived views. Prefer direct validated artifact descriptors when they are
simpler. If bindings are retained, one generator owns them, stale output fails, and clients
independently validate the artifact facts they use.

A checked-in derived artifact is allowed when packaging requires it, but the repository must prove
that public CLI build reproduces it from the tracked semantic repository.

## Prohibition on semantic build scripts

Do not retain `build.py`, `generate.py`, shell heredocs, private Rust builders, or similar programs as
the primary way to construct or evolve a maintained meaning graph.

Do not replace a Python graph builder with a Rust graph builder, a larger JSON fixture, a generated
source file, a Cargo `build.rs`, or a macro that owns the same duplicate meaning.

Temporary migration code may use a historical recipe to establish an initial semantic repository.
It must be isolated, independently checked against the old artifact, and deleted after direct
cutover.

Acceptance, workload, fault-injection, and measurement scripts may remain when they exercise public
product boundaries and do not own application meaning or build configuration.

## Application-first closure

Every substantial platform campaign selects a valuable complete application or product workflow that
determines whether the platform change succeeds.

The application owns domain state, validation, ordering, decisions, and typed outcomes in lkjscript
semantics.

A host client may own transport, rendering, explicit file selection, narrow adapter execution,
process lifecycle, and independent assertions. It may not own hidden business state or policy.

Build the smallest complete product slice first. Add a language, runtime, storage, interface, or
tooling mechanism only for an exact blocker revealed by that slice, then return to the product.

A capability is incomplete when the host reconstructs private state, suppresses invalid requests,
parses opaque responses for domain meaning, or remains the real workflow controller.

Run the completed product from a fresh checkout through public release binaries and dogfood the
semantic CLI on a real maintained-application change before completion.

Delete productless infrastructure, losing prototypes, stale examples, and intermediate artifacts
without a retained consumer.

## Semantic authority

Each accepted authority unit has one authoritative typed representation.

Natural-language intent and model output are untrusted proposals.

Text, JSON, documents, context packets, reviews, caches, indexes, IR, bytecode, profiles, memory
plans, machine code, renderings, terminal output, and generated bindings are proposals, views, or
derived state unless a specification deliberately assigns a narrower immutable authority.

No proposal, view, cache, generated form, or derived form bypasses deterministic validation.

Accepted authority never depends on rendering and reparsing.

Unknown, malformed, ambiguous, unsupported, foreign-domain, noncanonical, truncated, oversized,
duplicate, conflicting, stale, or trailing forms reject.

Derived facts never become a second mutable source of truth.

A human-readable source-like form is acceptable only when it deterministically normalizes through the
same typed validator and cannot silently diverge from accepted meaning.

## Identity and continuity

Assign durable identity only for a concrete continuity, sharing, reference, repair, attribution,
import, export, history, provenance, targeting, durable instance, product entity, build target, or
operational consumer.

Names, formatting, positions, order, paths, hashes, compiler indexes, artifact offsets, storage keys,
runtime handles, queue positions, process IDs, and addresses are not semantic identity unless a
closed contract assigns a narrower role.

Workspace, revision, revision record, change set, build target, release, application, instance,
product entity, command, outcome, grant, interface, adapter, deployment, executable, checkpoint,
backup, cache entry, profile, session handle, and runtime handle are distinct domains.

A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
capability identity.

Identity-preserving change requires an explicit validated rule.

Deleted durable identities are not silently reused.

Multiple exact versions may coexist only when references remain unambiguous.

A filesystem path locates an authority boundary; it does not create semantic authority.

Function-local identities remain bound to one function and exact revision unless a specification
adds a real continuity consumer.

## Publication and durability

Published workspace revisions, revision records, releases, applications, instance revisions, host
outcomes, declared authoritative checkpoints, backups, and other declared durable objects are
immutable within their domains.

Every durable namespace has one publication authority.

One successful publication creates exactly one accepted durable outcome.

Rejection and validate-only publish nothing and consume no durable identity.

A semantic no-change must not consume a revision merely to return a response.

Success is acknowledged only after the documented synchronization boundary.

A possibly visible but unconfirmed outcome is reported as unknown and never silently retried.

Recovery, replay, retention, checkpointing, compaction, corruption, backup, restore, deletion, and
garbage collection are explicit and validated.

Semantic state publication and externally visible host work remain separate unless atomicity is
proved.

Output failure cannot retroactively undo accepted authority.

A workspace revision and its revision record must not become independently visible in conflicting
combinations.

## Artifact and authority domains

Workspace snapshots own accepted development meaning at exact revisions.

Revision records own accepted development-history facts.

Build targets own exact derivation intent.

Releases own exact reusable semantics.

Applications own exact runnable closure and declared interfaces.

Instances own durable state and transition history.

Grants own authority.

Deployments own process, machine, account, namespace, and resource placement.

Backups own one exact transferable closure under their explicit contract.

Executables, compiled units, indexes, generated bindings, profiles, ordinary checkpoints, and build
outputs are derived unless a specification deliberately promotes one with exact validation.

Caches are disposable acceleration.

Compilation consumes one immutable accepted revision or independently validated distribution state
and lowers only the complete selected closure.

Coordinates, user versions, aliases, paths, and mutable lookup results are not exact dependencies.

## Mutation and query separation

Mutations and observations are separate semantic contracts.

A workspace mutation may reject, validate without publication, or publish exactly one revision and
revision record.

An application mutation may decline, report unchanged, publish one completed state, or publish one
suspended state and command under an application-owned typed decision.

A declined or unchanged application mutation publishes no state revision, command, attempt, outcome,
or HEAD change.

A pure query names exact authority and returns a typed value without publishing semantic or durable
state.

A query must not be implemented as a no-op event.

A product client must not decode private state as a second query authority.

Query output failure has no rollback meaning because no semantic publication occurred.

Query pagination, ordering, truncation, omissions, revision binding, and result-digest behavior are
exact and bounded.

Idempotency, stale-base behavior, and response retention remain explicit.

## Application interfaces and host authority

Pure deterministic computation remains the default.

Ambient host authority is forbidden.

Applications may declare exact host-interface requirements but never grants.

Instances bind requirements to exact grants.

A grant binds the exact sharing domain, interface identity, adapter kind, bounded descriptor,
applicable authority revision, and limits needed to prevent implicit broadening.

Host requests and outcomes are closed typed values.

Opaque bytes are acceptable only when the interface deliberately defines and bounds them.

An adapter cannot invent semantic state, application response, command intent, or authority.

Parallel commands require a current complete application and exact ordering, cancellation,
partial-result, retry, and replay contracts.

A live resource handle, stream, socket, file descriptor, process, timer, terminal session, secret, or
foreign object requires explicit acquire, use, transfer, consume, close, cancellation, timeout,
crash, and cleanup semantics.

Do not add live resources merely for adapter convenience.

Expected workflow outcomes may be nominal data.

Corruption, authority denial, resource exhaustion, infrastructure failure, stale state, and unknown
visibility remain distinguishable.

Non-idempotent work is never silently retried after possible partial execution.

Time, randomness, scheduling observations, terminal input, filesystem observations, and host
observations are explicit when observable.

## Values, language, and representation

Add a language or value capability only for a complete current application or the semantic
development CLI itself.

Text and variable-length collections require exact validation, canonical encoding, bounds,
deterministic equality and order, and a current consumer.

Do not add a conventional source language merely to replace a graph-construction script.

Do not add generics, traits, macros, maps, sets, iterators, mutable builders, slicing, normalization,
formatting, or operator syntax without an exact blocker and complete end-to-end use.

Representation, sharing, allocation, reclamation, checkpoint layout, cache entries, IR, bytecode,
profiles, and native code are unobservable or derived unless a specification deliberately defines a
narrower validated authority.

A simple independent allocation, execution, reconstruction, and build route remains the oracle for
optimized values, storage, caches, compaction, and execution tiers.

Cache miss, eviction, missing derived state, and process restart remain correct.

Full snapshots, deltas, journals, object stores, databases, collectors, bytecode, JITs, schedulers,
and supervisors must beat simpler safe designs on a complete representative workload and must be
deleted when they lose.

## Execution, scheduling, and concurrency

One simple executable route remains the correctness oracle.

Faster tiers are differential until direct cutover is justified.

Values, traps, order, state transitions, host requests, resource semantics, and diagnostics remain
stable across tiers unless accepted semantics deliberately change.

Operational scheduling and language semantics are separate.

Deterministic fuel is not wall-clock time.

A scheduler may use time, priority, quota, or load for admission and fairness only when those choices
do not silently change accepted semantic results.

Observable interruption, yield, cancellation, or timeout requires an accepted contract.

Per-workspace and per-instance mutations remain serial unless a specification defines a stronger
model.

Pure queries may run concurrently only after exact snapshot isolation, lifetime, ordering,
admission, shutdown, and mutation interaction are proved.

Cross-project or cross-instance parallelism requires exact isolation, bounded queues, deterministic
per-authority order, explicit overload, safe shutdown/restart, and differential tests against serial
execution.

Do not add a general async runtime, worker pool, scheduler, actor system, or daemon merely because a
foreground session exists.

## Resource governance

Semantic limits and operational limits are separate.

Semantic limits include fuel, frames, value depth, item count, text bytes, collection elements,
graph expansion, transaction operations, revision-record size, target closure, state size, query
work, and response size.

Operational limits include queue capacity, concurrent compilation, cache bytes, aggregate memory,
open files, adapter operations, process count, CPU share, temporary publication bytes, project count,
and deployment quotas.

Each category states its accounting unit, owner, reservation and release points, peak and retained
accounting, limit source, rejection class, retryability, observability, restart behavior, and
publication interaction.

Check lengths, counts, depth, and reservations before allocation or corresponding work.

Do not present allocator observations as exact semantic accounting.

Shared backing storage, checkpoints, caches, embedded artifacts, or target outputs must not bypass
per-project or per-request limits.

Overload must reject, queue within a bound, or shed work under an explicit policy.

Never create an unbounded hidden queue.

OS controls may strengthen deployment containment. They do not replace semantic or runtime
accounting.

## Security and trust

Accepted semantics cannot express unchecked memory access.

User-controlled depth does not consume unbounded native stack.

No local unsafe Rust is permitted unless the active user explicitly authorizes a replacement after a
concrete need, isolated ownership, safe public contract, and independent tests are recorded.

Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, path safety,
crash consistency, supply-chain trust, and hostile-code isolation are separate contracts.

Treat bytes, text, paths, project locators, filesystem metadata, semantic repositories, revision
records, artifacts, instance files, checkpoints, backups, caches, profiles, IPC peers, terminal
events, and adapter outcomes as hostile input.

A process boundary creates neither semantic authority nor a sandbox.

A runtime supervisor authenticates and authorizes every request under its deployment model.

A path, socket, UID, environment variable, or connection is not sufficient semantic authority by
itself.

Write the threat model before multi-user access, untrusted native code, broad filesystem or network
access, child processes, secrets, terminal control beyond trusted local use, or remote semantic
repository synchronization.

Large input, work, state, history, graph, output, queue, cache, diagnostics, and metrics are bounded,
streamed, paginated, chunked, or policy-controlled.

Human terminal output is escaped and bounded.

Machine output is framed separately and never contaminated by progress text.

Compactness never weakens validation, authorization, identity, durability, diagnostics, or
verification.

## Project discovery and path handling

Ordinary public commands should work from a project root or descendant directory and accept an
explicit project override.

Discovery must find exactly one strict marker under a documented parent-walk policy.

Reject symlinked markers, unsafe traversal, ambiguous nested authorities, nonregular files, foreign
workspace bindings, malformed locators, and path substitution.

Canonical paths are deployment facts and may change without changing semantic identity.

Do not require absolute paths at the public boundary solely for implementation convenience.

When an exact file is deliberately selected, bind the selected authority or content facts needed to
prevent time-of-check/time-of-use substitution.

Build output paths, dependency paths, and attachment paths remain explicit deployment inputs.

## Agent and provider economy

Context budget is a correctness, latency, and cost constraint.

There is no fixed byte ceiling for this root file. Every durable instruction must still pay recurring
context rent. Prefer one precise durable rule over repeated campaign prose, and permit a larger file
when it materially reduces ambiguity, repeated discovery, or correction.

State durable principles once.

Put volatile facts in the active prompt, status, evidence, or generated orientation.

Prefer compact orientation, task-scoped context, exact on-demand expansion, bounded review, stable
diagnostics, validate/apply parity, delta receipts, digest reuse, session-local handles, and explicit
omissions.

After identifying exact owners, stop broad discovery unless evidence invalidates the map.

Build a compact task ledger instead of rereading a long campaign prompt.

Expose only relevant tools and schema roots.

Add prompt rules, examples, schemas, or context only for a measured failure mode.

Compare equal tasks using:

- semantic success;
- unintended changes;
- correction depth;
- repeated discovery;
- action and observation bytes;
- request count;
- process count;
- files and source bytes opened;
- schema bytes emitted;
- build invocations;
- elapsed time; and
- failure quality.

Record provider model identity, token classes, pricing, and monetary cost only when directly exposed.

Bytes are not tokens.

Never infer provider cost from bytes.

The semantic CLI must not claim API-cost savings without exact comparable telemetry. It may claim
measured reductions in bytes, calls, processes, files opened, or elapsed time.

## Code ownership and dependencies

`docs/spec/` owns accepted contracts.

`docs/architecture.md` owns components and trust boundaries.

`docs/status.md` owns implemented reality.

`docs/performance.md` and structured evidence own measurements.

`docs/roadmap.md` owns unresolved consumer-driven reversal gates.

`README.md` owns concise orientation.

Campaign prompts are temporary execution artifacts.

Keep one executable owner for every type, field, operation, query, error, limit, format, interface,
grant, resource, command, target declaration, revision record, and machine descriptor.

Derive views only when staleness cannot be silent.

Organize code around stable ownership and changed-together behavior.

Split large files when bounded review, agent context, test isolation, compile locality, or ownership
clarity improves without duplicating invariants.

Do not preserve arbitrary file-size or directory-count limits.

Prefer the standard library and existing dependencies.

A new dependency must repay its supply-chain, build, binary, audit, operational, and maintenance
cost on the complete workflow.

Git history is the archive for deleted repository material. The semantic repository is the accepted
history for maintained lkjscript meaning. Delete stale active-tree copies and losing generated paths.

## TUI and rich-editor north star

A rich text editor with an explorer is a valid representative future application.

Treat it as several separate contracts:

- text-buffer semantics;
- cursor, selection, undo, and command semantics;
- project-tree semantics;
- filesystem read/write and atomic replacement;
- terminal input, resize, and rendering;
- session lifetime and cleanup;
- resource limits;
- crash recovery;
- application state versus file authority; and
- host trust and path confinement.

Do not call the platform editor-ready merely because it can render text or store bytes.

Do not add broad filesystem authority, terminal control, timers, event loops, mutable text buffers,
search indexes, maps, or concurrency as a speculative bundle.

A bounded editor-core or semantic-explorer prototype may be retained only when it is authored through
the public semantic CLI, has a complete user workflow, and exposes a concrete platform blocker or
delivers independent value.

## Testing and verification

Acceptance tests have exact immutable input, oracle, policy, selection, order, and result.

Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.

For changed boundaries, cover applicable:

- canonical and repeated success;
- validate-only parity;
- no-publication outcomes;
- pure-query no-write behavior;
- revision-record atomicity;
- stale and future base;
- duplicate and idempotency conflict;
- wrong domain;
- malformed, truncated, trailing, and excessive input;
- exact and one-over limits;
- ambiguous selectors;
- foreign authority;
- corruption;
- restart;
- interrupted publication;
- output failure;
- cleanup;
- concurrent access and authority busy;
- overload;
- replay;
- cache miss, hit, eviction, and corruption;
- checkpoint and reconstruction differential;
- build-target determinism;
- generated-view stale detection;
- artifact reproduction;
- backup and restore;
- relative-path and discovery safety;
- public CLI workflows; and
- first-party application dogfooding.

Use a simple independent reference model where semantics become substantial.

Run narrow checks first, then the full repository gates:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every affected retained public workflow and every selected complete application.

Use Miri, sanitizers, mutation tests, property tests, fuzzing, model checking, crash injection,
filesystem fault injection, or cross-platform execution when they target a real risk and are
available.

State scope and limitations.

Do not weaken an invariant test to make implementation pass.

Change specification, implementation, and oracle together when behavior deliberately changes.

## Evidence and claims

Evidence is not semantic authority.

Record exact environment, commit, command, input corpus, output, raw observations, units, sample
selection, warm/cold classification, and unavailable data.

Do not infer tokens from bytes.

Do not infer cost without exact model-specific token and dated price telemetry.

Do not call a warm-host sample cold.

Do not call summed command waits wall-clock time.

Do not call logical accounting RSS enforcement.

Do not call a digest provenance, signature, authorization, or freshness.

Do not call a process, worker, container, private directory, or project locator a sandbox.

Do not call compile success cross-platform support.

Do not claim full-history validation from a current-state check.

Retain bounded evidence for serious losing alternatives, not only the selected result.

Claims must be no stronger than the checkout and reproduced evidence.

## Decision standard

Treat every historical mechanism as provisional except enduring semantic, safety, and authority
contracts in the effective instructions.

Do not preserve a mechanism because it was difficult, planned, shipped recently, or heavily tested.

Reproduce relevant evidence before reversing working behavior.

Prefer complete useful verticals over isolated features.

Prefer one exact path over parallel convenience paths.

Prefer explicit domains over overloaded names.

Prefer direct semantic CLI operations over custom construction code.

Prefer automatic immutable history over manual reconstruction.

Prefer one topology-neutral implementation over duplicated process adapters.

Prefer local features over platforms built for one consumer.

Prefer deletion over permanent deprecation.

Prefer bounded prototypes over speculative architecture.

Prefer high-leverage corrections over accumulated exceptions.

Every retained abstraction, dependency, process, artifact, identity, schema, cache, optimization,
service, scheduler, worker, framework, source form, or generator needs a named current consumer.

Complexity must pay rent in a representative end-to-end workflow.

Before retaining a substantial choice, record:

- consumer;
- obligations;
- reproduced baseline;
- serious alternatives, including deletion;
- expected benefit;
- measured result;
- semantic and operational costs;
- independent oracle;
- direct-cutover deletions;
- stop rule; and
- reversal condition.

Delete losing prototypes completely.

Current absences are not permanent prohibitions without semantic reason.

## Change workflow

1. Inspect checkout, instructions, branch, commit, and unrelated changes.
2. Identify authoritative owners and active format identities.
3. Select the valuable application workflow and freeze an independent oracle.
4. Reproduce the current public authoring and build path.
5. State outcome, completion bar, non-goals, alternatives, and reversal condition.
6. Build the smallest complete semantic-CLI slice.
7. Use it immediately on a real maintained application.
8. Prototype uncertain questions in the smallest dependency-closed form.
9. Select one coherent design from evidence.
10. Implement the full vertical across semantic model, persistence, protocol, CLI, targets, product,
    tests, and documentation.
11. Publish changes through the new path and inspect automatic history.
12. Cut over directly and delete custom builders, duplicate public surfaces, compatibility paths, and
    stale facts.
13. Run focused, full, representative, restart, corruption, publication, and product checks.
14. Measure equal authoring tasks and record only supported savings.
15. Dogfood from a fresh checkout.
16. Leave a compact exact handoff.

Do not stop at a report when a safe complete implementation is authorized and feasible.

Do not scatter partial architecture.

Do not ask the user to decide ordinary engineering details the checkout and evidence can resolve.

## Completion and handoff

A semantic-development capability is complete only when it is:

- discoverable from an ordinary project directory;
- usable through supported public human and machine CLI contracts;
- exact-base-bound;
- automatically recorded as immutable revision history;
- validated by one semantic owner;
- boundedly inspectable and diffable;
- buildable through first-class target declarations;
- reproducible from a fresh checkout without a custom graph builder;
- covered by independent oracles;
- restart, corruption, limit, path, and publication tested;
- resource-accounted where applicable;
- documented by one owner;
- exercised by a useful first-party application;
- measured end to end; and
- free of superseded paths.

Before finishing, report:

- exact starting and ending state;
- selected design and serious rejected alternatives;
- changed authority and format contracts;
- automatic-history behavior;
- CLI workflows;
- build-target behavior;
- first-party application migration;
- deleted custom builders and stale paths;
- validation commands and exact results;
- representative application and authoring-economy evidence;
- provider telemetry only when exposed;
- known limits and trust assumptions;
- reversal gates; and
- every requested action not performed.

Claims must be no stronger than the checkout and reproduced evidence.
