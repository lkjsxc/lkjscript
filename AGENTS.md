# AGENTS.md

## Scope

This file governs the entire `lkjscript` repository.

Use English for repository artifacts unless the active task explicitly requires another language for a user-facing result.

A narrower subtree policy may refine local procedure, but it must not create a second language definition, semantic authority, identity model, ownership model, compiler route, runtime route, package authority, artifact authority, or documentation authority.

The active task chooses one objective.

This file governs how that objective is inspected, bounded, implemented, verified, documented, committed, and reported.

Prompt files, transcripts, scratch notes, and prior-turn handoffs are not repository authority.

Do not commit task prompts unless the active task explicitly makes a prompt a product artifact.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free, high-performance programming system.

AI-primary means deterministic semantic machinery lets agents discover, construct, edit, inspect, leave incomplete, validate, review, compile, and run programs without depending on source text as mutable authority.

Model inference must not participate in parsing correctness, type correctness, ownership correctness, effect correctness, optimization correctness, artifact acceptance, persistence correctness, or runtime correctness.

Optimize for long-term semantic coherence, low agent reasoning burden, and direct evidence.

Prefer one small complete architecture over several ambitious partial architectures.

Prefer a complete dependency-closed vertical over a broad framework with missing consumers.

## Current product boundary

The authoritative current state is `docs/status.md`, not this summary.

Unless the active checkout proves otherwise, the current product is:

- local package checking;
- local package execution;
- one in-process semantic workspace;
- immutable semantic snapshots;
- stable public identities over relocatable private compiler identities;
- source import as one input path;
- direct compilation from complete semantic snapshots;
- deterministic VM execution;
- bounded baseline-native specialization with pre-entry decline and VM fallback.

Do not infer a daemon, database, wire protocol, durable workspace format, distributed service, collaborative editor, plugin platform, cache hierarchy, or generalized orchestration layer from the project’s long-term ambition.

Add those only when a current consumer and measurements make them the smallest complete solution.

## Priority order

1. Coherent language semantics.
2. Memory safety and exact ownership behavior.
3. Failure atomicity and deterministic meaning.
4. One syntax-independent semantic authority.
5. Stable public identity over private relocation.
6. Scale-safe representations and algorithms.
7. One complete generic production compiler and runtime route.
8. Direct deterministic workflows for agents and ordinary tools.
9. Evidence before abstraction, caching, incrementality, parallelism, services, or distribution.
10. Optional specialization only behind the complete generic route.
11. Broader products only after current prerequisites and consumers exist.

A future platform idea is not present evidence.

Backward incompatibility is permission for a clean cutover, not permission for uncontrolled scope.

Prompt length, issue count, crate count, file count, and line count are not product goals.

## Authority

Classify a claim before deciding where it belongs.

- The active task owns the current objective, explicit acceptance criteria, product constraints, and stop condition.
- This file owns repository-wide engineering procedure and evidence discipline.
- `docs/spec/language.md` owns intended language semantics.
- `docs/spec/workspace.md` owns intended semantic-workspace behavior.
- Code, tests, manifests, commands, schemas, and lock files own checkout behavior.
- `README.md` owns the product introduction and first successful use.
- `docs/status.md` owns concise current implementation facts and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns measurement protocol, compact retained evidence, and reversal conditions.
- `docs/roadmap.md` owns planned ordering only.
- Sparse accepted files under `docs/decisions/` own durable, non-obvious rationale when a separate record is justified.
- Git history owns superseded implementation and prose.

When claims conflict:

1. classify the claim;
2. inspect the owning artifact;
3. inspect executable evidence;
4. preserve accepted semantics;
5. update or delete stale material in the same coherent change.

Do not manufacture authority through prompt archives, transcripts, copied status tables, global revisions, unconsumed registries, unconsumed descriptors, or unconsumed digests.

## Autonomy

Use autonomous judgment from the actual checkout, accepted specifications, focused tests, measurements, current consumers, and real failure boundaries.

Do not ask the user to choose among internal implementation alternatives that repository evidence can decide.

Ask only when a genuinely external product requirement is missing and no safe explicit assumption can complete the selected vertical.

When the active task grants broad authority, use it to make a clean decision.

Do not use broad authority to expand into unrelated work.

If the requested objective is already complete, verify it and report that fact.

Do not invent a replacement objective merely to consume the turn.

## Compatibility and cutovers

