# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add stricter rules for a genuine ownership boundary.

A deeper file may not weaken this file.

Use English for maintained code, tests, protocol fields, diagnostics, command output,
specifications, documentation, examples, benchmarks, evidence, commit messages, and handoffs.

Campaign-specific implementation detail belongs under `prompts/`.

Durable product and engineering policy belongs here.

## Mission

Build lkjscript into a general-purpose, meaning-oriented programming language and application
platform whose primary development substrate is a validated, revisioned semantic graph.

The accepted graph is the sole authored program authority.

The graph must be materially better than editable text for discovery, stable selection, mutation,
refactoring, impact analysis, validation, compilation, review, merge, and recovery.

Normal application development must be possible through the public `lkjscript` executable.

A released executable placed in an otherwise empty environment must be sufficient for an agent to
discover the platform, create a project, author and refactor meaning, add tests, validate, build,
run supported application shapes, inspect failures, back up the project, and recover it.

Rust is the bootstrap and generic host implementation.

Application policy remains graph-authored lkjscript meaning unless a capability is intrinsically
host-owned.

Optimize for the strongest coherent final system over the longest useful horizon.

Do not optimize for preservation of prior contracts, prior files, prior terminology, or sunk cost.

## Product thesis

lkjscript is not a text language with an unusual file format.

lkjscript is a semantic program database with language semantics, immutable history, exact
transactions, deterministic derivation, and an agent-oriented command protocol.

Modules are organizational namespaces.

Modules are not required to be storage shards, validation units, compiler units, cache units, or
runtime units.

Stable semantic owners are selected by typed identity.

Names are mutable presentation and namespace entries.

Accepted references use exact typed identities.

Source positions, tree paths, vector positions, module membership, and content hashes are not
stable declaration identity.

A recursive source-shaped AST plus a parallel identity table is not an acceptable long-term
semantic kernel.

A name-bearing AST plus a parallel relation table is not an acceptable long-term semantic kernel.

Canonical meaning should be normalized enough that a local semantic edit can be represented,
validated, queried, compiled, and published by touching the changed records and their conservative
dependency closure.

Graph superiority must be visible in complete workflows.

It is not enough to claim that the graph is superior because text files are absent.

## Primary outcomes

1. Maintain exactly one accepted program authority.
2. Normalize semantic owners around stable typed identities.
3. Make local graph work local in reads, writes, validation, compilation, queries, and model context.
4. Make the CLI and stdio protocol precise, discoverable, compact, deterministic, and economical.
5. Make one binary sufficient for ordinary offline first use.
6. Provide complete high-level semantic changes and refactorings.
7. Make semantic merge and typed conflict resolution better than line-oriented merge.
8. Keep independent full correctness and reconstruction oracles.
9. Preserve exact capability boundaries between program requirements and deployment grants.
10. Reach asymptotically sound storage, backup, retention, query, mutation, and build behavior.
11. Carry materially different maintained applications without application-specific Rust policy.
12. Delete superseded paths after direct incompatible cutover.
13. Reduce repeated model context, tool output, verification work, retries, and provider expense.
14. Make documentation and protocol discovery mechanically follow executable truth.
15. State security, performance, portability, and cost claims no more strongly than evidence allows.

## Authority and precedence

Apply instructions in this order:

1. The active user request.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative specifications under `docs/spec/`.
5. Executable validators, invariant tests, and public black-box contracts.
6. The current accepted graph revision and its canonical revision record.
7. Generated descriptions mechanically derived from an executable owner.
8. `docs/status.md`, `docs/architecture.md`, and `docs/security.md`.
9. Structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`, `README.md`, and application documentation.
11. Historical prompts, commits, branches, issues, discussions, and remembered plans.

A newer verified checkout outranks stale facts in any prompt.

A campaign prompt governs one campaign.

Length and recency do not turn a campaign prompt into permanent product authority.

A generated file is authoritative only to the degree that its named executable owner is
authoritative.

When behavior changes, update its specification, implementation, executable oracle, generated
registry, status, architecture, security implications, evidence, and user documentation in the
same completed cutover.

Do not resolve contradictions by silently selecting the convenient source.

Identify the owning layer, correct the owner, regenerate dependents, and delete stale duplicate
truth.

## Decision standard

Backward compatibility has no value unless the active user explicitly requires it.

Do not preserve an architecture because it was expensive to build.

Do not preserve a public contract because users might theoretically depend on it.

Do not preserve a storage format because it is canonical today.

Do not preserve a command hierarchy because tests mention it.

Do not preserve terminology because it appears in many files.

Do not preserve a limit because benchmarks were built around it.

Do not preserve an abstraction because a prior prompt requested it.

Prefer one completed architecture to editions, aliases, dual readers, dual writers, fallback
paths, compatibility shims, and permanent migration code.

A large rewrite is acceptable when it replaces rather than stacks, carries all maintained
consumers, restores equivalent or stronger evidence, deletes the predecessor, and leaves one
executable product.

A rewrite is not justified by aesthetic preference alone.

Use invariants, full workflows, scale evidence, failure evidence, and reduced conceptual
duplication to justify it.

Do not leave two competing abstractions for the same concept.

Do not leave a new path beside an old path for reassurance.

Do not substitute a roadmap for a dependency-closed implementation that is feasible now.

Do not optimize a representation before confirming that the representation should survive.

## Autonomy and responsibility

Resolve ordinary engineering decisions from the checkout, bounded prototypes, complete workflows,
measurements, and the priorities in this file.

Do not ask the user to select routine implementation details that evidence can decide.

Do not stop at analysis when a dependency-closed implementation is feasible.

Do not claim completion while maintained consumers rely on private builders, obsolete formats,
fixture-only paths, or undocumented host state.

State uncertainty honestly.

Distinguish observed behavior, inferred behavior, intended behavior, specified behavior, and proved
behavior.

Never turn one benchmark observation into a universal guarantee.

Never claim token savings, monetary savings, security properties, portability, durability, or
scalability without corresponding retained evidence.

When scope is large, prioritize the dependency chain that unlocks complete public workflows.

Partial internal scaffolding without a public vertical slice is not progress worth preserving.

## Repository safety

Before editing, inspect the actual worktree and every applicable instruction file.

Use at least:

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -16 --oneline
git remote -v
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true
find .. -name AGENTS.md -print
```

Preserve unrelated modified and untracked work.

Permission to redesign lkjscript is not permission to destroy unrelated work.

Do not use `git reset`, `git clean`, amend, rebase, merge, force-push, delete remote state, publish
a
release, or rewrite unrelated history unless the active user explicitly requests that exact action.

Stage explicit in-scope paths only.

Do not use `git add .`, `git add -A`, or `git add --all`.

Inspect staged and unstaged diffs before every commit.

Verify local and remote refs after an authorized push.

A push, when authorized, must be a normal fast-forward push of completed in-scope commits.

Never commit credentials, secrets, private transcripts, personal data, hidden reasoning, raw
provider events, unrelated files, or unlicensed corpora.

