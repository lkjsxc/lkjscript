# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add procedure for a real ownership boundary. It may not weaken the
repository-wide requirements in this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, benchmark labels, generated descriptions, commit messages, and
handoffs.

## Mission

Build `lkjscript` as an agent-native semantic programming system.

Coding agents are the primary program authors. Humans remain first-class for intent, governance,
security policy, explanation, review, operation, product ownership, and acceptance.

The product objective is:

> A coding agent can create, inspect, change, validate, test, compose, build, run, transfer, and
> review useful applications through compact exact interactions, while accepted program meaning is
> typed, deterministic, independently checkable, and auditable.

Optimize jointly for:

- semantic correctness;
- weak-model success;
- application usefulness;
- compact and local interaction;
- low correction depth and provider cost;
- deterministic history and artifacts;
- human reviewability;
- secure execution and explicit authority;
- implementation locality;
- distribution and composition;
- reproducibility;
- and long-term performance.

Do not optimize for novelty, sunk-cost preservation, compatibility with superseded repository
states, benchmark theater, or continuation of an old roadmap.

## Authority and Precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Generated descriptions derived from executable owners.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. Current structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and
    historical documents.

Newer verified checkout state outranks older plans.

An old prompt is historical evidence unless the active task explicitly selects it.

A campaign prompt owns one campaign's objective, hypotheses, gates, and handoff. It does not become
permanent semantic authority.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

A session that began before this file changed must not assume the new policy is loaded. Restart the
agent session or explicitly verify that the effective instructions contain the current file.

## Authorization and Worktree Safety