Backward compatibility is not an objective unless the active task names a current independent boundary that must remain compatible.

Language syntax, source encoding, Rust APIs, commands, packages, locks, modules, crates, compiler forms, runtime forms, fixtures, tests, and documentation may change incompatibly.

When cutting over:

1. update every active producer;
2. update every active consumer;
3. regenerate active artifacts when required;
4. delete the displaced implementation;
5. delete adapters and aliases;
6. delete migrations and dual readers;
7. delete feature flags that preserve the old path;
8. delete stale tests;
9. delete stale documentation.

Do not leave `legacy`, `v2`, `next`, edition, compatibility, dual-read, or dual-write architectures.

The `.lkjscript` extension is fixed.

Other provisional representations remain replaceable unless accepted semantics fix them.

Preserve unrelated tracked work, untracked work, credentials, host state, external data, and remote history.

Never reset, clean, rewrite, force-push, or otherwise destroy work you did not create.

## One-turn selection rule

One turn should normally complete one dependency-closed vertical.

A valid vertical has:

- one concrete user or system operation;
- one demonstrated defect, accepted gap, measured bottleneck, explicit product result, or current maintenance burden;
- a bounded producer-to-consumer path;
- focused failing evidence;
- a clear completion condition;
- explicit non-goals;
- a stop condition.

Do not combine adjacent roadmap items merely because they share a file.

Do not start an unrelated second vertical after the selected one passes.

Name the next problem in the report without implementing it.

## Multi-turn operating loop

### 1. Orient

1. Inspect branch, worktree, upstream, and recent history.
2. Record the starting commit.
3. Read the active task.
4. Read root `AGENTS.md`.
5. Read only the authority required for the selected problem.
6. Search owning symbols before opening large files.
7. Inspect representative producers and consumers.
8. Inspect existing tests before designing new APIs.
9. Check whether recent commits already changed the selected boundary.
10. Preserve unrelated work.

Keep a compact task-local orientation note in working memory or ignored scratch space.

Do not commit that note.

### 2. Bound

State, before broad implementation:

- the concrete operation;
- the observed gap;
- the intended semantic result;
- the smallest dependency-closed path;
- the invariants at risk;
- the focused evidence that will fail;
- the explicit non-goals;
- the stop condition;
- whether measurement is required.

If the task already supplies these, verify rather than restating them at length.

### 3. Characterize

Use the smallest executable evidence that distinguishes current from required behavior.

Prefer:

- one focused test;
- one existing example extended minimally;
- one exact query/projection assertion;
- one deterministic work counter;
- one existing benchmark workload;
- one malformed-input case at the real boundary.

Do not create a general harness when one focused test can prove the defect.

Do not add performance instrumentation unless it answers a stated complexity or bottleneck question.

### 4. Implement

1. Fix the root cause.
2. Reuse canonical identity, validation, ownership, and effect machinery.
3. Keep one mutable authority.
4. Keep one active route.
5. Preserve old immutable snapshots.
6. Preserve failure atomicity.
7. Use checked arithmetic and allocation.
8. Use iterative traversal for user-controlled depth.
9. Delete displaced code.
10. Avoid compatibility scaffolding.
11. Run the smallest falsifying command.
12. Review the diff as a consumer, adversary, and future coding agent.

### 5. Integrate

After focused evidence passes:

1. run affected crate tests;
2. inspect query and projection behavior;
3. inspect direct compilation behavior;
4. inspect VM and native route behavior where relevant;
5. inspect documentation authority;
6. inspect the complete diff;
7. remove scratch artifacts;
8. run the final relevant boundary once.

### 6. Finish

1. Update owning documentation.
2. Delete stale claims.
3. Commit one cohesive result when permitted.
4. Push only when explicitly requested.
5. Report exact evidence.
6. Report residual risk.
7. Report commit and worktree state.
8. Name, but do not begin, the next highest-leverage vertical.

Do not leave a half-cutover, hidden fallback, disabled check, stale current prose, temporary compatibility layer, unowned cache, or unreviewed subagent branch.

## Evidence gate for abstractions

Before adding a nontrivial abstraction, answer all of these with current evidence:

1. What exact current producer needs it?
2. What exact current consumer needs it?
3. Who owns it?
4. What is its lifetime?
5. What invalidates it?
6. What is its failure behavior?
7. Which duplicated semantics does it remove?
8. Which measured or demonstrated cost does it reduce?
9. Why is a local helper insufficient?
10. What is the reversal condition?
11. What is the deletion condition?
12. Can the selected vertical finish without it?

