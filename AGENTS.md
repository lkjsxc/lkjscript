# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add rules for a genuine ownership boundary, but it may not weaken any
applicable rule in this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, revision metadata,
evidence, and handoffs.

## Mission

Build `lkjscript` as a meaning-first, agent-native semantic software platform.

The primary editable authority for an lkjscript program is one validated typed meaning graph with
exact immutable development history.

Coding agents and humans must be able to discover, inspect, change, validate, test, build, run,
package, diagnose, recover, and evolve useful software through the public `lkjscript` CLI without
maintaining a custom graph-construction program.

Humans remain first-class for intent, product judgment, governance, security policy, explanation,
review, operations, and acceptance.

Coding agents are first-class program authors and maintainers.

Optimize jointly for:
- semantic correctness;
- complete useful software;
- direct CLI-native authoring;
- compact exact agent interactions;
- weak-model success;
- low correction depth;
- low provider and operator cost when measured;
- automatic immutable development history;
- deterministic artifacts;
- explicit authority;
- recoverable operation;
- bounded resource use;
- maintainable ownership;
- independently checkable evidence;
- interactive responsiveness where required;
- long-term performance;

Do not optimize for novelty, feature count, benchmark theater, syntax fashion, roadmap inertia, sunk
cost, compatibility with superseded states, or preservation of an implementation merely because it
was difficult to create.

## North star

A coding agent should be able to enter an ordinary project directory and use public `lkjscript`
commands to:
1. discover the exact semantic project and selected revision.
2. obtain a compact orientation without loading the whole graph or schema.
3. request only the typed subgraph, constraints, and examples relevant to one task.
4. prepare one bounded exact semantic change.
5. validate through the same owner that would publish it.
6. publish exactly one immutable revision and canonical revision record.
7. receive enough exact continuation state to avoid rediscovering unchanged meaning.
8. inspect the semantic diff, target impact, diagnostics, and history.
9. build, test, and run a named target derived from the selected revision.
10. recover after interruption, stale state, output loss, or unknown publication without unsafe
    retry.
11. continue development without reading or regenerating a large construction script.

The meaning graph is the center.

Text documents, command streams, JSON, rendered source-like views, TUI views, generated bindings,
compiled forms, indexes, caches, artifacts, and Git diffs are proposals, views, distribution
objects, or derived state according to explicit contracts.

None silently becomes a second editable source of truth.

A rich terminal workbench with an explorer and editor is a representative product objective.

It is not permission to add speculative terminal, filesystem, event-loop, collection, text,
concurrency, or persistence machinery without a complete consumer and exact authority model.

## Authority and precedence

When active artifacts disagree, use this order:
1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. The accepted semantic development repository and its exact revision records.
7. Generated descriptions mechanically derived from one executable owner.
8. `docs/status.md`.
9. `docs/architecture.md`.
10. Current structured evidence and `docs/performance.md`.
11. `docs/roadmap.md`.
12. `README.md`.
13. Comments, examples, historical prompts, branches, pull requests, commits, issues, and
    discussions.

Newer verified checkout state outranks older plans and remembered repository state.

A campaign prompt owns one campaign's objectives, hypotheses, gates, and handoff. It does not become
permanent semantic authority.

An old prompt is historical evidence unless the active task explicitly selects it.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

Do not let generated documentation, a checked artifact, a test fixture, or a commit message silently
outrank the semantic owner that produced it.

