# AGENTS.md

## Scope

This file applies to the entire repository and to every coding agent that inspects, changes,
tests, measures, documents, commits, publishes, or reports on lkjscript.

- Use English for code, public APIs, diagnostics, tests, documentation, commit messages, and
  final engineering reports unless the active task explicitly requires another language for
  a user-facing artifact.
- Exercise autonomous technical judgment. Do not ask the user to choose among implementation
  alternatives when the checkout, accepted specifications, focused experiments,
  measurements, or a reversible local assumption can decide.
- Ask a question only when a genuinely external product requirement is missing and no safe,
  explicit assumption can complete the active objective.
- The user authorizes incompatible changes, destructive simplification, specification
  revision, representation replacement, crate and file reorganization, and deletion of
  obsolete work.
- Backward compatibility is not a repository objective unless the active task names a
  currently consumed external boundary that must remain compatible.
- Do not preserve old syntax, serialized bytes, command shapes, Rust APIs, module layouts,
  package layouts, cache formats, fixtures, tests, or prose merely because they existed.
- The authorization to make broad changes is permission, not an instruction to maximize
  scope. Prefer the smallest dependency-closed change that produces a complete product
  result.
- Preserve unrelated local work, credentials, host state, external data, and remote history.
  Repository redesign authority is not authority to erase unrelated state.
- The `.lkjscript` extension is fixed. Every other notation, grammar, schema, byte layout,
  command, package format, compiler representation, runtime representation, and persistence
  choice remains replaceable unless an accepted specification fixes it.

## Mission

Build lkjscript into an AI-primary, statically typed, memory-safe, collector-free,
high-performance programming language and implementation.

AI-primary means a coding agent can discover supported behavior, construct and modify
programs, inspect semantic facts, check without executing program effects, compile and run
intentionally, compare outcomes, receive compact actionable failures, and verify changes
through deterministic interfaces. A model may propose an operation; deterministic
implementation machinery decides validity.

- Keep model inference outside parsers, compilers, validators, optimizers, runtimes,
  persistence layers, and correctness boundaries.
- Do not optimize the language around one model, tokenizer, provider, prompt style,
  benchmark, or demonstration.
- Do not hide semantic meaning from humans or ordinary tooling.
- Do not wrap rendered prose in JSON and call it structured data.
- Do not multiply protocols, schemas, descriptors, registries, stores, services, or agent
  metadata merely because an agent could consume them.
- Do not build an agent platform before a complete, measured local programming workflow
  exists.
- Prefer one syntax-independent semantic authority, source-free construction, explicit
  incomplete states, typed atomic transactions, deterministic queries and diffs, direct
  compilation, one complete generic execution route, compact review projections,
  reproducibility, and measured performance.

## Authority and truth

1. The active task owns the current objective and explicit acceptance criteria.
2. This file owns repository-wide engineering procedure and decision discipline.
3. Accepted files under `docs/spec/` own intended external language and semantic-workspace
   contracts.
4. Code, tests, manifests, command definitions, and schemas own behavior in the checkout.
5. `docs/status.md` summarizes the current implementation and known gaps.
6. `docs/architecture.md` explains current responsibilities, data flow, ownership, and trust
   boundaries.
7. `docs/performance.md` owns measurement method, workload definitions, compact evidence,
   and reversal conditions.
8. `docs/roadmap.md` owns ordering and intent only; it creates no architectural commitment.
9. Sparse accepted files under `docs/decisions/` own durable rationale when such a decision
   exists.
10. Git history owns superseded implementation and prose.

- Classify a conflicting claim by dimension, inspect the owning artifact and executable
  evidence, decide which artifact is wrong, and update or delete stale material in the same
  coherent change.
- Use labels such as Current, Target, Hypothesis, Historical, Unknown, and Blocked when
  unlabelled prose would be misleading.
- A task prompt is transport, not durable authority. Do not commit prompts, copied
  conversation context, transcript summaries, handoffs, checkpoints, completion capsules, or
  private scratch plans.
- Do not create another authority layer from a plan tree, digest, ledger, global revision,
  generated inventory, closure graph, or status shard.
- When a better design changes intended semantics, update the owning specification and
  perform one direct cutover. Do not silently contradict the specification or retain an
  obsolete compatibility route.

