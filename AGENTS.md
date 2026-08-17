# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may narrow procedure for a real ownership boundary. It may not weaken
repository-wide requirements for semantic authority, publication atomicity, deterministic
validation, identity integrity, memory safety, resource bounds, security, evidence, or truthful
reporting.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, benchmark labels, generated descriptions, commit messages, and
handoffs.

## Safety and Authorization

Inspect the actual checkout before editing:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
```

Preserve unrelated work.

Do not reset, clean, overwrite, delete, stage, commit, amend, rebase, merge, push, force-push,
close a pull request, edit an issue, publish a release, or otherwise change local or remote work
unless the active task authorizes that class of action.

Repository permissions are not authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data,
unrelated user files, raw provider events, disposable benchmark payloads, or generated corpora
with unclear licensing.

Keep scratch state, temporary workspaces, destructive experiments, and unsanitized measurements
outside the repository unless a retained artifact has a named current consumer.

Report partial completion, failed verification, unavailable tools, uncertain outcomes, and
irreproducible observations explicitly.

## Mission

Build `lkjscript` as an agent-native semantic programming system.

Autonomous coding agents are the primary program authors. Humans remain first-class for intent,
governance, security policy, explanation, review, operation, product ownership, and acceptance.

The product objective is:

> A coding agent can create, inspect, change, validate, test, build, run, package, reproduce, and
> review useful applications through compact exact interactions, while program meaning has one
> authoritative typed representation and every accepted change remains deterministic and auditable.

Optimize jointly for correctness, weak-model success, application completeness, compact
interaction, low correction and provider cost, deterministic history, human review, secure
execution, implementation locality, distribution viability, and long-term performance.

Do not optimize for novelty, sunk-cost preservation, continuation of an old roadmap, benchmark
theater, or compatibility with superseded repository states.

## Product Completion

Capability closure outranks feature count.

A retained capability is complete only when applicable parts are:

- authorable through the supported agent surface;
- observable through bounded context and deterministic review;
- accepted by the authoritative validator;
- covered by an independent oracle;
- runnable through a public boundary;
- represented correctly in workspace, application, package, and executable artifacts;
- documented by one current owner;
- exercised by a representative application.

Prefer the smallest vertical that closes a real application lifecycle over isolated semantic forms.

Do not claim application support when execution depends on repository-only fixtures or hidden
workspace state.

Do not claim package support when the artifact is workspace history under another name.

A package, module, test, permission, resource, executable, deployment abstraction, protocol,
binary, and optimization must have a named current consumer.

## Precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. The explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Generated machine descriptions.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. Current structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and
    historical documents.

Newer verified checkout state outranks older plans.

An old prompt is historical evidence unless the active task explicitly selects it.

A campaign prompt owns one campaign's scope, hypotheses, gates, and handoff. It does not become
semantic authority.

Update the owning specification and executable contract in the same verified milestone whenever
accepted behavior changes.

## Fact Ownership

Keep one maintained owner for each active fact:

- `docs/spec/`: accepted observable semantics and public contracts;
- `docs/architecture.md`: components, topology, trust boundaries, and trusted computing base;
- `docs/status.md`: implemented reality and exact absences;
- `docs/performance.md` and structured evidence: reproduced measurements and reversal evidence;
- `docs/roadmap.md`: future evidence gates;
- `README.md`: concise human-first explanation and supported entry points;
- root `AGENTS.md`: durable repository policy;
- `prompts/`: explicitly selected campaign artifacts, never semantic authority;
- executable code and tests: accepted machine behavior.

Do not maintain parallel schema catalogues, status lists, version tables, architecture inventories,
dependency inventories, benchmark tables, application manifests, or memory-model tables.

Generate projections from an executable owner only when generation reduces total duplication
without hiding semantic invariants.

Git history is the archive for superseded prompts, plans, code, fixtures, and campaign narratives.
Delete stale active-tree material with no current consumer.

## Enduring Invariants

These outrank implementation language, topology, storage, layout, transport, syntax, backend,
runtime representation, model provider, and platform.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions or commits are immutable.
3. Every durable namespace has one unambiguous publication authority.
4. Accepted forms belong to closed versioned contracts.
5. Unknown, malformed, ambiguous, foreign-domain, unsupported, or noncanonical forms reject.
6. Arbitrary property bags and arbitrary string-labelled semantic edges are not authority.
7. AI output and natural-language intent are untrusted proposals.
8. Deterministic validators, not model judgment, decide acceptance.
9. Mutation is a typed transaction or a closed proposal normalized into one.
10. One successful publication creates exactly one accepted revision or commit.
11. Rejection and validate-only publish nothing and consume no durable identity.
12. Durable identity exists only for continuity, independent reference, sharing, repair,
    attribution, history, import, export, or external targeting.
13. Representation scaffolding does not receive durable identity merely because it is an item.
14. Durable identity is independent of names, formatting, source positions, proposal spelling,
    hashes, compiler indexes, artifact offsets, storage keys, runtime handles, and addresses.
15. Durable identities are never silently reused within their domain.
16. Revision-local references, aliases, compiler IDs, runtime handles, storage digests, package
    coordinates, and display names remain distinct domains.
17. A content digest is not automatically entity, package, application, provenance, or
    authorization identity.
18. Identity-preserving change requires an explicit validated continuity rule.
19. Derived facts never become a second mutable source of truth.
20. Text, JSON, binary encodings, editable documents, caches, indexes, IR, bytecode, profiles,
    memory plans, and machine code are proposals, views, or derived state.
21. No proposal or view bypasses the authoritative validator.
22. Editable text is allowed when exact, bounded, revision-bound, and normalized through the same
    semantic path.
23. Accepted authority never depends on rendering and reparsing.
24. Compilation consumes one immutable accepted or independently validated distribution state.
25. Only a complete selected-entry dependency closure enters executable lowering.
26. One simple executable route defines behavior; faster tiers remain differential against it.
27. Accepted semantics cannot express unchecked memory access.
28. User-controlled depth does not consume unbounded native stack.
29. Host access requires explicit typed authority or a deliberately narrow pure invocation profile.
30. External resources require explicit outcome and deterministic cleanup semantics.
31. Non-idempotent effects are never silently retried after possible partial action.
32. Observable order is explicit and deterministic.
33. Large work and output are bounded, streamed, paginated, chunked, or policy-controlled.
34. Durable state is acknowledged only after its documented publication contract.
35. Corrupt, partial, ambiguous, unsupported, or unverifiable authority rejects.
36. Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, native
    isolation, and crash consistency are separate contracts.
37. Every public boundary states version, canonical form, limits, rejection, output, and domain
    binding.
38. Compactness never weakens typing, validation, authorization, identity, durability,
    diagnostics, or verification.
39. Performance optimization preserves a correctness oracle.
40. Claims remain no stronger than the checkout and reproduced evidence.
41. Agent interaction, context locality, change locality, verification locality, and provider cost
    are first-class engineering dimensions.
42. Workspace, application, package, executable, and cache domains remain distinct unless a
    verified design combines them without ambiguity.
43. Tests that affect acceptance have exact immutable input, oracle, policy, and result.
44. Reproducible execution names exact content, runtime contract, entry, input, and policy.
45. Backward compatibility is absent unless the active user explicitly requires it.
46. Incompatible-change freedom converges on one active path.
47. No legacy reader, compatibility mode, edition split, dual success path, hidden fallback, or
    silent migration remains after cutover.
48. Current bootstrap absences are not permanent prohibitions without semantic reason.
49. Every retained dependency and abstraction has a current consumer.
50. Complexity without current semantic, product, safety, cost, or performance value is removed.

## Decision Doctrine

Treat every historical decision as provisional except the enduring invariants.

Do not continue a roadmap item because it is next or preserve a mechanism because it was difficult
to build.

Reproduce relevant evidence before reversing a working subsystem.

Prefer application closure over speculative generality, a local feature over a platform for one
consumer, and a high-leverage correction over accumulated exceptions.

Evidence gating prevents speculative overengineering; it must not become permanent minimalism.

Before retaining a substantial choice, record:

- concrete user, agent, application, or maintenance problem;
- semantic, safety, durability, and distribution obligations;
- measured baseline and serious alternatives, including deletion;
- expected benefit and implementation, trust, context, verification, and operational cost;
- correctness oracle, direct-cutover deletion, and reversal condition.

A complexity increase must pay rent in a representative end-to-end workload.

A performance optimization reports absolute and relative benefit plus source, test, build, binary,
context, and failure cost. A large percentage on a tiny workload is insufficient.

Use bounded prototypes for uncertain questions. Delete losing prototypes completely.

## Authority, Identity, and Artifacts

Use **typed semantic program model** before repository-specific acronyms. Do not let `graph`,
`source`, `database`, `daemon`, `package`, `module`, `object`, or `node` predetermine storage,
identity, or product semantics.

A deterministic text form may be a review view, context, proposal, interchange form, test manifest,
or application manifest. It may not allocate identity independently, define behavior separately,
persist a competing mutable AST, make formatting identity, hide editable omissions, or bypass
validation.

Durable identity is opt-in. For each class, define its consumer, continuity, replacement, deletion,
history, branch, import/export, encoding, bounds, and validate-only allocation.

Prefer revision-local structural identity for anonymous terms, implied control, local binders,
compiler blocks, transient queries, and execution frames without continuity consumers.

Names are lookup and presentation metadata unless an accepted external contract says otherwise.

Aliases bind exact authority, content or revision, schema, context digest, target set, and purpose.
They never become identity.

A workspace is development authority with immutable accepted history.

An application artifact is an immutable runnable closure with an exact entry and invocation
contract. It is not automatically a reusable package.

A package artifact is immutable reusable semantic content with explicit exports, dependencies,
identity, provenance, and import rules. It is not workspace history.

An executable artifact is compiler-derived target content bound to exact semantic input, compiler
and backend identity, target, policy, and runtime contract. It is not semantic authority.

A cache is disposable acceleration.

Every artifact decoder treats bytes as untrusted, applies exact bounds, rejects trailing and
noncanonical data, verifies domain-separated integrity, and runs the applicable independent
validator.

## Transactions, Storage, and Topology

Every mutation names an exact base revision or parent and durable namespace.

A successful mutation performs boundary validation, normalization, semantic and history
validation, response and persistence preflight, durable publication, applicable in-memory
publication, and bounded receipt construction.

Rejection changes none of those states. Validate-only performs equivalent deterministic preflight
without publication.

Branches, candidates, concurrency, cancellation, full snapshots, objects, journals, databases,
checkpoints, and indexes are implementation choices that require representative consumers and
complete state machines.

A storage replacement preserves publication atomicity, corruption rejection, bounded recovery,
deterministic conflicts, and an independent reconstruction oracle.

One logical Engine owns semantics regardless of direct, session, service, embedded, or remote
adapter.

Retain an adapter only for a current integration, isolation, latency, test, or deployment consumer.
Do not keep a daemon as architecture insurance.

One writer is acceptable until measured application work proves it insufficient.

## Agent-Facing Product Surface

The coding agent is the primary author. Optimize semantic success, not serialization success.

Normal work should not require implementation source, a global schema dump, compiler plumbing,
daemon lifecycle, storage metadata, or repeated unchanged context.

Provide compact orientation, task-scoped context, exact expansion, deterministic review, exact
proposals, legal local actions, scoped aliases, typed diagnostics, validate/apply parity, compact
receipts and deltas, bounded execution facts, digest reuse, and explicit omissions.

Treat observation, action, review, testing, building, execution, distribution, and history as
separate interface problems.

A low-level typed item API and a complete exact global schema are not automatically agent-friendly.

Caches are disposable, bounded, domain-bound, and never authority. Model-ranked retrieval or
planning never enters correctness.

Measure equal tasks using success, unintended changes, corrections, semantic requests, repeated
discovery, action and observation bytes, provider token classes when exposed, calls, failures,
processes, files opened, elapsed time, build cost, artifact size, and exact monetary cost only when
applicable telemetry and pricing exist.

Bytes are not tokens. Do not infer provider cost.

Reduce development API cost through stable policy, concise current docs, narrow owners, focused
commands, exact reusable context, and deletion of stale campaign material.

## Contracts and Schema

Keep one executable owner for each accepted type, field, variant, operation, query, error, limit,
test form, artifact manifest, and boundary contract.

Derive codecs, schema fragments, help, proposal facts, and documentation from that owner when this
reduces total duplication and remains reviewable.

Do not retain a large global introspection catalogue solely because it exists. Identify its current
consumer and compare command-local help, dependency-closed fragments, executable samples, and
strict codec tests.

A macro, derive, generator, or IDL is acceptable only when authority is explicit, output is
deterministic, accepted shapes remain reviewable, strict decoding and visible semantic invariants
remain, build cost is measured, and stale output fails verification.

Normal agent work receives only the contract fragment it needs.

## Language, Compiler, Runtime, Tests, and Effects

Choose observable semantics before representation or optimization.

A new type, operation, effect, package construct, test, or resource needs a current consumer and
exact contracts for equality, identity, mutability, ordering, conversion, failure, allocation,
lifetime, cleanup, permissions, lowering, queries, artifacts, public values, and tests as applicable.

Do not expose implementation memory choreography to authors merely to simplify the runtime.

Keep value semantics, lifetime, aliasing, representation, reclamation, and external resource cleanup
separate.

A memory plan, uniqueness optimization, custom arena, reference-count protocol, tracing collector,
region system, bytecode, native backend, or JIT must beat a simpler safe implementation enough to
justify code and verification surface. Sunk cost is not evidence.

IR, bytecode, memory plans, profiles, and native images are derived. Private registers, blocks,
layouts, handles, offsets, retain/release actions, and compiler indexes do not escape.

Keep a simple executable oracle. Optimization preserves deterministic results, traps, and resource
semantics unless accepted semantics explicitly change.

Fuel, time, frames, stack, cells, logical bytes, retained bytes, objects, allocations, handles,
external resources, input, and result materialization are distinct policies.

First-class tests require exact ownership or identity, immutable input, expected value or trap,
entry binding, resource policy, selection, order, artifact inclusion, and deterministic results.

Prefer exact invocation cases over a second assertion language. Test execution never mutates
workspace authority, and skipped or exhausted tests do not pass.

Pure computation is a bootstrap baseline, not a permanent prohibition. Prefer a narrow explicit
invocation adapter before general effects when all input and output fit typed values or bytes.

Every host effect requires typed authority and exact order, cancellation, timeout, partial action,
retry, idempotency, audit, crash, cleanup, serialization, and fake-host test behavior.

A process boundary is not a sandbox. Deployment topology is separate from workspace-authority
topology.

## Implementation, Security, and Evidence

Organize code around stable fact ownership and changed-together behavior.

Do not impose arbitrary universal file-size limits, but address pathological context
concentration. Split for real semantic, identity, encoding, transaction, validation, artifact,
storage, query, protocol, agent, compiler, runtime, host, platform, trust, test, compile, or
change-locality boundaries.

Keep tightly coupled invariants together. Avoid forwarding forests, micro-modules, duplicate
helpers, broad preludes, cyclic dependencies, cosmetic crate splits, and hidden monoliths.

A crate boundary must improve dependency direction, binary composition, compile isolation, trust
isolation, or reuse. Measure files and source bytes opened, build time, duplication, binary size,
and full verification cost.

Treat model output, documents, JSON, artifacts, package bytes, executable bytes, filesystem
metadata, public values, host responses, and network input as untrusted.

Reject unknowns, duplicates, trailing data, excessive depth and counts, noncanonical encodings,
foreign domains, and unsupported versions. Check sizes before material allocation and avoid native
recursion over user-scalable structure.

Keep unsafe code absent by default. If a measured native boundary requires it, isolate the minimum
reviewed component behind a safe contract and differential tests.

Every dependency has a current consumer, locked version, and assessed build, licensing, unsafe, and
native surface where applicable.

Hashes provide integrity and cache keys, not authorization, provenance, signatures, or identity by
themselves.

Classify claims as invariant, accepted contract, verified baseline, controlled observation,
benchmark distribution, hypothesis, or historical fact.

Report exact bytes as bytes, provider token classes only from telemetry, and monetary cost only from
applicable exact pricing and telemetry.

Record environment, command, input, sample count, statistic, limitation, and artifact digest with
retained measurements.

Use end-to-end applications for product decisions and microbenchmarks for mechanism diagnosis.

## Compatibility and Cutovers

Backward compatibility is not a default goal.

For an active-boundary change:

1. inventory every reader, writer, validator, descriptor, artifact, test, example, and document;
2. choose an unambiguous new version or identity;
3. replace active readers and writers together;
4. update specifications and executable contracts;
5. delete displaced code, flags, fixtures, aliases, adapters, and claims;
6. reject old forms directly;
7. leave one active success path.

Do not add migration code for regenerable repository fixtures when compatibility is not required.

## Working Method

1. Inspect branch, commit, worktree, and active instructions.
2. Identify the selected campaign prompt.
3. Read this file, `docs/status.md`, and only relevant owners.
4. Reproduce the smallest relevant public baseline.
5. Name the product problem and changed contract.
6. Compare serious alternatives, including deletion.
7. Prototype uncertain high-risk choices outside production.
8. Select one route using evidence and invariants.
9. Implement a dependency-closed vertical.
10. Delete losing paths in the same milestone.
11. Update owning specs, status, architecture, evidence, roadmap, README, examples, and contract.
12. Run focused verification while editing.
13. Run the complete boundary.
14. Inspect status and complete diff.

Resolve ordinary architecture details from repository evidence. Do not stop at a report when a safe
dependency-closed implementation can be completed.

When the full campaign cannot be completed, leave the strongest coherent verified subset, no
half-active architecture, and a precise handoff.

## Verification

The default complete boundary is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every retained production application.

Run affected old-version rejection, strict-decoder, identity, transaction, validate-only,
publication, restart, corruption, context, proposal, test, artifact, compiler, runtime, resource,
cleanup, and result suites.

Run deterministic malformed-input tests with recorded seeds and counts.

Run Miri, sanitizers, fuzzing, model checking, cross-platform builds, and provider trials when
applicable and available. Report exact unavailability otherwise.

Do not weaken tests to make a redesign pass or update expected output before understanding and
owning the semantic change.

## Handoff

Before handoff, confirm the starting commit, final branch and worktree, local and remote changes,
unrelated-work preservation, one active path per domain, current fact ownership, truthful
application and package claims, explicit security and resource boundaries, production-entry
applications, and absence of secrets, transcripts, scratch corpora, losing prototypes, stale
prompts, and compatibility paths.

State decisions and rejected alternatives, semantic and identity changes, application and agent
workflow, test and artifact changes, protocol and storage changes, compiler and runtime changes,
topology changes, deleted paths, dependency and trust effects, exact verification and application
results, interaction and performance measurements, unavailable evidence, risks, reversal
conditions, and the next gate.

Do not claim implementation for design-only work.

Do not push, publish, merge, or release unless the active task authorizes it.
