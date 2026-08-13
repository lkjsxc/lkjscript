# AGENTS.md

## Scope

This file applies to the entire repository.

A deeper `AGENTS.md` may narrow local procedure but must not weaken repository-wide semantic, safety, identity, or verification requirements.

Use English for maintained code, tests, diagnostics, documentation, and commits unless an external protocol requires otherwise.

## Mission

Build lkjscript as an AI-primary, statically typed, memory-safe, collector-free programming system.

AI-primary means that agents can construct, inspect, revise, validate, compile, and execute programs through deterministic typed semantic operations.

Human-readable source remains an import and package format.

Source text is not the mutable semantic authority after import.

The in-process semantic workspace is the active authoring authority.

Long-term performance matters, but correctness, safety, failure atomicity, one coherent authority, stable identity, and generic runtime coverage come first.

## Current Product Boundary

The supported product is local package checking and execution plus an in-process semantic workspace API.

The active path is:

```text
verified source or source-free semantic construction
    -> SemanticProgram
    -> immutable WorkspaceSnapshot
    -> completeness gate
    -> complete HIR
    -> memory planning
    -> verified SSA
    -> validated bytecode
    -> VM or bounded baseline-native execution
```

`SemanticProgram` is the mutable staging authority inside one transaction.

`WorkspaceSnapshot` is the immutable published authority.

HIR, SSA, bytecode, native code, and projection text are derived.

Public semantic identities are opaque and stable.

Compiler IDs, addresses, ordinals, vector positions, slots, registers, and machine details remain private.

A semantic transaction publishes one complete revision or publishes nothing.

Old snapshots remain valid.

Presentation attachments and source provenance are optional and never become semantic identity.

The following are absent by design unless a current measured consumer proves otherwise:

- daemon or warm-service architecture;
- RPC, collaboration, or distributed protocols;
- persistence journals or databases;
- schedulers or orchestration;
- plugin platforms;
- generalized caches;
- compatibility layers for deleted designs.

Do not recreate these systems while solving a local compiler or workspace problem.

## Priority Order

When requirements compete, use this order:

1. semantic correctness;
2. memory and type safety;
3. transaction failure atomicity;
4. one semantic authority;
5. stable public identity;
6. deterministic observable behavior;
7. generic runtime coverage;
8. stack safety and scale correctness;
9. direct AI-agent usability;
10. measured performance;
11. implementation simplicity;
12. documentation precision;
13. compatibility with provisional behavior.

Compatibility is intentionally last.

## Authority

Use this authority order:

1. the current user task;
2. the nearest applicable `AGENTS.md`;
3. normative files under `docs/spec/`;
4. executable code and focused tests;
5. manifests, lock files, schemas, and command definitions;
6. `README.md`;
7. `docs/status.md`;
8. `docs/architecture.md`;
9. `docs/performance.md`;
10. `docs/roadmap.md`;
11. historical prompts, comments, and stale prose.

When code and prose disagree, identify the intended current contract with focused evidence.

Do not preserve an accidental implementation merely because it exists.

Do not rewrite a normative contract merely because one implementation is inconvenient.

Update every maintained authority affected by the final behavior.

## Compatibility and Cutovers

Backward compatibility is not an objective unless the current task explicitly makes it one.

Prefer one clean cutover.

Do not add legacy modes, migration modes, `v2` duplicates, `next` modules, deprecated aliases, compatibility adapters, dual authorities, or feature flags that preserve obsolete architecture.

Delete obsolete code, tests, schemas, prose, and dependency edges in the same change.

Persistent package and lock boundaries remain exact where they are current product contracts.

Everything else must earn compatibility from a real current consumer.

## Select One Verified Vertical

Each implementation turn should select one dependency-closed vertical.

A valid vertical has:

- one concrete user or agent operation;
- one identified authority;
- one end-to-end success path;
- one end-to-end rejection path;
- exact identity behavior;
- exact transaction behavior;
- focused executable evidence;
- a clear stop condition;
- explicit non-goals.