## Priority order

1. Coherent language semantics.
2. Memory safety, ownership, cleanup, failure atomicity, and deterministic meaning.
3. Scale-safe algorithms and representations without arbitrary language-validity quotas.
4. One complete semantic workspace and direct compiler input.
5. A small deterministic local development loop for coding agents.
6. Representative edit, query, compile, run, output, memory, and allocation measurements
   before incremental machinery.
7. One complete measured generic execution path before additional specialization.
8. Self-hosting and broader products only after lower layers are coherent.
9. Persistence, collaboration, daemonization, scheduling, and distribution only after a
   present consumer and measurements justify those boundaries.

Do not skip a prerequisite because a later platform idea is more exciting. Brainfuck, one
fixture, one benchmark, or one generated stress shape may expose a defect; none defines the
language.

## Multi-turn engineering contract

- One turn should normally complete one coherent product vertical, not attempt the whole
  roadmap.
- Select the highest-leverage dependency-closed problem supported by current evidence.
- State a falsifiable hypothesis, completion criteria, reversal condition, and stop
  condition in temporary working state.
- Fix a correctness, safety, or authority prerequisite inside the same vertical when it
  blocks completion.
- Do not use incidental findings as permission for an unrelated rewrite.
- Leave the repository coherent, documented, tested, and usable after every turn.
- Do not leave two active architectures, a half-cutover, disabled correctness checks, an
  unfinished required migration, or stale prose presented as current.
- Stop after the selected vertical is complete, the roadmap is updated, and the next
  highest-leverage problem is identified. Do not begin it merely to appear more ambitious.

## Evidence-first work selection

- Begin from a demonstrated defect, an accepted specification gap, a current roadmap item,
  or an explicit product requirement.
- Trace producers, consumers, mutable authority, derived representations, trust boundaries,
  failure paths, and current tests before choosing a mechanism.
- Characterize current behavior with the smallest focused test or measurement that can
  falsify the hypothesis.
- Fix the dependency-closed root cause rather than the most visible symptom.
- Change course when focused evidence disproves the proposed mechanism.
- Prefer deletion and semantic simplification before introducing machinery.
- Do not cite intuition as performance, scale, output-volume, or API-cost evidence when
  direct counters or measurements are practical.

## One active architecture and direct cutovers

- Maintain one active language definition, semantic authority, compiler path, ownership
  model, generic production execution route, package model, and documentation authority
  model for each current product boundary.
- Do not create editions, permanent `v2` systems, `next` trees, legacy modes, compatibility
  layers, or parallel canonical representations.
- A small independent evaluator may remain as a test oracle; it is not automatically a
  second production engine.
- Names and current crate boundaries have no authority. Preserve, merge, split, rename, or
  delete components according to cohesion, real ownership, safety boundaries, independently
  useful APIs, measured compile isolation, coupling, and current consumers.
- When architecture causes the defect, replace it instead of surrounding it with registries,
  adapters, caches, synchronization bookkeeping, or migration scaffolding.

1. Identify the intended replacement.
2. Update the owning specification when semantics change.
3. Update every active producer and consumer.
4. Replace active fixtures and examples.
5. Delete the displaced implementation and contract.
6. Delete adapters, aliases, feature flags, migration code, and compatibility tests.
7. Remove stale exports, dependencies, and documentation.
8. Verify that exactly one active route remains.

## Semantic authority

- Authoritative semantic state must be able to exist without source text, formatting, paths,
  spans, parser nodes, source hashes, or compiler-dense indexes.
- Source, files, formatting, and spans may be importer, presentation, provenance, cache, or
  interoperability attachments; they are not compiler or editing authority.
- Do not satisfy an invariant with dummy files, placeholder paths, fabricated hashes,
  synthetic declarations, fake entry points, reserved placeholder identities, hidden
  executable bodies beneath holes, or fallback executable meaning.
- One representation owns mutable semantic facts. Every derived representation needs a
  current producer, consumer, lifetime, invalidation rule, and deletion condition.
- Dense IDs, vector positions, physical slots, offsets, and layout indexes remain private
  and replaceable.
- Compilation consumes one complete semantic snapshot directly. Do not render and reparse
  source to cross an internal compiler boundary.

