# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository. A deeper `AGENTS.md` may add procedures
for a genuine ownership boundary, but it may not weaken this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, and handoffs.

## Mission

Build `lkjscript` as an agent-native semantic application platform whose primary program authors and
maintainers are coding agents. Humans remain first-class for intent, governance, security policy,
explanation, review, operations, product ownership, and acceptance.

The enduring objective is that a coding agent can create, inspect, change, validate, test, compose,
build, run, transfer, operate, diagnose, and evolve useful applications through compact exact
interactions, while accepted meaning, durable state, authority, externally visible outcomes,
resource use, and execution optimizations remain typed, bounded, auditable, and independently
checkable.

- Optimize jointly for correctness, complete useful applications, weak-model success, low correction
  and provider cost, compact context, deterministic artifacts and history, explicit authority,
  recoverable operation, maintainability, predictable resources, and long-term performance.
- Do not optimize for novelty, feature count, benchmark theater, roadmap inertia, sunk cost, or
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

- Newer verified checkout state outranks older plans and remembered repository state.
- A campaign prompt owns one campaign's objective, hypotheses, gates, and handoff. It does not
  become permanent semantic authority.
- An old prompt is historical evidence unless the active task explicitly selects it.
- When accepted behavior changes, update the owning specification and executable contract in the
  same verified milestone.

