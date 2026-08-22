# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add rules for a genuine ownership boundary, but it may not weaken this file.

Use English for maintained code, tests, protocol fields, command output, diagnostics, specifications, documentation, examples, benchmarks, evidence, commit messages, and handoffs.

Campaign-specific detail belongs under `prompts/`.

Durable architectural policy belongs here.

## Mission

Build lkjscript into a general-purpose, meaning-oriented programming language and application platform.

The canonical authored authority of an accepted lkjscript program is one validated, revisioned meaning graph.

The meaning graph must be a better development substrate than editable text, not merely a text syntax serialized differently.

Normal lkjscript application development must be possible through the public `lkjscript` CLI.

A released `lkjscript` executable, placed in an otherwise empty working environment, must be sufficient for an agent to discover the platform, create a project, author and refactor meaning, test it, build it, and run supported application shapes.

Rust is the bootstrap and generic host implementation.

Application policy must remain lkjscript meaning unless a capability is intrinsically host-owned.

The long-term objective is the strongest coherent final architecture, not preservation of prior decisions.

## Primary outcomes

1. Keep exactly one accepted program authority.
2. Make local graph work local in I/O, validation, compilation, and context.
3. Make the CLI precise, discoverable, compact, deterministic, and economical for agents.
4. Make one binary sufficient for ordinary application creation from an empty directory.
5. Provide orthogonal language abstractions that eliminate repetition without hidden authority.
6. Support materially different applications without application-specific Rust policy.
7. Preserve exact capability boundaries between program requirements and deployment grants.
8. Reach asymptotically sound storage, query, mutation, validation, compilation, and runtime behavior.
9. Keep independent correctness and reconstruction oracles.
10. Delete superseded paths after direct incompatible cutover.
11. Reduce repeated model context, tool output, verification work, retries, and provider expense.
12. State security, performance, portability, and cost claims no more strongly than retained evidence supports.

## Authority and precedence

Apply instructions in this order:

1. The active user request.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative specifications under `docs/spec/`.
5. Executable validators, invariant tests, and public black-box contracts.
6. The current accepted meaning-graph revision and its canonical revision record.
7. Generated descriptions mechanically derived from an executable owner.
8. `docs/status.md`, `docs/architecture.md`, and `docs/security.md`.
9. Structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`, `README.md`, and application documentation.
11. Historical prompts, commits, branches, issues, discussions, and remembered plans.

A newer verified checkout outranks stale facts in a prompt.

A campaign prompt governs one campaign.

It does not become permanent authority merely because it is long or recent.

When behavior changes, update its specification, implementation, executable oracle, status, architecture, security implications, evidence, and user documentation in the same completed cutover.

## Decision standard

Optimize for the best coherent system over the longest useful horizon.

Do not preserve an architecture because it was expensive to build.

Do not preserve a public contract because it is already documented.

Do not preserve a term because it appears in many files.

Do not preserve a limit because tests were written around it.

Do not preserve a storage representation because it is canonical today.

Do not preserve a command hierarchy because scripts use it today.

Backward compatibility has no value unless the active user explicitly requires it.

Prefer one completed architecture to compatibility layers, editions, aliases, dual readers, dual writers, fallback paths, and permanent migration code.

A large rewrite is acceptable when it:

- replaces rather than stacks;
- carries every maintained consumer;
- preserves or deliberately replaces correctness evidence;
- deletes the superseded implementation;
- leaves the repository in one executable state;
- improves the final model rather than only renaming it.

Do not leave two competing abstractions for the same concept.

Do not leave a new path beside an old path for reassurance.

Do not substitute a roadmap for a feasible implementation.

## Autonomy and responsibility

Resolve ordinary engineering decisions from the checkout, experiments, complete workflows, measurements, and stated priorities.

Do not ask the user to choose implementation details that evidence can decide.

Do not stop at analysis when a dependency-closed implementation is feasible.

Do not claim completion while maintained consumers use private, obsolete, or fixture-only paths.

State uncertainty honestly.

Distinguish observed behavior, inferred behavior, intended behavior, and proved behavior.

Never upgrade a benchmark observation into a universal guarantee.

Never claim token savings, monetary savings, security properties, portability, or scalability without corresponding evidence.

## Repository safety

Before editing, inspect the actual worktree and every applicable instruction file.

Use at least:

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -12 --oneline
git remote -v
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true
find .. -name AGENTS.md -print
```

Preserve unrelated modified and untracked work.

Permission to redesign lkjscript is not permission to destroy unrelated work.

