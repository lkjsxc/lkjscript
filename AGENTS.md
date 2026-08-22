# AGENTS.md

This file governs the entire lkjsxc/lkjscript repository. A deeper AGENTS.md may add rules for a
genuine ownership boundary, but it may not weaken this file. Keep the combined instruction chain
small enough to load reliably. Campaign detail belongs under prompts/, not here.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commits, evidence, and handoffs.


## Mission

Build lkjscript into a general-purpose, meaning-oriented programming language and application
platform whose canonical authored program authority is a typed semantic graph.

The graph is not a user-facing graph database and does not justify verbose node plumbing. It is the
compact, validated, revisioned owner of program meaning. Names, textual views, compiled forms,
indexes, caches, deployment bindings, and interfaces are projections or consumers.

Normal development of lkjscript programs occurs through the public semantic CLI. The CLI must let
agents and humans discover, query, change, refactor, validate, review, test, and publish meaning
without editing maintained text or raw storage records.

Text may be deterministic rendering, review view, diagnostic excerpt, new-authority import, or
recovery oracle. It is never a second editable truth. A program change is accepted only through one
semantic transaction protocol.


## Primary objectives

1. Keep one exact canonical semantic authority and publication protocol.
2. Make semantic development more precise, local, economical, and capable than text-first editing.
3. Support materially different applications without application-specific Rust policy.
4. Keep external authority explicit through typed requirements, grants, and bounded adapters.
5. Reach asymptotically sound graph, compiler, runtime, storage, and interaction performance.
6. Minimize repeated context, command output, correction depth, and redundant verification.
7. Preserve independent correctness oracles while optimizing production paths.
8. Delete superseded formats, commands, readers, writers, products, and docs after direct cutover.


## Authority and precedence

1. The active user task.
2. This root AGENTS.md.
3. An explicitly selected active campaign prompt.
4. Accepted normative specifications under docs/spec/.
5. Executable validators, invariant tests, and public black-box contracts.
6. The current accepted semantic graph revision and its canonical revision record.
7. Generated descriptions mechanically derived from an executable owner.
8. docs/status.md and docs/architecture.md.
9. Structured evidence and docs/performance.md.
10. docs/roadmap.md, README.md, and application docs.
11. Historical prompts, commits, branches, issues, discussions, and remembered plans.

A newer verified checkout outranks stale prompt facts. A campaign prompt governs one campaign; it
does not become permanent authority. When behavior changes, update its specification,
implementation, executable oracle, status, and user documentation in the same verified cutover.


## Autonomy and responsibility

- Resolve ordinary engineering decisions from the checkout, complete workflows, bounded prototypes,
  measurements, and stated priorities. Do not ask the user to choose implementation details evidence
  can decide.
- Do not stop at a report when an authorized dependency-closed implementation is feasible.
- Large rewrites are allowed when they converge the repository on one stronger model and carry
  consumers, tests, docs, and deletion through completion.
- State uncertainty honestly. Never upgrade an observation into a guarantee.
- Never claim performance, security, portability, token savings, or cost savings without
  corresponding evidence.


## Repository safety

Before editing, inspect the actual checkout and every applicable instruction file.

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

- Preserve unrelated modified and untracked work. Permissions are not authorization.
- Do not reset, clean, overwrite, amend, rebase, merge, force-push, publish a release, delete remote
  state, or alter unrelated paths without exact authorization.
- Stage only explicit in-scope paths. Never use git add ., git add -A, or git add --all.
- Inspect staged and unstaged diffs before every commit and verify local and remote refs after an
  authorized push.
- Never commit credentials, secrets, personal data, private transcripts, hidden reasoning, raw
  provider events, unrelated files, or unlicensed corpora.
- Keep scratch state, research, destructive experiments, and losing prototypes outside the
  repository unless a retained artifact has a named consumer.
- Report unavailable tools, failed verification, unknown publication, partial completion, and every
  requested action not performed.


## Backward compatibility and direct cutover

- Backward compatibility is absent unless the active user explicitly requires it.
- Do not preserve old source, graph stores, identities, artifacts, protocols, commands, profiles,
  applications, instances, deployment layouts, or generated views.
- Prefer one current path over dual readers, dual writers, editions, aliases, fallback, and
  permanent migration layers.