## Repository safety

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -1 --oneline
find .. -name AGENTS.md -print
```

- Read every applicable instruction file and preserve unrelated work.
- Reading in-scope files, editing in-scope files, and running non-destructive validation are
  authorized for implementation tasks unless the active task says otherwise.
- Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a
  release, close a pull request, or alter unrelated remote state without authorization for that
  action.
- Repository permissions are not user authorization.
- Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, raw
  provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
  unclear licensing.
- Keep scratch state, destructive experiments, unsanitized measurements, downloaded research, and
  losing prototypes outside the repository unless a retained artifact has a named consumer.
- Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
  irreproducible observations explicitly.

## Application-first closure

- Every substantial campaign selects a valuable complete application or product workflow that
  determines whether the platform change succeeds.
- The application owns domain state, validation, ordering, decisions, and typed outcomes in
  lkjscript semantics. A host harness may own transport, rendering, explicit file selection, and
  independent assertions, but no hidden business state or policy.
- Build the smallest complete product slice first. Add a language, runtime, storage, interface, or
  tooling mechanism only for an exact blocker revealed by that slice, then return to the product.
- A capability is incomplete when the host reconstructs private state, suppresses invalid requests,
  parses opaque responses for domain meaning, or remains the real workflow controller.
- Run the completed product from a fresh checkout through public release binaries and dogfood it on
  a real repository task before completion.
- Delete productless infrastructure, losing prototypes, stale examples, and intermediate artifacts
  without a retained consumer.

## Enduring semantic authority

- Each accepted authority unit has one authoritative typed semantic representation.
- Natural-language intent and model output are untrusted proposals.
- Text, JSON, documents, context, reviews, caches, indexes, IR, bytecode, profiles, memory plans,
  machine code, renderings, and terminal output are proposals, views, or derived state unless a
  specification deliberately defines an immutable distribution authority.
- No proposal, view, cache, generated form, or derived form bypasses deterministic validation.
- Accepted authority never depends on rendering and reparsing.
- Unknown, malformed, ambiguous, unsupported, foreign-domain, noncanonical, truncated, oversized,
  duplicate, conflicting, or trailing forms reject.
- Derived facts never become a second mutable source of truth.
- A human-readable source form is acceptable only when it deterministically normalizes through the
  same typed validator and cannot silently diverge from accepted meaning.

## Identity and continuity

- Assign identity only for a concrete continuity, sharing, reference, repair, attribution, import,
  export, history, provenance, targeting, durable instance, product entity, or operational consumer.
- Names, formatting, positions, order, paths, hashes, compiler indexes, artifact offsets, storage
  keys, runtime handles, queue positions, process IDs, and addresses are not semantic identity
  unless a closed contract assigns a narrower role.
- Workspace, release, application, instance, product entity, revision, command, outcome, grant,
  interface, adapter, deployment, executable, checkpoint, backup, cache entry, profile, and runtime
  handle are distinct domains.
- A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
  capability identity.
- Identity-preserving change requires an explicit validated rule.
- Deleted durable identities are not silently reused.
- Multiple exact versions may coexist only when references remain unambiguous.
- A filesystem path locates an authority boundary; it does not create semantic authority.

## Publication and durable history

- Published workspace revisions, releases, applications, instance revisions, host outcomes,
  checkpoints declared as authority, backups, and other declared durable objects are immutable
  within their domains.
- Every durable namespace has one publication authority.
- One successful publication creates exactly one accepted durable outcome.
- Rejection and validate-only publish nothing and consume no durable identity.
- A domain decline or semantic no-change must not consume a state revision merely to return a
  response.
- Success is acknowledged only after the documented synchronization boundary.
- A possibly visible but unconfirmed outcome is reported as unknown and never silently retried.
- Recovery, replay, retention, checkpointing, compaction, corruption, backup, restore, and deletion
  are explicit and validated.
- Semantic state publication and externally visible host work remain separate unless atomicity is
  proved.
- Output failure cannot retroactively undo accepted authority.

## Composition and artifact domains

- Workspace, release, application, instance, interface, grant, deployment, executable, checkpoint,
  backup, provenance, profile, and cache are separate domains unless a verified design combines them
  unambiguously.
- Workspaces own development history.
- Releases own exact reusable semantics.
- Applications own exact runnable closure and declared interfaces.
- Instances own durable state and transition history.
- Grants own authority.
- Deployments own process, machine, account, namespace, and resource placement.
- Backups own one exact transferable closure under their explicit contract.
- Executables, compiled units, indexes, profiles, and ordinary checkpoints are derived unless a
  specification deliberately promotes one with exact validation.
- Caches are disposable acceleration.
- Compilation consumes one immutable accepted revision or independently validated distribution state
  and lowers only the complete selected closure.
- Coordinates, versions, aliases, paths, and mutable lookup results are not exact dependencies.

## Mutation and pure query separation

- Mutations and observations are separate semantic contracts.
- A mutation may decline, report unchanged, publish one completed state, or publish one suspended
  state and command under an application-owned typed decision.
- A declined or unchanged mutation publishes no state revision, command, attempt, outcome, or HEAD
  change.
- A pure instance query names exact instance state authority and returns an application-owned typed
  value without publishing any semantic or durable state.
- A query must not be implemented as a no-op event, and a product client must not decode private
  application state as a second query authority.
- Query output failure has no rollback meaning because no semantic publication occurred.
- Query pagination, ordering, truncation, omissions, revision binding, and result-digest behavior
  are exact and bounded.
- Idempotency, stale-base behavior, and response retention remain explicit for mutations, including
  no-publication outcomes.

## Application interfaces and host authority

- Pure deterministic computation remains the default. Ambient host authority is forbidden.
- Applications may declare exact host-interface requirements but never grants.
- Instances bind requirements to exact grants.
- A grant binds the exact sharing domain, interface identity, adapter kind, bounded descriptor,
  applicable authority revision, and limits needed to prevent implicit broadening.
- Host requests and outcomes are closed typed values. Opaque bytes are acceptable only when the
  interface deliberately defines and bounds them.
- An adapter cannot invent semantic state, application response, command intent, or authority.
- At most one pending command remains the bootstrap model. Parallel commands require a current
  application plus exact ordering, cancellation, partial-result, retry, and replay contracts.
- A live resource handle, stream, socket, file descriptor, process, timer, secret, or foreign object
  requires explicit acquire, use, transfer, consume, close, cancellation, timeout, crash, and
  cleanup semantics.
- Do not add live resources merely for adapter convenience.
- Expected workflow outcomes may be nominal data. Corruption, authority denial, resource exhaustion,
  infrastructure failure, and unknown visibility remain distinguishable.
- Non-idempotent work is never silently retried after possible partial execution.
- Time, randomness, scheduling observations, and host observations are explicit when observable.

## Values, storage, and derived mechanisms

- Text and variable-length collections require exact validation, canonical encoding, bounds,
  deterministic equality and order, and a current complete-application consumer.
- Representation, sharing, allocation, reclamation, checkpoint layout, cache entries, IR,
  bytecode, profiles, and native code are unobservable or derived unless a specification
  deliberately defines a narrower validated authority.
- A simple independent allocation, execution, and reconstruction route remains the oracle for
  optimized managed values, checkpoints, caches, compaction, and execution tiers.
- Cache miss, eviction, missing derived checkpoint, and process restart remain correct.
- Full snapshots, events, journals, checkpoints, object stores, databases, collectors,
  bytecode, JITs, schedulers, and supervisors must beat simpler safe designs on a complete
  representative application and must be deleted when they lose.
- Backup and restore capture one exact self-consistent closure, use explicit publication and
  interruption semantics, and claim integrity only under the stated trust model.

## Execution, scheduling, and concurrency

- One simple executable route remains the correctness oracle. Faster tiers are differential until
  direct cutover is justified.
- Values, traps, order, state transitions, host requests, resource semantics, and diagnostics remain
  stable across tiers unless accepted semantics deliberately change.
- Operational scheduling and language semantics are separate. Deterministic fuel is not wall-clock
  time.
- A scheduler may use time, priority, quota, or load for admission and fairness only when those
  choices do not silently change accepted semantic results.
- Observable interruption, yield, cancellation, or timeout requires an accepted contract.
- Per-instance mutations remain serial unless a specification defines a stronger model.
- Pure queries may run concurrently only after exact snapshot isolation, state lifetime, ordering,
  admission, shutdown, and mutation interaction are proved.
- Cross-instance parallelism requires exact isolation, bounded queues, deterministic per-instance
  order, explicit overload, safe shutdown/restart, and differential tests against serial execution.
- Do not add a general async runtime, worker pool, scheduler, actor system, or daemon merely because
  a long-lived process is introduced.
- Prefer transition-boundary scheduling until a workload proves resumable mid-transition execution
  is necessary.

## Resource governance

- Semantic limits and operational limits are separate.
- Semantic limits include fuel, frames, value depth, item count, text bytes, collection elements,
  state size, query work, and response size.
- Operational limits include queue capacity, concurrent compilation, cache bytes, aggregate memory,
  open files, adapter operations, process count, CPU share, temporary publication bytes, and
  deployment quotas.
- Each category states its accounting unit, owner, reservation and release points, peak and retained
  accounting, limit source, rejection class, retryability, observability, restart behavior, and
  publication interaction.
- Check lengths, counts, depth, and reservations before allocation or corresponding work.
- Do not present best-effort allocator observations as exact semantic accounting.
- Shared backing storage, checkpoints, caches, or application embedding must not bypass per-instance
  or per-request limits.
- Overload must reject, queue within a bound, or shed work under an explicit policy. Never create an
  unbounded hidden queue.
- OS controls may strengthen deployment containment. They do not replace semantic or runtime
  accounting.

## Security and trust

- Accepted semantics cannot express unchecked memory access. User-controlled depth does not consume
  unbounded native stack.
- No local unsafe Rust is permitted unless the active user explicitly authorizes a replacement after
  a concrete need, isolated ownership, safe public contract, and independent tests are recorded.
- Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, path safety,
  crash consistency, supply-chain trust, and hostile-code isolation are separate contracts.
- Treat bytes, text, paths, filesystem metadata, project markers, artifacts, instance files,
  checkpoints, backups, caches, profiles, IPC peers, and adapter outcomes as hostile input.
- A runtime supervisor authenticates and authorizes every request under its deployment model. A
  path, socket, UID, or connection is not sufficient semantic authority by itself.
- A worker or container is not automatically a sandbox.
- Write the threat model before multi-user access, untrusted native code, broad filesystem or
  network access, child processes, or secrets.
- Large input, work, state, history, graph, output, queue, cache, diagnostics, and metrics are
  bounded, streamed, paginated, chunked, or policy-controlled.
- Human terminal output is escaped and bounded. Machine output is framed separately and never
  contaminated by progress text.
- Compactness never weakens validation, authorization, identity, durability, diagnostics, or
  verification.

## Compatibility

- Backward compatibility is absent unless the active user explicitly requires it.
- Use incompatible-change freedom to converge on one coherent path.
- After cutover, delete old readers, writers, aliases, fallbacks, compatibility tests,
  migration-only code, stale examples, dormant flags, and superseded documentation.
- Do not introduce editions, dual success paths, hidden fallback, or silent migration as insurance.
- Incompatible-change freedom is not permission for an unverified rewrite.
- A product state migration requires a current continuity consumer, exact old/new identities, a
  deterministic mapping, failure publication, rollback behavior, and independent tests.

## Decision standard

- Treat every historical mechanism as provisional except the enduring contracts above.
- Do not preserve a mechanism because it was difficult, planned, or heavily tested.
- Reproduce relevant evidence before reversing working behavior.
- Prefer complete useful verticals over isolated features.
- Prefer one exact path over parallel convenience paths.
- Prefer explicit domains over overloaded names.
- Prefer one topology-neutral implementation over duplicated process adapters.
- Prefer local features over platforms built for one consumer.
- Prefer deletion over permanent deprecation.
- Prefer bounded prototypes over speculative architecture.
- Prefer high-leverage corrections over accumulated exceptions.
- Every retained abstraction, dependency, process, artifact, identity, schema, cache, optimization,
  service, scheduler, worker, or framework needs a named current consumer.
- Complexity must pay rent in a representative end-to-end workload.
- Before retaining a substantial choice, record the consumer, obligations, reproduced baseline,
  serious alternatives including deletion, expected benefit, costs, oracle, cutover deletions, stop
  rule, and reversal condition.
- Delete losing prototypes completely. Current absences are not permanent prohibitions without
  semantic reason.

## Agent and provider economy

- Context budget is a correctness, latency, and cost constraint. Keep this root file at or below
  24 KiB and deeper instruction files materially smaller unless reproduced evidence justifies more.
- State durable instructions once; put volatile facts in the active prompt, status, or evidence.
- Prefer compact orientation, task-scoped context, exact expansion, bounded review, stable
  diagnostics, validate/apply parity, compact receipts, digest reuse, and explicit omissions.
- After identifying exact owners, stop broad discovery unless evidence invalidates the map.
- Build a compact task ledger instead of rereading long prompts, and expose only relevant tools.
- Add prompt rules, examples, schemas, or context only for a measured failure mode.
- Compare equal tasks using semantic success, unintended changes, correction depth, repeated
  discovery, action/observation bytes, calls, processes, files and source bytes opened, elapsed time,
  build cost, and failure quality.
- Record provider token classes, pricing, and monetary cost only when directly exposed.
- Bytes are not tokens. Never infer provider cost from bytes.

## Fact ownership, code, and dependencies

- `docs/spec/` owns accepted contracts; architecture owns components and trust; status owns
  implemented reality; performance and structured evidence own measurements; roadmap owns
  unresolved consumer-driven gates; README owns concise orientation; prompts are temporary.
- Keep one executable owner for every type, field, operation, query, error, limit, format,
  interface, grant, resource, and command. Derive views only when staleness cannot be silent.
- Organize code around stable ownership and changed-together behavior. Split large files when
  bounded review or build locality improves without duplicating invariants.
- Prefer the standard library and existing dependencies. A new dependency must repay its
  supply-chain, build, binary, audit, operational, and maintenance cost.
- Git history is the archive. Delete stale active-tree material and losing generated paths.

## Testing and verification

- Acceptance tests have exact immutable input, oracle, policy, selection, order, and result.
- Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.
- For changed boundaries, cover applicable canonical and repeated success, validate-only parity,
  no-publication outcomes, pure-query no-write behavior, unknown, duplicate, wrong-domain,
  stale-base, malformed, truncated, trailing, excessive, exact and one-over limits, foreign
  authority, corruption, restart, interrupted publication, cleanup, concurrency, overload, replay,
  idempotency, cache miss/hit/eviction/corruption, checkpoint differential, cross-tier differential,
  backup/restore, and public workflows.
- Use a simple independent product reference model where application semantics become substantial.
- Run narrow checks first, then the full repository gates.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

- Run every affected retained public workflow and the selected complete application.
- Use Miri, sanitizers, mutation, property tests, fuzzing, model checking, crash injection, or
  cross-platform execution when they target a real risk and are available. State scope and
  limitations.
- Do not weaken an invariant test to make implementation pass.
- Change specification, implementation, and oracle together when behavior deliberately changes.

## Change workflow

1. Inspect checkout, instructions, branch, commit, and unrelated changes.
2. Identify authoritative owners.
3. Select the valuable application workflow and freeze an independent oracle.
4. Reproduce the baseline and representative workload.
5. State outcome, completion bar, non-goals, alternatives, and reversal condition.
6. Build the smallest complete application slice and identify exact blockers.
7. Prototype uncertain questions in the smallest dependency-closed form.
8. Select one coherent design from evidence.
9. Implement the full vertical across all applicable layers.
10. Return to the application and prove the blocker is closed.
11. Cut over directly and delete losing paths and stale facts.
12. Run focused, full, representative, restart, corruption, and product checks.
13. Record evidence without overclaiming.
14. Dogfood the product when the campaign has a product surface.
15. Leave a compact handoff.

- Do not stop at a report when a safe complete implementation is authorized and feasible.
- Do not scatter partial architecture.
- Do not ask the user to decide ordinary engineering details the checkout and evidence can resolve.

## Completion and handoff

A capability is complete only when it is authorable, boundedly reviewable, validated by its
authority owner, represented in every relevant domain, covered by an independent oracle, runnable
through the supported topology, restart/corruption/limit/failure tested, resource-accounted where
applicable, documented by one owner, exercised by a useful application, measured end to end, and
free of superseded paths.

Before finishing, report:

- exact starting and ending state;
- selected design, serious rejected alternatives, changed contracts, and cutover deletions;
- validation commands and exact results;
- representative application, storage, resource, cold/warm, backup, audit, and dogfood evidence;
- provider telemetry only when exposed;
- known limits, trust assumptions, reversal gates, and every requested action not performed.

Claims must be no stronger than the checkout and reproduced evidence.