## Repository safety

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -3 --oneline
find .. -name AGENTS.md -print
```

Read every applicable instruction file.

Preserve unrelated modified and untracked work.

Reading in-scope files, editing in-scope files, and running non-destructive validation are
authorized for implementation tasks unless the active task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state without explicit authorization for that
action.

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

Each accepted project revision has one authoritative typed meaning graph.

The graph may include program declarations, function bodies, tests, build targets, release
projections, application composition, interface-role declarations, and other development meaning
only when their authority domain and consumer are explicit.

Names, formatting, source positions, file paths, command order, generated indexes, and renderings
are not semantic identity.

A human-readable document is a proposal or view. It may be convenient and editable, but accepted
meaning never depends on preserving, reparsing, or diffing its formatting.

A command stream is a proposal and may be retained as an exact recipe or audit fact. It is not
current program authority.

A checked application artifact may be immutable distribution authority under its specification. It
is not maintained development authority.

A custom Python, shell, Rust, macro, build-script, or generated program that reconstructs the graph
is not maintained semantic source.

Temporary migration code must be isolated, independently checked, and deleted after direct cutover.

Every maintained first-party lkjscript application must have a public-CLI-native path from its
tracked semantic project to validated targets and distribution artifacts.

Derived views may be regenerated or discarded without changing accepted meaning.

## Self-hosting gradient

Prefer building lkjscript development tools as lkjscript applications when the current language and
runtime can own their policy.

Use native code only for bootstrap, operating-system adaptation, terminal integration, explicit
resource handling, artifact embedding, deployment, or independently justified performance
boundaries.

A native shell must not become the hidden owner of application state transitions, edit policy,
project selection policy, ordering, validation, undo semantics, or domain decisions.

A first-party semantic tool must be authored and evolved through the same public CLI that ordinary
agents use.

A bootstrap exception is temporary. Record why it exists, what exact capability blocks removal, and
the cutover condition.

Self-hosting is a gradient, not a slogan. Do not move a boundary into lkjscript merely to increase a
percentage.

The winning boundary minimizes duplicate authority while preserving exact validation, performance,
recovery, and independent testing.

## Semantic development repository

A semantic development repository owns one project continuity and immutable accepted development
history.

It exposes exact workspace identity, selected revision, immutable revision objects, canonical
revision records, parent continuity, durable allocation, tombstones, semantic diff facts, named
targets, validation, history, diagnosis, backup, and reconstruction.

Git owns collaboration and distribution of repository files. The semantic repository owns accepted
evolution of lkjscript meaning. Neither is inferred from the other.

Do not require agents to reverse-engineer semantic history from binary Git diffs.

Do not use a Git commit hash as workspace, entity, revision, release, application, instance,
authorization, or capability identity.

Tracked first-party semantic repositories must be portable, bounded, deterministic under their trust
model, and usable from a fresh checkout through public commands.

A project path is a locator. It is not semantic identity.

Branching, merging, rebasing, distributed synchronization, and conflict-free replication require
current consumers and exact semantics. Do not copy Git features speculatively.

## Public semantic CLI

The public `lkjscript` CLI is the primary development interface.

Raw constructors, private library calls, test-only builders, custom generators, and direct store
mutation are not acceptable first-party authoring paths.

Human mode provides deterministic bounded help, orientation, status, inspection, history, diff,
validation, build, test, run, recovery, and actionable errors.

Machine mode provides one strict versioned typed contract with exact framing, request correlation,
stable error classes, explicit omissions, and no progress contamination.

A caller-owned foreground session may reuse validated state, contexts, and local handles. It is not
authority, a daemon, a queue, or a scheduler.

Ordinary commands discover a project from the current directory or an explicit relative or absolute
path.

Users must not supply workspace IDs, current revisions, schema digests, or internal state paths for
ordinary work when the CLI can discover them exactly.

Convenience never weakens exactness.

The CLI must preserve all of these invariants:
- project discovery resolves to one exact workspace;
- reads report the exact selected revision;
- mutations bind an exact expected revision;
- stale state rejects;
- no mutation is silently retried;
- selectors reject ambiguity;
- session-local handles never persist as semantic identity;
- all accepted changes pass the same typed validator;
- validate-only and commit use the same normalization and candidate path;
- output preflight happens before publication where output size can reject;
- an output failure cannot retroactively undo accepted authority;

## Human, machine, and interactive surfaces

One semantic owner may have human, machine, and interactive projections, but those projections must
not define competing semantics.

Human output is bounded, escaped, deterministic, and useful without requiring machine-field
archaeology.

Machine output is closed, versioned, correlated, deterministic, and complete about omissions and
continuations.

Interactive views are derived from exact revisions or explicit ephemeral state.

A TUI must never make rendered rows, cursor coordinates, pane order, or cached labels semantic
project identity.

A source-like editor view must carry exact base and context facts and normalize through the
transaction owner.

Formatting-only changes that normalize to the same meaning publish no semantic revision.

Interactive convenience must expose stale state, conflicts, unknown outcomes, and validation
failures rather than silently hiding them.

## Project discovery and paths

Relative paths are normal public input.

Resolve paths against one documented base, canonicalize and revalidate authority boundaries, reject
unsafe traversal and substitution, and keep paths out of semantic identity.

Discovery must find exactly one strict marker under a bounded parent-walk policy.

Reject symlinked markers, unsafe traversal, ambiguous nested authorities, nonregular files, foreign
workspace bindings, malformed locators, and path substitution.

Canonical paths are deployment facts and may change without changing semantic identity.

When an exact file is selected, bind the selected authority or content facts required to prevent
time-of-check/time-of-use substitution.

Build outputs, backups, imported documents, and selected filesystem roots remain explicit deployment
inputs.

## Context and semantic graph queries

Context budget is a correctness, latency, and provider-cost constraint.

Do not make routine agents request a global graph, full schema, full history, or full target
closure.

Provide compact orientation, typed task-scoped context, on-demand expansion, stable projections,
exact digests, continuations, and explicit omissions.

A context object is a disposable observation bound to one exact project, revision, schema, query
plan, bounds, and result digest.

A context alias or handle is session-local and revision-bound. It is never durable entity identity.

Known-digest reuse may return unchanged only when every bound fact agrees.

Queries must state traversal direction, ordering, page bounds, projection, truncation, and
continuation.

Graph queries must use a closed bounded vocabulary rather than an ambient general database language
unless a complete consumer proves the need.

After exact owners are identified, stop broad discovery unless evidence invalidates the ownership
map.

Context response growth, repeated rediscovery, and correction depth are measured on complete tasks.

## CLI-native change model

A semantic change may be one high-level command, one bounded atomic bundle, or one request in a
foreground session.

The change model must support multi-entity atomicity when no valid intermediate revision exists.

Draft state is not accepted meaning.

If an interactive draft exists, define its owner, lifetime, bounds, crash behavior, identity domain,
validation points, and retention.

Use exact base-bound selectors. Friendly names resolve only when unambiguous in the selected
revision.

Function-local references remain function- and revision-bound unless a concrete continuity consumer
justifies more.

A local edit selector may use exact base-local identity or a structural precondition without
granting durable cross-revision identity.

Prefer bounded subgraph edits and declaration-level operations over resending a large application
when they materially reduce context and correction risk.

Retain whole-function replacement as a simple independent oracle and escape hatch.

Validate-only and commit share parsing, normalization, semantic validation, target validation,
artifact preflight, response preflight, and relevant resource checks.

A successful apply may return a bounded continuation or context delta only when it is exact,
preflighted, idempotency-bound, and measurably useful.

Do not preserve a second edit protocol merely for compatibility.

## Automatic development history

Every successful public semantic mutation publishes exactly one immutable revision and one canonical
revision record.

Validation-only, rejection, semantic no-change, stale input, failed prepublication, and read-only
work publish no revision.

A revision record binds workspace, exact base and result, parent and result snapshots, accepted
change digest, exact semantic diff facts, durable entity changes, function-body changes, target
changes, and publication outcome.

Optional intent, actor, tool, or observed time is bounded untrusted metadata and cannot affect
semantic identity unless a future specification deliberately assigns that role.

Do not store hidden chain of thought, provider transcripts, prompts, credentials, or secrets as
revision metadata.

Normal accepted changes record themselves without requiring a manually authored commit message.

History is append-only. Revert or restoration publishes a new validated revision and never rewrites
accepted history.

Product undo and editor undo are application semantics and must not move semantic project HEAD
backward.

## Build targets and deterministic derivation

Build configuration for maintained lkjscript software belongs in an exact versioned target graph.

Do not hide build meaning in Python dictionaries, shell sequences, Cargo build scripts, private Rust
code, generated manifests, or undocumented command arguments.

Target identity and edges use exact semantic identities. Target names are lookup metadata.

A build selects one exact project revision and exact dependency artifacts.

No target resolves `latest`, mutable registry coordinates, ambient files, or unverified paths at the
semantic boundary.

Release and application artifacts remain separate immutable authority domains.

Target declarations do not silently become runtime grants, instance state, deployment authority, or
executable identity.

Generated bindings are derived views. Prefer direct validated artifact descriptors when they are
simpler.

A checked derived artifact is allowed only when packaging needs it and a public target build
reproduces it exactly or deliberately replaces it under a new specified identity.

Build, test, and run never publish a development revision.

Selective validation, caching, and incremental lowering are optimizations and require an independent
full oracle.

## Prohibition on semantic build scripts

Do not retain `build.py`, `generate.py`, shell heredocs, private Rust builders, macros, `build.rs`,
or similar programs as the primary way to construct or evolve maintained meaning.

Do not replace one graph builder with another language, a larger fixture, generated source, or
opaque serialized graph.

Acceptance, workload, fault-injection, and measurement scripts may remain when they exercise public
boundaries and do not own application meaning or build configuration.

## Application-first closure

Every substantial platform campaign selects a valuable complete application or user workflow.

The application owns domain state, validation, ordering, decisions, and typed outcomes in lkjscript
semantics.

A host client may own transport, terminal adaptation, rendering mechanics, explicit resource
selection, process lifecycle, and independent assertions.

A host client may not own hidden business state, edit policy, ordering, query semantics, or domain
decisions.

Build the smallest complete product slice first.

Add language, runtime, storage, interface, or tooling mechanisms only for an exact blocker revealed
by the slice.

Return to the product immediately after closing each blocker.

A capability is incomplete when the host reconstructs private state, suppresses invalid requests,
parses opaque responses for domain meaning, or remains the real workflow controller.

Run the completed product from a fresh checkout through public release binaries.

Dogfood the semantic CLI on a real maintained-application change before completion.

Delete productless infrastructure, losing prototypes, stale examples, and intermediate artifacts
without a retained consumer.

## Interactive application architecture

Interactive software separates accepted semantic meaning, ephemeral foreground session state,
durable application state, external resource authority, and rendered presentation.

Do not publish a durable application revision for every key event merely because a durable instance
mechanism exists.

Do not claim crash durability for ephemeral state.

Select pure foreground, durable, or hybrid interaction topology from a complete latency, recovery,
and authority comparison.

A foreground interactive session is caller-owned and disappears on process exit unless an explicit
recovery authority exists.

An application-defined update function owns state transition and action intent.

An application-defined render function owns semantic frame content.

A native runner owns terminal acquisition, event decoding, frame emission, signal handling, and
cleanup.

The runner must not interpret application domain state to decide behavior.

External actions are closed typed requests with explicit outcomes.

Possible external visibility stops automatic retry and enters an explicit reconciliation state.

Input queues, action queues, render queues, and background work are bounded.

Event ordering, coalescing, cancellation, stale results, shutdown, and restart are specified.

## Editor and text semantics

Buffer identity is distinct from file path, content digest, tab position, and display name.

Text indexing chooses and documents one semantic unit for each operation: UTF-8 byte, Unicode
scalar, grapheme cluster, line-column pair, or terminal cell.

Never call byte offsets character offsets.

Every text edit preserves valid UTF-8 or returns a typed rejection.

Cursor, anchor, selection, replacement, line break, end-of-buffer, and movement behavior are exact.

Selection direction and collapsed selection behavior are explicit.

Undo and redo define retention, bounds, grouping, branching after undo, external action interaction,
and crash behavior.

Editor undo is not semantic project restoration and is not filesystem rollback.

Multiple buffers define allocation, nonreuse, close, dirty state, origin, conflict, and reopen
behavior.

A file origin is a deployment locator plus exact observed base facts, not buffer identity.

Search defines exact matching unit, overlap, ordering, case behavior, normalization behavior,
bounds, and continuation.

Syntax highlighting, diagnostics, line indexes, and search indexes are derived and disposable.

Large-text representation is unobservable. Retain ropes, piece tables, line tables, or structural
sharing only when a complete editor workload beats the simple oracle.

A source-like semantic document is an editable proposal. The meaning graph remains authority.

## Terminal boundary

Applications do not emit raw terminal escape sequences.

Terminal input is decoded into one closed bounded event vocabulary.

Key code, modifiers, press/repeat/release state, paste, focus, mouse, and resize are distinct when
supported.

Unknown or malformed escape sequences do not become arbitrary application bytes.

Resize dimensions are explicit, bounded, and tested at zero, minimum, odd, and excessive sizes.

Frames use a closed cell/span/style model with exact clipping and cursor semantics.

Display width, combining marks, wide characters, control characters, invalid terminal responses, and
clipping have explicit behavior.

Terminal output is escaped by construction.

Acquire, raw mode, alternate screen, cursor visibility, signal handling, suspension, EOF, panic,
output error, and normal close all have tested cleanup behavior.

A terminal process boundary is not a sandbox.

Do not add a general terminal framework without a complete retained application.

## Filesystem boundary

Ambient broad filesystem authority is forbidden.

A filesystem grant selects one exact root and explicit operation classes under a documented trust
model.

Semantic paths are ordered validated relative components, not unchecked host strings.

Define encoding, separator, dot, dot-dot, empty component, reserved name, symlink, hard-link, mount,
case-sensitivity, and normalization behavior.

Directory listing defines ordering, pagination, metadata, truncation, races, and inaccessible
entries.

File read binds exact observed type, size, content digest or equivalent version fact, and maximum
bytes.

File write uses explicit expected-base semantics and no-clobber or atomic-replace publication.

Conflict, absence, permission denial, invalid type, excessive input, I/O failure, known success,
known failure, and unknown visibility are distinct.

A possibly visible write is never silently repeated.

Reconciliation determines present, absent, conflicting, or indeterminate state from independently
observed facts.

External modification produces an application-visible decision: reload, keep, compare, save-as, or
cancel.

Temporary files, synchronization, rename, directory synchronization, cleanup, and crash points are
tested.

The filesystem adapter cannot invent editor state, project meaning, or user intent.

## Semantic project host boundary

An lkjscript application that operates on another semantic project needs one explicit project grant.

The grant binds exact workspace identity, locator policy, allowed read and mutation classes, limits,
and applicable authority revision.

Project reads return exact revision-bound typed results.

Project mutations carry exact expected revision and idempotency facts.

The application may propose project actions; the project owner alone validates and publishes them.

Cross-authority atomicity between application state, project history, files, and terminal output is
absent unless explicitly proved.

A pending cross-authority action records enough facts to avoid duplicate execution and to reconcile
after interruption.

The project adapter does not expose private store mutation, raw lock manipulation, or unvalidated
graph insertion.

A project path is deployment state and cannot substitute for the granted workspace identity.

## Host interfaces and grants

Pure deterministic computation remains the default.

Ambient host authority is forbidden.

Applications declare exact host-interface requirements but never grants.

Instances or foreground runners bind requirements to exact grants.

A grant binds sharing domain, interface identity, adapter kind, bounded descriptor, applicable
authority revision, and limits.

Host requests and outcomes are closed typed values.

Opaque bytes are permitted only when the interface deliberately defines and bounds them.

An adapter cannot invent semantic state, application response, command intent, or authority.

Live resources require explicit acquire, use, transfer, consume, close, cancellation, timeout,
crash, and cleanup semantics.

Expected workflow outcomes may be nominal data.

Corruption, denial, exhaustion, infrastructure failure, stale state, conflict, and unknown
visibility remain distinguishable.

Non-idempotent work is never silently retried after possible partial execution.

Time, randomness, scheduling observations, terminal input, filesystem observations, and host
observations are explicit when observable.

## Mutation and query separation

Mutations and observations are separate semantic contracts.

A project mutation may reject, validate without publication, or publish exactly one revision and
record.

An application mutation may decline, report unchanged, publish one completed state, or publish one
suspended state and command under its profile.

A pure query returns a typed value without publishing semantic or durable state.

A query must not be implemented as a no-op event.

A product client must not decode private state as a second query authority.

Query output failure has no rollback meaning because no semantic publication occurred.

Pagination, ordering, truncation, omissions, revision binding, and result digests are exact and
bounded.

## Identity and continuity

Assign durable identity only for a concrete continuity, reference, repair, history, sharing, import,
export, target, instance, product, or operational consumer.

Names, formatting, positions, order, paths, hashes, compiler indexes, artifact offsets, storage
keys, runtime handles, queue positions, process IDs, and addresses are not semantic identity unless
a closed contract assigns a narrower role.

Workspace, revision, revision record, change, build target, release, application, instance, product
entity, command, outcome, grant, interface, adapter, deployment, executable, checkpoint, backup,
cache entry, session handle, buffer, file origin, terminal session, and runtime handle are distinct
domains.

A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
capability identity.

Identity-preserving change requires an explicit validated rule.

Deleted durable identities are not silently reused.

Multiple exact versions may coexist only when references remain unambiguous.

Function-local identities remain bound to one function and exact revision unless a real continuity
consumer justifies more.

## Publication and durability

Published revisions, records, releases, applications, instance records, host outcomes, authoritative
checkpoints, backups, and other declared durable objects are immutable within their domains.

Every durable namespace has one publication authority.

One successful publication creates exactly one accepted durable outcome.

Rejection and validate-only publish nothing and consume no durable identity.

Semantic no-change does not consume a revision merely to return a response.

Success is acknowledged only after the documented synchronization boundary.

A possibly visible but unconfirmed outcome is reported as unknown and never silently retried.

Recovery, replay, retention, checkpointing, compaction, corruption, backup, restore, deletion, and
garbage collection are explicit and validated.

Semantic state publication and externally visible host work remain separate unless atomicity is
proved.

Output failure cannot retroactively undo accepted authority.

Related durable objects must not become independently visible in conflicting combinations.

## Values, language, and representation

Add a language or value capability only for a complete current application or the semantic
development CLI itself.

Text and variable-length collections require exact validation, canonical encoding, bounds,
deterministic equality and ordering, and a current consumer.

Do not add a conventional source language merely to replace a graph-construction script.

Do not add generics, traits, macros, maps, sets, iterators, mutable builders, normalization,
formatting, reflection, or operator syntax without an exact blocker and complete end-to-end use.

Representation, sharing, allocation, reclamation, checkpoint layout, caches, IR, bytecode, profiles,
and native code are unobservable or derived unless deliberately promoted by specification.

A simple independent allocation, execution, reconstruction, and build route remains the oracle for
optimized values, storage, caches, compaction, and execution tiers.

Cache miss, eviction, missing derived state, and process restart remain correct.

Full snapshots, deltas, journals, object stores, databases, collectors, bytecode, JITs, schedulers,
and supervisors must beat simpler safe designs on a representative complete workload.

## Execution, scheduling, and concurrency

One simple executable route remains the correctness oracle.

Faster tiers are differential until direct cutover is justified.

Values, traps, order, state transitions, host requests, resource semantics, and diagnostics remain
stable across tiers unless accepted semantics deliberately change.

Operational scheduling and language semantics are separate.

Deterministic fuel is not wall-clock time.

A scheduler may use time, priority, quota, or load only when those choices do not silently change
semantic results.

Observable interruption, yield, cancellation, or timeout requires an accepted contract.

Per-project and per-instance mutations remain serial unless a stronger model is specified and
proved.

Pure queries may run concurrently only after snapshot isolation, lifetime, ordering, admission,
shutdown, and mutation interaction are proved.

Cross-project or cross-instance parallelism requires exact isolation, bounded queues, deterministic
per-authority order, overload behavior, safe shutdown, and differential tests against serial
execution.

Do not add a general async runtime, worker pool, scheduler, actor system, or daemon merely because
an interactive foreground session exists.

## Resource governance

Semantic limits and operational limits are separate.

Semantic limits include fuel, frames, value depth, item count, text bytes, collection elements,
graph expansion, transaction operations, revision record size, target closure, state size, query
work, and response size.

Operational limits include queue capacity, concurrent compilation, cache bytes, aggregate memory,
open files, terminal events, adapter operations, process count, CPU share, temporary publication
bytes, project count, and deployment quotas.

Each limit states accounting unit, owner, reservation and release points, peak and retained
accounting, source, rejection class, retryability, observability, restart behavior, and publication
interaction.

Check lengths, counts, depth, and reservations before allocation or corresponding work.

Do not present allocator observations as exact semantic accounting.

Shared backing storage, checkpoints, caches, embedded artifacts, and target outputs must not bypass
limits.

Overload rejects, queues within a bound, or sheds work under an explicit policy.

Never create an unbounded hidden queue.

OS controls may strengthen containment but do not replace semantic or runtime accounting.

## Security and trust

Accepted semantics cannot express unchecked memory access.

User-controlled depth does not consume unbounded native stack.

No local unsafe Rust is permitted unless the active user explicitly authorizes a replacement after a
concrete need, isolated ownership, safe public contract, and independent tests are recorded.

Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, path safety,
crash consistency, supply-chain trust, and hostile-code isolation are separate contracts.

Treat bytes, text, paths, locators, filesystem metadata, semantic repositories, revision records,
artifacts, instance files, checkpoints, backups, caches, profiles, IPC peers, terminal events, and
adapter outcomes as hostile input.

A process boundary creates neither semantic authority nor a sandbox.

A runtime supervisor authenticates and authorizes every request under its deployment model.

A path, socket, UID, environment variable, or connection is not sufficient semantic authority by
itself.

Write the threat model before multi-user access, untrusted native code, broad filesystem or network
access, child processes, secrets, terminal control beyond trusted local use, or remote
synchronization.

Large input, work, state, history, graph, output, queue, cache, diagnostics, and metrics are
bounded, paginated, chunked, streamed, or policy-controlled.

Human terminal output is escaped and bounded.

Machine output is framed separately and never contaminated by progress text.

Compactness never weakens validation, authorization, identity, durability, diagnostics, or
verification.

## Agent and provider economy

Context budget is part of correctness, latency, and cost.

This root policy has no arbitrary byte ceiling, but every durable instruction pays recurring context
rent.

Keep permanent rules here and volatile campaign facts in the active prompt, status, evidence, or
generated orientation.

State durable principles once.

Prefer compact orientation, task-scoped graph queries, exact on-demand expansion, bounded review,
stable diagnostics, validate/apply parity, delta receipts, digest reuse, session-local handles, and
explicit omissions.

Build a compact campaign ledger instead of repeatedly reading a long prompt.

Expose only relevant schema roots and tools.

Add prompt rules, examples, schemas, or context only for a measured failure mode.

A larger response is justified when it prevents more expensive rediscovery or correction; byte
minimization alone is not the objective.

Compare equal tasks using semantic success, unintended changes, correction depth, repeated
discovery, action bytes, observation bytes, request count, process count, files opened, source bytes
opened, schema bytes, build invocations, elapsed time, and failure quality.

Record provider model identity, token classes, cache classes, dated prices, and monetary cost only
when directly exposed.

Bytes are not tokens.

Never infer provider cost from bytes.

Do not claim API-cost savings without exact comparable telemetry.

Measured reductions in bytes, calls, processes, files opened, correction depth, or elapsed time may
be claimed precisely.

## Code ownership and dependencies

`docs/spec/` owns accepted contracts.

`docs/architecture.md` owns components and trust boundaries.

`docs/status.md` owns implemented reality.

`docs/performance.md` and structured evidence own measurements.

`docs/roadmap.md` owns unresolved consumer-driven reversal gates.

`README.md` owns concise orientation.

Campaign prompts are temporary execution artifacts.

Keep one executable owner for every type, field, operation, query, error, limit, format, interface,
grant, resource, command, target, record, and machine descriptor.

Derive views only when staleness cannot be silent.

Organize code around stable ownership and changed-together behavior.

Split large files when bounded review, agent context, test isolation, compile locality, or ownership
clarity improves without duplicating invariants.

Do not preserve arbitrary file-size or directory-count limits.

Prefer the standard library and existing dependencies.

A new dependency must repay supply-chain, build, binary, audit, operational, and maintenance cost on
the complete workflow.

Git history is the archive for deleted repository material. Delete stale active-tree copies and
losing generated paths.

## Testing and verification

Acceptance tests have exact immutable input, oracle, policy, selection, order, and result.

Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.

For changed boundaries, cover every applicable case:
- canonical success;
- repeated success;
- validate-only parity;
- semantic no-change;
- no-publication outcomes;
- pure-query no-write behavior;
- revision-record atomicity;
- stale and future base;
- duplicate and idempotency conflict;
- wrong identity domain;
- malformed input;
- truncated input;
- trailing input;
- oversized input and output;
- exact and one-over limits;
- ambiguous selectors;
- foreign authority;
- corruption;
- restart;
- interrupted publication;
- unknown visibility;
- reconciliation;
- output failure;
- cleanup;
- concurrent access;
- authority busy;
- overload;
- replay;
- cache miss, hit, eviction, and corruption;
- checkpoint and reconstruction differential;
- build-target determinism;
- generated-view stale detection;
- artifact reproduction;
- backup and restore;
- relative-path and discovery safety;
- terminal resize and cleanup;
- filesystem substitution and conflict;
- public CLI workflows;
- first-party application dogfooding;

Use a simple independent reference model where semantics become substantial.

Run narrow checks first, then the full repository gates.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every affected retained public workflow and every selected complete application.

Use Miri, sanitizers, mutation tests, property tests, fuzzing, model checking, crash injection,
filesystem fault injection, pseudo-terminal tests, or cross-platform execution when they target a
real risk and are available.

State scope and limitations.

Do not weaken an invariant test to make implementation pass.

Change specification, implementation, and oracle together when behavior deliberately changes.

## Evidence and claims

Evidence is not semantic authority.

Record exact environment, commit, command, input corpus, output, raw observations, units, sample
selection, warm or cold classification, and unavailable data.

Do not infer tokens from bytes.

Do not infer cost without exact model-specific token and dated price telemetry.

Do not call a warm-host sample cold.

Do not call summed command waits wall-clock time.

Do not call logical accounting RSS enforcement.

Do not call a digest provenance, signature, authorization, or freshness.

Do not call a process, worker, container, private directory, or project locator a sandbox.

Do not call compile success cross-platform support.

Do not claim full-history validation from a current-state check.

Do not call a terminal smoke test a production editor.

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

Prefer one topology-neutral owner over duplicated process adapters.

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
- stop rule;
- reversal condition;

Delete losing prototypes completely.

Current absences are not permanent prohibitions without semantic reason.

## Change workflow

1. Inspect checkout, instructions, branch, commit, and unrelated work.
2. Identify authoritative owners and active format identities.
3. Select the valuable complete application workflow and freeze an independent oracle.
4. Reproduce the current public authoring, build, run, and recovery paths.
5. Create a compact campaign ledger with outcomes, non-goals, alternatives, stop rules, and
   verification state.
6. Build the smallest complete semantic-CLI slice.
7. Use it immediately on a real maintained application.
8. Prototype uncertain questions in the smallest dependency-closed form.
9. Select one coherent design from evidence.
10. Implement the full vertical across semantic model, persistence, protocol, CLI, targets, product,
    tests, and documentation.
11. Publish semantic changes through the new path and inspect automatic history.
12. Cut over directly and delete builders, duplicate surfaces, compatibility paths, and stale facts.
13. Run focused, full, representative, restart, corruption, publication, interaction, and product
    checks.
14. Measure equal authoring and product tasks and record only supported savings.
15. Dogfood from a fresh checkout.
16. Leave a compact exact handoff.

Do not stop at a report when a safe complete implementation is authorized and feasible.

Do not scatter partial architecture.

Do not ask the user to decide ordinary engineering details that checkout evidence can resolve.

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
- measured end to end;
- free of superseded paths;

Before finishing, report:
- exact starting and ending state;
- selected design and serious rejected alternatives;
- changed authority and format contracts;
- automatic-history behavior;
- CLI workflows;
- interactive-session behavior when applicable;
- terminal, filesystem, and project grants when applicable;
- build-target behavior;
- first-party application migration or creation;
- deleted builders and stale paths;
- validation commands and exact results;
- representative product and authoring-economy evidence;
- provider telemetry only when directly exposed;
- known limits and trust assumptions;
- reversal gates;
- every requested action not performed;

Claims must be no stronger than the checkout and reproduced evidence.