Inspect the actual checkout before editing:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
```

Preserve unrelated work.

For an implementation task, reading in-scope files, editing in-scope files, and running
non-destructive validation are authorized unless the task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state unless the active task authorizes that class
of action.

Repository permissions are not authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data,
raw provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
unclear licensing.

Keep scratch state, temporary workspaces, destructive experiments, unsanitized measurements, and
downloaded research outside the repository unless a retained artifact has a named current consumer.

Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
irreproducible observations explicitly.

## Enduring Product Invariants

The following requirements outrank implementation language, process topology, storage engine, file
layout, transport, syntax, backend, runtime representation, model provider, and platform.

### Semantic authority

- Accepted program meaning has one authoritative typed semantic representation per immutable
  authority unit.
- Natural-language intent, model output, editable text, JSON, binary encodings, context packets,
  caches, indexes, IR, bytecode, profiles, memory plans, and machine code are proposals, views, or
  derived state unless a specification explicitly defines one as an immutable distribution
  authority.
- No proposal, view, cache, or derived format bypasses deterministic validation.
- Accepted authority never depends on rendering and reparsing.
- Unknown, malformed, ambiguous, unsupported, foreign-domain, noncanonical, truncated, oversized,
  or trailing forms reject.
- Derived facts never become a second mutable source of truth.

### Publication and history

- Published revisions and releases are immutable.
- Every durable namespace has one unambiguous publication authority.
- One successful publication creates exactly one accepted revision or release.
- Rejection and validate-only publish nothing and consume no durable identity.
- Durable success is acknowledged only after the documented atomicity and synchronization contract.
- A publication outcome that may be ambiguous is reported as unknown; it is never guessed or
  silently retried.
- History rules are explicit, deterministic, and independently validated.

### Identity

- Identity is assigned only for a concrete continuity, sharing, reference, repair, attribution,
  import, export, provenance, history, or external-targeting consumer.
- Representation scaffolding does not receive durable identity merely because it is represented.
- Names, source positions, formatting, proposal spelling, order, paths, hashes, compiler indexes,
  artifact offsets, storage keys, runtime handles, and addresses are not semantic identity unless a
  closed contract deliberately assigns one of them a narrower identity role.
- Workspace identity, release identity, package coordinate, content digest, declaration identity,
  revision-local reference, alias, compiler ID, runtime handle, and display name are separate
  domains.
- A content digest is never *implicitly* entity, release, provenance, authorization, or signature
  identity. A specification may deliberately make a domain-separated digest the exact identity of
  immutable content after collision assumptions, equality, and failure behavior are stated.
- Identity-preserving change requires an explicit validated continuity rule.
- Deleted durable identities are never silently reused within their domain.
- Multiple exact versions may coexist only when references remain unambiguous.

### Composition and artifacts

- Workspace, reusable semantic release, application, executable, deployment, and cache are distinct
  domains unless a verified design combines them without ambiguity.
- A workspace is development authority and history.
- A reusable release has explicit exports, exact dependencies, identity, a provenance contract or
  explicit provenance absence, import rules, and canonical bytes.
- An application is an exact runnable closure with an entry, invocation contract, resource policy,
  and release evidence.
- An executable is derived target content bound to exact semantic input, compiler/backend identity,
  target, policy, and runtime contract.
- A cache is disposable acceleration.
- A package name or version is not an exact dependency unless a resolver has produced an immutable
  bound result.
- Compilation consumes one immutable accepted workspace revision or independently validated
  distribution state.
- Only a complete selected-entry dependency closure enters executable lowering.

### Execution and effects

- One simple executable route remains the correctness oracle.
- Faster tiers remain differential against that oracle.
- Accepted semantics cannot express unchecked memory access.
- User-controlled depth does not consume unbounded native stack.
- Observable order is explicit and deterministic.
- Host access requires explicit typed authority or a deliberately narrow pure invocation adapter.
- External resources have explicit acquire, use, consume, close, cancellation, timeout,
  partial-action, retry, idempotency, audit, crash, and cleanup semantics.
- Non-idempotent effects are never silently retried after possible partial action.
- A process boundary is not a sandbox.
- Deployment topology is separate from semantic-authority topology.
- Ordinary immutable-value reclamation is separate from affine external-resource semantics.

### Bounds and security

- Potentially large input, work, state, and output are bounded, streamed, paginated, chunked, or
  policy-controlled.
- Lengths and counts are checked before corresponding allocation.
- Public boundaries state version, canonical form, limits, domain binding, rejection behavior,
  output policy, and failure classes.
- Decoders treat bytes and filesystem metadata as untrusted.
- Corrupt or unverifiable authority rejects.
- Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, native
  isolation, path safety, and crash consistency are separate contracts.
- Compactness never weakens typing, validation, authorization, identity, durability, diagnostics,
  or verification.

### Compatibility

- Backward compatibility is absent unless the active user explicitly requires it.
- Incompatible-change freedom is used to converge on one coherent active path.
- After a cutover, delete old readers, writers, aliases, fallback modes, compatibility tests,
  migration-only code, stale examples, and superseded documentation.
- Do not introduce editions, dual success paths, hidden fallback, or silent migration as
  architecture insurance.

## Decision Doctrine

Treat every historical decision as provisional except the enduring invariants.

Do not preserve a mechanism because it was difficult to build or because it appears next in a
roadmap.

Reproduce relevant evidence before reversing a working subsystem.

Prefer:

- a complete application vertical over isolated feature count;
- a local feature over a platform built for one consumer;
- one exact path over parallel convenience paths;
- an explicit domain model over overloaded names;
- direct deletion over permanent deprecation scaffolding;
- a bounded prototype over speculative architecture;
- and a high-leverage correction over accumulated exceptions.

A retained abstraction, dependency, process, artifact, identity class, schema catalogue, cache,
index, optimization, test language, package layer, or service must have a named current consumer.

A complexity increase must pay rent in a representative end-to-end workload.

Before retaining a substantial choice, record:

- the concrete user, agent, application, or maintenance problem;
- semantic, safety, durability, composition, and distribution obligations;
- the measured baseline;
- serious alternatives, including deletion;
- expected benefit;
- implementation, trust, context, verification, build, and operational cost;
- the correctness oracle;
- the direct-cutover deletion plan;
- and the reversal condition.

Use bounded prototypes for uncertain questions. Delete losing prototypes completely.

Do not use evidence gates as permanent minimalism. Current absences are not permanent prohibitions
without semantic reason.

Do not use incompatible-change freedom as permission for an unverified rewrite.

## Agent-Native Product Surface

Optimize semantic success, not serialization success.

Normal authoring should not require implementation source, a global schema dump, compiler plumbing,
storage metadata, daemon lifecycle, or repeated unchanged context.

Provide, where justified:

- compact orientation;
- task-scoped context;
- exact expansion on demand;
- deterministic review;
- closed typed proposals;
- legal local actions;
- scoped aliases;
- typed diagnostics with stable codes and targets;
- validate/apply parity;
- compact receipts and deltas;
- bounded execution facts;
- immutable interface summaries;
- digest reuse;
- and explicit omissions.

Observation, action, review, validation, testing, building, execution, composition, distribution, and
history are separate interface problems. Do not force one oversized protocol shape across all of
them.

A low-level item API, a complete global schema, or a source-like grammar is not automatically
agent-friendly. Compare equal tasks.

Prefer a small stable instruction prefix and command-local exact facts. Do not repeat generic
guidance in every prompt, help surface, or response.

Add prompt rules, examples, tools, schema projections, and context only when they fix a measured
failure mode.

After each tool or command result, ask whether the task can now proceed from exact evidence. Avoid
discovery loops that do not change the next decision.

When independent reads are needed, batch them. When one result determines the next action, keep the
work sequential.

Machine output intended for agents is canonical, bounded, and easy to reduce programmatically.

Caches are disposable, bounded, version-bound, and never authority. Model-ranked retrieval or model
judgment never enters correctness.

Measure equal tasks using:

- task success;
- unintended semantic changes;
- correction count and depth;
- semantic requests;
- repeated discovery;
- action bytes;
- observation bytes;
- provider input, cached input, output, and reasoning tokens when actually exposed;
- provider calls;
- exact monetary cost only when telemetry and pricing are known;
- processes;
- engine opens;
- files and source bytes opened;
- elapsed time;
- build cost;
- artifact size;
- and failure quality.

Bytes are not tokens. Never infer provider cost from bytes.

## Contracts and Fact Ownership

Keep one maintained owner for each active fact.

- `docs/spec/` owns accepted observable semantics and public contracts.
- `docs/architecture.md` owns components, dependency direction, topology, trust boundaries, and the
  trusted computing base.
- `docs/status.md` owns implemented reality and exact absences.
- `docs/performance.md` and structured evidence own reproduced measurements and reversal evidence.
- `docs/roadmap.md` owns future evidence gates.
- `README.md` owns the concise human-first product explanation and supported entry points.
- Root `AGENTS.md` owns durable repository policy.
- `prompts/` contains explicitly selected campaign artifacts, never semantic authority.
- Executable code and tests own accepted machine behavior.

Do not maintain parallel schema catalogues, version tables, status lists, architecture inventories,
dependency inventories, benchmark tables, artifact manifests, or memory-model tables.

Keep one executable owner for every accepted type, field, variant, operation, query, error, limit,
test form, artifact manifest, and boundary contract.

Derive codecs, strict parsers, schema fragments, command help, examples, and documentation from that
owner when derivation reduces total duplication and keeps invariants visible.

A macro, derive, generator, or IDL is acceptable only when:

- authority is explicit;
- output is deterministic;
- accepted shapes remain reviewable;
- duplicate and unknown input still reject;
- cross-field semantic validation remains explicit;
- build and debugging cost are measured;
- generated output cannot silently become stale;
- and the displaced manual owner is deleted.

Git history is the archive for superseded prompts, plans, code, fixtures, and campaign narratives.
Delete stale active-tree material without a current consumer.

## Language, Compiler, Runtime, and Memory

Choose observable semantics before representation or optimization.

A new type, operation, effect, package construct, test, resource, or executable form needs a current
consumer and exact contracts for the applicable dimensions:

- equality;
- identity;
- mutability;
- ordering;
- conversion;
- failure;
- allocation;
- lifetime;
- cleanup;
- permissions;
- lowering;
- artifacts;
- public values;
- queries;
- authoring;
- and tests.

Do not expose implementation memory choreography to program authors merely to simplify the runtime.

Keep value semantics, lifetime, aliasing, representation, reclamation, and external-resource cleanup
separate.

A custom ownership plan, arena, reference-count protocol, tracing collector, region system, bytecode,
native backend, or JIT must beat a simpler safe implementation enough to justify its code and
verification surface.

A tracing collector is not the default. Introduce one only for a representative cyclic or mutable
workload when it reduces total semantic and implementation complexity while preserving deterministic
resource policy and a simpler oracle.

IR, bytecode, memory plans, profiles, and native images are derived. Private registers, blocks,
layouts, handles, offsets, retain/release actions, and compiler indexes do not escape as semantic
authority.

Fuel, time, frames, native stack, cells, logical bytes, visible bytes, retained bytes, objects,
allocations, handles, external resources, input, and result materialization are distinct policies.

Optimization preserves deterministic results, traps, ordering, cleanup, and resource semantics
unless the accepted specification deliberately changes one.

## Testing and Verification

Tests that affect acceptance have exact immutable input, oracle, policy, selection, order, and
result.

Prefer exact invocation cases over a second assertion language until a real workload proves them
insufficient.

Test execution never mutates semantic authority. Skipped, exhausted, cancelled, or unavailable tests
do not pass.

For every changed public boundary, cover applicable cases:

- canonical success;
- validate-only parity;
- deterministic repeated success;
- unknown fields or tags;
- duplicate fields or entries;
- wrong version or domain;
- stale base or digest;
- malformed values;
- truncation;
- trailing data;
- oversized lengths and counts;
- allocation-boundary behavior;
- foreign references;
- identity confusion;
- corruption;
- interrupted publication;
- cleanup after success and failure;
- and differential behavior against the oracle.

After implementation, run the narrowest useful checks first, then the full applicable repository
checks.

The default full checks are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run retained public-path examples affected by the change.

Use Miri, sanitizers, deterministic mutation, property testing, fuzzing, model checking, or
cross-platform execution when they target a real risk and are available. State the exact scope and
tool limitations.

Do not weaken a failing invariant test to make an implementation pass. Change the specification,
implementation, and oracle together when behavior is deliberately changed.

## Implementation Locality

Organize code around stable fact ownership and changed-together behavior.

Do not impose arbitrary universal file-size, line-count, directory-entry, semantic-node, or document
limits. Add a limit only for a semantic, security, resource, transport, or tooling reason and test
its boundary.

Large files are not automatically wrong. Split when it improves ownership, review locality, build
locality, or agent context without duplicating invariants.

Keep production logic out of test-only helpers and keep independent oracles independent from the
implementation under test.

Prefer standard library and existing dependencies when they are adequate. Add a dependency only
when its current value exceeds supply-chain, build, binary, audit, and maintenance cost.

No local unsafe Rust is permitted under the current crate policy. A task that proposes unsafe code
must first establish a concrete need, a safe public contract, isolated ownership, independent tests,
and explicit user authorization to replace that policy.

Do not hide errors with broad fallbacks, lossy conversion, panics, unchecked indexing, or
best-effort acceptance.

## Change Workflow

For substantial work:

1. Inspect the checkout, active instructions, current branch, and unrelated changes.
2. Identify the authoritative owners of the facts being changed.
3. Reproduce the relevant baseline.
4. State the user-visible outcome, completion bar, non-goals, alternatives, and reversal condition.
5. Build the smallest dependency-closed prototype that can decide the uncertain question.
6. Select one coherent design from evidence.
7. Implement the complete vertical through authoring, validation, persistence, artifacts, runtime,
   tests, documentation, and examples as applicable.
8. Cut over directly.
9. Delete the losing paths and stale facts.
10. Run focused checks, full applicable checks, and representative public workflows.
11. Record current evidence without overstating it.
12. Leave a compact handoff.

Do not stop at a report when a safe dependency-closed implementation is authorized and feasible.

Do not scatter partial architecture across unrelated domains. A smaller complete milestone is
better than many half-introduced abstractions.

Do not ask the user to decide ordinary engineering details that the checkout and evidence can
resolve. Ask only when a missing user value materially changes product intent, authorization, or
acceptance.

## Completion and Handoff

A capability is complete only when the applicable parts are:

- authorable through the supported agent surface;
- observable through bounded context and deterministic review;
- accepted by the authoritative validator;
- represented correctly in every relevant immutable artifact;
- covered by an independent oracle;
- runnable through a public boundary;
- corruption- and limit-tested;
- documented by one current owner;
- exercised by a representative application;
- and free of superseded active paths.

Before finishing, report:

- the exact starting and ending commit or worktree state;
- the selected design and rejected serious alternatives;
- files and public contracts changed;
- direct-cutover deletions;
- validation commands and results;
- representative workload evidence;
- known limits;
- remaining reversal gates;
- and any action not performed.

Claims must be no stronger than the checkout and reproduced evidence.