Keep destructive experiments, generated bulk scale data, helper-agent transcripts, and losing
prototypes outside the repository unless a retained artifact has a named consumer.

Report unavailable tools, failed gates, unknown publication state, partial completion, and
requested actions not performed.

## Direct incompatible cutover

The default migration strategy is direct incompatible cutover.

Old programs, graph objects, stores, identities, artifacts, commands, schemas, deployment layouts,
caches, indexes, and generated views may become unreadable.

A one-time converter is allowed only when it is the shortest safe path to carrying maintained
authority into the new model.

A converter must not become a permanent reader.

After maintained consumers migrate and verify:

- delete predecessor readers;
- delete predecessor writers;
- delete compatibility aliases;
- delete duplicate schemas;
- delete old object encoders;
- delete obsolete tests;
- delete stale generated data;
- delete stale documentation;
- reject predecessor formats exactly at public boundaries.

Historical reconstruction is not compatibility.

Git history may contain old bytes.

The current executable must not interpret predecessor bytes as current authority.

Incompatible-change freedom is not permission to leave the repository between architectures.

Do not commit a state in which half of the maintained consumers use one authority model and half
use another.

Use coherent vertical commits when a complete cutover cannot fit safely in one commit.

Every intermediate commit intended for retention must build and have a clear single-authority
interpretation.

## Canonical semantic kernel

Each accepted project revision has exactly one canonical typed semantic graph.

The graph owns program meaning, including:

- repository and package metadata;
- modules and namespaces;
- exact dependencies;
- declarations;
- visibility;
- types;
- expressions;
- lexical bindings;
- components;
- ports;
- capability requirements;
- targets;
- tests;
- semantically retained documentation;
- semantic annotations;
- stable ownership and reference relationships;
- deletion continuity records only when a named consumer requires them.

The graph does not own:

- secrets;
- deployment grants;
- live handles;
- runtime resources;
- host coordinates;
- source coordinates;
- tree paths used only for rendering;
- compiler-local dense indexes;
- bytecode offsets;
- object-pack offsets;
- cache entries;
- query indexes;
- logs;
- formatting choices;
- model transcripts;
- provider telemetry.

Logical graph authority does not require one physical object per logical node.

Physical storage may use persistent maps, packed objects, immutable segments, journals, snapshots,
content-addressed packs, and disposable indexes.

Choose physical forms by correctness, locality, structural sharing, bounded loading, write
amplification, crash consistency, merge behavior, transport behavior, compaction behavior, and
measured complete-task performance.

Do not encode the canonical graph as verbose recursive JSON.

Do not allocate one general-purpose heap object or filesystem inode per semantic atom.

Do not require whole-project loading for ordinary local query, mutation, validation, compilation,
or publication.

Accepted revisions are complete, valid, and executable within their declared requirements.

Holes, unresolved selectors, speculative alternatives, and conflicts belong to explicit
non-executable draft authority.

Every read identifies the exact observed revision.

Every write names or session-resolves to an exact base revision before publication.

Every successful accepted write publishes at most one revision atomically.

Validation, rejection, stale input, no-change, reads, plans, previews, and derived-index rebuilds
publish no accepted revision.

## Normalization requirements

The canonical kernel must use normalized stable-ID records rather than a recursive source-shaped
module AST with parallel metadata.

A semantic owner has one typed stable identity and one canonical current record.

An accepted declaration reference identifies the exact package and declaration identity.

An accepted declaration reference must not redundantly carry a mutable module locator.

An accepted field reference identifies the exact package and field identity.

An accepted variant-case reference identifies the exact package and case identity.

An accepted interface-operation reference identifies the exact package and operation identity.

An accepted port reference identifies the exact package and port identity when cross-package
selection is allowed.

Parent ownership is read from the canonical owner record or a deterministic authenticated
ownership witness.

Do not make callers rewrite when a referenced declaration moves between modules.

Do not make callers rewrite when a referenced declaration, member, binding, or module is renamed.

Do not store variable references as names after request normalization.

Do not store nominal field selection as a field name after request normalization.

Do not store nominal variant construction or matching as a case name after request normalization.

Do not store capability calls as an unverified capability-alias and operation-name pair after
request normalization.

Do not store lexical binding identity as an expression-tree path.

Do not store expression identity as a child-index path.

Paths and positions may be generated for review or diagnostics.

They do not define stable semantic identity.

The canonical graph must not retain two mutable representations of the same reference.

If relations can be derived exactly from canonical records, relations are derived witness and
query data, not a parallel editable graph.

If a secondary canonical structure is unavoidable, define one writer and an invariant proving
equality with its primary owner.

## Modules and namespaces

Modules organize meaning and names.

Modules are not presumed to be compiler units.

Modules are not presumed to be validator units.

Modules are not presumed to be storage shards.

Modules are not presumed to be cache invalidation units.

Modules are not presumed to be runtime loading units.

A declaration may move between modules without changing declaration identity.

Module membership affects namespace, visibility, review organization, and selector resolution.

It must not be embedded redundantly in every exact declaration reference.

Imports or use-bindings, when retained, are authoring namespace conveniences unless a specification
gives them an executable semantic role.

Accepted package dependencies remain exact semantic closure.

Do not force graph-authored code to retain source-language import machinery merely because the
bootstrap once used text.

Visibility and export are explicit declaration properties.

Namespace uniqueness is checked through exact deterministic indexes or witnesses.

Namespace lookup must not require scanning every declaration in a module.

## Owner records

Use a closed typed owner domain.

The owner domain should cover every construct that needs independent continuity, selection,
history, merge, refactoring, or diagnostics.

Do not allocate stable IDs for incidental syntax fragments with no continuity consumer.

Do allocate stable IDs when a public operation needs to select or preserve the construct across
edits.

Every owner record defines:

- its identity domain;
- kind;
- current parent or containment owner when semantically required;
- mutable presentation name when applicable;
- semantic payload;
- ordered child identities when order is semantic;
- exact references;
- documentation linkage where applicable;
- canonical encoding;
- validation contract.

Large payloads use separately addressed immutable objects.

A persistent map value should normally be a compact owner binding or object digest.

Do not exceed map value bounds by embedding arbitrarily large bodies, literals, child lists, or
documentation.

Use deterministic chunked objects for large ordered collections.

Chunk boundaries are physical details and must not become semantic identity.

## Types

Types are semantic values.

Stable type identity is required only for named declarations and type parameters.

Structural types should normally use canonical content-addressed type objects or another interned
deterministic representation.

A structural type digest identifies exact type meaning in its contract domain.

It is not a declaration identity.

Type objects must be closed, bounded, canonical, and independently decodable.

Recursive named types refer through stable declaration identities.

Type-parameter references use stable type-parameter identities after normalization.

Do not retain type-parameter names as semantic references.

Do not make type equality depend on insertion order, map iteration order, source spelling, or
physical object position.

Define canonical behavior for structural record field order.

Define canonical behavior for function parameter order.

Define canonical behavior for package-qualified named types.

Every type decoder checks length and recursion or work budget before allocation.

## Expressions and bindings

Expressions that are public selection or refactoring targets use stable expression identities.