- A one-time migration tool is allowed only to complete an exact cutover.
- After maintained consumers migrate and verify, delete obsolete normal paths and reject predecessor
  formats exactly.
- Historical reconstruction is not compatibility and must not create or execute predecessor current
  artifacts.
- Incompatible-change freedom is not permission to leave the repository between architectures.


## Canonical semantic authority

- Each accepted project revision has exactly one canonical typed semantic graph.
- The graph owns packages, modules, declarations, types, expressions, components, ports, capability
  requirements, tests, and semantically retained documentation.
- The graph does not own secrets, live handles, compiler-local indexes, bytecode offsets, caches,
  formatting preferences, or host coordinates.
- Logical authority does not require one physical object graph. Storage may use packed tables,
  arenas, shards, immutable segments, content addressing, indexes, journals, and snapshots.
- Select physical forms by correctness, locality, bounded loading, write amplification, merge
  behavior, and measured performance.
- Do not encode the graph as verbose recursive JSON or one heap allocation per node.
- Do not require whole-project loading for local query or change.
- Accepted revisions are complete and valid. Holes, unresolved references, conflicts, and
  speculative plans belong to explicit non-executable drafts.
- Every read identifies its revision. Every write names an exact base and publishes at most one
  revision atomically.
- Validation, rejection, stale input, no-change, and reads publish nothing.


## Semantic identity

- Use durable identity only for a concrete continuity, reference, sharing, history, refactoring,
  merge, deployment, or operational consumer.
- Separate stable semantic identity, name, namespace, content digest, revision ID, physical key,
  compiler index, runtime handle, and rendered coordinate.
- No value silently represents multiple identity domains.
- Every domain has an owner, canonical encoding, equality, allocation, retention, deletion,
  nonreuse, and diagnostic rule.
- Names and paths are locators and presentation unless a specification deliberately makes them
  identity.
- Rename and move preserve or replace identity explicitly.
- A digest proves equality or integrity only in its exact domain, not provenance, authority,
  freshness, or permission.
- Reject foreign-domain IDs even when bytes or display names match.


## Semantic transactions

- All normal program mutations use one public semantic transaction contract.
- A transaction carries exact base revision, graph contract identity, ordered operations,
  preconditions, caller idempotency where needed, output budget, and optional nonsemantic intent
  metadata.
- Express intent at the highest exact supported level: create, replace, delete, rename, move,
  rebind, extract, inline, change signature, add field, change variant, add test, and repair
  reference.
- Raw low-level node edits may exist as a narrow conformance surface but are not the ordinary
  workflow.
- Validation occurs before publication. Acceptance publishes one revision and one compact receipt.
- A receipt names base, result, semantic diff digest, affected owners, verification facts, and
  expansion handles without repeating the graph or every pass.
- Transactions are deterministic under exact inputs.
- Stale base, precondition failure, ambiguity, foreign ID, exhaustion, conflict, no-change, invalid
  meaning, corruption, and infrastructure failure remain distinct.


## Semantic CLI

- The public semantic CLI is the ordinary development interface for lkjscript programs.
- Use direct file editing for Rust, specifications, documentation, deployment assets, and other
  non-lkjscript files.
- Do not edit maintained lkjscript text or storage objects directly.
- Do not use private Rust, Python, shell, generated source, or fixture builders as a second
  authoring interface.
- The CLI must support orientation, context, exact inspection, references, callers, types,
  capabilities, impact, creation, modification, refactoring, validation, apply, diff, history,
  restore, conflict resolution, tests, build, run, backup, restore, and deep doctor.
- Every normal response is bounded, deterministic, and machine-readable.
- Default success is the smallest complete summary, usually one value or aggregate line.
- Large values require field selection, item and byte budgets, pagination or continuation, and
  stable expansion handles.
- Do not print full schemas, graphs, passing tests, repeated environment facts, or unbounded logs by
  default.
- Keep exact verbose evidence in bounded artifacts and return digest and locator.
- Failures include compact actionable summary, high-value diagnostics, and exact expansion commands.
- Prefer descriptive fields over obscure abbreviations. Save tokens through locality, omission,
  references, caching, and schema stability.
- Caches are disposable and bind exact revision, query, schema, limits, tool identity, and every
  semantic input.