Do not use `git reset`, `git clean`, amend, rebase, merge, force-push, delete remote state, publish a release, or rewrite unrelated history unless the active user explicitly requests that exact action.

Stage explicit in-scope paths only.

Do not use `git add .`, `git add -A`, or `git add --all`.

Inspect staged and unstaged diffs before every commit.

Verify local and remote refs after an authorized push.

A push, when authorized, must be a normal fast-forward push of completed in-scope commits.

Never commit credentials, secrets, private transcripts, personal data, hidden reasoning, raw provider events, unrelated files, or unlicensed corpora.

Keep destructive experiments, generated bulk data, helper-agent transcripts, and losing prototypes outside the repository unless a retained artifact has a named consumer.

Report unavailable tools, failed gates, unknown publication state, partial completion, and requested actions not performed.

## Direct incompatible cutover

The default migration strategy is direct incompatible cutover.

Old programs, stores, identities, artifacts, commands, deployment layouts, caches, indexes, and generated views may become unreadable.

A one-time converter is allowed only when it is the shortest safe route to completing the cutover.

It must not become a permanent reader.

After maintained consumers migrate and verify:

- delete predecessor readers;
- delete predecessor writers;
- delete compatibility aliases;
- delete duplicate schemas;
- delete obsolete tests;
- delete stale documentation;
- reject predecessor formats exactly at the public boundary.

Historical reconstruction is not compatibility.

Git history may contain old bytes.

The current executable must not interpret them as current authority.

Incompatible-change freedom is not permission to leave the repository between architectures.

## Canonical meaning graph

Each accepted project revision has exactly one canonical typed meaning graph.

The graph owns program meaning, including:

- repository and package metadata;
- modules and namespaces;
- declarations;
- types;
- expressions;
- bindings;
- components;
- ports;
- capability requirements;
- targets;
- tests;
- exact dependencies;
- semantically retained documentation;
- stable semantic relations;
- deletion tombstones when continuity requires them.

The graph does not own:

- secrets;
- live handles;
- runtime resources;
- deployment grants;
- host coordinates;
- compiler-local dense indexes;
- bytecode offsets;
- caches;
- query indexes;
- logs;
- formatting preferences;
- model transcripts.

Logical graph authority does not require one physical object per logical node.

Physical storage may use persistent maps, packed tables, arenas, immutable segments, content-addressed packs, journals, snapshots, and indexes.

Choose physical forms by:

- correctness;
- locality;
- structural sharing;
- bounded loading;
- write amplification;
- crash consistency;
- merge behavior;
- transport behavior;
- compaction behavior;
- measured cold and warm performance.

Do not encode the meaning graph as verbose recursive JSON.

Do not allocate one general-purpose heap object or filesystem object per semantic atom.

Do not require whole-project loading for ordinary local query, mutation, validation, or compilation.

Accepted revisions are complete, valid, and executable within their declared requirements.

Holes, unresolved references, speculative alternatives, and conflicts belong to explicit non-executable draft authority.

Every read identifies the exact observed revision.

Every write names or resolves to an exact base before publication.

Every successful write publishes at most one revision atomically.

Validation, rejection, stale input, no-change, reads, plans, and previews publish nothing.

## Meaning graph over text

Maintained text is not a second editable lkjscript program authority.

Text may be:

- deterministic review projection;
- bounded diagnostic excerpt;
- human-oriented explanation;
- new-project template description;
- external data consumed by a program;
- independent test oracle;
- recovery evidence.

A text projection must identify its source revision and non-authoritative status.

It must not be silently applicable as a source patch.

The absence of editable source raises the required quality of the CLI.

Do not compensate with giant transaction JSON, raw graph records, generated source, or private fixture builders.

Graph authoring must expose semantic intent at a higher level than text editing.

## One-binary application bootstrap

A released `lkjscript` executable must support a complete offline first-use path from an empty directory.

The binary must provide enough self-description to discover:

- current command capabilities;
- the language and graph contract;
- supported declaration and expression forms;
- project creation;
- change construction;
- validation;
- testing;
- building;
- running;
- diagnostics;
- expansion of bounded results.

Creating the first project must not require:

- a repository checkout;
- Cargo;
- Rust source;
- Python;
- a preexisting `.lkja` file;
- network access;
- a registry;
- undocumented environment state;
- direct editing of graph storage.

Any embedded standard package, prelude, template, or bootstrap artifact must be:

- exact and versioned;
- inspectable;
- exportable when useful;
- integrity checked;
- generated from one maintained authority;
- never a hidden second writer.