An expression record stores one closed operation and exact child expression identities.

Root expression identities are referenced by their owning declaration or member.

Child containment is validated as an acyclic ownership tree unless a future specification
deliberately permits shared semantic expression DAGs.

Do not infer expression identity from child position.

Do not regenerate every expression identity when one sibling is inserted.

Lexical references use exact parameter or binding identities.

Binding names are mutable presentation.

Renaming a binding must not rewrite variable references.

Nominal record construction uses exact field identities.

Nominal field access uses exact field identities.

Nominal variant construction and match arms use exact case identities.

Capability invocation uses exact requirement and operation identities.

Function and constant references use exact package and declaration identities.

Expression order is explicit where evaluation order matters.

Large text and byte literals may use content-addressed blob objects.

The accepted expression graph must reject unreachable live expression records, multiple semantic
parents where sharing is forbidden, cycles, foreign-domain children, missing roots, and scope
violations.

Diagnostics identify the exact semantic owner and expression identity.

A rendered path may be included as a convenience.

The path is not authority.

## Relations and semantic dependencies

Canonical references are the primary relation authority.

Forward and reverse relation maps are deterministic derived validation and query data.

One relation extractor owns the mapping from canonical records to typed edges.

Do not maintain a handwritten relation table independently from the records that contain the
references.

Relation kinds must distinguish at least:

- namespace membership;
- type use;
- value reference;
- call;
- lexical reference;
- field use;
- variant construction;
- variant match;
- interface conformance;
- capability requirement;
- capability operation;
- component port binding;
- target binding;
- test dependency;
- generic instantiation when present;
- deployment-relevant requirement.

Relation keys use typed exact identities.

Relation order is deterministic.

Changing presentation-only data must not invent semantic dependency edges.

Every derived relation generation has an independent full rebuild oracle.

## Validation witness

Incremental correctness may depend on deterministic revision-bound validation witness data.

A validation witness is not editable program meaning.

A validation witness may include:

- owner semantic summaries;
- namespace uniqueness entries;
- ownership and parent entries;
- forward typed relations;
- reverse typed relations;
- test ownership;
- public-interface digests;
- implementation digests;
- effect and capability digests;
- presentation digests;
- compiler dependency summaries.

The accepted revision may commit to a compact digest of exact witness roots.

If it does, call the commitment what it is.

Do not describe a revision-committed witness root as an unrelated cache.

The witness pages may be disposable and rebuildable.

The witness root commitment is accepted validation evidence.

A missing witness rebuilds from canonical meaning.

A rebuilt witness that disagrees with the accepted commitment is corruption.

Witness generation is versioned by one mechanically registered contract.

Witness update is driven from the exact canonical delta.

Do not run independent handwritten update logic for each public operation class.

The full witness rebuild must remain implementation-disjoint enough to catch incremental defects.

## One authority over text

Maintained text is not a second editable lkjscript program authority.

Text may be:

- deterministic review projection;
- bounded diagnostic excerpt;
- human-oriented explanation;
- external data consumed by a program;
- independent test-oracle input;
- recovery evidence;
- a non-authoritative change request;
- generated protocol reference.

A review projection identifies its exact source revision and non-authoritative status.

A review projection has no silent apply path.

The absence of editable source raises the required quality of graph authoring.

Do not compensate with raw storage records.

Do not compensate with giant recursively nested transaction JSON as the only practical authoring
surface.

Do not compensate with private Rust fixture builders.

Do not compensate with generated source that is later reparsed into authority.

A structured change request is temporary intent.

It is not a second program database.

A text parser retained only for tests must not own current language semantics or force the
canonical graph back into a source-shaped representation.

## Identity

Use durable identity only when a concrete consumer needs continuity.

Concrete consumers include references, rename, move, history, merge, refactoring, deployment
binding, diagnostics, and persistent external linkage.

Separate:

- stable semantic identity;
- mutable name;
- namespace;
- package identity;
- content digest;
- revision identity;
- transaction identity;
- physical object digest;
- physical pack coordinate;
- compiler index;
- runtime handle;
- rendered coordinate;
- request-local symbol;
- session-local short handle.

No value may silently represent multiple identity domains.

Every identity domain defines:

- owner;
- canonical binary encoding;
- canonical text encoding;
- equality;
- allocation;
- retention;
- deletion;
- nonreuse rule;
- diagnostics;
- transport;
- collision behavior;
- foreign-domain rejection.

Names and paths are locators and presentation unless a normative specification explicitly makes
them identity.

Rename and move preserve or replace identity deliberately.

A content digest proves equality or integrity only in its exact domain.

A digest does not prove provenance, authority, freshness, permission, or authorship.

Reject foreign-domain identities even when display bytes coincide.

Do not require clients to preallocate every stable ID.

The change engine allocates stable IDs atomically from one normalized request.

Support request-local symbolic identities for references among new owners in one atomic change.

Return the allocated stable-ID map in the compact receipt.

Session-local short handles may reduce repeated long IDs.

A short handle binds one session, one repository, one exact revision, and one typed identity.

A short handle must fail closed after session invalidation.

A short handle is never serialized into accepted meaning.

If stable identities are deterministically derived, bind derivation to repository, exact base,
normalized request digest, identity domain, and canonical allocation ordinal.

Do not derive stable identity from mutable name, module membership, tree path, or current content.

## Revisions and publication

Accepted history is immutable and exact.

A revision commits to every semantic input needed to reconstruct accepted meaning.

A revision may also commit to exact deterministic validation-witness roots required for trusted
incremental reuse.

A revision record must remain bounded.

A revision record must not repeat the complete graph.

A receipt must remain bounded.

A receipt must not repeat every affected owner, every passing check, or complete child logs inline.

Retain expandable exact evidence out of band.

Publication has one atomic visibility point.

Readers observe the old complete revision or the new complete revision.

Writers compare against the exact current base under the publication lock or equivalent
serialization boundary.

New immutable data becomes durable before visibility changes.

The public visibility record becomes durable after the immutable data it names.

Publication uncertainty is reconciled by reading current authority and retained receipts.

Do not retry blindly after indeterminate visibility.

No derived-index failure may partially publish accepted meaning.

A derived-index failure after accepted publication must be recoverable by rebuild.

## Changes, transactions, and deltas

All ordinary program mutations lower through one exact semantic edit pipeline.

The public surface expresses high-level intent.

The normalized internal form expresses exact stable-ID record edits.

The physical publication form expresses exact persistent-map and immutable-object deltas.

Do not expose physical edits as ordinary authoring.

The public operation set should cover:

- create;
- replace;
- delete;
- rename;
- move;
- clone;
- restore;
- rebind;
- extract;
- inline;
- introduce binding;
- change signature;
- change effect;
- change visibility;
- introduce type parameter;
- add, remove, rename, reorder, or change a field;
- add, remove, rename, reorder, or change a variant case;
- add, remove, rename, reorder, or change an interface operation;
- add, remove, rename, reorder, or change a parameter;
- add, remove, or change a capability requirement;
- add, remove, rename, reorder, or change a component port;
- replace or splice an expression;
- create or update a test;
- create or update a target;
- add, replace, or remove an exact dependency;
- resolve a typed conflict.

