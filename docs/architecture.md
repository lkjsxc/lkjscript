# Current architecture

`lkjscript` is a meaning-first semantic development and application platform. One immutable typed
representation owns accepted meaning in each authority domain. JSON, semantic documents, context
capsules, renderings, indexes, caches, Core IR, native descriptors, artifacts, and evidence have only
their explicitly narrower proposal, derived, or distribution roles.

## Authority flow

```text
project locator (.lkjscript/project)
                 |
                 v
semantic repository -- HEAD --> exact development revision + revision record
                 |                         |
                 |                         +-- typed program graph
                 |                         +-- exact build-target graph
                 |                                      |
                 |                              deterministic lowering
                 |                                      v
                 |                          release --> application
                 |                                         |
                 |                                  instance + grants
                 |                                         v
                 +-- immutable history             durable product state
```

Project identity, revision identity, target identity, release identity, application identity,
instance identity, grants, and deployment paths are distinct. Git transports outer-repository files;
it does not own semantic history. Paths locate authority; names resolve only within one exact
revision. Digests establish equality/integrity under their domains and imply no provenance,
authorization, signature, or freshness.

## Project discovery and repository

`src/project.rs` owns initialization, strict locator decoding, explicit selection, bounded parent
discovery, project-level reads/changes, backup, and response preflight. A marker contains only its
version, exact workspace identity, and checksum. The repository is stored under
`.lkjscript/repository` and reuses the existing single-workspace owner rather than introducing a
second project graph.

Initialization stages a complete private `.lkjscript` directory, creates revision zero, writes the
marker, synchronizes, and publishes with no replacement. Opening validates every directory/marker
kind, requires exactly the marker-selected workspace, and asks `src/persistence.rs` to decode the
complete selected closure. Nested discoverable authorities are ambiguous unless one is selected
explicitly. Symlinks, nonregular files, lexical parent traversal, malformed locators, and foreign
identity reject.

The current conservative topology holds the engine lock for the lifetime of an opened project,
including reads. This proves one authority owner and exact snapshot lifetime but leaves concurrent
readers as a measured future optimization.

## Meaning graph and change normalization

`src/schema.rs` owns the closed node/type/operation vocabulary, now including durable build-target
nodes. `src/graph.rs` owns snapshots and durable/local identity domains. `src/validate.rs` owns graph,
type, scope, dominance, target, and completeness invariants. `src/transaction.rs` is the one change
normalizer and allocator. Validation and commit prepare the same exact candidate; allocation is
provisional until publication.

A project JSON change is a thin exact-project/base envelope around the transaction operations. The
semantic document parser in `src/workbench/document.rs` offers a human-editable proposal with
proposal-local symbols and packet aliases. Context construction and views live under
`src/workbench/`. Neither representation is authority, and both pass through the transaction owner.
Omission never means deletion. Whole-function replacement remains the simple oracle; fine-grained
operation replacement and typed-hole refinement remain available.

`src/bin/lkjscript/project.rs` owns the public one-shot grammar and a strict correlated JSON-lines
foreground session. The session may retain one exact context capsule, but aliases expire on any HEAD
advance and restart loses all handles. The raw engine RPC is retained solely as a lower-level
conformance/embedding transport, not an alternate semantic project.

## Automatic development history

`src/history.rs` owns canonical revision records and semantic history summaries. Every genesis,
accepted mutation, or restoration record binds project identity, exact parent/result snapshot,
parent record, accepted change-set digest, semantic diff digest, change counts, created/deleted/
modified durable identities, function-body changes, target-definition changes, affected targets,
and publication outcome.

`src/persistence.rs` publishes an immutable canonical snapshot and record before replacing HEAD.
HEAD10 binds the exact current revision, graph hash, record digest, and retained idempotency receipt.
Failure before HEAD leaves old authority; visibility-capable failure is unknown and never silently
repeated. Ordinary open validates every retained path, every compact record, their contiguous links,
HEAD, the selected graph, and the retained idempotency receipt without decoding unrelated historical
graphs. Historical selection validates the exact requested artifact against its record. Deep doctor
loads every snapshot, checks every adjacent identity transition, and recomputes every record's exact
semantic diff facts. Log reads compact summaries; show expands one record; diff compares exact
endpoint graphs without inferring identity from names or structure.

Restoration copies only legally retained meaning from a historical snapshot into a new candidate at
the next revision. Current allocation frontier and tombstones remain authoritative. Any historical
durable identity absent now would be resurrected, so the automatic restoration rejects; a caller
must author an explicit create/rewrite/delete proposal with new identities.