The binary-only path must have black-box acceptance tests in an isolated temporary environment.

Stateless per-command operation must remain correct.

An optional resident or stdio agent session may accelerate repeated work, but it must not own meaning.

## Vocabulary discipline

Use one public term for one concept.

Use different terms only when the concepts have different invariants.

Maintain a concise canonical glossary for externally visible concepts.

Audit new terminology against existing terminology before adding it.

Prefer established programming-language and systems terms when they fit exactly.

Do not invent branded vocabulary for ordinary concepts.

Do not expose physical storage terminology as ordinary authoring terminology.

Do not make product-specific terms universal language terms.

Do not use `meaning`, `semantic`, `graph`, `owner`, `entity`, and `object` interchangeably.

Define which term belongs to which layer.

A reasonable default separation is:

- meaning graph: canonical program authority;
- declaration or member: language-level program construct;
- revision: accepted history node;
- change: user-visible atomic semantic edit request;
- transaction: exact internal publication protocol when that distinction is useful;
- object or segment: physical storage unit;
- artifact: executable or distributable derived package;
- receipt: compact evidence of a completed operation;
- draft: non-executable work authority.

Public command names and fields must be descriptive and stable.

Remove aliases rather than preserving synonyms.

A command registry, schema, or generated help owner must prevent documentation drift.

## Identity

Use durable identity only when a concrete consumer needs continuity.

Concrete consumers include:

- references;
- rename and move;
- semantic history;
- merge;
- refactoring;
- deployment binding;
- diagnostics;
- persistent external linkage.

Separate:

- stable semantic identity;
- mutable name;
- namespace;
- content digest;
- revision identity;
- physical storage key;
- compiler index;
- runtime handle;
- rendered coordinate;
- temporary local symbol.

No value may silently represent multiple identity domains.

Every identity domain must define:

- owner;
- canonical encoding;
- equality;
- allocation;
- retention;
- deletion;
- nonreuse;
- diagnostics;
- transport;
- collision behavior.

Names and paths are locators and presentation unless a specification explicitly makes them identity.

Rename and move must preserve or replace identity deliberately.

A content digest proves equality or integrity only in its exact domain.

It does not prove provenance, authority, freshness, or permission.

Reject foreign-domain identities even when raw bytes or display names coincide.

Do not require clients to preallocate every stable ID when the transaction engine can allocate them atomically.

Support request-local symbolic identities or another exact mechanism for referring to newly created owners within one change.

Return the allocated stable-ID map in the compact receipt.

## Revisions and publication

Accepted history is immutable and exact.

A revision commits to every semantic input needed to reconstruct its accepted meaning.

Publication must have one atomic visibility point.

Readers observe the old complete revision or the new complete revision.

Writers must compare against the exact current base under the publication lock or equivalent serialization boundary.

New immutable data must become durable before visibility changes.

Publication uncertainty must be reconciled by reading current authority and retained receipts.

Do not retry blindly after indeterminate visibility.

A revision record must not repeat the whole graph.

A receipt must not repeat every validation pass or every affected owner inline.

Retain expandable exact evidence out of band.

## Changes and transactions

All ordinary program mutations lower to one exact semantic change protocol.

The public surface should express high-level intent.

Examples include:

- create;
- replace;
- delete;
- rename;
- move;
- rebind;
- extract;
- inline;
- change signature;
- introduce type parameter;
- add or remove a field;
- add or remove a variant case;
- create a test;
- update an expectation;
- create a target;
- compose a component;
- resolve a conflict.

Raw table, arena, byte-offset, or storage-record editing is not an ordinary public workflow.

A change carries:

- exact base or an exact session-bound base;
- graph and protocol identity;
- ordered operations;
- preconditions;
- request-local symbols where needed;
- resource budget;
- optional idempotency;
- bounded nonsemantic intent.

Planning, validation, and publication must share one normalized request and one lowering implementation.

Do not require clients to repeat an expensive deterministic phase merely because modes are exposed as separate commands.

A combined prepare-and-commit workflow may perform exact validation once and publish under a rechecked base.

Stale base, precondition failure, ambiguity, foreign identity, exhaustion, conflict, no-change, invalid meaning, corruption, cancellation, and infrastructure failure remain distinct.

## Drafts and conflicts

Drafts are explicit non-executable authority.

A draft binds:

- repository;
- exact base revision;
- generation;
- normalized pending changes;
- holes;
- diagnostics;
- conflicts;
- bounded intent.

Draft mutation cannot change accepted HEAD.