One generic preparation pipeline should perform:

1. strict decode;
2. contract check;
3. selector resolution at the exact base;
4. request-local identity allocation;
5. high-level normalization;
6. primitive owner and object edits;
7. overlay construction;
8. canonical delta extraction;
9. witness delta extraction;
10. impact analysis;
11. incremental validation;
12. test and compiler planning;
13. predicted revision construction;
14. publication preparation.

Planning, dry-run, validation, and commit share that pipeline.

Do not implement operation-specific transaction fast paths as the long-term incremental engine.

Do not duplicate import-closure traversal in separate rename, create, and body-replacement
functions.

Preconditions are exact point or bounded-query checks.

The mere presence of a precondition must not force complete graph reconstruction.

A precondition identifies its exact observed revision and selected semantic fact.

A prepared commit handle is valid only for its exact base, normalized request, contract registry,
validator contract, and process or persistent-handle policy.

Under the publication lock, recheck the base and every publication-critical assumption.

Do not repeat deterministic semantic validation when the exact prepared result remains valid.

Stale base, stale handle, precondition failure, ambiguity, foreign identity, invalid meaning,
conflict, resource exhaustion, cancellation, semantic no-change, corruption, and infrastructure
failure remain distinct.

## Overlay and delta invariants

A candidate overlay reads unchanged records from the exact base and stores only changed records and
new immutable objects.

The overlay must not clone the complete owner map.

The overlay must not clone every module or declaration.

The delta records exact before and after digests.

An insertion asserts prior absence.

A replacement asserts the exact prior digest.

A deletion asserts the exact prior digest.

A no-change is explicit and publishes nothing.

The delta includes every changed canonical owner.

The delta includes every newly reachable canonical object.

The delta identifies every newly unreachable canonical object for retention accounting.

The delta extractor produces deterministic order.

The same normalized change against the same exact base produces the same semantic result.

Physical pack placement does not affect semantic or revision identity.

## Incremental validation

Ordinary local work scales with the changed owner set plus the conservative affected closure.

Whole-project work is explicit.

Design the graph, storage API, validator, compiler, query engine, and tests so locality is visible
in types and measurements.

An ordinary mutation API that requires `Vec<AllModules>` is presumptively wrong.

An ordinary mutation API that reconstructs a recursive project AST is presumptively wrong.

An ordinary rename that scans unrelated owners is presumptively wrong.

An ordinary move that rewrites exact callers is presumptively wrong.

Maintain exact dependency and reverse-dependency information for all semantically relevant edge
kinds.

Incremental reuse is valid only when every semantic input is proven identical.

Reuse keys include:

- graph contract;
- validator contract;
- exact owner content digest;
- exact type-object digests;
- dependency interface digests;
- relevant implementation digests;
- package contracts;
- feature and target policy;
- semantic environment;
- compiler contract when compiler reuse is involved.

Classify changes precisely.

Useful classes include:

- presentation only;
- namespace only;
- private implementation;
- public interface;
- type structure;
- effect;
- capability;
- component or target;
- dependency;
- test only;
- deletion or ownership move;
- mixed.

Presentation-only changes must not trigger compilation or unrelated tests.

Stable-ID rename must not invalidate exact semantic callers.

Private implementation changes revalidate the changed owner and retest conservative execution
dependents.

Public-interface changes revalidate conservative type, call, capability, target, and test
dependents.

Effect and capability changes propagate through their exact edge kinds.

Mixed changes use the union of exact deltas and conservative frontiers.

Do not fall back to complete validation merely because multiple primitive edits appear in one
request.

Keep an implementation-disjoint full validator and full witness rebuild.

Differential tests compare incremental and full outcomes over long deterministic mutation
sequences.

Corrupt or missing derived state rebuilds or fails clearly without changing accepted meaning.

## Drafts and conflicts

Drafts are explicit non-executable authority.

A draft binds:

- repository identity;
- exact base revision;
- generation;
- normalized pending edits;
- unresolved selectors or holes;
- diagnostics;
- typed conflicts;
- bounded intent.

Draft mutation cannot change accepted HEAD.

A draft with holes or conflicts cannot build, run, deploy, or publish.

Rebase is explicit.

Merge conflicts persist as typed records with stable conflict identities.

A conflict identifies:

- semantic owner;
- conflict kind;
- merge base;
- left value or absence;
- right value or absence;
- relevant child or relation role;
- allowed resolution forms.

Do not encode conflicts as prose, source markers, or untyped blobs.

Resolution is an exact semantic operation.

Resolution may select base, left, right, deletion, or a custom normalized value when the conflict
kind allows it.

Dropping a draft cannot affect accepted authority.

A merge implementation must exploit stable identity.

Do not reduce semantic merge to rendered-text merge.

## Public CLI

The public CLI is the ordinary development environment for lkjscript applications.

It supports:

- first-use discovery;
- project creation;
- compact orientation;
- exact inspection;
- bounded search;
- references and callers;
- type and capability use;
- task-scoped context;
- impact analysis;
- high-level changes;
- semantic refactoring;
- drafts;
- validation;
- publication;
- diff;
- merge;
- typed conflict resolution;
- history;
- tests;
- build;
- run;
- deployment inspection;
- backup;
- restore;
- repair;
- deep doctor;
- agent stdio operation.

Do not require users to understand physical graph storage.

Do not require users to know Rust enum layouts.

Do not make giant strict JSON the only practical way to create ordinary meaning.

A structured request protocol is necessary.

Ceremony is not correctness.

Prefer a small compositional command grammar over a flat catalog of aliases.

Use one public term for one concept.

Remove aliases rather than preserving synonyms.

Every normal response is:

- deterministic;
- bounded;
- machine-readable;
- revision-pinned;
- schema-identifiable;
- expandable;
- free of secrets.

Default success is the smallest complete summary.

Large results use selected fields, item and byte budgets, continuation, output files, or stable
revision-bound expansion handles.

Do not print full schemas on every call.

Do not print complete graphs.

Do not print every passing test.

Do not print child build logs by default.

Do not print repeated environment facts.

Do not print stack traces for ordinary failures.

Failures provide a compact actionable summary, stable diagnostic identities, the highest-value
diagnostics, exact expansion commands or handles, and retained complete logs when applicable.

Human rendering may exist as an explicit projection.

Human rendering derives from the same structured result.

## Stdio agent protocol

A resident stdio protocol may accelerate repeated agent work.

Stateless per-command operation remains fully correct.

The protocol uses strict framed or line-delimited messages with explicit version negotiation.

The default transport should remain easy for coding agents to generate and inspect.

A session handshake returns:

- protocol contract;
- schema digest;
- changed schema sections when requested;
- session identity;
- opened repository identity when applicable;
- exact pinned revision;
- default budgets;
- supported operations;
- feature and non-claim summary.

Each request has a client request identity.

Each response echoes that identity.

Responses may complete out of order only when the protocol explicitly permits it.

A session may retain:

- opened repository metadata;
- exact revision pins;
- decoded schemas;
- disposable indexes;
- task context handles;
- prepared change handles;
- compiler data;
- short typed identity handles.

A session must not own accepted meaning.

A session invalidates every retained item on any relevant contract, repository, revision, request,
budget, or environment change.

External HEAD movement must produce explicit stale state.

Session cancellation is request-scoped.

Session queues are bounded.

Session memory is accounted.

Session shutdown is deterministic.

A session crash cannot corrupt accepted meaning.

No session-only handle may enter accepted graph records, artifacts, backups, or durable drafts.

## Protocol registry and generated truth

One executable registry owns public protocol metadata.

The registry covers:

- current contract identities and versions;
- commands and subcommands;
- request and response schemas;
- change forms;
- selector forms;
- type forms;
- expression forms;
- owner kinds;
- relation kinds;
- diagnostic classes;
- exit statuses;
- templates;
- runner kinds;
- limits and their classifications;
- capability and non-claim declarations.

Do not maintain parallel handwritten lists for parsing, help, capability discovery, documentation,
and tests.

Generate or mechanically verify those views from one owner.

Every generated view includes an owner digest or schema digest.

`docs/status.md` must not manually drift from executable contract versions.

README examples must be checked against current schema or black-box tests.

A contract change updates registry, codecs, schemas, examples, tests, predecessor rejection, and
generated reference in one cutover.

Generated files are checked for freshness in verification.

Do not add an unbounded reflection framework when a small closed registry suffices.

## Agent context and provider economy

Treat model context, provider requests, cached input, output tokens, tool calls, verification time,
and correction depth as scarce resources.

Do not weaken correctness merely to reduce output.

Provide task-scoped context bundles derived from ownership and dependency closure.

A context bundle states:

- exact repository and revision;
- selected seeds;
- selection profile;
- inclusion reasons;
- omissions;
- item count;
- byte count;
- semantic work consumed;
- truncation;
- diagnostics;
- expansion operations.

Use content-addressed or revision-bound handles so unchanged context is not resent.

Permit field projection.

Return summaries before bodies.

Use short handles inside one exact session.

Do not dump the repository or complete graph when a bounded slice is sufficient.

Do not require a model to infer the current schema from error messages.

Use schema digests and changed-section discovery.

Keep a concise campaign ledger of durable facts, decisions, measurements, risks, receipts, and
next actions.

Do not record hidden reasoning.

Measure provider usage when the surrounding harness exposes it.

Track, when available:

- input tokens;
- cached input tokens;
- output tokens;
- requests;
- retries;
- tool calls;
- elapsed time;
- monetary cost.

Bytes are not tokens.

Tokens are not money without provider pricing and cache behavior.

Compare equal complete tasks, including failures, retries, corrections, and verification.

Use cheaper models for bounded mechanical work when they are adequate.

Use stronger models for architecture, invariants, adversarial review, and difficult integration
when expected correction savings justify them.

Do not create redundant multi-agent work merely because agents are available.

## Limits and resource policy

Do not treat every constant as one kind of limit.

Classify each limit as one of:

- encoding or addressability limit;
- hostile-decoder safety limit;
- semantic invariant;
- explicit request budget;
- deployment resource policy;
- default pagination size;
- implementation limitation;
- benchmark fixture limit.

A hard public semantic limit requires a documented invariant or unavoidable representation bound.

An implementation limitation is labeled temporary and appears in status.

A default does not masquerade as a maximum.

A growing result paginates, streams, shards, or writes out of band rather than failing at an
incidental count.

A local change consumes explicit work proportional to its semantic impact.

Do not raise memory, queue, timeout, recursion, response, transaction, history, or owner-count
limits to hide an algorithmic defect.

Do not impose arbitrary file-count, line-count, directory-count, module-count, declaration-count,
or operation-count policy.

Decoder and allocator bounds remain mandatory at hostile boundaries.

Use checked arithmetic before allocation.

Expose relevant budgets through machine-readable discovery.

Resource exhaustion reports consumed work and a safe continuation or narrower next action when
possible.

## Physical storage

Separate logical graph shape from physical storage.

Canonical identity is independent of pack name, file path, offset, compression block, or catalog
entry.

Use a narrow content-addressed object-store interface.

The interface supports bounded read, immutable write, existence check, staged publication,
iteration for repair, and corruption classification.

Persistent maps remain canonical only through their logical roots and page bytes.

Map pages may be stored inside immutable object packs.

Large owner records, type objects, literals, child-list chunks, revisions, receipts, and artifacts
may share the same generic immutable-object substrate when domain separation remains exact.

Avoid one filesystem inode per small canonical object at large scale.

Use immutable packs or segments when complete-task evidence shows material benefit.

A pack has:

- closed contract;
- deterministic entry encoding where semantic digest depends on it;
- entry digest;
- entry length;
- bounded index;
- checksums;
- sealed footer;
- corruption detection;
- no in-place mutation after sealing.

Physical packing order must not affect semantic, object, root, revision, or artifact identity.

A disposable catalog maps object digest to physical pack coordinates.

The catalog is rebuildable from sealed pack indexes.

A catalog is not program authority.

Staged publication may write one or more private packs, seal and synchronize them, publish
immutable revision data, and then atomically change HEAD.

Crash recovery distinguishes invisible staged packs, visible accepted packs, corrupt packs, and
unreferenced packs.

Do not delete an old object or pack merely because current HEAD does not reference it.

## Backup, restore, retention, and compaction

Backup transports exact accepted authority.

Backup is not a writer until verified restore creates visible authority.

Backup enumeration must be streaming or externally bounded.

Do not retain an unbounded in-memory set proportional to every reachable object.

Use deterministic spill, sorted runs, persistent scratch maps, or another bounded method.

A backup names exact root revisions, drafts, pins, and included object domains.

A backup may physically contain extra unreachable immutable objects only when the contract states
that fact and restore still verifies the exact reachable authority.

Restore validates every declared object and every reachable binding before visibility.

Restore writes into a private stage and publishes the destination atomically.

Retention policy explicitly defines:

- retained accepted revisions;
- branch or merge roots;
- live drafts;
- explicit pins;
- registered backup roots;
- active reader leases;
- deployment or artifact pins when applicable;
- derived data;
- unknown objects;
- grace periods.

Garbage collection begins as exact dry-run evidence.

Destructive collection is allowed only after root enumeration, reader leases, pins, backup roots,
crash behavior, and independent reachability oracle are complete.

With packed storage, object reclamation normally occurs through compaction.

Compaction copies live immutable objects into new packs, verifies them, publishes a rebuilt
catalog, waits for safe lease conditions, and only then removes old packs.

Compaction does not change accepted semantic or revision identity.

Interrupted compaction must leave at least one complete readable object copy.

Unknown files and foreign formats fail closed or remain conservatively retained.

## Queries and indexes

Queries are revision-pinned.

Ordering does not depend on hash iteration, object-pack position, or filesystem enumeration.

Continuations bind exact normalized query, revision, schema, projection, and cursor.

