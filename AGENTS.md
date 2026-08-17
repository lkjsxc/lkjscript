# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository. A deeper `AGENTS.md` may narrow local
procedure but may not weaken these repository-wide requirements.

Use English for maintained code, tests, diagnostics, protocol fields, machine output, specifications,
documentation, generated descriptions, benchmark labels, commit messages, and handoffs.

## Repository Safety

Preserve unrelated work. Before editing, inspect:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
```

Never reset, clean, overwrite, delete, stage, commit, amend, rebase, merge, push, force-push, close a
pull request, or otherwise change work or remote state unless the active task explicitly authorizes that
exact action. Repository permissions are not authorization.

Never commit credentials, access tokens, private transcripts, hidden model reasoning, provider secrets,
personal data, unrelated artifacts, or generated corpora with unclear licensing. Keep disposable
payloads, transcripts, prototypes, and benchmark scratch data outside the repository.

## Mission

Build `lkjscript` as an **agent-native semantic programming environment**.

Autonomous coding agents are the primary program authors. Humans remain first-class at the levels of
intent, governance, security policy, explanation, review, operation, and product ownership.

The plain product explanation is:

> A coding agent develops a program by querying and editing a typed, versioned semantic model. The service validates changes, publishes immutable revisions, and compiles and runs selected revisions.

The authoritative model is formally called the **Semantic Program Graph** (`SPG`). “Graph” describes
typed semantic entities and relations; it does not prescribe a graph database, pointer-rich storage,
mutable object graph, or physical layout.

The goal is not “source code without text.” The goal is one authoritative typed representation with
derived views and proposal forms optimized for reliable, compact, inspectable agent and human work.

Text, diagrams, generated source, compact commands, JSON, tool calls, caches, indexes, Core IR,
profiles, and machine code may be useful views, proposals, or derived state. They must not become
competing program authorities.

## Instruction Precedence

When active artifacts disagree, use this order:

1. Active user task.
2. Root `AGENTS.md`.
3. Active campaign prompt.
4. Normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. Machine descriptions derived from executable contracts.
7. `docs/status.md`.
8. `docs/architecture.md`.
9. `docs/performance.md`.
10. `docs/roadmap.md`.
11. `README.md`.
12. Comments, examples, old prompts, branches, pull requests, commits, issues, and historical documents.

Newer verified state outranks older plans. Treat stale pull requests and old prompts as historical
evidence unless explicitly selected.

A campaign prompt owns one campaign’s scope, sequence, hypotheses, evidence gates, and non-goals. It
does not silently redefine accepted semantics. Update the owning specification in the same verified
milestone when semantics change.

## Fact Ownership

Keep one maintained owner for each fact:

- `docs/spec/semantic-graph.md`: authority, identity, revisions, transactions, history, semantic
  artifacts.
- `docs/spec/language.md`: types, values, operations, control, effects, lifetime, cleanup, observable
  execution.
- `docs/spec/protocol.md`: transport, requests, responses, framing, boundary forms, schema discovery,
  cursors, CLI-to-service projection.
- `docs/architecture.md`: components, process topology, trusted computing base, trust boundaries.
- `docs/status.md`: implemented reality and exact absences.
- `docs/performance.md`: reproduced measurements, comparisons, regressions, resource and
  agent-interaction observations.
- `docs/roadmap.md`: ordered future gates and deferred choices.
- `README.md`: concise human-first explanation and supported entry points.
- This file: durable repository policy.
- `prompts/`: campaign execution artifacts, never permanent semantic authority.

Do not duplicate status catalogues, version registries, schemas, roadmaps, architecture inventories,
benchmark tables, dependency inventories, or memory-model tables. Machine descriptions, derived text
views, and compact agent interfaces must derive from executable contracts rather than hand-maintained
parallel definitions.

## Enduring Invariants

These outrank the current implementation language, module layout, transport, artifact format, storage
engine, runtime representation, backend, platform, and model provider.

1. Program meaning has one authoritative typed semantic model.
2. Published revisions are immutable.
3. Durable publication has one unambiguous logical commit authority per durable namespace.
4. Accepted semantic forms belong to a closed typed schema; unknown forms reject.
5. Arbitrary property bags and string-labelled semantic edges are not authority.
6. Mutation is a typed transaction or a closed typed proposal deterministically normalized into one.
7. One successful commit publishes exactly one accepted revision.
8. Rejection and validate-only publish nothing and consume no persistent identities.
9. Persistent identity is independent of names, formatting, source position, hashes, compiler indexes,
   artifact offsets, runtime handles, addresses, and storage keys.
10. Persistent identities are never reused within their identity domain.
11. Names are lookup and presentation metadata, not identity.
12. Identity-preserving change exists only under an explicit continuity rule.
13. Derived facts never become a second mutable source of truth.
14. The compiler consumes an immutable semantic revision directly.
15. Only a complete selected-entry dependency closure enters executable lowering.
16. One simple execution route defines behavior; faster tiers remain differential against it.
17. AI output and natural-language intent are untrusted; deterministic validators decide acceptance.
18. Accepted language semantics cannot express unchecked memory access.
19. Host access requires explicit typed authority.
20. External resources require explicit outcome and deterministic cleanup semantics.
21. User-controlled depth does not consume unbounded native stack.
22. Observable order is explicit and deterministic.
23. Potentially large results are bounded, streamed, paginated, or policy-controlled.
24. Durable state is acknowledged only after the documented publication contract.
25. Corrupt, ambiguous, unsupported, or partially published authority rejects rather than being guessed
    valid.
26. Compactness never weakens typing, validation, identity, authorization, durability, diagnostics, or
    verification.
27. Performance optimization preserves a simple correctness oracle.
28. Backward compatibility is absent unless the active user explicitly requires it.
29. Incompatible-change freedom converges on one path; no legacy mode, edition split, compatibility
    reader, silent migration, dual success path, or hidden fallback remains after cutover.
30. Memory safety, resource exhaustion, cleanup, concurrency, permission security, and crash consistency
    are separate contracts.
31. Every retained dependency has a named current consumer.
32. Every public boundary has exact versioning, bounds, canonical forms, rejection behavior, and output
    policy.
33. No non-idempotent effect is silently retried after possible partial action.
34. Runtime handles never substitute for semantic identities and raw pointers never cross untrusted or
    persistent boundaries.
35. Human-facing claims remain no stronger than the checkout and reproduced evidence.
36. Agent action quality and observation quality are first-class product properties.
37. Context locality, change locality, and verification locality are engineering dimensions.

## Claims and Decisions

Classify material statements as: enduring invariant, accepted semantic contract, current verified
baseline, operational policy, evidence-gated choice, experimental hypothesis, or historical fact.

Do not turn a bootstrap absence into a permanent prohibition, or a current mechanism into language
semantics without necessity. Do not present a hypothesis as implemented, one agent run as a model
benchmark, one machine or seed as universal performance, or safe Rust as a complete proof.

Bytes are not tokens. Cached and uncached tokens are not equivalent. Logical, visible, retained,
encoded, copied, and peak bytes are distinct. Use provider-reported token and price telemetry when
available; otherwise report exact proxies without relabelling them.

Before retaining a substantial choice, name its concrete consumer, user or agent problem,
semantic/safety/durability obligations, correctness oracle, measured baseline, expected benefit,
implementation and trusted-surface cost, context cost, and reversal condition.

Evidence gating prevents speculative overengineering. It must not become permanent minimalism. Multiple
representative consumers exposing the same missing abstraction justify evaluating that abstraction
directly.

## Architectural Posture and Cutovers

The current SPG service is a strong baseline, not an untouchable monument. Do not restore text as
coequal authority, perform a total rewrite merely because compatibility is unimportant, preserve a
mechanism because it was expensive, or follow the roadmap automatically.

Replace a subsystem when the replacement is dependency-closed, materially clearer or more capable, and
verified against the invariants.

For an active-boundary replacement:

- identify every reader, writer, validator, descriptor, format, test, example, and document;
- choose an unambiguous new version or identity where old bytes could be misread;
- replace active readers and writers together;
- delete displaced code, flags, fixtures, tags, adapters, and claims;
- update specification and status in the same milestone;
- leave one active path.

Git history is the archive for superseded implementation.

## Authority, Views, and Proposal Forms

Source independence is an authority property, not hostility toward text.

An agent must be able to construct, inspect, revise, validate, compile, execute, package, and debug
without a mutable source file as authority.

A derived text view may improve compact observation, review, semantic diffing, debugging, search,
exchange, and test minimization. A proposal syntax may improve compact typed editing.

A view or proposal form may not bypass validation, allocate persistent identities independently, own
semantic state, persist a parallel authoritative AST, make formatting identity, define new behavior,
require render-and-reparse editing for correctness, or drift from executable contracts.

Prefer one-way derived views before claiming round-trip interchange. If a parseable proposal form is
retained, normalize it into the same typed transaction path and discard its syntax tree after
diagnostics and normalization.

Use this wording:

> Program meaning is stored in a typed semantic model. Text may be a view or proposal form, but it is not a second authoritative representation.

Do not lead with `source-free`.

## Semantic Identity and Incomplete Programs

The SPG is a closed typed model, not a generic property graph. Every kind defines exact attributes,
owners, ordered children, references, uses, results, cardinality, completeness, continuity, deletion,
lowering, query, and artifact obligations.

Unknown schema elements reject. Do not preserve unknown fields for hypothetical compatibility. Prefer
direct closed types and static descriptors over runtime registration. Reuse one code-owned contract
across validation, codecs, queries, machine descriptions, views, proposal parsers, history, and lowering
where practical.

Distinguish persistent Node IDs, transaction-local symbols, revision-bound aliases, context labels,
query labels, revisions, hashes, dense compiler IDs, runtime handles, names, and view positions.

Give persistent identity only when continuity, targeting, sharing, repair, history, attribution, or
external reference requires it. Do not remove useful identity merely to shorten payloads.

A revision-bound alias must bind exact workspace, revision, context, and schema domains; reject stale,
foreign, ambiguous, or out-of-scope use. It never becomes semantic identity.

Incomplete programs are valid semantic states. A typed placeholder or equivalent exact form blocks
execution only when reachable from the selected entry. Repair context is deterministic typed data; model
output remains outside correctness. Identity-preserving repair requires an explicit continuity rule, not
unrestricted semantic morphing.

## Transactions and Candidate Work

Every mutation names a workspace, exact base revision or parent relation, commit or validate-only mode,
optional supported idempotency key, closed ordered proposal, and bounded response selection.

Successful commit validates the boundary, resolves local references, normalizes proposals, allocates
candidate identities deterministically, applies canonical edits, validates semantics/history, derives
changes, preflights response/durable bytes, durably publishes authority, publishes memory state, and
returns the preflighted receipt.

Rejection changes none of those states. Validate-only follows the same semantic preparation and
publication preflight without publication.

Prefer semantically meaningful operations over transport scaffolding. Retain fine-grained edits for
maintenance. Do not introduce a macro language, rewrite system, parser framework, or hidden model
planner for one convenience case.

A multi-turn candidate session, branch, or change set is allowed only with exact authority status,
identity domains, lease/cleanup, resource limits, base conflicts, publication, query/run,
crash/cancellation, stale-client, and abandonment semantics. It must not become an ambiguous second
mutable program.

## Agent-Facing Interface

The external coding agent is the primary author. The interface must optimize semantic success, not
merely serialization correctness.

Provide compact bootstrap discovery, exact on-demand expansion, semantically local context, stable
identities and scoped aliases, revision-bound reads, legal actions, meaningful proposals, compact
receipts, typed errors, visible values, legal constructors, repair facts, diffs, bounded execution
diagnostics, exact runtime values, digest-based unchanged results, and deterministic review views.

A low-level node API is not agent-friendly because it is typed. A huge exhaustive schema is not
agent-friendly because it is complete. Treat observation and action as separate design problems.

Prefer task-specific context packets over world dumps. A packet is exact, revision-bound, bounded,
deterministic, sufficient for its declared task, explicit about omissions, invalidated by relevant
changes, and never authority.

Do not require agents to author compiler indexes, CFG plumbing that can be implied, artifact offsets,
checksums, publication records, cache keys, memory plans, retain/release operations, allocator metadata,
or durability metadata.

Do not invent compact syntax from intuition. Compare candidates on equal semantic tasks. Measure
completion, semantic requests, correction count, discovery repetition, observation/action bytes, tool
calls, shell launches, processes, daemon connections, files opened, elapsed time, implementation and
verification cost, and provider telemetry when available. Never reduce interaction cost by weakening
correctness.

## Queries, Diagnostics, and Review

Queries are pure over exact immutable revisions. Large results are bounded, streamed, or paginated.
Cursors bind enough context to reject cross-revision, cross-target, and cross-purpose use.

Caches and indexes are disposable derived state and never decide meaning.

Diagnostics are stable typed data first; prose is presentation. Identify the smallest exact semantic
target explaining rejection. For execution failures, prefer bounded semantic call paths, operation
identities, selected branches, values, and policy facts over implementation backtraces.

Review views should be deterministic and stable under unrelated edits. Never leak allocator addresses,
private IR indexes, or accidental map order.

## Language, Compiler, and Runtime

Choose observable semantics before representation.

A new type or operation needs a real consumer and exact contracts for equality, identity, mutability,
duplication, order, conversion, failure, allocation, lifetime, cleanup, effects, lowering, queries,
artifacts, boundaries, runtime values, and tests.

Generics, collections, text, recursive values, effects, packages, resources, and concurrency are
evidence-gated, not forbidden. When an application is blocked, compare the smallest dedicated feature
with the reusable abstraction repeated consumers may justify.

The execution route is:

```text
immutable semantic revision
    -> completeness and semantic validation
    -> derived executable IR
    -> independent verification
    -> execution engine