A draft with holes or conflicts cannot build, run, deploy, or publish.

Rebase is explicit.

Conflict persistence and resolution must use typed semantic operations.

Do not encode conflicts as unstructured prose or source markers.

Dropping a draft cannot affect accepted meaning.

## Public CLI

The public CLI is the ordinary development environment for lkjscript applications.

The CLI must support:

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
- refactoring;
- drafts;
- validation;
- publication;
- diff;
- merge;
- conflict resolution;
- history;
- tests;
- build;
- run;
- deployment inspection;
- backup;
- restore;
- repair and deep doctor.

Do not require users to understand physical graph storage.

Do not require users to know Rust enum layouts.

Do not make giant strict JSON the only practical way to create ordinary meaning.

A structured request protocol is necessary.

A ceremony-heavy protocol is not sufficient.

Prefer a small compositional command grammar over a flat catalog of unrelated commands.

Presume a universal `semantic` namespace is redundant unless multiple genuinely distinct public command domains justify it.

Do not retain aliases for compatibility.

Every normal response must be:

- deterministic;
- bounded;
- machine-readable;
- revision-pinned;
- schema-identifiable;
- expandable;
- free of secrets.

Default success is the smallest complete summary.

Large results require selected fields, item and byte budgets, continuation, output files, or stable expansion handles.

Do not print:

- full schemas on every call;
- complete graphs;
- every passing test;
- child build logs;
- repeated environment facts;
- stack traces;
- unbounded diagnostics.

Failures must provide:

- a compact actionable summary;
- the highest-value diagnostics;
- stable diagnostic identities;
- exact expansion commands or handles;
- retained complete logs when applicable.

Human rendering may exist as an explicit projection.

It must derive from the same structured result.

## Agent protocol and context economy

Treat model context, provider requests, output tokens, cached input, verification time, and correction depth as scarce resources.

Do not reduce evidence merely to reduce output.

Provide task-scoped context bundles derived from ownership and dependency closure.

A bundle must state:

- exact revision;
- selected seeds;
- inclusion reasons;
- omissions;
- item count;
- byte count;
- work consumed;
- diagnostics;
- expansion commands.

Use content-addressed or revision-bound handles so unchanged context is not resent.

Permit field projection and compact summaries before body expansion.

Do not dump the repository or complete graph when a bounded slice is sufficient.

An optional stdio session protocol may retain:

- opened repository metadata;
- exact revision;
- disposable indexes;
- decoded schemas;
- task context handles;
- prepared compiler data.

A session must invalidate on every relevant input.

A session must never become canonical authority.

Keep a concise campaign ledger of durable facts, decisions, measurements, risks, receipts, and next actions.

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

Bytes alone are not tokens or money.

Compare equal complete tasks, including failures and corrections.

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

An implementation limitation must be labeled temporary.

A default must not masquerade as a hard maximum.

A growing result should paginate, stream, shard, or write out of band rather than fail at an incidental count.

A local change should consume explicit work proportional to its semantic impact.

Do not raise work, memory, queue, timeout, recursion, response, or transaction limits to hide an algorithmic defect.

Do not impose arbitrary file-count, line-count, directory-count, module-count, declaration-count, or operation-count policy.

Decoder and allocator bounds remain mandatory at hostile input boundaries.

Use checked arithmetic before allocation.

Expose relevant budgets through machine-readable discovery.

Return resource exhaustion with consumed work and a safe continuation or narrower next action when possible.

## Incremental computation

Ordinary local work must scale with the changed semantic slice plus conservative affected closure.

Whole-project work must be explicit.

Design the graph, storage API, validator, compiler, query engine, and tests so locality is visible in types and measurements.

An ordinary mutation API that requires cloning `Vec<AllModules>` is presumptively wrong.

An ordinary rename that scans every module is presumptively wrong.

Canonical references should use stable identities so presentation-name changes do not rewrite unrelated dependents.

Maintain exact dependency and reverse-dependency information for:

- imports;
- exports;
- names and namespaces;
- type use;
- value references;
- calls;
- fields;
- variant cases;
- interfaces;
- capabilities;
- components;
- ports;
- targets;
- tests;
- generic instantiations when present.

Incremental validation may reuse a prior fact only when every semantic input is proven identical.

Reuse keys must include:

- validator contract;
- exact owner content;
- dependency summary digests;
- relevant package contracts;
- feature or target policy;
- every semantic environment input.

Incremental compilation, query indexes, and prepared execution are derived acceleration.