Broad intentions such as “improve architecture” or “make the language AI-friendly” are not implementation verticals.

A vertical may include a prerequisite representation cleanup when the current representation makes the selected operation dishonest.

That prerequisite must be necessary, remove an existing contradiction, and avoid unrelated redesign.

Stop after the selected vertical is integrated and verified.

Do not begin the next roadmap item in the same turn.

## Multi-Turn Workflow

Assume development continues across multiple turns.

At the start:

1. record the current commit;
2. inspect `git status --short`;
3. read applicable instructions;
4. read the exact current roadmap item;
5. inspect recent relevant commits;
6. locate the executable authority;
7. run the smallest characterization needed;
8. keep a compact task ledger.

The ledger should record the selected operation, current contradiction, authority, likely files, invariants, focused tests, non-goals, and stop condition.

Do not commit the ledger unless it becomes maintained documentation.

During implementation:

1. characterize current behavior;
2. make one coherent representation decision;
3. update the producer;
4. update every current consumer;
5. delete obsolete alternatives;
6. add focused tests;
7. run focused verification;
8. update maintained documentation;
9. run the full boundary once;
10. inspect the final diff;
11. commit one cohesive change when permitted.

At the end, report the starting and ending commits, semantic result, representation changes, tests, commands, unrun verification, remaining gaps, and exact worktree state.

## Evidence Before Abstraction

Before adding an abstraction, answer:

- Which current producers and consumers use it?
- Which invariant does it own?
- Why can the invariant not remain local?
- What duplication or invalid state does it remove?
- What measurable work does it avoid?
- What current test proves its value?
- Does it create another authority or identity domain?
- Does it require persistence, versioning, or a service?
- Does it narrow the generic path?
- Does it increase agent search space?
- Is a local helper sufficient?
- What is its deletion condition?

If the answers are weak, do not add the abstraction.

A new type is not automatically better modeling.

A new crate is not automatically modularity.

A registry is not automatically authority.

A digest is not automatically integrity.

A protocol is not automatically agent usability.

A cache is not automatically performance.

## Preferred Solution Order

Try solutions in this order:

1. delete the obsolete path;
2. use an existing identity or typed value;
3. make one representation authoritative;
4. move validation to the owning boundary;
5. replace repeated scans with one local index;
6. replace recursive user-depth traversal with an explicit work stack;
7. simplify a data structure;
8. add a narrow helper;
9. add a local measurement;
10. add a cache only with measured reuse;
11. add process or protocol machinery only with a current consumer.

Do not start at step ten or eleven.

## Anti-Overengineering Rules

Do not add a generic tree-edit framework, visitor framework, planner, registry, service boundary, serializer, digest, cache, or identity domain for one local operation.

Do not expose private coordinates or compiler IDs.

Do not turn presentation names into identity.

Do not turn measurements into language limits.

Do not turn tests into architecture.

Do not turn historical documentation into compatibility.

Do not generalize around hypothetical future consumers.

Do not add configuration for a single valid behavior.

Do not preserve an impossible state solely so a rejection test can manufacture it.

Keep helpers local until multiple current consumers justify broader placement.

## AI-Primary Design

Prefer typed operations, stable identities, immutable snapshots, exact continuations, structured diagnostics, deterministic pagination, finite candidates, explicit unsupported results, source-free construction, source-free compilation, failure atomicity, and direct execution evidence.

Avoid mutable source strings as semantic authority, heuristic rebinding, hidden automatic resolution, implicit global state, unstable coordinates, prose-only success criteria, broad natural-language command interpreters, background services, opaque retries, stale caches, and partial publication.

Weak models benefit more from a small exact API than from a large speculative platform.

## Semantic Authority

Source parsing is a boundary.

Resolve source names during import.

Move analyzed meaning into `SemanticProgram`.

Retain source provenance only for current diagnostics and package verification.