A changed query, projection, schema, or revision invalidates continuation.

Indexes are disposable unless a normative specification deliberately promotes one into accepted
validation evidence.

Exact owner lookup touches a bounded persistent-map path and one owner object.

Exact references do not require a broad query index.

Name lookup uses a deterministic namespace index.

Relation lookup uses deterministic forward or reverse relation indexes.

Broad traversal uses explicit work budgets.

Cold orientation does not decode every body.

Index update is delta-driven.

Missing or corrupt indexes rebuild from canonical authority.

Keep independent query oracles for tests.

Context and impact queries explain inclusion reasons.

A query response may assign session-local short handles.

A query response never changes accepted meaning.

## Packages and dependencies

Packages and modules are graph meaning, not filesystem conventions.

Define package identity, module identity, namespace, visibility, dependencies, cycles,
initialization, and diagnostics explicitly.

Accepted dependencies bind exact immutable package and semantic-revision identities.

Do not resolve accepted builds from mutable tags, ambient directories, undeclared network state,
current-working-directory accidents, credentials, or latest-version lookup.

A released binary may carry exact built-in package artifacts for offline bootstrap.

Embedded artifacts are visible, versioned, inspectable, exportable when useful, and reproducible
from maintained graph authority.

Dependency staging, embedding, export, backup, restore, and project initialization converge on one
package-object contract.

Package closure enumeration is bounded and cycle-checked.

Cross-package references contain the minimum exact identity needed for stable resolution.

Do not redundantly embed mutable module location in declaration and member references.

## One-binary bootstrap

A released `lkjscript` executable supports complete offline first use from an empty directory.

The binary provides enough self-description to discover:

- current protocol and schema;
- supported semantic forms;
- project creation;
- change construction;
- selector syntax;
- validation;
- testing;
- build;
- run;
- diagnostics;
- result expansion;
- backup and restore.

Creating the first project does not require:

- a repository checkout;
- Cargo;
- Rust source;
- Python;
- network access;
- a registry;
- a preexisting external artifact;
- undocumented environment state;
- direct graph-storage editing.

Any embedded standard package, prelude, template, or bootstrap artifact is exact, versioned,
inspectable, integrity-checked, and generated from one maintained authority.

A template is a normalized change recipe.

A template is not a hidden second writer.

The binary-only path has black-box acceptance in an isolated temporary environment.

## Language design

Prefer a small orthogonal language core.

Move reusable policy into graph-authored libraries.

Do not add product-shaped primitives.

Make explicit:

- evaluation order;
- equality;
- ordering;
- integer overflow;
- indexing;
- Unicode behavior;
- serialization;
- effects;
- failure classes;
- allocation-relevant behavior;
- cancellation;
- resource lifetime.

Avoid implicit coercion, order-dependent inference, ambient overload resolution, hidden authority,
hidden global state, and nondeterministic expansion.

Expected program outcomes are typed values.

Traps, capability failures, possible external visibility, exhaustion, cancellation, corruption,
and infrastructure failure are distinct.

Pure functions remain independent from time, randomness, deployment, scheduling, and external
state.

The canonical graph retains enough explicit type and reference information for deterministic
validation, review, and compilation.

Request-time elaboration may infer convenience information only when the accepted normalized graph
stores the resolved result.

Do not let authoring convenience make accepted meaning depend on ambient query state.

## Abstraction mechanisms

The language provides enough abstraction for reusable libraries and diverse applications without
copy-paste graph expansion.

New abstraction mechanisms require exact semantics and multiple complete consumers.

Evaluate, when demanded by real consumers:

- parametric polymorphism;
- generic data declarations;
- constrained generic functions and data;
- higher-order functions;
- lexical closures and capture;
- reusable component composition;
- type aliases or newtypes;
- graph-native change recipes;
- semantic refactorings.

Do not conflate capability interfaces with type constraints unless their invariants truly coincide.

Do not introduce a second macro language casually.

A graph-native recipe produces a normalized change request.

A recipe is not accepted program authority.

Generic declaration identity and concrete instantiation identity are distinct.

Type inference should reduce ceremony without making accepted meaning context-dependent or
order-dependent.

Compiler specialization is derived.

Specialization does not change language meaning.

## Components, effects, and capabilities

One component and port model should cover command, HTTP, interactive, batch, worker, and test
runners where semantics genuinely align.

Runner kinds are target or deployment metadata, not language editions.

Applications declare typed capability requirements.

Deployment grants bind adapters, authority, secrets, sharing domains, and limits.

Artifacts contain requirements.

Artifacts do not contain grants, credentials, or live resources.

Generic adapters own protocol and resource mechanics.

Generic adapters do not own application routes, schemas, authorization roles, SQL policy, object
keys, retry policy, rendering, or domain transitions.

Every live resource defines acquisition, owner, permitted operations, close, cancellation,
timeout, cleanup, observability, and non-persistence.

Production and deterministic test adapters should be behaviorally comparable and
implementation-disjoint where practical.

## Runtime and compiler

Keep one runtime kernel for preparation, admission, execution, capability routing, task ownership,
resource accounting, cancellation, shutdown, and observations.

Concurrency is bounded and structured.

Do not create hidden unbounded queues.

Do not create detached ownerless tasks.

Graceful shutdown defines admission stop, drain, cancellation, non-cancellable publication,
resource cleanup, timeout, and exit status.

A process boundary is not semantic identity.

A process boundary is not a hostile-code sandbox.

Maintain an independently checkable semantic execution route.

Bytecode, specialized interpreters, AOT, JIT, and caches are derived tiers.

They require equivalence, invalidation, resource accounting, and fallback evidence.

Do not add a JIT because it is fashionable.

Do not reject a JIT merely because it was previously deferred.

Use maintained workloads and profiles to decide.

Compilation units should follow semantic dependency and reuse boundaries, not module boundaries by
default.

A compiler unit key binds:

- compiler contract;
- exact declaration or semantic unit digest;
- relevant dependency interface digests;
- effect and capability contract;
- target;
- optimization policy;
- relevant environment.

Incremental and clean builds of the same accepted revision produce byte-identical artifacts.

Artifacts are segmented or streamed when one monolithic container would impose incidental limits.

Artifacts contain only the semantic metadata required for execution, inspection, dependency
binding, and declared reproducibility.

Do not embed credentials or deployment grants.

Keep stable semantic identities out of hot runtime representation unless behavior needs them.

After validation, lower to compact dense indexes.

## TLS is out of scope

lkjscript does not plan to implement TLS in the current product direction.

Do not add HTTP TLS termination, certificate parsing, certificate issuance, certificate rotation,
ACME, PostgreSQL TLS, a speculative TLS abstraction layer, or TLS-specific language primitives.

Deployments requiring encrypted transport use an appropriate external trusted boundary or a
different adapter outside current scope.

Keep plaintext HTTP and current database-transport limitations explicit.

Do not imply that external termination creates hostile multi-tenant isolation.

## Performance

Long-term performance is a first-class requirement.

Optimize complete workflows, not isolated instruction folklore.

Require asymptotically sound structures before micro-optimization.