They must be invalidatable and rebuildable.

Keep an implementation-disjoint full validator and reconstruction path.

Differential tests must compare incremental and full outcomes over long mutation sequences.

Corrupt or missing derived state must rebuild or fail clearly without changing accepted meaning.

## Physical storage

Separate logical graph shape from physical storage.

The root representation must not require rewriting or decoding a linear list of every module for a local change.

Use a persistent indexed structure when measurements justify it.

Candidate structures may include:

- content-addressed B-trees;
- radix trees;
- HAMT-like persistent maps;
- sorted immutable pages;
- packed multi-owner segments;
- log-structured immutable runs;
- transactional embedded indexes.

Select through measured prototypes, not fashion.

At large scale, avoid one filesystem inode per tiny semantic owner when pack files or segments are materially better.

Canonical packed storage must preserve:

- exact deterministic encoding where required;
- integrity checks;
- structural sharing;
- bounded decode;
- crash consistency;
- transport;
- backup;
- restore;
- compaction;
- garbage collection;
- independent reconstruction.

Only newly created or newly referenced immutable data should require publication-time verification when prior data is already bound by trusted accepted history.

Deep doctor remains the explicit full verification path.

Define revision retention, reachability, pinning, drafts, backups, compaction, and garbage collection exactly before deleting anything.

## Queries and indexes

Queries are revision-pinned.

Ordering must not depend on hash iteration or physical pack position.

Continuations bind the exact normalized query, revision, schema, and cursor.

A changed query or revision invalidates the continuation.

Indexes are disposable unless a specification explicitly promotes one into canonical meaning.

Exact lookup should touch a bounded index path and the owning semantic shard.

Broad traversal should use prepared relation summaries or explicit broad work.

Cold orientation should not decode every module body.

Index update should be delta-driven where possible.

Missing or corrupt indexes must rebuild from canonical authority.

Keep an independent query oracle for tests.

## Packages and dependencies

Packages and modules are graph meaning, not filesystem conventions.

Define:

- package identity;
- module identity;
- namespaces;
- visibility;
- imports;
- exports;
- dependencies;
- cycles;
- initialization;
- diagnostics.

Accepted dependencies bind immutable exact identities.

Do not resolve accepted builds from:

- mutable tags;
- ambient directories;
- undeclared network state;
- current working directory accidents;
- credentials;
- latest-version lookup.

A released binary may carry exact built-in package artifacts for offline bootstrap.

Those artifacts must be visible, versioned, inspectable, and reproducible from maintained graph authority.

Dependency staging, embedding, export, and project initialization must converge on one package-object contract.

## Language design

Prefer a small orthogonal language core.

Move reusable policy into graph-authored libraries.

Do not add product-shaped primitives.

Make explicit:

- evaluation order;
- equality;
- ordering;
- overflow;
- indexing;
- Unicode behavior;
- serialization;
- effects;
- failure classes;
- allocation-relevant behavior;
- cancellation;
- resource lifetime.

Avoid:

- implicit coercion;
- order-dependent inference;
- ambient overload resolution;
- hidden authority;
- hidden global state;
- nondeterministic macro expansion.

Expected program outcomes are typed values.

Traps, capability failures, possible external visibility, exhaustion, cancellation, corruption, and infrastructure failure are distinct.

Pure functions remain independent from time, randomness, deployment, scheduling, and external state.

## Abstraction mechanisms

The language must provide enough abstraction to express reusable libraries and diverse applications without copy-paste graph expansion.

New abstraction mechanisms require multiple complete consumers and exact semantics.

Evaluate at least:

- parametric polymorphism;
- constrained generic functions and data;
- higher-order functions;
- closure capture;
- reusable component composition;
- type aliases or newtypes;
- graph-native authoring templates;
- semantic refactoring operations.

Do not conflate capability interfaces with type constraints unless their invariants truly coincide.

Do not introduce a second macro language casually.

A graph-native template or recipe may create a normalized change request.

It is not accepted program authority.

Generic declaration identity and concrete instantiation identity must be distinct.

Type inference should reduce ceremony without making accepted meaning context-dependent or order-dependent.

The canonical graph must retain enough explicit type information for deterministic validation, review, and compilation.

Compiler specialization is derived.

It must not change language meaning.

## Components, effects, and capabilities

One component and port model should cover command, HTTP, interactive, batch, worker, and test runners where their semantics genuinely align.

Runner kinds are deployment or target metadata, not language editions.

Applications declare typed capability requirements.

Deployment grants bind adapters, authority, secrets, sharing domains, and limits.