A post-import semantic edit must not reload or reparse source.

A source-free program must compile without synthetic source.

Do not render and reparse semantic edits.

Do not retain parallel mutable HIR and workspace authorities.

Do not retain stale derived representations in snapshots.

Do not reconstruct source identity for source-free nodes.

## Identity

Public identities must be opaque, namespace-scoped, generation-aware, stable across unrelated edits and private compaction, invalid after deletion, preserved in old snapshots, and checked before use.

Private identities may be dense and may relocate during staging.

Every relocation that affects a surviving public entity must be reconciled explicitly.

Do not infer continuity from equal content, names, hashes, preorder, or vector position.

A surviving entity keeps its public identity.

A deleted entity is tombstoned.

A same-name recreation receives a fresh generation.

A replacement subtree receives new descendant identities unless the operation explicitly defines movement continuity.

An identity-preserving operation must prove one-to-one continuity.

## Names

Names are presentation and lookup data unless a normative language rule explicitly defines otherwise.

Resolve source names to identities once.

Use identities for subsequent semantic references.

Renaming must not create a new entity, rebind references by text, alter runtime identity, or rewrite unrelated semantic structure.

Do not store redundant mutable names in identity-bearing private types.

Name collision rules belong to the namespace that owns the name.

Do not impose global uniqueness on a local member namespace.

A no-op edit must not create misleading semantic history unless its contract explicitly documents no-op publication.

## Transactions

Every transaction checks its base revision.

Every target identity is checked for namespace, generation, liveness, and kind.

The identity allocator is staged.

All fallible work occurs before publication.

Failure publishes no snapshot, consumes no public identity, and leaves current `Arc`, revision, diagnostics, blockers, attachments, provenance, and allocator state unchanged.

Success publishes exactly one immutable revision and one deterministic base-to-final diff.

Diffs describe semantic changes and never expose private addresses or compaction churn.

Do not emit reference or call rewiring when a stable target is unchanged.

Batch behavior must be explicit.

Reject unsupported overlap rather than adding a generalized planner.

Use final-state validation where the existing contract requires it.

Do not silently make edit order irrelevant or observable contrary to the current contract.

## Partial Programs

Incomplete snapshots are first-class queryable values.

Holes and unresolved value references are semantic nodes.

Completeness blockers are structured.

Diagnostics are derived from current incomplete state.

Incomplete snapshots must not compile.

Do not install executable placeholders, invent fallback values, or auto-resolve ambiguity.

Requested-name intent is not a selected target.

## Types

Public workspace types use stable semantic identities.

Compiler-local types may use private identities, but compiler-local names must not masquerade as identity.

Recursive type operations must remain stack-safe.

A type representation change must inventory constructors, pattern matches, clone, drop, equality, hashing, substitution, validation, dependency collection, compaction, query conversion, projection, diagnostics, tests, and persistent boundaries.

Use existing identity domains.

Do not add an identity merely to avoid updating consumers.

Do not update one type consumer and leave another name-based or index-based authority behind.

## Compiler Pipeline

`compile_snapshot` is the sole semantic-snapshot compilation boundary.

It rejects incompleteness before HIR memory planning, SSA, bytecode, native lowering, or execution.

Complete HIR must pass canonical consistency and ownership validation.

Memory planning remains authoritative for ownership obligations.

SSA must be verified.

Bytecode must be validated.

Derived stages must not observe holes or unresolved references.

Do not bypass canonical validation for a convenient semantic edit.

Do not duplicate compiler logic inside workspace staging.

Use the ordinary compiler path as an independent oracle.

## Runtime

The VM is the generic validated execution route.

Baseline native execution is a bounded specialization.

Native eligibility, lowering, installation, or typed pre-entry preparation may decline.

A pre-entry decline runs the unchanged validated program in the VM.

After native entry begins, its result is final.

Do not retry in the VM after native entry.

Do not narrow valid language behavior because native lowering is incomplete.

Exactly-once effects and exact cleanup are semantic requirements.