Measure at least:

- cold and warm discovery;
- blank-directory project creation;
- orientation;
- exact owner lookup;
- name lookup;
- relation lookup;
- context construction;
- local mutation;
- mixed mutation;
- wide mutation;
- rename;
- move;
- expression splice;
- signature change;
- validation;
- witness update;
- publication;
- diff;
- merge;
- conflict persistence and resolution;
- build;
- test selection;
- startup;
- execution;
- service;
- worker;
- backup;
- restore;
- retention inventory;
- compaction;
- fresh checkout;
- binary-only bootstrap;
- stdio session handshake;
- repeated session operations.

Include many tiny owners, large owners, dense relation fanout, deep types and expressions, large
literals, long history, branch conflicts, corrupt inputs, sparse edits, and mixed edits.

Record, where available:

- wall time;
- CPU time;
- peak RSS;
- allocation count or allocated bytes;
- bytes read;
- bytes written;
- fsync count;
- object count;
- pack count;
- storage growth;
- output bytes;
- semantic work;
- validation frontier;
- reused facts;
- compiler units rebuilt;
- tests selected;
- cache state;
- binary size;
- provider usage.

A local edit benchmark reports changed owners, witness edits, invalidation closure, reused
summaries, compiler reuse, selected tests, and full-oracle comparison.

A scale claim names the exact graph shape.

A million independent tiny declarations is not evidence for a million densely connected owners.

A warm-cache result is not a cold-cache result.

A one-process session result is not a stateless command result.

Do not hide failed scale attempts.

Retain the failure mode and the last valid measurement.

## Verification

Use the narrowest sufficient gate during iteration.

Run a complete authoritative gate before final publication.

Change-aware selection is convenience, not proof.

Uncertainty widens to full verification.

Skipped, unavailable, flaky, timed-out, exhausted, cancelled, or unrun is not pass.

All-pass verification is quiet.

Return one aggregate summary and exact receipt locator.

Retain bounded stdout and stderr per child gate.

On failure, return a bounded high-value excerpt and exact log locators.

Do not print every passing test.

Build verification as an explicit dependency graph.

Do not rerun an identical expensive gate merely because multiple profiles mention it.

A reused pass requires an exact fingerprint of every input.

A final full gate may deliberately require fresh execution.

Verification receipts distinguish fresh and reused evidence.

Test:

- formatting;
- static analysis;
- locked build;
- generated-file freshness;
- protocol registry coherence;
- graph invariants;
- hostile decoders;
- object-pack integrity;
- publication;
- crash interruption;
- incremental/full equality;
- witness/full-rebuild equality;
- query/index oracle equality;
- compiler/reference equality;
- clean/incremental artifact equality;
- property sequences;
- fuzzing where useful;
- CLI contracts;
- stdio protocol contracts;
- binary-only bootstrap;
- predecessor rejection;
- application acceptance;
- backup and restore;
- restart;
- cancellation;
- overload;
- deterministic artifacts;
- retention and compaction;
- fresh checkout;
- Git diff integrity.

## Testing policy

Test public behavior at the public boundary.

Use private unit tests for local invariants.

Do not use private tests as substitutes for public acceptance.

Maintain at least one implementation-disjoint oracle for every high-risk optimization.

Generate long deterministic mutation sequences.

Include shrinking or exact reproduction for property failures.

Test no-change and rejection paths for absence of publication.

Test stale concurrent writes.

Test idempotent replay.

Test foreign identity.

Test stale session handles.

Test wrong-revision short handles.

Test budget exhaustion before excessive allocation.

Test corruption of each canonical object class.

Test corruption of each object-pack index and footer class.

Test loss and corruption of each derived index class.

Test interrupted publication at every durability boundary that can be simulated.

Test interrupted compaction at every visibility boundary.

Test binary-only project creation without the repository checkout.

Test that templates and embedded packages can be inspected and reproduced.

Test that ordinary local operations do not perform full-graph work through semantic counters,
object-read tracing, or filesystem tracing.

Test declaration move without caller rewrite.

Test binding rename without expression rewrite.

Test field and case rename without nominal-use rewrite.

Test presentation-only edits without compiler invalidation.

Test mixed changes without unconditional full reconstruction.

## Security and trust

Treat as hostile decoding input:

- graph objects;
- map pages;
- pack files;
- pack indexes;
- artifacts;
- backups;
- change requests;
- drafts;
- continuations;
- session frames;
- context handles;
- deployment descriptors;
- network input;
- database rows;
- object-store responses;
- queue records;
- environment values.

Use closed contracts, exact bounds, duplicate rejection, trailing-data rejection, checked
arithmetic, pre-allocation checks, canonical identity encodings, path and symlink defense, secret
redaction, and typed failures.

Do not claim hostile-code sandboxing, multi-tenant isolation, constant-time behavior,
cryptographic provenance, distributed consensus, cross-node transactions, portability, TLS, or
authenticated artifacts without complete evidence and an active requirement.

Accepted lkjscript programs are trusted program inputs unless the threat model changes explicitly.

A digest is not a signature.

A local process is not a tenant boundary.

A content-addressed pack does not authenticate its publisher.

A validation witness does not authorize deployment.

## Rust engineering

Use stable Rust 2024 unless a verified campaign deliberately changes the bootstrap.

Prefer explicit ownership, typed domain wrappers, checked arithmetic, bounded allocation,
iterative traversal, compact representations, narrow interfaces, typed errors, and deterministic
collections where order matters.

First-party `unsafe` is forbidden unless root policy is deliberately changed with a documented
invariant and focused evidence.

Warnings are defects.

Do not silence lints broadly.

Avoid panic on untrusted or operational input.

`expect`, `unwrap`, `todo`, and `unimplemented` remain prohibited by repository lint policy outside
narrowly justified test policy.

Large files are not automatically defects.

A file that owns unrelated protocol, storage, validation, rendering, and command responsibilities
is a defect even when tests pass.

Split by semantic ownership, invariant boundary, and testability.

Do not split by arbitrary line count alone.

Do not create indirection-only modules.

Public module boundaries should follow:

- semantic kernel;
- storage;
- edit normalization;
- validation;
- witness and indexes;
- protocol;
- compiler and artifact;
- runtime and adapters;
- verification support.

Avoid cyclic ownership between those layers.

The semantic kernel must not depend on CLI parsing.

The store must not depend on application adapters.

The compiler must not mutate accepted authority.

The query layer must not become a second writer.

## Dependencies

A dependency needs a named complete consumer, narrower implementation than writing it locally,
acceptable build cost, acceptable binary-size cost, acceptable security surface, active
maintenance prospects, and narrow features.

Do not add a graph database, parser generator, RPC stack, compiler framework, or runtime merely to
avoid understanding the system.

Do not reject a dependency categorically when it materially improves the final design.

Prototype and measure major dependency choices.

Remove dependencies made obsolete by a cutover.

Keep the locked dependency closure reproducible.

A schema-generation dependency is acceptable only if it truly removes parallel protocol truth.

An embedded database is acceptable only if it materially improves the final storage model and
does not become a hidden second authority.