Artifacts contain requirements.

Artifacts do not contain grants, credentials, or live resources.

Generic adapters own protocol and resource mechanics.

They do not own application routes, schemas, authorization roles, SQL policy, object keys, retry policy, rendering, or domain transitions.

Every live resource defines:

- acquisition;
- owner;
- permitted operations;
- close;
- cancellation;
- timeout;
- cleanup;
- observability;
- non-persistence.

Production and deterministic test adapters should be behaviorally comparable and implementation-disjoint where practical.

## Runtime and compiler

Keep one runtime kernel for:

- preparation;
- admission;
- execution;
- capability routing;
- task ownership;
- resource accounting;
- cancellation;
- shutdown;
- observations.

Concurrency is bounded and structured.

Do not create hidden unbounded queues.

Do not create detached ownerless tasks.

Graceful shutdown defines:

- admission stop;
- drain;
- cancellation;
- non-cancellable publication;
- resource cleanup;
- timeout;
- exit status.

A process boundary is not semantic identity.

A process boundary is not a hostile-code sandbox.

Maintain an independently checkable semantic execution route.

Bytecode, specialized interpreters, AOT, JIT, and caches are derived tiers.

They require equivalence, invalidation, resource accounting, and fallback evidence.

Do not add a JIT because it is fashionable.

Do not reject a JIT merely because it was previously deferred.

Use complete maintained workloads to decide.

Compilation and caches bind:

- exact semantic revision;
- dependency closure;
- compiler contract;
- target;
- optimization policy;
- relevant environment.

Keep stable semantic identities out of hot runtime representation unless runtime behavior needs them.

Use compact dense indexes after validation.

## TLS is out of scope

lkjscript does not plan to implement TLS in the current product direction.

Do not add:

- HTTP TLS termination;
- certificate parsing;
- certificate issuance;
- certificate rotation;
- ACME;
- PostgreSQL TLS;
- a TLS abstraction layer;
- TLS-specific language primitives.

Do not spend campaign effort preparing speculative TLS hooks.

Document that deployments requiring encrypted transport use an appropriate external trusted boundary or different adapter outside the current scope.

Keep plaintext HTTP and current database transport limitations explicit.

Do not imply that external termination makes the runtime a hostile multi-tenant sandbox.

## Performance

Long-term performance is a first-class requirement.

Optimize complete workloads, not isolated instruction folklore.

Require asymptotically sound structures before micro-optimization.

Measure at least:

- cold and warm first-use discovery;
- project creation;
- orientation;
- exact lookup;
- context construction;
- local mutation;
- wide mutation;
- rename and move;
- validation;
- publication;
- diff;
- merge;
- conflict resolution;
- build;
- test;
- startup;
- execution;
- service;
- worker;
- backup;
- restore;
- compaction;
- fresh checkout;
- binary-only bootstrap.

Include:

- realistic projects;
- many tiny modules;
- dense relation fanout;
- deep types and expressions;
- long history;
- large literals;
- branch conflicts;
- adversarial invalid input.

Record, where available:

- wall time;
- CPU time;
- peak RSS;
- allocation counts;
- bytes read and written;
- fsync count;
- storage growth;
- output bytes;
- semantic work counts;
- cache state;
- binary size;
- provider usage.

A local edit benchmark must report changed owners, invalidation closure, reused facts, and full-oracle comparison.

## Verification

Use the narrowest sufficient gate during iteration.

Run a complete authoritative gate before final publication.

Change-aware selection is convenience, not proof.

Uncertainty widens to full verification.

Skipped, unavailable, flaky, timed-out, exhausted, cancelled, or unrun is not pass.

All-pass verification is quiet.

Return one aggregate summary and an exact receipt locator.

Retain bounded stdout and stderr per child gate.

On failure, return a bounded high-value excerpt and exact log locators.

Do not print every passing test.

Build verification as an explicit dependency graph.

Do not rerun an identical expensive gate merely because multiple profiles mention it.

A reused pass requires an exact fingerprint of every input.

A final full gate may deliberately require fresh execution.

Verification receipts must distinguish fresh and reused evidence.

Test:

- formatting;
- static analysis;
- locked build;
- graph invariants;
- hostile decoders;
- publication;
- crash interruption;
- incremental/full equality;
- query/index oracle equality;
- compiler/reference equality;
- property sequences;
- fuzzing where useful;
- CLI contracts;
- binary-only bootstrap;
- predecessor rejection;
- application acceptance;
- backup and restore;
- restart;
- cancellation;
- overload;
- deterministic artifacts;
- compaction and garbage collection;
- fresh checkout;
- Git diff integrity.