```

Core IR, memory plans, profiles, bytecode, and native images are derived. Keep a simple executable
oracle. Optimized interpreters, JITs, AOT backends, native workers, and specialized kernels remain
differential against it.

Do not expose private blocks, registers, layouts, handles, or offsets to authors. Do not preselect LLVM,
Cranelift, a custom JIT, or a memory framework without workload evidence. Native acceleration expands
the trusted computing base and must be isolated and identity-bound to revision, target, policy, backend,
and memory contract.

Fuel, frames, cells, logical bytes, retained bytes, handles, allocations, stack, time, and external
resources are distinct policies. Optimization preserves deterministic failures.

## Memory Safety, Effects, and Resources

Memory safety is enduring. Keep package-wide `unsafe_code = "forbid"` unless an active task explicitly
authorizes a narrow isolated exception after safe alternatives are shown inadequate.

Review separately language expressiveness, semantic validation, untrusted decoding, implementation
memory safety, allocation bounds, stack safety, lifetime/aliasing, cleanup, concurrency, permissions,
native/foreign isolation, and crash behavior.

Do not collapse value semantics, use discipline, lifetime, representation, and reclamation into
“ownership” or “GC.” For each value class define equality, observable identity, mutability, duplication,
access, escape, lifetime, cleanup, representation, reclamation, accounting, concurrency, and foreign
behavior.

Prefer inference from semantic use graphs, control flow, types, and escape facts over agent-authored
memory choreography. Do not expose Rust-like lifetimes, retain/release, allocator calls, raw regions, or
borrow bookkeeping solely for implementation convenience.

Use validated domain-bound handles rather than raw pointers at interpreter, artifact, protocol, cache,
and foreign boundaries.

Pure computation is a baseline, not a permanent prohibition. Every host effect requires explicit typed
authority. Permission values state what may be attempted; resource values state what must be released,
consumed, committed, or closed.

An effect defines authority, validation, order, cancellation, timeout, partial action, retry, cleanup,
audit, isolation, and crash behavior. Never rely on nondeterministic finalization for required resource
cleanup.

## Persistence, Branching, and Concurrency

Published revisions are immutable. Full snapshots and full history are current baselines.

Any journal, chunk store, database, compaction, pruning, or branch representation preserves commit
authority, revision and semantic identity, non-reuse, crash consistency, restart validation, corruption
rejection, bounded recovery, historical queries, and a reconstruction oracle.

Single-head and single-writer mechanics are not eternal bans on branches, replicas, or workers. A
branch/merge design requires explicit parentage, head identity, allocation rules, deterministic conflict
representation, stale-base behavior, merge validation, publication ordering, recovery, query/execution
semantics, deletion/tombstone rules, and a sequential oracle.

Do not add async or threads without measured queueing, latency, throughput, isolation, or utilization
need. Define snapshot selection, writer serialization, cancellation, lock ordering, conflicts, shutdown,
and deterministic tests.

## Implementation Architecture and Context Locality

File boundaries follow fact ownership and change locality. Do not impose arbitrary line, file,
directory, or byte limits, but do not ignore pathological context concentration.

A file should not force ordinary local changes to load large unrelated concerns. Split only for a real
semantic, API, validation, format, transport, target, dependency, unsafe, compile-isolation, process,
test-oracle, or change-locality boundary. Keep invariants together; avoid forwarding forests, duplicate
helpers, micro-modules, and nominal splits that preserve coupling.

For context-locality refactors, compare relevant files and source bytes opened for representative tasks,
compile/test selection, duplicated facts, cross-module surface, and full-build cost. Large files are
inspection signals, not automatic violations.

## Dependencies and Supply Chain

Every dependency needs a current consumer. Before adding or upgrading, inspect license, lockfile impact,
transitives, build scripts, native and unsafe code, enabled features, platforms, maintenance, security
information, compile cost, binary size, and trusted-surface effect.

Prefer standard library or existing dependencies when adequate. A mature dependency may beat bespoke
security code; a direct implementation may beat a large framework. Decide from the actual boundary.
Delete unused dependencies in the same milestone. Keep `Cargo.lock` authoritative.

## Tests and Representative Applications

Test acceptance and rejection, including semantic validity, malformed boundaries, type/kind/scope
errors, identity, rollback, validate-only, idempotency, diffs, artifact round trips, restart,
corruption, incompleteness/repair, old revisions, compiler/verifier rejection, runtime traps, policies,
stack safety, handle validation, memory accounting, cleanup, cancellation, stale aliases/packets, branch
conflicts, and old-version rejection as applicable.

Use generated sequences, property tests, fuzzing, Miri, sanitizers, or model checking for named retained
risks. Do not call deterministic mutation testing fuzzing or claim unavailable tools ran.

A retained application uses the public production path, has a deterministic oracle, exercises
interacting features and rejection, includes maintenance across revisions and restart where relevant,
avoids private semantic fixtures, remains maintainable, and justifies its capability.

Agent-interface work requires a sealed equal-task evaluation when feasible. Distinguish deterministic
replay, one controlled observation, and a general benchmark.

## Documentation and Work Procedure

Use plain meaning before specialized terms:

- “typed, versioned semantic model” before “Semantic Program Graph”;
- “named record type” before “nominal product”;
- “variant type” before “closed sum”;
- “typed placeholder” before “hole”;
- “explicit permission value” before “capability”;
- “immutable managed value” before a reclamation technique;
- “derived text view” before “source projection.”

README is not an agent manual. Specifications are not status. Status is not roadmap. Performance
evidence is not marketing. Prompts are not permanent specifications.

Before substantial work:

1. Inspect branch, commit, status, and active instructions.
2. Read this file, the active prompt, owning specs, status, architecture, performance, and roadmap.
3. Reproduce the smallest relevant public-path baseline.
4. Name the user or agent problem and changed contract.
5. Compare alternatives and reversal conditions.
6. Enumerate affected readers, writers, validators, descriptors, formats, examples, tests, and
   documents.
7. Choose the smallest dependency-closed milestone answering the product question.
8. Define correctness, rejection, interaction, and performance oracles.
9. Record non-goals.

Do not implement from an old prompt without checking the checkout. Resolve details from repository
evidence rather than asking unnecessarily. Delete displaced code in the same milestone; do not leave
stale readers, dead flags, commented alternatives, stale generated output, or campaign TODOs.

## Verification and Handoff

Normal final verification is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run required production examples and focused boundary, corruption, restart, rollback, memory, interface,
and performance commands. A failed final command invalidates the boundary; fix it and rerun the complete
boundary. Never report stale results after later edits.

Record exact unavailability of Miri, sanitizers, fuzzers, model checkers, isolated agent trials, or
provider telemetry. Do not weaken policy to manufacture a green claim.

Before handoff, inspect status and the complete diff; confirm unrelated work and remote state are
untouched, no dual path remains after cutover, fact ownership is singular, documentation matches the
checkout, claims match evidence, safety/resource boundaries are explicit, the agent interface is
discoverable without implementation source, and no secrets or disposable transcripts entered the
repository.

The handoff states the starting commit, final worktree, decisions and rejected alternatives,
semantic/protocol/artifact/interface/storage/runtime/memory changes, trusted-surface effects, exact
verification and application results, measured interaction/performance changes, unavailable evidence,
unresolved risks, and next gate.

Do not claim implementation for design-only work. Do not hide partial completion. Do not push or publish
unless explicitly requested.