## Identity, revisions, and incomplete state

- Use opaque logical identity only where meaning must survive rename, movement, or private
  compaction.
- For every public identity, define what it identifies, its uniqueness lifetime, allocator,
  namespace and generation validation, survival rules, and tombstone condition.
- Names, paths, spans, formatting, source order, and hashes are not universal mutable
  identity.
- Surviving public identities remain stable when meaning survives private compaction. Old
  immutable snapshots remain valid.
- Incomplete semantic state is valid editing state.
- Missing declarations, bodies, expressions, references, choices, and conflict resolutions
  must be explicit blockers, holes, or recovery facts.
- Never retain an executable fallback behind an incomplete node.
- Compilation of an incomplete snapshot stops before ownership planning, memory planning,
  SSA, bytecode, native code, or execution.

## Transactions and public semantic APIs

- Transactions publish one coherent final revision or nothing.
- Validate namespace, generation, kind, owner, and base revision before mutation.
- Failure must not consume stable identities, mutate allocator state, change the published
  snapshot, poison caches, or partially publish derived state.
- When a batch promises order independence, validate the intended final semantic graph
  rather than edit-list order.
- Containment and dependency are different. Container deletion may remove facts whose
  semantic existence it owns; it must not silently delete independent dependents.
- Public APIs expose semantic meaning, not parser nodes, display strings, private addresses,
  dense indexes, debug formatting, or unvalidated compiler objects.
- Use one structured public model per concept unless input and output have genuinely
  different semantics.
- Transaction-local handles are acceptable before stable entities exist, but they must be
  typed, scoped, validated, non-persistent, and impossible to confuse with stable identity.
- Do not proliferate `Input`, `View`, `Ref`, `Descriptor`, `Resolved`, `Wire`, and DTO
  variants for conversion convenience.
- A display message may accompany structured data; it must not be the only machine-readable
  meaning when the producer knows structured facts.
- Do not parse a rendered diagnostic to reconstruct facts its producer already knew.
- Machine-facing results are deterministic, schema-explicit, stably ordered,
  completeness-explicit, bounded or paginated where necessary, and never silently truncated.
- Public recursive values must be safe to construct, clone, compare, hash where required,
  project, validate, convert, and destroy without unbounded native stack.

## Types, generics, ownership, and execution

- Generic declarations, substitutions, bounds, instantiations, and witnesses are semantic
  facts, not parser decoration.
- Source import and source-free editing converge on one exact instantiation and
  trait-selection path.
- Inference is an authoring convenience; exact substitutions and compiler-derived witnesses
  are the result.
- Do not introduce a general higher-rank framework, implicit inference engine, or generic
  rewrite system before a current language requirement needs it.
- Ordinary execution is collector-free and non-tracing.
- Do not add tracing collection, language-visible hidden reference counting, raw-pointer
  language surfaces, retain/release, general `free`, or parallel GC and non-GC modes to
  simplify implementation.
- Preserve exact move and borrow laws, deterministic cleanup where promised, cleanup on
  normal and all failure exits, no double release, stack-safe destruction, explicit
  host-resource ownership, and failure-atomic publication.
- Maintain one complete generic production execution route.
- Optional native or specialized execution may decline only before effects and must leave
  the generic route intact.
- Once specialized entry begins, its result or failure is final. Never re-execute effects
  through fallback.
- Validate fail-closed at real untrusted boundaries. Inside one trusted typed synchronous
  pipeline, do not repeatedly serialize, hash, reconstruct, or independently revalidate
  values without a consumer boundary.
- Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented
  safe-caller contract, focused malformed-input tests, and appropriate Miri, sanitizer,
  fuzz, or property coverage.

## Scale and resource policy

- Language validity follows semantic laws, not project-selected size quotas.
- Do not reject a trusted program because it exceeds an arbitrary count of bytes, tokens,
  nesting levels, declarations, fields, variants, parameters, arguments, locals, functions,
  files, modules, blocks, IR nodes, identities, values, diagnostics, handles, or analysis
  steps.
- Do not disguise a limit by raising it, widening an integer, moving it, renaming it, or
  calling it a safety profile.
