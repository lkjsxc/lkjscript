# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add procedures for a genuine ownership boundary.
It may not weaken this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, and handoffs.

## Mission

Build `lkjscript` as an agent-native semantic application platform.

Coding agents are the primary program authors and maintainers.
Humans remain first-class for intent, governance, security policy, explanation, review,
operations, product ownership, and acceptance.

The enduring objective is:

> A coding agent can create, inspect, change, validate, test, compose, build, run, transfer,
> operate, diagnose, and evolve useful applications through compact exact interactions, while
> accepted meaning, durable state, authority, externally visible outcomes, resource use, and
> execution optimizations remain typed, bounded, auditable, and independently checkable.

Optimize jointly for correctness, useful complete applications, weak-model success, low correction
and provider cost, compact context, deterministic artifacts and history, explicit authority,
recoverable operation, maintainability, predictable resources, and long-term performance.

Do not optimize for novelty, feature count, benchmark theater, roadmap inertia, sunk cost, or
compatibility with superseded repository states.

## Authority and precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Generated descriptions derived from one executable owner.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. Current structured evidence and `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, discussions, and
    historical documents.

Newer verified checkout state outranks older plans and remembered repository state.

A campaign prompt owns one campaign's objective, hypotheses, gates, and handoff.
It does not become permanent semantic authority.
An old prompt is historical evidence unless the active task explicitly selects it.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

A session that began before this file changed must verify the effective instructions or restart.

## Authorization and repository safety