If the answers are weak, do not add the abstraction.

One current consumer does not justify a framework.

Two syntactically similar call sites do not prove shared semantics.

A future serialization, daemon, cache, plugin, distributed consumer, or self-hosted compiler does not justify present machinery.

## Preferred solution order

When several solutions satisfy semantics, prefer this order:

1. Delete unused work.
2. Delete duplicated authority.
3. Simplify semantics.
4. Simplify representation.
5. Reuse an existing invariant.
6. Reuse a canonical validator.
7. Improve one local traversal.
8. Make an invalid state unrepresentable.
9. Extract a shared helper after multiple current consumers prove shared semantics.
10. Cache after measured repeated work.
11. Parallelize after measured separable work.
12. Add incrementality after measured recomputation dominates.
13. Add a process boundary after measured demand.

The first adequate solution is not always the smallest.

The most general solution is rarely the smallest.

## Explicit anti-overengineering rules

Do not add speculative:

- daemons;
- services;
- sessions;
- journals;
- databases;
- CRDTs;
- schedulers;
- registries;
- plugin systems;
- rewrite DSLs;
- cache frameworks;
- proof ecosystems;
- wire protocols;
- target matrices;
- deoptimization;
- PGO;
- self-hosting scaffolding;
- orchestration platforms.

Do not build a universal framework for one:

- traversal;
- state;
- event;
- recovery case;
- descriptor;
- validation step;
- transaction;
- edit kind;
- query;
- benchmark;
- test fixture.

Do not refactor unrelated code for symmetry.

Do not split files or crates solely to satisfy aesthetic quotas.

Do not preserve a generalized extension point without a current extension.

Do not add configuration for a decision the project can make directly.

Do not add metadata that no current consumer reads.

Do not add a digest where type identity and canonical validation already suffice.

Do not add a version where producer and consumer are the same build.

Do not add a schema merely because data can be rendered as JSON.

Do not add a service merely to reduce prompt size.

## Boundary classification

Classify a value before deciding whether it needs a contract, version, digest, codec, registry entry, or independent validation.

### Persistent or transmitted artifact

Bytes can outlive the producing process or binary.

Producer and consumer may come from different builds.

Use explicit canonical encoding, exact identity, bounded decoding, and fail-closed validation appropriate to the threat model.

### Untrusted or unsafe boundary

The consumer crosses FFI, executable memory, generated entry, operating-system input, external bytes, or another safety boundary.

Validate structure and policy at the actual boundary.

Same-build typing does not replace validation of untrusted bytes.

A digest does not replace structural validation.

### Independent machine-facing output

A real script, tool, test harness, or user can retain and parse output independently.

Give it stable schema identity only when compatibility or exact interpretation is a current requirement.

### Same-build typed value

Producer and consumer are compiled together and exchange a private or typed in-memory value.

Rust type identity, private construction, canonical validation, and rebuilding normally provide continuity.

Do not serialize, hash, register, version, or compare the value with the current build merely to prove provenance.

### Shared canonical vocabulary

Capability kinds, resource kinds, operation identities, semantic traits, and similar terms may be shared typed vocabulary.

Vocabulary does not automatically require a descriptor registry or content digest.

Keep one direct owner.

### Derived observation

Timings, counts, indexes, projections, candidates, and diagnostics derive from authority.

Do not promote them into mutable authority.

Retain them only while a current consumer justifies lifecycle and cost.

## Semantic architecture invariants

Maintain one:

- language definition;
- semantic authority;
- public identity model;
- incomplete-state model;
- compiler input route;
- ownership model;
- generic production execution route;
- package model;
- documentation authority model.

Semantic meaning must be able to exist without:

- source text;
- paths;
- spans;
- formatting;
- parser nodes;
- source hashes;
- a running service.

Source and files are importer inputs, provenance, presentation, persistence, and interoperability forms.

They are not mutable semantic authority.

Incomplete state is valid editing state.

Incomplete state is never executable state.

Compilation consumes one complete immutable semantic snapshot directly.

Never render and reparse semantic state for compilation.

Dense IDs, slots, offsets, registers, layouts, ordinals, and traversal positions remain private.

Stable public identities survive private relocation.

Removal tombstones identities.

