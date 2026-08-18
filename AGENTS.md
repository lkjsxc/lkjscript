# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add procedures for a genuine ownership boundary. It may not weaken this
file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, and handoffs.

## Mission

Build `lkjscript` as an agent-native semantic programming system for useful applications.

Coding agents are the primary program authors and maintainers. Humans remain first-class for intent,
governance, security policy, explanation, review, operations, product ownership, and acceptance.

The enduring objective is:

> A coding agent can create, inspect, change, validate, test, compose, build, run, transfer,
> operate, and review useful applications through compact exact interactions, while accepted
> program meaning, durable state, authority, and externally visible outcomes remain typed,
> deterministic where promised, independently checkable, bounded, and auditable.

Optimize jointly for semantic correctness, useful applications, weak-model success, low
correction and provider cost, compact local context, deterministic artifacts and history, explicit
authority, secure and recoverable operation, reproducibility, composition, maintainability, and
long-term performance.

Do not optimize for novelty, feature count, benchmark theater, roadmap inertia, sunk-cost
preservation, or compatibility with superseded repository states.

## Authority and precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Generated descriptions derived from an executable owner.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. Current structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and
    historical documents.

Newer verified checkout state outranks older plans and remembered repository state.

A campaign prompt owns one campaign's objective, hypotheses, gates, and handoff. It does not become
permanent semantic authority.

An old prompt is historical evidence unless the active task explicitly selects it.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

A session that began before this file changed must not assume that the new policy is loaded. Restart
the session or verify the effective instructions explicitly.

