# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow procedure for a real local ownership boundary. It may not weaken repository-wide requirements for semantic authority, publication atomicity, identity integrity, deterministic validation, memory safety, resource bounds, security, evidence, or truthful reporting.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, specifications, documentation, benchmark labels, generated descriptions, commit messages, and handoffs.

## Safety and Authorization

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
```

Preserve unrelated work.

Do not reset, clean, overwrite, delete, stage, commit, amend, rebase, merge, push, force-push, close a pull request, edit an issue, publish a release, or otherwise change local or remote work unless the active task authorizes that class of action.

Repository permissions are not authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, unrelated user files, disposable benchmark payloads, or generated corpora with unclear licensing.

Keep scratch state, raw provider events, temporary workspaces, and destructive experiments outside the repository unless a retained sanitized artifact has a named current purpose.

Do not hide partial completion, failed verification, unavailable tools, or uncertain outcomes.

## Mission

Build `lkjscript` as an agent-native semantic programming system.

Autonomous coding agents are the primary program authors.

Humans remain first-class for intent, governance, security policy, explanation, review, operation, product ownership, and acceptance.

The product objective is:

> A coding agent can inspect, create, change, validate, test, compile, run, package, and review a program through compact exact semantic interactions, while program meaning has one authoritative typed representation and every accepted change remains deterministic and auditable.

Optimize jointly for correctness, weak-model success, compact interaction, low correction and provider cost, human review, deterministic history, secure execution, implementation locality, package viability, and long-term performance.

Do not optimize for novelty, preservation of sunk cost, compatibility with superseded repository states, or continuation of an old roadmap.

The current Semantic Program Graph, daemon topology, JSON protocol, artifact format, full-snapshot persistence, per-node identity model, workbench grammar, Core IR, interpreter, ownership plan, Rust module layout, and terminology are verified baselines.

They are not permanent architecture merely because they exist.

## Precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. The explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Machine descriptions generated from executable contracts.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. current structured performance evidence and `docs/performance.md`;
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and historical documents.

Newer verified checkout state outranks older plans.

An old prompt is historical evidence unless the active task explicitly selects it.

A campaign prompt owns one campaign's scope, hypotheses, gates, non-goals, and handoff requirements. It does not silently become permanent semantic authority.

When accepted semantics change, update their owning specification and executable contract in the same verified milestone.

## Fact Ownership

Keep one maintained owner for each active fact.

- `docs/spec/`: accepted observable semantics and public contracts;
- `docs/architecture.md`: current components, topology, trust boundaries, and trusted computing base;
- `docs/status.md`: exactly what the current checkout implements and lacks;
- `docs/performance.md` and structured evidence: reproduced current measurements and reversal evidence;
- `docs/roadmap.md`: ordered future evidence gates;
- `README.md`: concise human-first product explanation and supported entry points;
- root `AGENTS.md`: durable repository policy;
- `prompts/`: campaign execution artifacts, never semantic authority;
- executable code and tests: accepted machine behavior.

Do not maintain parallel schema catalogues, status lists, version tables, architecture inventories, dependency inventories, benchmark tables, or memory-model tables.

Generated descriptions derive from executable contracts.

Git history is the archive for superseded prompts, plans, code, fixtures, and campaign narratives. Active documentation is not an append-only journal.

Delete stale active-tree material when it has no current consumer.

## Enduring Invariants

These invariants outrank the current implementation language, process topology, storage engine, file layout, transport, syntax, backend, runtime representation, model provider, and platform.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions or commits are immutable.
3. Every durable namespace has one unambiguous logical publication authority.
4. Accepted semantic forms belong to closed versioned contracts.
5. Unknown, malformed, ambiguous, foreign-domain, or unsupported forms reject.
6. Arbitrary property bags and arbitrary string-labelled semantic edges are not authority.
7. AI output and natural-language intent are untrusted proposals.
8. Deterministic validators, not model judgment, decide acceptance.
9. A mutation is a typed transaction or a closed proposal normalized into one.
10. One successful publication creates exactly one accepted revision or commit.
11. Rejection and validate-only publish nothing and consume no durable identity.
12. Durable identity exists only where continuity, independent reference, sharing, repair, history, attribution, or external targeting requires it.
13. Implementation scaffolding does not receive durable identity merely because it is represented as a node.
14. Durable identity is independent of names, formatting, source positions, proposal spelling, content hashes, compiler indexes, artifact offsets, storage keys, runtime handles, process addresses, and allocator choices.
15. Durable identities are never silently reused within their domain.
16. Revision-local references, aliases, local binders, compiler IDs, runtime handles, storage digests, and display names remain distinct domains.
17. A content digest may identify immutable bytes; it is not automatically semantic identity.
18. Identity-preserving change exists only under an explicit validated continuity rule.
19. Derived facts never become a second mutable source of truth.
20. Text, JSON, binary encodings, editable documents, caches, indexes, IR, bytecode, profiles, memory plans, and machine code may be views, proposals, or derived state.
21. A view or proposal cannot bypass the authoritative validator.
22. Editable text is allowed when it is exact, bounded, revision-bound, and normalized through the same semantic path.
23. Rendering and reparsing are never required to recover accepted authority.
24. The compiler consumes one immutable accepted state.
25. Only a complete selected-entry dependency closure enters executable lowering.
26. One simple executable route defines behavior; faster tiers remain differential against it.
27. Accepted language semantics cannot express unchecked memory access.
28. User-controlled depth does not consume unbounded native stack.
29. Host access requires explicit typed authority.
30. External resources require explicit outcome and deterministic cleanup semantics.
31. Non-idempotent effects are never silently retried after possible partial action.
32. Observable order is explicit and deterministic.
33. Potentially large work and output are bounded, streamed, paginated, chunked, or policy-controlled.
34. Durable state is acknowledged only after its documented publication contract.
35. Corrupt, partially published, ambiguous, or unsupported authority rejects rather than being guessed valid.
36. Memory safety, resource exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, native isolation, and crash consistency are separate contracts.
37. Every public boundary states schema or version, canonical form, limits, rejection behavior, output policy, and domain binding.
38. Compactness never weakens typing, validation, authorization, identity, durability, diagnostics, or verification.
39. Performance optimization preserves an executable correctness oracle.
40. Human-facing claims remain no stronger than the checkout and reproduced evidence.
41. Agent interaction, context locality, change locality, verification locality, and provider cost are first-class engineering dimensions.
42. Backward compatibility is absent unless the active user explicitly requires it.
43. Incompatible-change freedom is used to converge on one coherent active path.
44. No legacy reader, compatibility mode, edition split, dual success path, hidden fallback, or silent migration remains after a cutover.
45. Current bootstrap absences do not become permanent prohibitions without semantic reason.

## Decision Doctrine

Treat every historical decision as provisional except the enduring invariants.

Do not continue a roadmap item because it is next or preserve a mechanism because it was difficult to build.

Reproduce evidence before reversing a working subsystem.

Prefer a high-leverage architectural correction over accumulating local exceptions when it closes more total complexity.

Prefer a local feature over a platform when only one consumer exists.

Evidence gating prevents speculative overengineering; it must not become permanent minimalism.

Before retaining a substantial choice, record:

- the concrete user, agent, application, or maintenance problem;
- accepted semantic and safety obligations;
- measured baseline;
- serious alternatives;
- expected benefit;
- implementation, trusted-surface, context, and verification cost;
- the correctness oracle;
- reversal condition;
- what is deleted if the choice wins.

Revalidate architecture when local features repair representation friction, facts are copied, tasks require world dumps, owners combine unrelated concerns, durable IDs lack continuity consumers, history dominates restart, lifecycle fails, or optimization verification exceeds benefit.

Use controlled prototypes for uncertain questions.

Delete losing prototypes completely.

Do not retain dormant alternatives, compatibility readers, duplicate success paths, or speculative frameworks.

## Authority and Projections

Use the plain term **typed semantic program model** before repository-specific acronyms.

The formal model name and physical shape may change.

Do not let "graph," "source," "database," "daemon," "object," or "node" predetermine storage or identity.

One authority does not require hostility toward text.

A deterministic text form may be a review view, context document, editable proposal, import/export form, or package manifest.

A parseable proposal may round-trip when that improves agent performance. It names an exact base, binds scope and schema, rejects stale or ambiguous references, normalizes through the same typed mutation path, and discards syntax after normalization.

A projection may not allocate durable identity independently, define behavior independently, persist a competing mutable AST, treat formatting as identity, hide editable omissions, or bypass validation.

Compare generated source, canonical text, binary forms, and semantic APIs on equal tasks.

## Identity Discipline

Durable identity is opt-in.

For each durable identity class, define its cross-revision consumer, continuity rules, replacement rules, deletion, history, branch behavior, public encoding, bounds, and validate-only allocation.

Prefer revision-local structural identity for anonymous immutable terms, implied control scaffolding, canonical terminators, compiler blocks, local binders, and transient query objects when no cross-revision consumer exists.

A declaration, independently renamed member, public function, first-class test, explicit permission, resource, repairable placeholder, or other continuity-bearing entity may justify durable identity.

Do not infer durable identity from in-memory shape.

Do not use a content hash as semantic identity merely because storage is content-addressed.

Names are lookup and presentation metadata unless an accepted external contract says otherwise.

Aliases bind exact workspace or package, revision, schema, context digest, target set, and purpose. They never become semantic identity.

Continuity and migration maps are explicit, deterministic, complete for their declared scope, and validated. Ambiguity rejects.

## Revisions, Transactions, and Storage

Every mutation names an exact base revision or parent commit and durable namespace.

A successful mutation performs boundary validation, proposal normalization, semantic and history validation, response and persistence preflight, durable publication, in-memory publication when applicable, and bounded receipt construction.

Rejection changes none of those states.

Validate-only performs the same relevant semantic and byte preflight without publication.

A multi-turn candidate is allowed only as an explicit immutable unpublished commit chain or equally precise model with defined parentage, authority status, identity domain, resource bounds, stale-base behavior, query/run semantics, publication, cancellation, crash recovery, and abandonment.

Full snapshots, tombstones, serial allocation, one writer process, HEAD files, journals, object stores, databases, and branches are implementation choices.

A replacement preserves logical publication, corruption rejection, bounded recovery, historical reconstruction or explicit retention, deterministic conflicts, and an independent reconstruction oracle.

Keep mutable workspace history, publishable package artifacts, derived executable artifacts, and caches conceptually distinct.

Do not make a workspace database the permanent package format by accident.

## Agent-Facing Product Surface

The external coding agent is the primary author.

Optimize semantic success, not serialization success.

Normal tasks should not require implementation source, a global schema dump, compiler plumbing, daemon lifecycle knowledge, storage metadata, or repeated unchanged context.

Provide compact orientation, task-scoped context, exact expansion, deterministic review, exact editable proposals when they win, legal local actions, scoped aliases, typed diagnostics, validate/apply parity, compact receipts and deltas, bounded execution facts, digest-based reuse, and explicit omissions.

Treat observation, action, review, execution, and history as separate interface problems.

A low-level node API is not agent-friendly merely because it is typed.

A complete global schema is not agent-friendly merely because it is exact.

Use digest-keyed derived caches when measurements justify them. Caches are disposable, domain-bound, bounded, and never authority.

Do not add model-ranked retrieval or planning to the correctness path.

Measure equal tasks using success, intended and unintended rejection, correction count, semantic requests, discovery repetition, action and observation bytes, provider token classes when exposed, tool calls, failed commands, processes, connections, files opened, elapsed time, compile cost, binary size, implementation surface, and monetary cost only when exact pricing is available.

Do not infer tokens from bytes or price from tokens without applicable provider rules.

Minimize the API cost of developing `lkjscript` itself through stable policy, concise current documentation, narrow owners, focused commands, and deletion of stale campaign material.

## Contracts and Schema

Keep one executable owner for each accepted type, field, variant, operation, query, error, limit, and boundary contract.

Derive codecs, schema discovery, help, proposal forms, validation metadata, and documentation fragments from that owner where practical.

Do not maintain a giant parallel hand-written description of DTOs.

Do not use runtime registration for a closed bootstrap vocabulary without a real dynamic-extension consumer.

A macro, derive, generator, or IDL is acceptable only when its authority is explicit, output deterministic, accepted shapes reviewable, strict decoding preserved, semantic invariants visible, build cost measured, and stale output detected.

Schema projection is on-demand and dependency-closed.

Normal agent work receives only the fragment it needs.

## Implementation Architecture

Organize code around stable fact ownership and changed-together behavior.

Do not impose arbitrary universal file-size limits, but do not ignore pathological context concentration.

Split for real semantic, identity, encoding, transaction, validation, storage, query, protocol, agent, compiler, runtime, effect-host, platform, unsafe-isolation, test-oracle, compile-dependency, or change-locality boundaries.

Keep tightly coupled invariants together.

Avoid forwarding forests, one-function micro-modules, duplicate helpers, broad preludes, cyclic dependencies, and cosmetic directory churn.

Public visibility is minimal.

A crate boundary must improve dependency direction, binary composition, compile isolation, trust isolation, or reuse.

For context-locality refactors, measure files and source bytes opened, dependency edges, compile selection, focused-test selection, duplication, binary size, and full-build cost.

Generated bulk is isolated from hand-maintained logic.

Tests are owned by current invariants, not old campaign names.

Delete catch-all campaign test modules after their cases have current owners.

## Language, Compiler, Runtime, and Memory

Choose observable semantics before representation or optimization.

A new type, operation, effect, package construct, or resource needs a real consumer and exact contracts for equality, identity, mutability, duplication, ordering, conversion, failure, allocation, lifetime, cleanup, permissions, lowering, queries, persistence, public values, and tests.

Do not expose implementation memory choreography to authors solely to simplify the runtime.

Prefer inference from types, use graphs, control flow, and escape facts when semantics do not require explicit ownership.

Do not collapse value semantics, lifetime, aliasing, representation, reclamation, and resource cleanup into "ownership" or "GC."

A memory plan, uniqueness optimization, custom arena, reference-count protocol, tracing collector, region system, or native backend must beat a simpler safe implementation enough to justify its code and verification surface.

Sunk cost is not evidence.

Keep a simple executable oracle.

IR, bytecode, memory plans, profiles, and native images are derived.

Do not expose private registers, blocks, layouts, handles, offsets, retain/release operations, allocator calls, or compiler indexes.

Do not preselect LLVM, Cranelift, a custom JIT, AOT, GC, reference counting, regions, or ownership syntax without workload evidence.

Native acceleration expands the trusted computing base and requires an isolated validated boundary and differential evidence.

Fuel, time, frames, stack, cells, logical bytes, retained bytes, objects, allocations, handles, external resources, and result materialization are distinct policies.

Optimization preserves deterministic failures unless accepted semantics explicitly change.

## Effects and Resources

Pure computation is a bootstrap baseline, not a permanent product boundary.

Every host effect requires explicit typed authority.

Permission values describe what may be attempted.

Resource values describe what must be released, consumed, committed, or closed.

Every effect defines authority acquisition, validation, order, cancellation, timeout, partial action, idempotency, retry, cleanup, audit, isolation, crash behavior, and deterministic testing.

Never rely on nondeterministic finalization for required cleanup.

Do not add convenient host calls that bypass the eventual permission model.

## Evidence, Dependencies, and Tests

Measure before and after substantial changes.

Separate semantic, serialization, storage, compilation, execution, physical memory, agent interaction, and repository-development work.

Report exact environments, commands, inputs, seeds, warm/cold state, sample count, and limitations.

Do not present one model run, machine, seed, or microbenchmark as universal.

Do not present safe Rust as a formal proof or deterministic mutation as coverage-guided fuzzing.

A benchmark or model trial has a stopping rule and cost budget.

Reuse a valid sealed baseline rather than paying to repeat it without reason.

Run cheap deterministic gates before expensive trials.

Before changing a dependency, inspect its consumer, license, maintenance, security, features, transitives, build scripts, proc macros, native or unsafe code, platforms, build and binary cost, reproducibility, and removal cost.

Delete unused dependencies in the same milestone and keep the lockfile authoritative.

Test acceptance and rejection, identity domains, rollback, validate-only, idempotency, stale bases, continuity, diffs, canonical storage, publication failure, restart, corruption, repair, historical execution, compiler/verifier rejection, runtime traps, resources, stack safety, cleanup, caches, and old-version rejection as applicable.

Use generated sequences, property tests, fuzzing, Miri, sanitizers, model checking, or concurrency testing for named retained risks. State exactly what ran.

A retained application uses the public path, has an independent deterministic oracle, exercises interacting capability and rejection, includes meaningful maintenance and restart when relevant, avoids private semantic fixtures, remains comprehensible, and justifies its capability.

Agent-interface work requires sealed equal-task evidence when feasible.

Distinguish deterministic replay, controlled observation, and general benchmark.

## Documentation and Repository Hygiene

Use plain meaning before specialized terminology.

Prefer "typed semantic program model," "named record type," "variant type," "typed placeholder," "explicit permission value," "immutable managed value," and "editable semantic document" before narrower jargon.

Do not lead with what the system lacks.

Explain the positive product boundary first.

README is not an agent manual. Specifications are not status. Status is not roadmap. Performance evidence is not marketing. Prompts are not permanent specifications.

Keep current documents concise.

Move durable facts to their owner.

Delete superseded prompt files and campaign narratives when Git history is sufficient and no current consumer remains.

Do not make every future agent reread historical campaigns.

Do not rewrite this file cosmetically.

Stable instruction prefixes have prompt-cache value.

## Work Procedure

Before substantial work:

1. Inspect branch, commit, worktree status, and active instructions.
2. Identify the active campaign prompt.
3. Read this file, `docs/status.md`, and only the owning specifications, architecture, evidence, and source relevant to the task.
4. Do not read every historical prompt by default.
5. Reproduce the smallest relevant public-path baseline.
6. Name the product problem and changed contract.
7. Compare serious alternatives and reversal conditions.
8. Enumerate affected readers, writers, validators, descriptors, formats, queries, views, examples, tests, and documents.
9. Define semantic, rejection, durability, safety, interaction, and performance oracles.
10. Choose a dependency-closed milestone and record non-goals.
11. Implement through coherent verified milestones when local commits are authorized.
12. Delete displaced code and losing prototypes in the same campaign.
13. Recheck the checkout after external or concurrent changes.

Resolve repository facts from evidence rather than asking unnecessary questions.

Do not implement blindly from an old prompt.

Do not stop at a design report when implementation is requested.

Do not leave stale readers, dead flags, commented alternatives, shadow schemas, campaign TODOs, or undocumented fallback paths.

## Direct Cutovers

When replacing an active boundary:

- enumerate every reader, writer, validator, descriptor, fixture, example, cache, artifact, and document;
- choose an unambiguous new version, schema identity, tag set, magic, or path;
- replace active producers and consumers together;
- reject old forms directly;
- delete displaced adapters, readers, writers, flags, fixtures, tags, and claims;
- update specifications and current status in the same milestone;
- retain one active path.

Backward compatibility is not a default virtue.

Churn without convergence is not a virtue either.

## Verification and Handoff

The normal final verification boundary is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run required public examples and focused boundary, identity, persistence, corruption, restart, rollback, document/proposal, cache, runtime, memory, and performance commands.

A failed final command invalidates the boundary. Fix it and rerun the complete boundary.

Never report stale results after later edits.

Record exact unavailability of Miri, sanitizers, fuzzers, model checkers, isolated agent trials, provider telemetry, or platform targets.

Do not weaken policy to manufacture a green result.

Before handoff, inspect status and the complete diff; confirm unrelated and remote state are untouched, one active path remains, facts have one owner, documents and claims match evidence, boundaries are explicit, the agent surface is discoverable, and no secret or disposable artifact entered the repository.

The handoff states starting commit, final worktree, decisions and rejected alternatives, semantic and identity changes, proposal/protocol/storage/compiler/runtime/topology changes, deleted paths, trusted-surface effects, exact verification and application results, measured interaction/storage/runtime/repository-context changes, unavailable evidence, unresolved risks, and next gate.

Do not push or publish unless explicitly requested.