## Testing policy

Test public behavior at the public boundary.

Use private unit tests for local invariants, not as substitutes for public acceptance.

Maintain at least one implementation-disjoint oracle for high-risk optimization.

Generate long deterministic mutation sequences.

Include shrinking or precise reproduction for property failures.

Test no-change and rejection paths for absence of publication.

Test stale concurrent writes.

Test idempotent replay.

Test foreign identity.

Test budget exhaustion before excessive allocation.

Test corruption of each canonical object class.

Test loss and corruption of each derived index class.

Test interrupted publication at every durability boundary that can be simulated.

Test binary-only project creation without the repository checkout.

Test that templates and embedded packages can be inspected and reproduced.

Test that ordinary local operations do not perform full-graph work through semantic counters or I/O tracing.

## Security and trust

Treat as hostile decoding input:

- graph objects;
- artifacts;
- backups;
- change requests;
- drafts;
- continuations;
- deployment descriptors;
- network input;
- database rows;
- object-store responses;
- queue records;
- environment values.

Use:

- closed contracts;
- exact bounds;
- duplicate rejection;
- trailing-data rejection;
- checked arithmetic;
- pre-allocation checks;
- canonical identity encodings;
- path and symlink defense;
- secret redaction;
- typed failures.

Do not claim:

- hostile-code sandboxing;
- multi-tenant isolation;
- constant-time behavior;
- cryptographic provenance;
- distributed consensus;
- cross-node transactions;
- portability;
- TLS;
- authenticated artifacts;

without complete evidence and an active requirement.

Accepted lkjscript programs are trusted program inputs unless the threat model changes explicitly.

A digest is not a signature.

A local process is not a tenant boundary.

## Rust engineering

Use stable Rust 2024 unless a verified campaign changes the bootstrap.

Prefer:

- explicit ownership;
- typed domain wrappers;
- checked arithmetic;
- bounded allocation;
- iterative traversal;
- compact representations;
- narrow modules;
- typed errors;
- deterministic collections where order matters.

First-party `unsafe` is forbidden unless the root policy is deliberately changed with a documented invariant and focused evidence.

Warnings are defects.

Do not silence lints broadly.

Avoid panic on untrusted or operational input.

`expect`, `unwrap`, `todo`, and `unimplemented` remain prohibited by repository lint policy outside narrowly justified test policy.

Large files are not automatically defects.

Split a module when ownership, testing, compile time, or comprehension improves.

Do not split by arbitrary line count.

Do not create indirection-only modules.

## Dependencies

A dependency needs:

- a named complete consumer;
- a narrower implementation than writing it locally;
- acceptable build cost;
- acceptable binary-size cost;
- acceptable security surface;
- active maintenance prospects;
- narrow features.

Do not add a database, graph engine, parser generator, RPC stack, compiler framework, or runtime merely to avoid understanding the current system.

Do not reject a dependency categorically when it materially improves the final design.

Prototype and measure major dependency choices.

Remove dependencies made obsolete by a cutover.

Keep the locked dependency closure reproducible.

## Auxiliary coding agents and models

The development environment may provide `herdr`, `pi` with Qwen Cloud, `antigravity`, and other coding-agent tools.

Discover their actual interfaces with local help before use.

Use them when an independent audit, bounded prototype, adversarial review, or parallel evidence collection materially improves the result.

Give auxiliary agents task-scoped context.

Do not send the complete repository by default.

Do not send secrets, credentials, private data, hidden reasoning, or unrelated files.

Treat auxiliary output as untrusted advice.

The primary agent owns architecture, integration, tests, and final claims.

Do not blindly copy generated code.

Verify every adopted result in the checkout.

Retain only concise durable conclusions, not full transcripts.

Use cheaper models for bounded mechanical audits when they are adequate.

Use stronger models for architecture, invariant review, and difficult integration when the expected correction savings justify them.

Do not create redundant multi-agent work merely because tools are available.

## Documentation and evidence

Normative behavior belongs under `docs/spec/`.

Current implementation reality belongs in `docs/status.md`.

Layer ownership belongs in `docs/architecture.md`.

Threat model and non-claims belong in `docs/security.md`.

Reproduced measurements belong in `docs/performance.md` and structured evidence.

Future evidence-gated work belongs in `docs/roadmap.md`.

User workflow belongs in `README.md` and application documentation.

A campaign ledger records:

- audited baseline;
- decisions;
- alternatives;
- experiments;
- measurements;
- migrations;
- deletions;
- verification;
- limitations.

Evidence names:

- exact commit;
- worktree identity;
- toolchain;
- platform;
- command;
- inputs;
- cache state;
- receipt;
- limitations.

Delete obsolete current documentation after cutover.

Historical evidence may remain only when labeled as historical.

Do not duplicate exhaustive command grammar outside its executable owner.

Generate reference material from the command registry or schema when possible.

## Required working method

1. Inspect the worktree and effective instructions.
2. Reproduce current behavior before redesign.
3. Build an authority and terminology map.
4. Identify whole-graph work, duplicated truth, redundant ceremony, and incidental limits.
5. Define measurable destination invariants.
6. Prototype the riskiest architectural alternatives in bounded form.
7. Select by complete-task evidence.
8. Implement dependency-closed vertical slices.
9. Migrate maintained consumers through public paths.
10. Delete superseded code and contracts.
11. Run focused verification continuously.
12. Run scale, failure, recovery, and binary-only acceptance.
13. Update specs, docs, evidence, and status.
14. Run a fresh complete gate.
15. Inspect diffs, commit coherent changes, and push only when authorized.
16. Deliver exact verification, publication, limitations, and unperformed actions.

## Forbidden shortcuts

Do not maintain source and graph as independently editable truths.

Do not make text the normal mutation path for lkjscript applications.

Do not expose raw storage records as the authoring API.

Do not replace text with equally verbose recursive JSON.

Do not require full-repository context for local work.

Do not infer stable identity from name, path, position, or content hash without an explicit rule.

Do not use generated lkjscript source, private Rust builders, or opaque fixtures as maintained program authority.

Do not add application-specific Rust business policy.

Do not bypass typed requirements and grants with ambient host calls.

Do not treat caches, indexes, bytecode, logs, validation summaries, or projections as program authority.

Do not preserve predecessor formats through editions, aliases, fallback readers, or dual writers.

Do not keep two command names for the same behavior.

Do not claim API cost savings from output bytes alone.

Do not print every passing test or complete child log by default.

Do not raise limits to hide poor algorithms.

Do not implement TLS.

Do not introduce Lean files, toolchains, dependencies, experiments, or references.

Do not commit hidden reasoning, secrets, helper-agent transcripts, or unrelated work.

Do not stop after writing architecture documents when implementation is feasible.

Do not leave a new architecture layered over the old one.

## Completion standard

A campaign is complete only when every applicable item is true.

- [ ] One canonical meaning-graph authority remains.
- [ ] The public CLI completes every changed application workflow.
- [ ] A single released binary creates and develops an application from an empty directory.
- [ ] Embedded bootstrap meaning is exact, inspectable, and reproducible.
- [ ] Maintained applications use public paths rather than private builders.
- [ ] Text and predecessor paths are deleted or explicitly non-authoritative.
- [ ] Public vocabulary is smaller, defined, and free of unnecessary aliases.
- [ ] Stable identity, revision, transaction, publication, and failure classes are exact.
- [ ] Request-local creation does not require preallocating every stable ID.
- [ ] Local operations load and validate bounded affected slices.
- [ ] Whole-project work is explicit and measured.
- [ ] Incremental and full validators agree over deterministic mutation sequences.
- [ ] Query indexes agree with canonical reconstruction.
- [ ] Production and independent execution paths agree.
- [ ] Language abstraction is sufficient for maintained reusable libraries.
- [ ] No product-specific Rust policy was added.
- [ ] No TLS implementation or speculative TLS layer was added.
- [ ] Resource limits are classified and justified.
- [ ] Arbitrary public count ceilings are removed or replaced by explicit resource policy.
- [ ] Storage supports measured large-graph behavior, compaction, and recovery as applicable.
- [ ] Quiet verification retains exact expandable evidence.
- [ ] Verification avoids proven duplicate work without overstating reused evidence.
- [ ] Maintained consumers pass acceptance.
- [ ] Binary-only bootstrap passes in an isolated environment.
- [ ] Backup, restore, corruption, interruption, and fresh-checkout tests pass.
- [ ] Performance, security, portability, and cost claims match evidence.
- [ ] Specifications, status, architecture, security, performance, roadmap, README, and application docs agree.
- [ ] Obsolete code, tests, formats, commands, dependencies, and documentation are removed.
- [ ] Staged and unstaged diffs contain only intended work.
- [ ] Final handoff states commits, verification, publication, limitations, and unperformed actions.