- Use checked arithmetic and checked narrowing at representation boundaries.
- User-controlled depth must not consume unbounded native stack; use iterative traversal or
  a justified heap-backed work stack.
- An untrusted product may impose explicit coarse input, memory, output, elapsed-time,
  cancellation, and concurrency policy.
- Resource exhaustion is a typed host result, not a semantic error.
- Do not design detailed untrusted policy before an actual untrusted product exists.

## AI-facing local development

1. Discover supported language and tool behavior.
2. Create or modify a program through an authoritative interface.
3. Check it without executing program effects.
4. Inspect compact actionable structured results.
5. Run it intentionally.
6. Verify the exact outcome and relevant side effects.

- Meet this workflow with the smallest complete local boundary.
- Prefer authoritative executable examples, concise authoring documentation, a compile-only
  command, structured diagnostics, and one-shot operations over a long-lived process.
- Add a daemon only after measurements show process startup or repeated import dominates a
  real workflow.
- Do not infer that an agent workflow needs a database, journal, session broker, scheduler,
  network protocol, CRDT, persistent semantic store, or broad agent framework.
- A check command must not execute program effects. A run command is intentional execution.
- Human and machine renderers may share structured facts but must not parse each other.
- Command names, arguments, exit behavior, stdout, and stderr are deterministic and tested.
- Successful high-frequency validation is quiet by default.
- A one-shot command must not pretend identities survive across invocations without a real
  namespace and snapshot lifetime.

## Attention, output, and API-cost discipline

Model context, tool output, developer attention, wall time, and API spend are engineering
resources. Reduce irrelevant work and output without weakening correctness or hiding
evidence.

- Search before opening large files.
- Read focused ranges and focused diffs before full files.
- Retrieve full detail only for a failing, ambiguous, or cross-cutting portion.
- Do not repeatedly dump unchanged files, repository-wide diffs, generated IR, machine code,
  massive JSON, or complete projections into context.
- Run the smallest command that can falsify the current hypothesis.
- Prefer a focused test over a crate test and a crate test over a workspace test during
  iteration.
- Do not repeat an identical successful command when no relevant input changed.
- Reserve the complete required suite for the final relevant state.
- Quiet success still requires a known command, completed exit status, and known boundary.
- Never hide a non-zero status, compiler error, test failure, warning promoted by policy,
  sanitizer finding, fuzz failure, malformed-output failure, or environment error.
- Do not use `|| true`, broad filtering, or redirection to make a failing command appear
  successful.
- For noisy successful commands, prefer a native quiet flag. Otherwise capture the complete
  log outside Git and report a bounded summary; expose the relevant section and full-log
  path on failure.
- Machine commands emit one deterministic document or documented stream without progress
  chatter.
- Do not add a task runner, command registry, output broker, logging framework, cache, or
  service merely to silence commands.
- When agent efficiency is an objective, measure command count, round trips, stdout and
  stderr bytes, line count, duplicate diagnostics, wall time, repeated work, and context
  needed for the next decision.
- Do not claim provider-token or billing reduction from byte counts alone. Report exactly
  what was measured.

## Anti-overengineering gate

Before adding an abstraction, identify its present problem, producer, consumer, authority,
lifetime, invalidation, failure behavior, measurable or structural benefit, why local code
is insufficient, and deletion condition.

- An abstraction should remove meaningful duplication, make an invalid state
  unrepresentable, isolate a real boundary, expose a current useful API, enable a measured
  property, or materially simplify reasoning.
- The mechanism must be smaller than the problem and must not duplicate authority.
- Prefer deletion of unused work first.
- Then prefer semantic or representation simplification.
- Then reuse an existing invariant or canonical validator.
- Then replace repeated discovery with one local derived fact.
- Then improve layout or traversal.
- Then make an invalid state unrepresentable.
- Add caching only after measured repeated work.
- Add parallelism only for large separable work.
- Add target specialization only behind a complete generic route and measured need.
- Do not refactor unrelated code for symmetry, aesthetics, novelty, or theoretical
  completeness.
- Prefer a small explicit mechanism when only one current use exists.
- Prefer deletion over documenting machinery without a consumer.
- Do not solve complexity with bookkeeping.