## Auxiliary coding agents and models

The development environment may provide Codex subagents, `herdr`, `pi` with Qwen Cloud,
`antigravity`, and other coding-agent tools.

Discover actual local interfaces with help commands before use.

Use auxiliary agents for independent audits, bounded prototypes, adversarial review, mechanical
migration, test generation, or parallel evidence collection when that materially improves the
result.

Give each auxiliary agent task-scoped context and exact deliverables.

Do not send the complete repository by default.

Do not send secrets, credentials, private data, hidden reasoning, or unrelated files.

Treat auxiliary output as untrusted advice.

The primary agent owns architecture, integration, tests, and final claims.

Do not blindly copy generated code.

Verify every adopted result in the checkout.

Retain concise durable conclusions, not full transcripts.

Do not allow auxiliary agents to make overlapping edits to the same ownership boundary without an
explicit integration plan.

## Documentation and evidence

Normative behavior belongs under `docs/spec/`.

Current implementation reality belongs in `docs/status.md`.

Layer ownership belongs in `docs/architecture.md`.

Threat model and non-claims belong in `docs/security.md`.

Reproduced measurements belong in `docs/performance.md` and structured evidence.

Future evidence-gated work belongs in `docs/roadmap.md`.

User workflows belong in `README.md` and application documentation.

Generated protocol and contract reference belongs under a clearly generated path.

A campaign ledger records audited baseline, decisions, alternatives, experiments, measurements,
migrations, deletions, verification, and remaining limitations.

Evidence names exact commit, worktree identity, toolchain, platform, command, inputs, cache state,
receipt, and limitations.

Delete obsolete current documentation after cutover.

Historical evidence remains only when labeled historical.

Do not duplicate exhaustive command grammar outside its executable owner.

Generate reference material from the protocol registry.

Contract version tables must be generated or mechanically checked.

## Required working method

1. Inspect the worktree and effective instructions.
2. Reproduce current public behavior before redesign.
3. Build an authority, identity, relation, and contract map.
4. Identify recursive source-shaped authority, duplicated truth, whole-graph work, redundant
   ceremony, and incidental limits.
5. Define measurable destination invariants.
6. Prototype the riskiest storage and normalization choices in bounded form.
7. Select by complete-task evidence.
8. Implement dependency-closed vertical slices.
9. Migrate standard packages, templates, and maintained applications through public paths.
10. Delete superseded code and contracts.
11. Run focused verification continuously.
12. Run scale, failure, recovery, and binary-only acceptance.
13. Update registry, specs, docs, evidence, and status.
14. Run a fresh complete gate.
15. Inspect diffs, commit coherent changes, and push only when authorized.
16. Deliver exact commits, verification, publication state, limitations, and unperformed actions.

## Forbidden shortcuts

Do not maintain source and graph as independently editable truths.

Do not make text the normal mutation path.

Do not keep a recursive source AST as canonical meaning beside normalized owner records.

Do not keep path-based expression identity after normalized stable expression records exist.

Do not keep module-bearing declaration references after package-plus-declaration identity is
sufficient.

Do not expose raw storage records as authoring.

Do not replace text with equally verbose recursive JSON.

Do not require full-repository context for local work.

Do not infer stable identity from name, path, position, module, or content hash without an explicit
domain rule.

Do not use generated lkjscript source, private Rust builders, or opaque fixtures as maintained
program authority.

Do not add application-specific Rust business policy.

Do not bypass typed requirements and grants with ambient host calls.

Do not treat caches, bytecode, logs, validation summaries, projections, or physical catalogs as
program meaning.

Do not preserve predecessor formats through editions, aliases, fallback readers, or dual writers.

Do not keep two command names for the same behavior.

Do not maintain manual contract tables that can drift from source.

Do not claim API-cost savings from output bytes alone.

Do not print every passing test or complete child log by default.

Do not raise limits to hide poor algorithms.

Do not implement TLS.

Do not introduce Lean files, toolchains, dependencies, experiments, or references.

Do not commit hidden reasoning, secrets, helper-agent transcripts, or unrelated work.

Do not stop after writing architecture documents when implementation is feasible.

Do not leave a new architecture layered over the old one.

## Completion standard

A campaign is complete only when every applicable item is true.

- [ ] One canonical normalized semantic-graph authority remains.
- [ ] Recursive source-shaped authority and parallel identity/relation truth are removed.
- [ ] Exact references contain no redundant mutable module locator.
- [ ] Stable declaration move does not rewrite exact callers.
- [ ] Stable member and binding rename does not rewrite exact uses.
- [ ] Expression selection uses stable identity rather than tree path.
- [ ] The public CLI completes every changed application workflow.
- [ ] The stdio protocol, when introduced, is bounded and authority-free.
- [ ] A single released binary creates and develops an application from an empty directory.
- [ ] Embedded bootstrap meaning is exact, inspectable, and reproducible.
- [ ] Maintained applications use public paths rather than private builders.
- [ ] Text and predecessor paths are deleted or explicitly non-authoritative.
- [ ] Public vocabulary is smaller, defined, and free of unnecessary aliases.
- [ ] Stable identity, revision, edit, publication, and failure classes are exact.
- [ ] Request-local creation does not require preallocating stable IDs.
- [ ] Local operations read and validate bounded affected slices.
- [ ] Mixed edits do not unconditionally reconstruct the full graph.
- [ ] Preconditions do not unconditionally force full reconstruction.
- [ ] Whole-project work is explicit and measured.
- [ ] Incremental and full validators agree over deterministic mutation sequences.
- [ ] Incremental and full validation-witness generation agree.
- [ ] Query indexes agree with canonical reconstruction.
- [ ] Production and independent execution paths agree.
- [ ] Clean and incremental artifacts are byte-identical.
- [ ] Language abstraction is sufficient for maintained reusable libraries.
- [ ] No product-specific Rust policy was added.
- [ ] No TLS implementation or speculative TLS layer was added.
- [ ] Resource limits are classified and justified.
- [ ] Arbitrary public count ceilings are removed or replaced by explicit resource policy.
- [ ] Storage supports measured large-graph behavior and bounded backup enumeration.
- [ ] Compaction and destructive retention are enabled only with exact safety roots and leases.
- [ ] Quiet verification retains exact expandable evidence.
- [ ] Verification avoids proven duplicate work without overstating reused evidence.
- [ ] Protocol discovery, generated schemas, docs, and executable versions agree.
- [ ] Maintained consumers pass acceptance.
- [ ] Binary-only bootstrap passes in an isolated environment.
- [ ] Backup, restore, corruption, interruption, restart, and fresh-checkout tests pass.
- [ ] Performance, security, portability, durability, and cost claims match evidence.
- [ ] Specifications, status, architecture, security, performance, roadmap, README, and application
      docs agree.
- [ ] Obsolete code, tests, formats, commands, dependencies, artifacts, and documentation are
      removed.
- [ ] Staged and unstaged diffs contain only intended work.
- [ ] Final handoff states commits, verification, publication, limitations, and unperformed
      actions.