Do not add a public engine selector or automatic transition policy without a current product requirement.

## Ownership and Unsafe Boundaries

Collector-free ownership behavior is a current design constraint.

Moves, borrows, drops, cleanup ranges, control transfer, and failure paths must remain exact.

A semantic edit that changes structure or control must pass canonical ownership validation.

Do not copy affine values, suppress cleanup, or weaken validation to make a feature pass.

Unsafe code must remain localized and state its safety argument.

FFI inputs and outputs must be validated.

Pre-entry native installation must remain failure-atomic.

Executable mappings must preserve W^X behavior.

## Stack Safety and Scale

User-controlled depth must not consume unbounded native stack.

Use explicit work stacks or another proven bounded-stack mechanism for expression, pattern, type, value, dependency, compaction, projection, query, and validation traversal.

Do not impose arbitrary semantic limits to avoid fixing recursion.

Check conversions to host indexes.

Do not silently narrow user-scale counts.

Use generated fixtures for scale.

Keep expensive equivalent geometry in explicit locked-release stress tests when appropriate.

## Boundary Classification

Classify a value before designing its contract.

Persistent bytes require exact encoding, validation, corruption handling, and a current compatibility policy.

Unsafe or FFI values require checked representation, explicit safety invariants, and failure atomicity.

Machine-readable CLI output requires one deterministic schema and consumer-style decoding tests.

Same-build in-process values normally need Rust types and constructor validation, not a digest, registry, serializer, or protocol version.

Closed vocabulary normally needs a closed enum or direct match, not a global registry.

Metrics and diagnostics are observations, not semantic authority.

## Crates, Modules, and Dependencies

A crate boundary must earn itself through unsafe or FFI isolation, an independently useful API, a supported target boundary, measured compile isolation, low coupling, or a current product boundary.

Do not add a crate to hide fan-in or improve graph aesthetics.

Do not merge a genuine unsafe or FFI boundary merely to reduce member count.

A module should own one coherent responsibility.

Prefer mature dependencies when they remove substantial machinery or risk.

Keep local code when it is smaller, clearer, safer, or measurably better.

Do not add a dependency or proc macro for a trivial fixed transformation.

## Errors and Determinism

Reject invalid input at the owning boundary.

Use structured errors where callers need structured facts.

Do not stringify a typed error and parse it later.

Do not fabricate source paths, spans, or identities.

Do not swallow host failures.

Do not panic on untrusted or user-scale input.

Avoid new production `unwrap` or `expect` without a proven invariant and repository convention.

Observable output must not depend on hash-map iteration, allocator accidents, or private vector order.

Sort by semantic keys where order is not otherwise defined.

Preserve declaration and evaluation order where the language defines them.

Continuations bind namespace, revision, and query.

## Performance

Performance work requires a workload, baseline, suspected cost, proposed structural change, correctness oracle, and reversal condition.

Measure the production path.

Separate startup, source loading, parsing, staging, indexing, HIR derivation, ownership, SSA, bytecode, native lowering, installation, VM execution, and cleanup.

Do not infer performance from code size, allocation count, or one sample.

Do not add caching, parallelism, or a warm service without measured repeated work and a current consumer.

Do not turn performance observations into semantic admission limits.

Update `docs/performance.md` only with retained measured evidence.

Do not fabricate numbers or claim gains that were not measured.

## Agent Attention and API Cost

Treat coding-agent attention and API spend as finite.

Save cost by reducing search and rework, not by reducing verification.

Use `rg` before opening whole files.

Read narrow ranges.

Do not repeatedly dump large files or full compiler logs.

Capture long output once and inspect the relevant failure region.

Batch mechanically related representation updates.

After a deliberate enum or type-shape change, one targeted compiler check may serve as a migration inventory.

Do not invoke a compiler check after every individual match arm.

Run focused tests before the workspace suite.

Do not rerun an unchanged full boundary.