Do not build speculative daemons, services, sessions, process boundaries, persistence,
journals, databases, distributed stores, CRDTs, schedulers, resource topologies, process
cells, custom allocators, universal registries, descriptor systems, plugin frameworks,
rewrite DSLs, general incremental or cache frameworks, proof ecosystems, wire protocols,
broad target matrices, multi-tier JIT policy, deoptimization, PGO machinery, platform
products, or self-hosting scaffolding.

A task may introduce one of those mechanisms only with a demonstrated present boundary, an
end-to-end current consumer, measured need, explicit acceptance criteria, explicit ownership
and failure behavior, and a reversal condition.

- Do not create a universal graph engine for one walk.
- Do not create a rewrite framework for a few identity remaps.
- Do not create an event system for one synchronous result.
- Do not create a cache for unmeasured work.
- Do not create a protocol merely because an in-process API exists.
- Do not create a daemon for hypothetical tools.
- Do not create an interner merely because types are recursive.
- Do not create an arena merely to avoid one traversal.
- Do not create a trait hierarchy to share two short functions.
- Do not create a diagnostic framework to preserve one diagnostic.
- Do not create a command framework for one command.
- Do not impose numeric file-length, directory-width, directory-depth, module-count,
  plan-count, or repository-shape rules.

## Performance and measurement

- Profile before optimizing and measure the selected product path.
- State equivalent semantics, workload, environment, build and cache state, sample protocol,
  selection criterion, reversal condition, and stop condition before comparison.
- Relevant evidence may include wall and phase time, startup, throughput, edit and query
  latency, peak and retained memory, allocations and bytes, copied or parsed bytes, rendered
  or serialized bytes, stdout and stderr bytes, line and command count, agent round trips,
  traversal counts, scale behavior, generated-code size, and binary size.
- Prefer deterministic work counters over noisy timing when they answer the question.
- Generated scale tests establish correctness and complexity shape; they do not establish
  application performance.
- Keep raw samples outside Git and commit only compact reproducible evidence.
- Do not turn developer-machine noise into a correctness gate.
- Keep an optimization only when end-to-end benefit justifies compile time, memory, code
  size, complexity, tests, and maintenance.
- Full recomputation may remain correct until representative edits justify incrementality.
- Temporary instrumentation should be test-only or measurement-only and should be removed
  when it has no continuing consumer.
- Do not claim latency, memory, allocation, output-volume, throughput, or API-cost
  improvement without equivalent evidence.

## Implementation workflow

1. Inspect branch, worktree, upstream state, and recent history without destroying unrelated
   work.
2. Read the authority documents relevant to the task.
3. Map producers, consumers, authority, derived state, and failure paths.
4. Characterize current behavior with focused tests or measurements.
5. Choose the highest-leverage dependency-closed problem.
6. Record a falsifiable hypothesis, completion criteria, reversal condition, and stop
   condition in temporary state.
7. Implement the smallest coherent root-cause correction.
8. Delete the displaced path and stale claims.
9. Add focused regression, convergence, and scale tests as relevant.
10. Update the owning documentation in the same change.
11. Run final verification after the final relevant edit.
12. Commit cohesive changes when permitted.
13. Push only when publication is explicitly requested.

## Structure and dependencies

- Organize code by coherent responsibility, not line counts or symmetry.
- A crate boundary should represent a real trust or unsafe boundary, an independently useful
  library, a supported target, measured compile isolation, or a low-coupling subsystem.
- Merge crates that mainly exchange internal descriptors, re-exports, or adapters.
- Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny
  modules, redundant models, and conversion layers when recombination improves ownership and
  retrieval.
- Split a large module only when the split establishes ownership and reduces change
  coupling.
- Use mature dependencies when they remove substantial machinery or risk.
- Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better.
- Do not add a dependency for functionality clearer as local code.
- Do not add a benchmark, logging, serialization, or allocation framework when a small
  current-purpose harness is sufficient.

## Tests

- Tests protect intended semantics and public invariants, not accidental topology.
- Cover relevant type, generic, trait, effect, capability, ownership, control-flow, and
  cleanup laws.
- Cover completeness and explicit incomplete state.
- Cover stable identity, namespaces, generations, revisions, deletion, and deterministic
  ordering.