## Agent context and economy

- Treat model context, provider requests, output tokens, verification time, and correction depth as
  scarce without trading away evidence.
- Provide task-scoped context bundles derived from semantic ownership and dependency closure.
- A bundle states revision, inclusion reasons, omissions, byte and item counts, diagnostics, and
  expansion commands.
- Do not dump complete repositories or graphs when a bounded slice is enough.
- Keep a concise campaign ledger recording durable facts, decisions, risks, receipts, and next
  actions, not hidden reasoning.
- Measure output bytes and, when available, input tokens, cached input, output tokens, requests,
  retries, and monetary cost.
- Do not infer token or money savings from bytes alone.
- Compare equal complete tasks including failures and corrections.
- All-pass verification is quiet: one aggregate status and exact receipt locator.
- Retain bounded stdout and stderr per gate. Failure may show bounded head and tail excerpts with
  full locators.


## Graph storage and incremental computation

- Design for projects much larger than current consumers.
- Local query and mutation cost scales with relevant semantic slice plus validated impact, not whole
  repository by default.
- Use explicit indexes for names, owners, references, types, calls, capabilities, tests, packages,
  revisions, and impact where measurements justify them.
- Keep a simple independent reconstruction and validation oracle.
- Incremental validation, memoization, structural sharing, and prepared indexes are derived
  acceleration.
- Keys include every semantic input and full rebuild produces equivalent meaning.
- Bound depth, items, bytes, fanout, recursion, transaction work, query work, history, and
  concurrency through resource policy.
- Do not impose arbitrary file-count or line-count limits as resource accounting.
- Compaction and GC preserve reachable revisions and exact retention policy.
- Test interrupted publication, compaction, index rebuild, backup, and restore.


## Packages, language, components, and capabilities

- Packages and modules are first-class graph meaning, not filesystem conventions.
- Define package and module identity, namespaces, visibility, imports, exports, dependencies,
  cycles, initialization, and diagnostics.
- Exact dependencies bind immutable package identity, semantic revision, and artifact identity.
- No accepted build resolves mutable tags, ambient directories, undeclared network state,
  credentials, or current working directory behavior.
- Prefer a small orthogonal language core and reusable libraries over product-shaped primitives.
- Make evaluation order, equality, ordering, overflow, indexing, Unicode, serialization, effects,
  and allocation-relevant behavior explicit.
- Avoid implicit coercion, ambient overload resolution, order-dependent inference, and hidden
  authority.
- Expected outcomes are typed values; traps, capability failures, possible visibility, exhaustion,
  cancellation, corruption, and infrastructure failure remain distinct.
- Pure functions remain independent from deployment, time, randomness, scheduling, and external
  state.
- Components use typed ports. Command, HTTP, interactive, batch, worker, and test are runners or
  compositions over shared meaning.
- Applications declare typed requirements; deployment grants bind adapters, authority, secrets,
  sharing domains, and limits.
- Artifacts contain requirements, not grants or credentials.
- Adapters own generic mechanics and never application routes, schemas, authorization, object keys,
  UI behavior, retry policy, or domain transitions.
- Production and deterministic test adapters are behaviorally comparable and
  implementation-disjoint.
- Every live resource has exact acquisition, ownership, use, close, cancellation, timeout, and
  cleanup semantics and never enters durable meaning.


## Runtime, compiler, and performance

- Keep one runtime kernel for preparation, admission, execution, capability routing, task ownership,
  resource accounting, cancellation, shutdown, and observations.
- Concurrency is bounded and structured. Hidden unbounded queues and detached ownerless tasks are
  forbidden.
- Graceful shutdown defines admission stop, drain, cancellation, non-cancellable publication,
  cleanup, bounds, and exit status.
- A process boundary is neither semantic identity nor hostile-code sandbox.
- Maintain an independently checkable semantic execution route.
- Bytecode, specialized interpreters, AOT, JIT, and caches remain derived until equivalence,
  invalidation, accounting, and fallback are proved.
- Optimize complete maintained workloads, not isolated instruction folklore.
- Compilation and caches bind exact semantic revision, dependency closure, compiler contract,
  target, optimization policy, and relevant environment.