## Authorization and repository safety

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -1 --oneline
```

Read every applicable `AGENTS.md` before changing its scope.

Preserve unrelated work.

For an implementation task, reading in-scope files, editing in-scope files, and running
non-destructive validation are authorized unless the active task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state unless the active task authorizes that action.

Repository permissions are not user authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, raw
provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
unclear licensing.

Keep scratch state, temporary workspaces, destructive experiments, unsanitized measurements, and
downloaded research outside the repository unless a retained artifact has a named current consumer.

Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
irreproducible observations explicitly.

## Enduring product contracts

These contracts outrank implementation language, process topology, storage, file layout, syntax,
transport, backend, runtime representation, model provider, and platform.

### Semantic authority

- Each accepted authority unit has one authoritative typed semantic representation.
- Natural-language intent and model output are untrusted proposals.
- Text, JSON, context, review, caches, indexes, IR, bytecode, profiles, memory plans, machine code,
  and rendering are proposals, views, or derived state unless a specification deliberately defines
  an immutable distribution authority.
- No proposal, view, cache, generated form, or derived form bypasses deterministic validation.
- Accepted authority never depends on rendering and reparsing.
- Unknown, malformed, ambiguous, unsupported, foreign-domain, noncanonical, truncated, oversized,
  duplicate, conflicting, or trailing forms reject.
- Derived facts never become a second mutable source of truth.

### Identity and continuity

- Assign identity only for a concrete continuity, sharing, reference, repair, attribution, import,
  export, history, provenance, external targeting, or durable-instance consumer.
- Names, formatting, source positions, order, paths, hashes, compiler indexes, artifact offsets,
  storage keys, runtime handles, and addresses are not semantic identity unless a closed contract
  gives one a narrower role.
- Workspace, release, application, instance, state revision, digest, declaration, local reference,
  alias, compiler ID, runtime handle, capability, and display name are distinct domains.
- A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
  capability identity.
- Identity-preserving change requires an explicit validated continuity rule.
- Deleted durable identities are not silently reused.
- Multiple exact versions may coexist only when references remain unambiguous.

### Publication and durable history

- Published workspace revisions, releases, applications, and durable-instance revisions are
  immutable within their documented domains.
- Every durable namespace has one unambiguous publication authority.
- One successful publication creates exactly one accepted durable outcome.
- Rejection and validate-only publish nothing and consume no durable identity.
- Success is acknowledged only after the documented atomicity and synchronization boundary.
- A possibly visible but unconfirmed outcome is reported as unknown; it is never guessed or
  silently retried.
- Recovery, replay, retention, compaction, corruption, and deletion behavior are explicit and
  independently validated.
- External outcomes and semantic state publication remain separate unless one atomic contract is
  proved.

### Composition and artifacts

- Workspace, reusable release, application, durable instance, deployment, executable, capability
  grant, provenance, and cache are separate domains unless a verified design combines them without
  ambiguity.
- Workspaces own development history; releases own exact reusable semantics; applications own exact
  runnable closure; instances own explicit durable state/history; executables are derived; caches
  are disposable acceleration.
- Releases have exact exports, dependencies, canonical bytes, identity, and explicit provenance or
  explicit provenance absence.
- Applications have exact entries, policies, tests, and distribution closure.
- Compilation consumes one immutable accepted workspace revision or independently validated
  distribution state and lowers only the complete selected-entry closure.
- Human coordinates, versions, aliases, and paths are not exact dependencies.

### Execution, state, effects, and security

- One simple executable route remains the correctness oracle; faster tiers are differential.
- Accepted semantics cannot express unchecked memory access, and user-controlled depth does not
  consume unbounded native stack.
- Observable order is explicit, and pure deterministic computation remains the default.
- Durable application state is explicit typed authority, not hidden runtime memory.
- Ambient host authority is forbidden. Host access uses an explicit typed capability, closed
  command/result boundary, or deliberately narrow pure adapter.
- External resources define acquire/use/consume/close, cancellation, timeout, partial action, retry,
  idempotency, audit, crash, and cleanup semantics.
- Non-idempotent actions are never silently retried after possible partial execution.
- Time, randomness, scheduling, concurrency, and host observations are explicit inputs or
  capabilities when observable.
- A process boundary is not a sandbox; deployment topology is separate from semantic authority.
- Immutable-value reclamation is separate from external-resource cleanup.
- Potentially large input, work, state, history, graph, output, and diagnostics are bounded,
  streamed, paginated, chunked, or policy-controlled.
- Lengths and counts are checked before corresponding allocation or work.
- Public boundaries state version, canonical form, domain binding, limits, rejection, output, and
  failure classes.
- Decoders treat bytes and filesystem metadata as hostile input.
- Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, isolation,
  path safety, crash consistency, and supply-chain trust are separate contracts.
- Compactness never weakens typing, validation, authorization, identity, durability, diagnostics,
  or verification.

### Compatibility

- Backward compatibility is absent unless the active user explicitly requires it.
- Use incompatible-change freedom to converge on one coherent active path.
- After cutover, delete old readers, writers, aliases, fallbacks, compatibility tests,
  migration-only code, stale examples, and superseded documentation.
- Do not introduce editions, dual success paths, hidden fallback, or silent migration as
  architecture insurance.
- Incompatible-change freedom is not permission for an unverified rewrite.

## Decision standard

Treat every historical mechanism as provisional except the enduring contracts above.

Do not preserve a mechanism because it was difficult to build, appears next in a roadmap, or has a
large test suite.

Reproduce relevant evidence before reversing a working subsystem.

Prefer:

- a complete useful application vertical over isolated feature count;
- one exact path over parallel convenience paths;
- explicit domains over overloaded names;
- a local feature over a platform built for one consumer;
- direct deletion over permanent deprecation scaffolding;
- a bounded prototype over speculative architecture;
- and a high-leverage correction over accumulated exceptions.

A retained abstraction, dependency, process, artifact, identity class, schema catalogue, cache,
index, optimization, grammar, package layer, service, or test framework must have a named current
consumer.

A complexity increase must pay rent in a representative end-to-end workload.

Before retaining a substantial choice, record the concrete consumer and problem, applicable
semantic/safety/durability obligations, reproduced baseline, serious alternatives including
deletion, expected benefit, implementation and operational costs, correctness oracle, cutover
deletions, and reversal condition.

Use bounded prototypes for uncertain questions. Delete losing prototypes completely.

Current absences are not permanent prohibitions without semantic reason.

## Agent and context economy

Context budget is a correctness, latency, and cost constraint.

Keep the root `AGENTS.md` at or below 24 KiB unless reproduced evidence proves that a larger durable
prefix improves complete-task success. Keep deeper instruction files materially smaller.

State each durable instruction once.

Keep stable instructions stable and early. Put volatile campaign facts in the selected prompt,
current status, or structured evidence rather than repeatedly rewriting the durable prefix.

Normal work should not require implementation-wide source reads, a global schema dump, repeated
unchanged context, daemon lifecycle, or historical prompts.

Prefer:

- compact orientation;
- task-scoped exact context;
- exact expansion on demand;
- bounded deterministic review;
- closed typed proposals;
- legal local actions;
- stable diagnostic codes and targets;
- validate/apply parity;
- compact receipts and deltas;
- immutable interface summaries;
- digest reuse;
- and explicit omissions.

After exact owners are known, stop broad discovery unless new evidence invalidates the map.

Batch independent reads and checks. Keep dependent decisions sequential.

Do not repeatedly reread a long campaign prompt. Build a compact task ledger from exact repository
facts, then revisit only relevant sections.

Expose or invoke only tools relevant to the current decision.

Add prompt rules, examples, tools, schema projections, and context only when they fix a measured
failure mode.

Machine output intended for agents is canonical, bounded, stable, and easy to reduce
programmatically.

Caches are disposable, bounded, version-bound, and never authority. Model ranking or model judgment
never enters correctness.

Measure equal tasks using semantic success, unintended changes, correction count and depth,
repeated discovery, action/observation bytes, provider token classes and calls when exposed, exact
cost only when telemetry and pricing are known, processes, engine opens, files/source bytes opened,
elapsed time, build/artifact cost, and failure quality.

Bytes are not tokens. Never infer provider cost from bytes.

## Fact ownership

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

## Language, compiler, runtime, and storage

Choose observable semantics before representation or optimization.

A new type, operation, effect, state form, package construct, test, capability, resource, or
executable form needs a current consumer and exact contracts for applicable identity/equality,
ordering/mutability, conversion/failure, memory/lifetime/cleanup, authority, lowering/artifacts,
public values, authoring/review, persistence/recovery, replay/concurrency, and tests.

Do not expose implementation memory choreography to program authors merely to simplify the runtime.

Keep value semantics, lifetime, aliasing, representation, reclamation, durable state, and external
resource cleanup separate.

A custom ownership plan, arena, reference-count protocol, tracing collector, region system,
bytecode, native backend, JIT, journal, object store, resolver, or scheduler must beat a simpler safe
implementation enough to justify its code and verification surface.

A tracing collector is not the default. Introduce one only for a representative cyclic or mutable
workload when it reduces total semantic and implementation complexity while preserving a simpler
oracle.

IR, bytecode, memory plans, profiles, indexes, journals, and native images are derived unless a
specification deliberately defines an independently validated authority.

Fuel, time, frames, native stack, cells, logical bytes, visible bytes, retained bytes, objects,
allocations, handles, external resources, state size, history size, input, and result materialization
are distinct policies.

Optimization preserves deterministic results, traps, observable ordering, state transitions,
cleanup, and resource semantics unless the accepted specification deliberately changes one.

Organize code around stable fact ownership and changed-together behavior.

Do not impose arbitrary universal file-size, line-count, directory-entry, semantic-node, or document
limits. Add a limit only for a semantic, security, resource, transport, or tooling reason and test
its boundary.

Large files are not automatically wrong. Split when ownership, review locality, build locality, or
agent context improves without duplicating invariants.

Prefer the standard library and existing dependencies when adequate. Add a dependency only when its
current value exceeds supply-chain, build, binary, audit, and maintenance cost.

No local unsafe Rust is permitted under the current crate policy. Replacing that policy requires a
concrete need, a safe public contract, isolated ownership, independent tests, and explicit user
authorization.

Do not hide errors with broad fallbacks, lossy conversion, panics, unchecked indexing, or
best-effort acceptance.

## Testing and verification

Tests that affect acceptance have exact immutable input, oracle, policy, selection, order, and
result.

Test execution never mutates semantic authority unless mutation is the explicit subject of the test.

Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.

For every changed public boundary, cover canonical and repeated success, validate-only parity,
unknown/duplicate/wrong-domain input, stale bases, malformed/truncated/trailing input, exact and
one-over limits, foreign identity, corruption/restart, interrupted or ambiguous publication,
cleanup, concurrency, replay/idempotency, and differential behavior against the oracle.

Run the narrowest useful checks first, then the full applicable repository checks.

The default full checks are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every retained public workflow affected by the change.

Use Miri, sanitizers, deterministic mutation, property testing, fuzzing, model checking,
crash-fault injection, or cross-platform execution when they target a real risk and are available.
State the exact scope and tool limitations.

Do not weaken a failing invariant test to make an implementation pass. Change the specification,
implementation, and oracle together when behavior is deliberately changed.

## Change workflow

For substantial work:

1. Inspect the checkout, active instructions, branch, commit, and unrelated changes.
2. Identify authoritative owners of every fact being changed.
3. Reproduce the relevant baseline and representative workflow.
4. State the user-visible outcome, completion bar, non-goals, serious alternatives, and reversal
   condition.
5. Build the smallest dependency-closed prototype that can decide each uncertain question.
6. Select one coherent design from evidence.
7. Implement the complete vertical through authoring, validation, persistence, artifacts, runtime,
   tests, documentation, examples, and operations as applicable.
8. Cut over directly.
9. Delete losing paths and stale facts.
10. Run focused checks, full applicable checks, and representative public workflows.
11. Record current evidence without overstating it.
12. Leave a compact handoff.

Do not stop at a report when a safe dependency-closed implementation is authorized and feasible.

Do not scatter partial architecture across unrelated domains. A smaller complete milestone is better
than many half-introduced abstractions.

Do not ask the user to decide ordinary engineering details that the checkout and evidence can
resolve. Ask only when a missing user value materially changes product intent, authorization, or
acceptance.

## Completion and handoff

A capability is complete only when it is authorable through the supported agent surface,
observable through bounded exact context and review, accepted by the authoritative validator,
represented in every relevant authority, covered by an independent oracle, runnable publicly,
restart/corruption/limit/failure tested, documented by one current owner, exercised by a useful
application, and free of superseded active paths.

Before finishing, report:

- the exact starting and ending commit or worktree state;
- the selected design and rejected serious alternatives;
- files and public contracts changed;
- direct-cutover deletions;
- validation commands and results;
- representative workload evidence;
- known limits and trust assumptions;
- remaining reversal gates;
- and every requested or implied action not performed.

Claims must be no stronger than the checkout and reproduced evidence.