- Cover malformed, stale, foreign, wrong-kind, wrong-owner, duplicate, missing, and
  invisible input.
- Cover transaction and artifact failure atomicity.
- Cover exactly-once effects and generic/specialized equivalence where specialization
  exists.
- Cover cancellation, resource failure, host error, and cleanup when relevant.
- Cover deep input, deep destruction, scale, and checked representation boundaries when
  user-controlled depth is involved.
- Cover machine-output decoding, determinism, stdout, stderr, and exit behavior for machine
  interfaces.
- Cover effect-free checking and real product integration.
- Add a focused regression test for each root cause.
- Use generated fixtures for scale; keep fast default tests separate from ignored
  locked-release stress while exercising the same algorithm at smaller size.
- Use differential, property, model, or test-only reference implementations when an
  independent oracle is cheap.
- Delete tests that preserve provisional syntax, old bytes, obsolete APIs, deleted
  machinery, arbitrary limits, private topology, or accidental details.
- Never weaken a test merely to make a redesign pass.
- A convergence test compares semantic outcomes, not only text.
- A failure-atomicity test verifies the prior snapshot and allocator remain unchanged.
- A stack-safety test covers construction, transformation, and destruction on a small stack
  where user depth matters.
- A machine test decodes output as a consumer would; do not validate JSON by substring
  matching.
- A quiet-success test asserts both streams empty.
- A no-effects check test uses a program whose execution would be observable.

## Documentation

- `README.md` owns product introduction and first successful use.
- `docs/spec/` owns intended external semantics and target contracts.
- `docs/status.md` owns current implementation and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust
  boundaries.
- `docs/performance.md` owns method, workloads, compact evidence, and reversal conditions.
- `docs/roadmap.md` contains only Now, Next, and Later.
- `docs/decisions/` contains sparse durable decisions.
- Update the owning document and delete stale text in the same change.
- Do not add digests, global revisions, fact shards, copied tables, per-commit evidence
  records, transcripts, handoffs, prompt archives, completion capsules, or duplicate
  roadmaps.
- Write a decision only when a choice is durable, non-obvious, expensive to rediscover, and
  has a meaningful reversal condition.
- Do not describe target as current, hypothesis as measurement, private movement as public
  movement, planned systems as supported, or developer-machine observation as a product
  guarantee.
- Examples must use active APIs and semantics.
- Agent-facing documentation should be executable or mechanically checked where practical.
- Do not maintain a hand-written capability table when authoritative implementation facts
  can answer the same question.

## Git and publication

- Inspect worktree and branch state before editing.
- Preserve unrelated tracked and untracked work.
- Do not use destructive reset, checkout, clean, history rewrite, or force push against work
  you did not create.
- Commit only coherent repository changes.
- Exclude prompts, raw logs, raw benchmark samples, temporary plans, generated scratch
  files, credentials, and unrelated changes.
- Use a commit message naming the semantic or architectural result.
- Push only when the active task requests publication.
- Never force push for convenience.
- After a requested push, verify the local branch, tracking branch, and pushed commit.
- If publication fails, preserve the verified local commit and report the exact failure.

## Verification

During iteration, run the smallest focused command that can disprove the current change.

After the final relevant edit, run at least the semantic equivalent of:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Run retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

- Run additional focused release stress, differential, property, small-stack, deep-input,
  malformed-boundary, cancellation, allocation-failure, Miri, sanitizer, fuzz, benchmark,
  documentation, example, and machine-output checks when relevant.
- Run the full suite after the final relevant change, not repeatedly after unchanged inputs.
- If the environment blocks a command, report the exact command, failure category, relevant
  output, whether the change caused it, successful remaining evidence, and unverified risk.
- Never claim a command passed when it did not complete successfully.

## Final report

- Report the completed objective and root cause.
- Report the principal design and any replaced or deleted paths.
- Report focused tests, convergence evidence, and measurements.
- Report output-volume and API-cost evidence when relevant.
- Report exact verification commands and outcomes.
- Report environment-limited checks and remaining risk.
- Report documentation changes.
- Report commit and publication state.
- Name the next highest-leverage problem and why work stopped before beginning it.
- Keep the report factual and compact.
- Do not reproduce the prompt, paste complete successful logs, or claim future work is
  implemented.