Before editing, inspect:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -1 --oneline
```

Read every applicable `AGENTS.md`.
Preserve unrelated work.

Reading in-scope files, editing in-scope files, and running non-destructive validation are
authorized for implementation tasks unless the active task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state without authorization for that action.

Repository permissions are not user authorization.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, raw
provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
unclear licensing.

Keep scratch state, destructive experiments, unsanitized measurements, downloaded research, and
losing prototypes outside the repository unless a retained artifact has a named consumer.

Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
irreproducible observations explicitly.

## Enduring system contracts

These contracts outrank implementation language, process topology, storage, transport, syntax,
backend, runtime representation, model provider, and platform.

### Semantic authority

- Each accepted authority unit has one authoritative typed semantic representation.
- Natural-language intent and model output are untrusted proposals.
- Text, JSON, documents, context, reviews, caches, indexes, IR, bytecode, profiles, memory plans,
  machine code, and renderings are proposals, views, or derived state unless a specification
  deliberately defines immutable distribution authority.
- No proposal, view, cache, generated form, or derived form bypasses deterministic validation.
- Accepted authority never depends on rendering and reparsing.
- Unknown, malformed, ambiguous, unsupported, foreign-domain, noncanonical, truncated, oversized,
  duplicate, conflicting, or trailing forms reject.
- Derived facts never become a second mutable source of truth.

### Identity and continuity

- Assign identity only for a concrete continuity, sharing, reference, repair, attribution, import,
  export, history, provenance, targeting, durable instance, or operational consumer.
- Names, formatting, positions, order, paths, hashes, compiler indexes, artifact offsets, storage
  keys, runtime handles, queue positions, process IDs, and addresses are not semantic identity
  unless a closed contract assigns a narrower role.
- Workspace, release, application, instance, revision, command, outcome, grant, interface, adapter,
  deployment, executable, cache entry, profile, and runtime handle are distinct domains.
- A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
  capability identity.
- Identity-preserving change requires an explicit validated rule.
- Deleted durable identities are not silently reused.
- Multiple exact versions may coexist only when references remain unambiguous.

### Publication and durable history

- Published workspace revisions, releases, applications, instance revisions, host outcomes, and
  other declared durable objects are immutable within their domains.
- Every durable namespace has one publication authority.
- One successful publication creates exactly one accepted durable outcome.
- Rejection and validate-only publish nothing and consume no durable identity.
- Success is acknowledged only after the documented synchronization boundary.
- A possibly visible but unconfirmed outcome is reported as unknown and never silently retried.
- Recovery, replay, retention, compaction, corruption, and deletion are explicit and validated.
- Semantic state publication and externally visible host work remain separate unless atomicity is
  proved.

### Composition and artifacts

- Workspace, release, application, instance, interface, grant, deployment, executable, profile,
  provenance, and cache are separate domains unless a verified design combines them unambiguously.
- Workspaces own development history.
- Releases own exact reusable semantics.
- Applications own exact runnable closure and declared interfaces.
- Instances own durable state and transition history.
- Grants own authority.
- Deployments own operational placement.
- Executables, compiled units, indexes, and profiles are derived.
- Caches are disposable acceleration.
- Compilation consumes one immutable accepted revision or independently validated distribution
  state and lowers only the complete selected closure.
- Coordinates, versions, aliases, paths, and mutable lookup results are not exact dependencies.

## Stable terminology

Use precise terms:

- `semantic engine`: workspace validation, publication, query, and pure execution authority.
- `execution engine`: derived lowering and execution, including interpreter or compiled tiers.
- `runtime kernel`: topology-neutral application loading, instance transition, resource admission,
  adapter dispatch, and derived-cache orchestration.
- `runtime supervisor`: optional long-lived topology around the runtime kernel.
- `host interface`: closed typed request/outcome contract declared by an application.
- `host adapter`: trusted implementation of one host interface.
- `grant`: exact authority binding an instance and interface to an adapter descriptor.
- `instance store`: durable instance authority.
- `scheduler`: operational admission and ordering, not hidden semantic nondeterminism.
- `deployment`: process, machine, account, namespace, and resource placement.
- `cache` and `profile`: reconstructible derived acceleration and observation.

Do not overload `runtime`, `host`, `engine`, or `service` across these domains.

A process boundary creates neither semantic authority nor a sandbox.
A long-lived process is an operational choice, not a product invariant.

## Application interfaces and host authority

Pure deterministic computation remains the default.
Ambient host authority is forbidden.

Applications may declare exact host-interface requirements but never grants.
Instances bind requirements to exact grants.

A grant binds the exact sharing domain, interface identity, adapter kind, bounded descriptor,
applicable authority revision, and limits needed to prevent implicit broadening.

Host requests and outcomes are closed typed values.
Opaque bytes are acceptable only when the interface deliberately defines and bounds them.

An adapter cannot invent semantic state, application response, command intent, or authority.

At most one pending command may remain the bootstrap model.
Parallel commands require a current application plus exact ordering, cancellation, partial-result,
retry, and replay contracts.

A live resource handle, stream, socket, file descriptor, process, timer, secret, or foreign object
requires explicit acquire, use, transfer, consume, close, cancellation, timeout, crash, and cleanup
semantics.
Do not add live resources merely for adapter convenience.

Expected workflow outcomes may be nominal data.
Corruption, authority denial, resource exhaustion, infrastructure failure, and unknown visibility
remain distinguishable.

Non-idempotent work is never silently retried after possible partial execution.
Time, randomness, scheduling observations, and host observations are explicit when observable.

## Execution, scheduling, and concurrency

One simple executable route remains the correctness oracle.
Faster tiers are differential until direct cutover is justified.

Values, traps, order, state transitions, host requests, and resource semantics remain stable across
tiers.

Operational scheduling and language semantics are separate.
Deterministic fuel is not wall-clock time.

A scheduler may use time, priority, quota, or load for admission and fairness only when those choices
do not silently change accepted semantic results.
Observable interruption, yield, cancellation, or timeout requires an accepted contract.

Per-instance transitions remain serial unless a specification defines a stronger model.

Cross-instance parallelism requires exact isolation, bounded queues, deterministic per-instance
order, explicit overload, safe shutdown/restart, and differential tests against serial execution.

Do not add a general async runtime, worker pool, or scheduler merely because a long-lived process is
introduced.
Prefer transition-boundary scheduling until a workload proves resumable mid-transition execution is
necessary.

## Resource governance

Semantic limits and operational limits are separate.

Semantic limits include fuel, frames, value depth, value size, and state size.
Operational limits include queue capacity, concurrent compilation, cache bytes, aggregate memory,
open files, adapter operations, process count, CPU share, and deployment quotas.

Each category states its accounting unit, owner, reservation and release points, peak and retained
accounting, limit source, rejection class, retryability, observability, restart behavior, and
publication interaction.

Check lengths, counts, and reservations before allocation or work.

Do not present best-effort allocator observations as exact semantic accounting.
Shared caches or backing storage must not bypass per-instance limits.

Overload must reject, queue within a bound, or shed work under an explicit policy.
Never create an unbounded hidden queue.

OS controls may strengthen deployment containment.
They do not replace semantic or runtime accounting.

## Derived execution, caching, and JIT

IR, bytecode, layouts, ownership plans, executable images, compiled units, profiles, indexes, and
native code are derived unless a specification deliberately defines validated distribution
authority.

A cache key binds every correctness-relevant input, including applicable semantic artifact,
schema, compiler/verifier contract, target and features, representation, policy assumptions,
host ABI, and tier.

A cache miss is correct.
A hit rejects stale, foreign, corrupt, or incompatible material.
Caches are bounded, disposable, reconstructible, and semantically invisible.

Profiles are bounded, version-bound, privacy-reviewed, disposable, and never authority.

Bytecode, a baseline compiler, JIT, optimizing compiler, deoptimization path, or persistent compiled
cache must beat simpler execution on representative end-to-end workloads.
Measure cold and warm behavior separately.
Do not optimize a synthetic kernel while regressing application operation.

An optimized tier preserves values, traps, order, publication, host interaction, resource semantics,
and diagnostics unless accepted semantics deliberately change.

Keep a simple independent oracle until replacement is proven.

Generated native code and executable memory require an explicit threat model, invalidation contract,
platform contract, and supply-chain review.

## Security and trust

Accepted semantics cannot express unchecked memory access.
User-controlled depth does not consume unbounded native stack.

No local unsafe Rust is permitted unless the active user explicitly authorizes a replacement after
a concrete need, isolated ownership, safe public contract, and independent tests are recorded.

Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, path safety,
crash consistency, supply-chain trust, and hostile-code isolation are separate contracts.

Treat bytes, paths, filesystem metadata, caches, profiles, IPC peers, and adapter outcomes as
hostile input.

A runtime supervisor authenticates and authorizes every request under its deployment model.
A path, socket, UID, or connection is not sufficient semantic authority by itself.

A worker or container is not automatically a sandbox.
Write the threat model before multi-user access, untrusted native code, broad filesystem or network
access, child processes, or secrets.

Large input, work, state, history, graph, output, queue, cache, diagnostics, and metrics are bounded,
streamed, paginated, chunked, or policy-controlled.

Compactness never weakens validation, authorization, identity, durability, diagnostics, or
verification.

## Compatibility

Backward compatibility is absent unless the active user explicitly requires it.

Use incompatible-change freedom to converge on one coherent path.
After cutover, delete old readers, writers, aliases, fallbacks, compatibility tests, migration-only
code, stale examples, dormant flags, and superseded documentation.

Do not introduce editions, dual success paths, hidden fallback, or silent migration as insurance.

Incompatible-change freedom is not permission for an unverified rewrite.

## Decision standard

Treat every historical mechanism as provisional except the enduring contracts above.
Do not preserve a mechanism because it was difficult, planned, or heavily tested.
Reproduce relevant evidence before reversing working behavior.

Prefer:

- complete useful verticals over isolated features;
- one exact path over parallel convenience paths;
- explicit domains over overloaded names;
- one topology-neutral implementation over duplicated process adapters;
- local features over platforms built for one consumer;
- deletion over permanent deprecation;
- bounded prototypes over speculative architecture;
- and high-leverage corrections over accumulated exceptions.

Every retained abstraction, dependency, process, artifact, identity, schema, cache, optimization,
service, scheduler, worker, or framework needs a named current consumer.

Complexity must pay rent in a representative end-to-end workload.

Before retaining a substantial choice, record the consumer, obligations, reproduced baseline,
alternatives including deletion, expected benefit, costs, oracle, cutover deletions, stop rule, and
reversal condition.

Delete losing prototypes completely.
Current absences are not permanent prohibitions without semantic reason.

## Agent and context economy

Context budget is a correctness, latency, and cost constraint.

Keep root `AGENTS.md` at or below 24 KiB unless evidence proves a larger durable prefix improves
complete-task success.
Keep deeper instruction files materially smaller.

State durable instructions once.
Put volatile facts in the active prompt, status, or structured evidence.

Normal work should not require repository-wide reads, a global schema dump, repeated unchanged
context, manual service lifecycle, or historical prompts.

Prefer compact orientation, task-scoped context, exact expansion, bounded review, closed proposals,
stable diagnostics, validate/apply parity, compact receipts and deltas, immutable interface
summaries, digest reuse, and explicit omissions.

After finding exact owners, stop broad discovery unless evidence invalidates the map.
Batch independent reads and checks; keep dependent decisions sequential.

Build a compact task ledger instead of repeatedly rereading a long prompt.
Expose only tools relevant to the decision.

Add prompt rules, examples, tools, schemas, or context only for a measured failure mode.

Agent-facing output is canonical, bounded, stable, and easy to reduce programmatically.

Measure equal tasks using semantic success, unintended changes, correction depth, repeated
discovery, action/observation bytes, provider token classes and calls when exposed, exact cost only
with telemetry and pricing, processes, engine opens, files/source bytes opened, elapsed time, build
cost, and failure quality.

Bytes are not tokens.
Never infer provider cost from bytes.

## Fact ownership

Keep one maintained owner for each active fact:

- `docs/spec/`: accepted semantics and public contracts.
- `docs/architecture.md`: components, dependencies, topology, trust, and TCB.
- `docs/status.md`: implemented reality and exact absences.
- `docs/performance.md` and structured evidence: reproduced measurements.
- `docs/roadmap.md`: future evidence gates.
- `README.md`: concise product explanation and supported entry points.
- Root `AGENTS.md`: durable repository policy.
- `prompts/`: selected campaign artifacts, never semantic authority.
- Executable code and tests: accepted machine behavior.

Do not maintain parallel catalogues, version tables, status lists, inventories, benchmark tables,
manifests, interface catalogues, or memory-model tables.

Keep one executable owner for each accepted type, field, variant, operation, query, error, limit,
test form, manifest, interface, grant, adapter, resource category, and boundary.

Derive codecs, parsers, schema fragments, help, examples, and documentation when derivation reduces
duplication and keeps invariants visible.

A macro, derive, generator, or IDL is acceptable only when authority is explicit, output is
deterministic and reviewable, strict rejection and semantic validation remain explicit, build and
debug cost are measured, staleness cannot be silent, and the displaced owner is deleted.

Git history is the archive.
Delete stale active-tree material without a consumer.

## Code and dependencies

Choose observable semantics before representation or optimization.

A new type, operation, effect, state, interface, capability, resource, executable, cache, process, or
scheduler needs a current consumer and exact applicable contracts.

Do not expose memory choreography to authors.
Keep value semantics, lifetime, aliasing, representation, reclamation, durable state, external
cleanup, and scheduling separate.

A custom ownership plan, arena, collector, region system, bytecode, native backend, JIT, journal,
object store, resolver, scheduler, or supervisor must beat a simpler safe implementation enough to
justify its code and verification surface.

A tracing collector is not the default.
Use one only when a representative cyclic or mutable workload proves lower total complexity while
preserving a simple oracle.

Organize code around stable fact ownership and changed-together behavior.

Do not impose arbitrary universal size or count limits.
Add and test limits only for semantic, security, resource, transport, or tooling reasons.

Large files are not automatically wrong.
Split when ownership, review, build locality, or agent context improves without duplicated
invariants.

Prefer the standard library and existing dependencies.
Add a dependency only when value exceeds supply-chain, build, binary, audit, operational, and
maintenance cost.

Do not hide errors with fallbacks, lossy conversion, panics, unchecked indexing, or best-effort
acceptance.

## Testing and verification

Acceptance tests have exact immutable input, oracle, policy, selection, order, and result.

Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.

For changed boundaries, cover applicable canonical and repeated success, validate-only parity,
unknown/duplicate/wrong-domain input, stale bases, malformed/truncated/trailing/excessive input,
exact and one-over limits, foreign authority, corruption/restart, interrupted publication, cleanup,
concurrency/overload, replay/idempotency, cache miss/hit/eviction/corruption, cross-tier
differentials, and public workflows.

Run narrow checks first, then:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every affected retained public workflow.

Use Miri, sanitizers, mutation, property tests, fuzzing, model checking, crash injection, or
cross-platform execution when they target a real risk and are available.
State scope and limitations.

Do not weaken an invariant test to make implementation pass.
Change specification, implementation, and oracle together when behavior deliberately changes.

## Change workflow

For substantial work:

1. Inspect checkout, instructions, branch, commit, and unrelated changes.
2. Identify authoritative owners.
3. Reproduce the baseline and representative workflow.
4. State outcome, completion bar, non-goals, alternatives, and reversal condition.
5. Prototype uncertain questions in the smallest dependency-closed form.
6. Select one coherent design from evidence.
7. Implement the full vertical across all applicable layers.
8. Cut over directly and delete losing paths and stale facts.
9. Run focused, full, and representative checks.
10. Record evidence without overclaiming.
11. Leave a compact handoff.

Do not stop at a report when a safe complete implementation is authorized and feasible.
Do not scatter partial architecture.
Do not ask the user to decide ordinary engineering details the checkout and evidence can resolve.

## Completion and handoff

A capability is complete only when it is authorable, observable through bounded exact review,
validated by the authority owner, represented in every relevant domain, covered by an independent
oracle, runnable through the supported topology, restart/corruption/limit/failure/overload tested,
resource-accounted where applicable, documented by one owner, exercised by a useful application,
measured end to end, and free of superseded paths.

Before finishing, report:

- exact starting and ending state;
- selected design and rejected alternatives;
- changed contracts and files;
- cutover deletions;
- validation commands and results;
- representative cold and warm evidence where relevant;
- resource-accounting evidence;
- provider telemetry only when exposed;
- known limits and trust assumptions;
- remaining reversal gates;
- and every requested or implied action not performed.

Claims must be no stronger than the checkout and reproduced evidence.