Full canonical snapshots remain the selected persistence design. They provide simple independent
reconstruction, arbitrary diff, backup, corruption localization, and exact historical reads. The
retained 100-change `lkjwork` workload occupies 21,580,665 bytes; ordinary current open is 3.224 s
after lazy historical decoding, while complete deep reconstruction is 332.369 s. Journals,
content-addressed objects, Merkle sharing, packing, and garbage collection therefore remain rejected
complexity for ordinary use, with deep-audit time and retained bytes recorded as explicit reversal
gates rather than hidden behind every command.

## Build-target graph and derivation

`src/target.rs` owns build-target schemas, dependency cycles, exact reference validation, target
summaries, and lowering. A target is a durable root-owned graph node. Release targets select exact
package roots, exports, target dependencies/imports, metadata, and immutable cases. Application
targets select exact release targets, entries, pure/stream/stateful profiles, nominal mappings,
requirements, resource policies, and cases. Product targets select an exact application target for
native packaging.

Every accepted project candidate validates and prepares every retained target. This makes an invalid
distribution impossible to publish and keeps a single correctness route. It is deliberately eager;
target-impact analysis identifies downstream products, but selective validation/caching is not yet
authority. Target edges use durable IDs and cycles reject before publication.

Target lowering builds `ReleaseBuildRequest` and `ApplicationBuildRequest` values in memory from the
validated graph, then calls the existing release/application owners. No shell, Python, Rust callback,
ambient file, mutable coordinate, deployment path, or grant enters target meaning. Build responses
are preflighted before output. Publication is explicit, no-overwrite, synchronized, and may report
unknown output visibility after a visibility-capable step. Builds never mutate project history.

## Release, application, and execution

`src/release/` projects one selected package closure into canonical release format 2. Workspace and
revision identities disappear; exact release-local nominal identity, dependencies/imports, exports,
and cases remain. `src/application.rs` composes one complete exact release graph into application
format 5, validates profiles and host requirements, exposes a bounded interface description, and
owns public application-value conversion.

`src/compile.rs`, `src/core_ir.rs`, and `src/interpret.rs` lower the complete selected closure and run
one explicit-frame interpreter. Core IR and ownership plans are derived and independently checked.
Bytes/text use a generation-checked managed store; nominal sequences use immutable `Arc` elements.
Visible cells/bytes, retained bytes, values, depth, items, frames, and fuel are separately bounded.
No semantic value exposes allocation, sharing, address, or representation identity.

## Product instances and host authority

`src/instance.rs` exclusively owns instance selection, state transitions, pure queries, history,
checkpoints, idempotency, grants, commands, attempts, outcomes, and publication. Declined/unchanged
decisions publish nothing; completed/suspended decisions publish exactly one hash-linked record.
Genesis and every 64th revision carry full semantic checkpoints; a HEAD-bound bounded manifest
accelerates current access. Missing/corrupt acceleration falls back to full replay, while deep doctor
reexecutes the whole chain.

Applications declare host-interface requirements. Instances bind those requirements to exact
deployment grants. The only production interface is the bounded immutable blob namespace. A
visibility-capable put records an attempt before host work; known failure, known success, possible
visibility, and reconciliation remain distinct. Adapters cannot invent command intent, semantic
state, or application responses.

`src/runtime.rs` and `src/runtime_protocol.rs` provide one-shot and foreground-session adapters over
the same instance owner. There is one synchronous lock and no daemon, hidden queue, scheduler, worker,
async runtime, or retry engine.

## lkjwork packaging

`applications/lkjwork/.lkjscript` is the checked semantic development repository. It contains the
imported product history, first-class release/application/product targets, and three dogfood
revisions for `why`. `lkjscript target build lkjwork` deterministically reproduces
`applications/lkjwork/lkjwork.lkja`; no graph builder or generated binding file remains.

`src/bin/lkjwork/bindings.rs` constructs and decodes application-owned values only after discovering
the exact exported types, fields, variants, and functions through the validated artifact interface.
`src/bin/lkjwork/project.rs` owns private product locator/deployment handling, grant construction,
attachment routing, and backup/restore. `render.rs` owns deterministic JSON and escaped bounded human
rendering. The client never decodes private state or recomputes readiness, blockers, filters,
ordering, context, export, or `why` policy.

An installed product project remains:

```text
PROJECT/.lkjwork/
  locator
  instance-store/
  blobs/
```

This product state is independent from the semantic development project and its history.

## Trust boundary and absences

The bootstrap trust boundary is one local operator/OS account, Rust implementation, validated
artifacts, and the narrow blob adapter. Graph bytes, JSON, documents, text, paths, locators, records,
artifacts, manifests, outcomes, backups, and blobs are hostile input. Every authority decoder is
closed, bounded, canonical, and consumes all input. Terminal safety is independent of text semantics.

There is no native sandbox, network, multi-user authorization, secrets system, broad filesystem or
terminal interface, child-process interface, daemon, scheduler, database, general VCS, package
registry, build hook, build cache, bytecode/JIT/native tier, automatic migration, or compatibility
path.