Old immutable snapshots remain valid.

Queries and diffs are deterministic, revision-labelled, structured, stably ordered, and never silently truncated.

Derived indexes are rebuilt or invalidated from authority.

They do not become independent truth.

## Transaction invariants

One successful transaction publishes one revision.

A failed transaction publishes nothing.

A failed transaction consumes no future identity state.

Validation applies to the staged semantic result.

Edit order may be meaningful where the existing transaction contract makes it meaningful.

Do not invent order-independent planning, swapping, conflict resolution, or batching semantics without a concrete consumer.

A rename is not deletion plus creation.

A move is not deletion plus creation.

A replacement does not silently rewire unrelated references.

Every public identity input must reject:

- foreign workspace identities;
- stale identities;
- wrong entity kinds;
- wrong owners;
- malformed graph structure.

Do not expose dense compiler addresses as public transaction inputs.

Keep transaction errors deterministic and specific enough for an agent to recover.

Do not add a second mutable staging representation unless the existing authority cannot express the required incomplete state.

## Compiler and runtime invariants

Keep one complete generic production execution route.

Checking must not execute effects.

Baseline-native specialization may decline only before effects and generated entry.

After native entry begins, its result or failure is final.

Never rerun effects through VM fallback after native entry.

A test-only evaluator may remain an independent oracle.

It is not a production engine.

Ordinary execution is collector-free and non-tracing.

Preserve exact:

- move laws;
- borrow laws;
- loan laws;
- cleanup laws;
- return laws;
- trap laws;
- cancellation laws;
- allocation laws;
- resource laws;
- host-resource laws;
- teardown laws.

Unsafe code belongs in narrow named mechanisms with explicit invariants and malformed-input evidence.

Keep FFI, executable-memory, relocation, W^X, and generated-entry boundaries fail-closed.

## Scale and performance

Language validity follows semantic laws, not arbitrary project quotas.

Use checked arithmetic.

Use checked narrowing.

Use fallible reservation where allocation failure is part of the repository’s error model.

Use iterative traversal for user-controlled depth.

Avoid accidental quadratic work.

Before introducing a cache, incremental graph, parallel path, or specialized representation:

1. define the workload;
2. measure the current path;
3. identify the dominant phase;
4. define semantic equivalence;
5. define the threshold;
6. define the reversal condition;
7. choose a mechanism smaller than the removed work.

Generated scale tests establish correctness and complexity shape.

They are not substitutes for representative application measurements.

Do not use single-run developer-machine timings as hard regression gates.

Do not optimize a phase that is not material to the selected end-to-end operation.

Do not add a metadata-only fast path for one edit unless measurement shows the existing complete path is material and the shortcut preserves every invariant.

## Agent attention and API spend

Treat model context, tool output, attention, wall time, CPU, memory, disk, CI minutes, and API spend as engineering resources.

Reduce them through deletion, direct ownership, focused inspection, and nonduplicated verification.

Never reduce them by hiding failures or weakening evidence.

Search before opening large files.

Read focused ranges, symbols, call sites, and diffs.

Reuse facts established earlier in the turn.

Keep one compact task ledger containing only:

- current objective;
- authoritative facts;
- open questions;
- decisions;
- tests run;
- remaining gates.

Do not copy whole source files into the ledger.

Do not ask multiple agents to rediscover the same map.

Use the smallest useful set of read-only subagents for genuinely independent questions.

One lead agent owns:

- architecture;
- writes;
- integration;
- verification;
- commits;
- final report.

Do not let two agents edit the same file or evolve the same semantic model independently.

Run focused tests before crate, workspace, release, or container boundaries.

Do not repeat an identical successful command after unchanged relevant inputs.

Use quiet commands while preserving exit status and diagnostics.

Keep full failure logs outside Git.

Surface only the relevant excerpt.

Do not dump unchanged files, repository-wide diffs, generated IR, bytecode, machine code, large JSON, complete projections, or successful logs without a consumer.

Do not commit prompt archives, transcripts, raw subagent packets, token ledgers, or per-turn fact registries.

Do not claim API-cost savings without measurement.

Removed bytes, lines, symbols, edges, reads, and command invocations are structural proxies, not direct token-cost measurements.

## Crates, modules, and dependencies

A crate boundary must earn itself through at least one current property:

- trust or unsafe boundary;
- independently useful API;
- supported target boundary;
- measured compile isolation;
- low-coupling subsystem;
- current product boundary.