Do not assign overlapping work to multiple agents.

Use subagents only for truly independent bounded investigation.

The lead agent owns design, integration, and final verification.

Do not spend tokens generating speculative alternatives after evidence selects one design.

Do not create API-cost instrumentation without a current consumer.

Task prompts are execution inputs and are not committed unless explicitly made repository documentation.

## Tests

Tests protect intended semantics and public invariants, not provisional topology.

Add the smallest focused evidence for each root cause.

Prefer table-driven cases and shared fixtures when clearer.

Use independent oracles where practical.

Relevant dimensions include success, malformed input, wrong namespace, stale or deleted identity, wrong kind or owner, revision mismatch, failure atomicity, allocator rollback, identity preservation, old snapshots, deterministic diffs and projection, type preservation, ownership, cleanup, exactly-once effects, source-free compilation, no post-import parsing, VM and native behavior, stack safety, compaction, and complexity shape.

Do not create one test function per checklist row when one coherent scenario is better.

Do not weaken a test merely to make a redesign pass.

Delete tests that protect obsolete APIs or representations.

Quiet-success CLI tests assert both streams are empty.

Machine-output tests decode output as a consumer.

Ignored stress tests state why they are ignored and how to run them.

## Verification

Escalate only after focused evidence passes.

Use the smallest relevant package and test filter first.

Then run the full repository boundary once:

```bash
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run `cargo fmt --all` before the check form when formatting changed.

Do not omit `--locked`, all targets, all features, or the release build.

Do not claim Docker verification if Docker was unavailable.

Do not infer CI health from absence of reported checks.

If a command fails environmentally, preserve the diagnostic and distinguish environment failure from product failure.

Run `git diff --check`.

Inspect `git status --short` and the final diff for prompts, logs, caches, generated files, or build artifacts.

## Documentation

Documentation describes current behavior.

Use `docs/spec/` for normative behavior, `docs/status.md` for the checkout, `docs/architecture.md` for responsibility and data flow, `docs/performance.md` for measured evidence, `docs/roadmap.md` for selected next work and gates, and `README.md` for the concise entry point.

Do not write future architecture as though it exists.

Do not duplicate large authorities across documents.

Remove stale claims in the same change.

A completed roadmap item leaves `Now`.

Do not invent speculative work merely to keep `Now` populated.

## Git

Record the starting commit.

Do not overwrite unrelated user changes or destructively reset work you did not create.

Keep generated and build artifacts out of commits.

Use one cohesive commit for one verified vertical when permitted.

The commit subject describes the semantic result, not the task prompt.

Do not push or open a pull request unless explicitly requested.

Before commit:

1. inspect `git status --short`;
2. inspect `git diff --stat`;
3. inspect the substantive diff;
4. run `git diff --check`;
5. confirm required verification;
6. confirm maintained documentation;
7. confirm no task prompt is staged.

After commit, inspect `git status --short`, record the commit, and report any intentionally untracked file.

## Completion

A task is complete only when the selected operation works end to end, invalid input rejects deterministically, failed transactions publish nothing, identities follow the declared continuity rule, old snapshots remain valid, canonical validation still runs, runtime behavior is correct, focused tests protect the root cause, maintained documentation matches the checkout, required verification passed or is honestly reported, obsolete paths are deleted, and the final diff contains no speculative adjacent system.

The final report states starting and ending commits, the selected vertical, the representation decision, compatibility breaks, tests, focused and full commands, unrun verification, measurements if any, remaining gaps, and worktree state.

Do not claim broader language support or performance than the evidence proves.

## Stop Rules

Stop and narrow the task if a second semantic authority, new identity domain, generic planner, unmeasured cache, consumerless protocol, narrowed VM path, weakened atomicity, invalid old snapshot, user-depth recursion, or unrelated feature expansion appears.

The usual correction is to reuse an existing identity, localize a helper, delete a redundant representation, or defer an unproven system.

Do not solve uncertainty by building more infrastructure.