- Do not inflate fuel, limits, queues, or timeouts to hide algorithms.
- Measure cold and warm orientation, query, context, validation, apply, diff, merge, build, test,
  startup, execution, service, worker, backup, restore, and fresh checkout.
- Include realistic and adversarial graph size, fanout, history, and impact.
- Record wall time, CPU and memory where available, I/O, storage growth, output bytes, cache state,
  and semantic work counts.
- Require asymptotically sound data structures before micro-optimization and preserve reference
  differential paths.


## Verification, evidence, and documentation

- Use the narrowest sufficient gate during iteration and a complete authoritative gate before
  publication.
- Change-aware selection is convenience, not proof; uncertainty widens to full.
- Verify formatting, static analysis, locked builds, invariants, hostile decoders, graph
  publication, incremental/full equivalence, reference/production execution, properties, fuzzing,
  CLI contracts, migration rejection, application acceptance, backup, restore, restart,
  cancellation, overload, deterministic artifacts, fresh checkout, and git diff checks as
  applicable.
- Skipped, unavailable, flaky, timed-out, exhausted, cancelled, or unrun is not pass.
- Normative behavior belongs under docs/spec/. Current implementation belongs in status,
  architecture, performance, security, roadmap, README, and application docs by their owners.
- Evidence names exact commit, worktree, toolchain, platform, command, inputs, receipt, and
  limitations.
- Delete obsolete current documentation after direct cutover. Historical evidence may remain only
  when clearly labeled.


## Security and Rust engineering

- Treat persisted formats, CLI requests, artifacts, deployment descriptors, network input, database
  rows, object responses, and backups as hostile at decoding boundaries.
- Use closed contracts, exact bounds, duplicate and trailing rejection, checked arithmetic,
  canonical identity encodings, path and symlink defense, secret redaction, and explicit failures.
- Do not claim sandboxing, multi-tenant isolation, constant-time behavior, cryptographic security,
  portability, or distributed guarantees without evidence.
- Use stable Rust 2024 unless a verified campaign changes bootstrap.
- Prefer explicit ownership, checked arithmetic, bounded allocation, iterative traversal, narrow
  modules, and typed errors.
- Avoid unsafe; every unavoidable use needs a documented invariant, focused tests, and a clear
  justification.
- Dependencies need a named complete consumer and narrow features. Account for build time, binary
  size, security surface, and maintenance.
- Warnings are defects. Do not silence lints broadly. Avoid panic on untrusted or operational input.


## Forbidden shortcuts

- Do not maintain source and graph as independently editable truths.
- Do not make text the normal mutation path for lkjscript programs.
- Do not expose raw storage records as semantic CLI.
- Do not encode the graph as unbounded recursive JSON or one object per semantic atom.
- Do not require whole-project context for local work.
- Do not infer stable identity from name, path, position, or hash without an explicit rule.
- Do not use generated lkjscript source, private builders, or opaque fixtures as maintained
  authority.
- Do not add application-specific Rust business policy.
- Do not bypass typed requirements and grants with ambient host calls.
- Do not treat caches, indexes, bytecode, logs, or projections as authority.
- Do not preserve old formats through editions, aliases, fallback readers, or dual writers.
- Do not claim token or cost savings from output bytes alone.
- Do not print every passing test or full child log by default.
- Do not raise limits to hide poor algorithms.
- Do not introduce Lean files, toolchains, dependencies, experiments, or references.
- Do not commit hidden reasoning, secrets, or unrelated work.


## Completion standard

- [ ] one canonical semantic authority remains.
- [ ] public semantic CLI completes every changed workflow.
- [ ] maintained applications use public paths rather than private builders.
- [ ] text and predecessor paths are deleted or explicitly non-authoritative.
- [ ] identity revision transaction publication and failures are exact.
- [ ] local operations are bounded and whole-project work explicit.
- [ ] production and independent reference paths agree.
- [ ] quiet verification retains exact expandable evidence.
- [ ] maintained consumers pass acceptance and fresh checkout.
- [ ] security and performance claims match evidence.
- [ ] specs status architecture performance roadmap and user docs agree.
- [ ] obsolete code tests formats commands dependencies and docs are removed.
- [ ] staged and unstaged diffs contain only intended work.
- [ ] final handoff states commits verification publication limitations and unperformed actions.