Do not merge genuine unsafe or FFI boundaries merely to reduce member count.

Do not add a crate merely to hide fan-in or move a dependency edge.

Do not move types solely to improve graph aesthetics.

Prefer mature dependencies when they remove substantial machinery or risk.

Keep local code when it is smaller, clearer, safer, or measurably better.

Do not add a dependency for a trivial transformation.

Keep helper scope as narrow as its semantic reuse.

Do not create a repository-wide visitor, registry, planner, or framework for one local operation.

## Tests

Tests protect intended semantics and public invariants.

They do not protect provisional topology or compatibility.

Add the smallest focused regression or characterization for each selected root cause.

Cover relevant:

- success semantics;
- malformed input;
- wrong identity domain;
- wrong kind;
- wrong owner;
- stale identity;
- failure atomicity;
- identity preservation;
- old-snapshot preservation;
- deterministic output;
- ownership and cleanup;
- effect-free checking;
- exactly-once effects;
- stack safety;
- complexity shape.

Consolidate cases when one table-driven or generated test is clearer.

Do not create one test function for every checklist row.

Decode machine output as a consumer would.

Quiet-success tests assert both streams are empty.

Use generated fixtures for scale.

Keep costly equivalent geometry in explicit locked-release stress when justified.

Use differential, property, model, fuzz, Miri, sanitizer, or small-stack evidence when it is the cheapest independent oracle.

Delete tests that preserve obsolete APIs, fabricated boundaries, old formats, arbitrary limits, or private topology.

Never weaken a test merely to make a redesign pass.

Do not preserve a configurable impossible state solely so a rejection test can manufacture it.

## Verification

Escalate only after focused evidence passes.

Do not repeat the full boundary after unchanged relevant inputs.

### Focused boundary

Run the smallest relevant test target and filter.

Then run affected crates, binaries, features, integration targets, package fixtures, and machine-output consumers.

Use `cargo test --quiet -p <package> --locked <filter>` or the closest exact command.

### Native repository boundary

Run once after final relevant inputs are stable:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

### Retained product and container boundary

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run the container boundary when changes can affect:

- dependencies;
- packaging;
- release compilation;
- compiler behavior;
- executable behavior;
- VM behavior;
- native code;
- host capabilities;
- examples;
- smokes;
- system libraries;
- generated package artifacts;
- publication confidence.

Documentation-only work need not rebuild it.

If the environment blocks a command, report:

- the exact command;
- the failure category;
- the relevant output;
- successful remaining evidence;
- residual risk.

Never claim a command passed unless it completed successfully.

## Documentation

Update the owning document and delete stale text in the same change.

Do not copy implementation inventories into multiple files.

`docs/roadmap.md` contains only `Now`, `Next`, and `Later` ordering.

Create a decision record only for durable, non-obvious, expensive-to-rediscover rationale with a meaningful reversal condition.

Do not describe:

- target as current;
- hypothesis as measurement;
- planned systems as supported;
- private relocation as public movement;
- one-host observation as portable behavior;
- source attachment as semantic authority;
- a draft API as a persistence format.

Keep examples active and mechanically checked where practical.

Documentation length is not rigor.

Prefer one compact authoritative statement.

## Git and publication

Inspect status before editing.

Inspect status before committing.

Preserve unrelated work.

Do not reset, clean, rewrite history, or force-push work you did not create.

Commit one cohesive semantic, architectural, or measured result.

Do not split a single cutover by file type.

Exclude:

- task prompts;
- raw logs;
- scratch notes;
- generated temporary files;
- credentials;
- unrelated work.

Use a commit message that names the actual result.

Push only when explicitly requested.

After requested publication, verify local commit, tracking branch, and remote result.

## Final report

State:

- completed objective;
- evidence gate;
- starting commit;
- final commit;
- branch;
- upstream;
- worktree state;
- principal design;
- displaced paths deleted;
- boundary classifications that governed retained or removed validation;
- focused tests;
- measurements when relevant;
- exact final verification outcomes;
- environment-limited checks;
- residual risk;
- documentation status;
- commit status;
- publication status;
- material subagent use;
- next highest-leverage problem;
- why the next problem was not started.

Keep the report factual and compact.

Do not reproduce the task prompt.

Do not paste successful logs.

Do not claim unmeasured savings.

Do not describe future work as implemented.
