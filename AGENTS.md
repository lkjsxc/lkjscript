# AGENTS.md

## Scope and operating contract

- This file applies to the entire repository and to every agent that inspects, changes, tests,
  measures, documents, commits, publishes, or reports on it.
- Use English for code, comments, public APIs, diagnostics, tests, documentation, commit
  messages, measurement labels, and final engineering reports unless the active task explicitly
  requires a different language for one user-facing artifact.
- Exercise autonomous technical judgment. The user authorizes incompatible language changes,
  destructive simplification, specification revision, representation replacement, crate and file
  reorganization, and deletion of obsolete work.
- Backward compatibility is not a repository objective unless the active task identifies one
  currently consumed external boundary that must remain compatible.
- Do not preserve old syntax, serialized bytes, command shapes, Rust APIs, module layouts,
  package layouts, cache formats, compiler representations, runtime representations, fixtures,
  tests, or prose merely because they existed.
- The user's authorization permits decisive work; it does not require maximal scope. Prefer the
  smallest dependency-closed change that completes one product result.
- Historical prompts, prior assistant answers, old plans, branches, and Git history are context,
  not permanent requirements.
- Preserve unrelated local work, credentials, machine state, external data, and remote history.
- The `.lkjscript` extension is fixed. Every other notation, grammar, schema, byte layout,
  command, package format, compiler representation, runtime representation, and persistence
  decision remains replaceable unless an accepted specification fixes it.
- Do not ask the user to choose between technical alternatives when the checkout, accepted
  specification, focused test, measurement, or reversible local assumption can decide.
- Ask only when a genuinely external product requirement is missing and no safe assumption can
  complete a useful vertical.

## Mission

Build lkjscript into an AI-primary, statically typed, memory-safe, collector-free,
high-performance programming language and implementation.

- AI-primary means a coding agent can discover supported behavior, construct and modify semantic
  programs, inspect exact facts, check without executing program effects, compile and run
  intentionally, compare outcomes, receive compact actionable failures, and verify changes
  through deterministic interfaces.
- A model may propose operations. Deterministic implementation machinery decides validity.
- Do not place model inference inside a parser, compiler, validator, optimizer, runtime, storage
  layer, or correctness boundary.
- Do not optimize the language around one model, tokenizer, provider, prompt style, benchmark,
  or demo.
- Do not hide semantic meaning from humans or ordinary tooling.
- Do not wrap display strings in JSON and call the result structured.
- Do not multiply protocols, schemas, descriptors, registries, stores, services, or agent
  metadata merely because an agent could consume them.
- Do not build an agent platform before a complete local programming workflow exists.
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
5. `docs/status.md` summarizes current implementation and known gaps.
6. `docs/architecture.md` explains current responsibilities, flow, ownership, and trust
   boundaries.
7. `docs/performance.md` owns measurement method and compact reproducible evidence.
8. `docs/roadmap.md` owns ordering only; it does not create an architectural commitment.
9. Sparse accepted decisions own durable non-obvious rationale.
10. Git history owns superseded implementation and prose.
- Use `docs/authority.md` to resolve ownership by claim dimension.
- Classify a conflicting claim, inspect its owning artifact and executable evidence, decide
  which artifact is wrong, and update or delete stale material in the same coherent change.
- Use Current, Target, Hypothesis, Historical, Unknown, and Blocked labels when unlabelled prose
  would mislead.
- A task prompt is transport, not durable authority. Do not commit prompts, transcript
  summaries, handoffs, checkpoints, completion capsules, or copied context.
- Do not create another authority layer from a plan tree, digest, ledger, global revision,
  generated inventory, closure graph, status shard, or benchmark diary.
- A roadmap item is ordering, not proof that its anticipated mechanism is correct.
- A test proves only what it exercises. A benchmark proves only its stated workload and
  protocol.
- When a better design changes intended semantics, update the owning specification and perform
  one direct cutover. Do not silently contradict the specification or retain an obsolete
  compatibility route.

## Priority order

1. Coherent language semantics.
2. Memory safety, ownership, cleanup, failure atomicity, and deterministic meaning.
3. Scale-safe algorithms and representations without arbitrary language-validity quotas.
4. One complete semantic workspace and direct compiler input.
5. A small deterministic local development loop for coding agents.
6. Representative edit, query, projection, compile, run, output, memory, and allocation evidence
   before incremental machinery.
7. One complete measured generic execution path before additional specialization.
8. Self-hosting and broader products only after lower layers are coherent.
9. Persistence, collaboration, daemonization, scheduling, and distribution only after a present
   consumer and measurements justify those boundaries.
- Do not skip a prerequisite because a later platform idea is more exciting.
- A benchmark, generated stress shape, or demo may expose a defect; none defines the language.
- The current `Now` roadmap item governs normal work selection until evidence or an explicit
  active task establishes a more severe prerequisite.

## Evidence-first work selection

- Begin from a demonstrated defect, accepted specification gap, current roadmap item, or
  explicit product requirement.
- Trace producers, consumers, mutable authority, derived representations, trust boundaries,
  failure paths, current tests, and present measurements before choosing a mechanism.
- State a falsifiable hypothesis, completion criteria, reversal condition, and stop condition in
  temporary working state.
- Run the smallest command that can falsify the current hypothesis.
- Fix the dependency-closed root cause rather than the most visible symptom.
- Change course when focused evidence disproves the proposed mechanism.
- Fix a blocking correctness, safety, or authority prerequisite inside the same vertical, but do
  not use incidental findings as permission for an unrelated rewrite.
- One turn should normally complete one product vertical.
- Multi-turn progress is expected; future representations and platforms need not be decided now.
- Leave the repository coherent, documented, tested, and usable after every turn.
- A measurement-only result is complete when it supports retaining a simpler architecture.
- Do not construct evidence around a preselected favorite mechanism.

## One active architecture and direct cutovers

- Maintain one active language definition, mutable semantic authority, compiler path, ownership
  model, complete generic production execution route, package model, and documentation authority
  model for each current product boundary.
- Do not create editions, permanent `v2` systems, `next` trees, legacy modes, compatibility
  layers, or parallel canonical representations.
- A small independent evaluator may remain as a test oracle; it is not automatically a second
  production engine.
- Names and current crate boundaries have no authority.
- Preserve, merge, split, rename, or delete components according to cohesion, real ownership,
  safety boundaries, independently useful APIs, measured compile isolation, coupling, and
  current consumers.
- When architecture causes the defect, replace it instead of surrounding it with registries,
  adapters, caches, synchronization bookkeeping, or another descriptor layer.
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
- Source, files, formatting, spans, and hashes may be importer, presentation, provenance, cache,
  or interoperability attachments; they are not compiler or editing authority.
- Text may construct semantic state. Compilation, editing, querying, validation, and correctness
  must not require rendering and reparsing it.
- Do not satisfy an invariant with dummy files, placeholder paths, fabricated hashes, synthetic
  declarations, fake entry points, reserved placeholder identities, hidden executable bodies
  beneath holes, or fallback executable meaning.
- One representation owns mutable semantic facts.
- Every derived representation needs a current producer, consumer, lifetime, invalidation rule,
  failure behavior, and deletion condition.
- Derived representations are not coequal authorities.
- Dense IDs, vector positions, slots, offsets, code addresses, and layout indexes remain private
  and replaceable.
- Do not serialize, hash, reconstruct, or independently revalidate a typed in-process value
  unless a real untrusted, persistence, cache, transfer, or executable-artifact boundary needs
  it.

## Identity

- Use opaque logical identity only where meaning must survive rename, movement, formatting, or
  private compaction.
- State what an identity identifies, its namespace, uniqueness lifetime, allocator, validation,
  survival rules, stale behavior, and tombstone condition.
- Names, paths, spans, formatting, source order, and hashes are not universal mutable identity.
- Stable public identity remains separate from compiler-dense identity.
- Surviving public identities remain stable when semantic meaning survives private compaction.
- Deleted identities tombstone and cannot silently resolve to later meaning.
- Old immutable snapshots remain valid while retained.
- Transaction-local handles are acceptable before stable entities exist, but they must be typed,
  scoped, validated, non-persistent, and impossible to confuse with stable identity.
- Do not proliferate Input, View, Ref, Descriptor, Resolved, Wire, and DTO variants for
  conversion convenience.

## Incomplete state

- Incomplete semantic state is valid editing state.
- Missing declarations, bodies, expressions, references, choices, and conflict resolutions must
  be explicit blockers, holes, unresolved facts, ambiguities, conflicts, or recovery nodes.
- Never retain executable fallback meaning behind an incomplete node.
- Preserve every sound fact available around an error.
- Diagnostics should attach to semantic identities when those identities exist.
- Compilation of an incomplete snapshot stops before ownership planning, memory planning, SSA,
  bytecode, native code, or execution.
- Queries and projections remain available for incomplete snapshots.
- Do not fabricate a complete state to simplify downstream code.

## Transactions

- Transactions publish one coherent final revision or nothing.
- Validate namespace, generation, kind, owner, base revision, and operation shape before
  publication.
- Validate deletion and dependency semantics against the intended final staged semantic graph
  when the operation contract promises order independence.
- Containment and dependency are different.
- Container deletion may remove facts whose semantic existence it owns; it must not silently
  delete independent dependents.
- Failure must not consume stable identities, mutate allocator state, change the published
  snapshot, poison retained derived state, or partially publish diagnostics.
- Allocation failure, cancellation, host failure, and resource-policy exhaustion remain
  failure-atomic.
- Structural replacement may preserve the targeted root identity while descendants receive new
  identity according to the accepted workspace contract.
- A semantic diff reports semantic change, not private relocation.
- Do not make edit-list order observable when final-state semantics are specified as order
  independent.
- Do not turn a partial edit log into semantic authority.

## Queries, projections, and machine interfaces

- Public APIs expose semantic meaning, not parser nodes, display strings, private addresses,
  dense indexes, debug formatting, or unvalidated compiler objects.
- Use one structured public model per concept unless input and output have genuinely different
  semantics.
- Queries are deterministic and revision-labelled.
- Continuations bind namespace, revision, query identity, and offset or equivalent exact state.
- Large results use stable ordering, filters, pagination, and explicit completeness.
- Never silently truncate.
- Compact headers and identities should permit selective expansion.
- Projections are review and debug views, not identity input or semantic authority.
- Projection labels are response-local spellings unless an accepted contract says otherwise.
- A display message may accompany structured data; it must not be the only machine-readable
  meaning when the producer knows structured facts.
- Do not parse a rendered diagnostic or projection to reconstruct facts its producer already
  knew.
- Human and machine renderers may share structured facts but must not parse each other.
- Machine commands emit one deterministic document or documented stream without progress
  chatter.
- Successful high-frequency human validation is quiet by default.
- Command names, arguments, exit behavior, stdout, and stderr are deterministic and tested.
- A one-shot command must not pretend identities survive across invocations without a real
  namespace and snapshot lifetime.

## Types, generics, traits, and effects

- Generic declarations, binders, substitutions, bounds, instantiations, and witnesses are
  semantic facts, not parser decoration.
- Source import and source-free editing converge on one exact instantiation and trait-selection
  path.
- Inference is an authoring convenience; exact substitutions and compiler-derived witnesses are
  the result.
- Stable semantic or explicit builtin identities represent nominal types, binders, and traits at
  public boundaries.
- Do not expose compiler-local names, dense IDs, layout IDs, or binder strings as universal
  identity.
- Do not introduce a general higher-rank framework, implicit inference engine, or generic
  rewrite system before a current language requirement needs it.
- Unknown effects remain explicit while semantic state is incomplete.
- Effect summaries must be exact enough for checking, optimization, queries, and review; do not
  fabricate purity from missing analysis.
- Public recursive type values must be safe to construct, clone, compare, hash where required,
  convert, project, and destroy without unbounded native stack.

## Ownership, cleanup, and execution

- Ordinary language execution is collector-free and non-tracing.
- Do not add tracing collection, language-visible hidden reference counting, raw-pointer
  language surfaces, retain/release, general `free`, or parallel GC and non-GC modes to simplify
  implementation.
- Preserve exact move and borrow laws.
- Preserve deterministic cleanup where promised.
- Cleanup must occur on normal return, early return, trap, call failure, policy failure, and
  teardown according to ownership.
- Prevent double release, use after move, stale loan, and hidden aliasing.
- Affine values may transfer to callers only through explicit canonical semantics.
- Host resources have explicit ownership and typed failure behavior.
- Maintain one complete generic production execution route.
- Optional native or specialized execution may decline only before effects.
- Once specialized entry begins, its result or failure is final.
- Never re-execute effects through fallback.
- Validate fail-closed at real untrusted boundaries.
- Inside one trusted typed synchronous pipeline, do not repeatedly serialize, hash, reconstruct,
  or revalidate values without a consumer boundary.
- Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented
  safe-caller contract, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or
  property coverage.

## Scale and resource policy

- Language validity follows semantic laws, not project-selected size quotas.
- Do not reject a trusted program because it exceeds an arbitrary count of bytes, tokens,
  nesting levels, declarations, fields, variants, parameters, arguments, locals, functions,
  files, modules, blocks, IR nodes, identities, values, diagnostics, handles, or analysis steps.
- Do not disguise a limit by raising it, widening an integer, moving it, renaming it, or calling
  it a safety profile.
- Use checked arithmetic and checked narrowing at representation boundaries.
- User-controlled depth must not consume unbounded native stack; use iterative traversal or a
  justified heap-backed work stack.
- Resource exhaustion is a typed host result, not a semantic error.
- An untrusted product may impose explicit coarse input, memory, output, elapsed-time,
  cancellation, and concurrency policy.
- Do not design detailed untrusted policy before an actual untrusted product exists.
- Generated scale tests establish correctness and complexity shape; they do not establish
  representative application performance.
- Benchmark geometry and test parameters are not public validity policy.

## Performance and evidence

- Profile before optimizing and measure the selected product path.
- State the hypothesis, equivalent semantics, workload, environment, build and cache state,
  sample protocol, selection criterion, reversal condition, and stop condition before
  comparison.
- Prefer deterministic work counters over noisy timing when they answer the question.
- Relevant evidence may include wall and phase time, startup, throughput, edit and query
  latency, peak and retained memory, allocations and bytes, copied or parsed bytes, rendered or
  serialized bytes, output lines, command count, round trips, traversal counts, scale behavior,
  generated-code size, and binary size.
- Keep raw samples outside Git and commit only compact reproducible evidence.
- Do not turn developer-machine noise into a correctness gate.
- Full recomputation may remain correct and preferable until representative edits justify more
  machinery.
- Do not add incrementality, caching, parallelism, specialization, or warm processes from
  intuition.
- Keep an optimization only when end-to-end benefit justifies compile time, memory, code size,
  complexity, tests, and maintenance.
- Do not claim latency, memory, allocation, output-volume, throughput, or API-cost improvement
  without equivalent evidence.
- Missing observations are Unknown or null, not measured zero.
- A local one-purpose correction is preferred over a general framework.

## AI-facing local development

1. Discover supported language and tool behavior.
2. Create or modify a program through an authoritative interface.
3. Check it without executing program effects.
4. Inspect compact actionable structured results.
5. Run it intentionally.
6. Verify the exact outcome and relevant side effects.
- Meet this workflow with the smallest complete local boundary.
- Prefer authoritative executable examples, concise authoring documentation, an effect-free
  check command, structured diagnostics, and one-shot operations over a long-lived process.
- Add a daemon only after measurements show process startup or repeated import dominates a real
  workflow.
- Do not infer that an agent workflow needs a database, journal, session broker, scheduler,
  network protocol, CRDT, persistent semantic store, or broad agent framework.
- A check command must not execute program effects.
- A run command is intentional execution.
- Legal-constructor or action queries must distinguish established legality from candidates that
  still require canonical ownership or contextual validation.

## Attention, output, and API-cost discipline

- Model context, tool output, developer attention, wall time, and API spend are engineering
  resources.
- Reduce irrelevant work and output without weakening correctness or hiding evidence.
- Search before opening large files.
- Read focused ranges and focused diffs before full files.
- Retrieve full detail only for a failing, ambiguous, or cross-cutting portion.
- Do not repeatedly dump unchanged files, repository-wide diffs, generated IR, machine code,
  massive JSON, or complete projections into context.
- Prefer a focused test over a crate test and a crate test over a workspace test during
  iteration.
- Do not repeat an identical successful command when no relevant input changed.
- Reserve the complete required suite for the final relevant state.
- Quiet success still requires a known command, completed exit status, and known boundary.
- Never hide a nonzero status, compiler error, test failure, warning promoted by policy,
  sanitizer finding, fuzz failure, malformed-output failure, or environment error.
- Do not use `|| true`, broad filtering, or redirection to make a failing command appear
  successful.
- For noisy successful commands, prefer a native quiet flag.
- Otherwise capture the complete log outside Git and report a bounded summary; expose the
  relevant section and full-log path on failure.
- When agent efficiency is an objective, measure bytes, lines, commands, round trips, duplicate
  facts, wall time, and context needed for the next decision.
- Do not claim provider-token or billing reduction from byte counts alone.
- Do not make output cryptic or incomplete merely to reduce bytes.

## Anti-overengineering gate

Before adding an abstraction, identify its present problem, producer, consumer, authority,
lifetime, invalidation, failure behavior, measurable or structural benefit, why local code is
insufficient, and deletion condition.

- An abstraction should remove meaningful duplication, make an invalid state unrepresentable,
  isolate a real boundary, expose a current useful API, enable a measured property, or
  materially simplify reasoning.
- The mechanism must be smaller than the problem and must not duplicate authority.
- Prefer deletion of unused work first.
- Then prefer semantic or representation simplification.
- Then reuse an existing invariant or canonical validator.
- Then reuse an existing canonical order or response-local derived fact.
- Then replace repeated discovery with one local retained fact.
- Then improve layout or traversal.
- Then make an invalid state unrepresentable.
- Add caching only after measured repeated work and a defined lifetime.
- Add parallelism only for large separable work.
- Add target specialization only behind a complete generic route and measured need.
- Do not refactor unrelated code for symmetry, aesthetics, novelty, or theoretical completeness.
- Prefer a small explicit mechanism when only one current use exists.
- Prefer deletion over documenting machinery without a consumer.
- Do not solve complexity with bookkeeping.

Do not build speculative daemons, services, sessions, process boundaries, persistence, journals,
databases, distributed stores, CRDTs, schedulers, resource topologies, process cells, custom
allocators, universal registries, descriptor systems, plugin frameworks, rewrite DSLs, general
incremental or cache frameworks, proof ecosystems, wire protocols, broad target matrices,
multi-tier JIT policy, deoptimization, PGO machinery, platform products, or self-hosting
scaffolding.

- A task may introduce one of those mechanisms only with a demonstrated present boundary, an
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
- Do not impose numeric file-length, directory-width, directory-depth, module-count, plan-count,
  or repository-shape rules.

## Implementation workflow

1. Inspect branch, worktree, upstream state, and recent history without destroying unrelated
   work.
2. Read the authority documents relevant to the task.
3. Map producers, consumers, authority, derived state, trust boundaries, and failure paths.
4. Characterize current behavior with focused tests or measurements.
5. Choose the highest-leverage dependency-closed problem.
6. Record a falsifiable hypothesis, completion criteria, reversal condition, and stop condition
   in temporary state.
7. Implement the smallest coherent root-cause correction.
8. Delete the displaced path and stale claims.
9. Add focused regression, convergence, atomicity, and scale tests as relevant.
10. Update the owning documentation in the same change.
11. Run final verification after the final relevant edit.
12. Commit cohesive changes when permitted.
13. Push only when publication is explicitly requested.
- Do not leave two active architectures, a half-cutover, disabled correctness checks, an
  unfinished required migration, stale prose presented as current, a compatibility route hiding
  an incomplete replacement, or dependence on a scratch artifact.
- Prefer a smaller complete vertical over a larger partial program.
- Stop after completing the selected vertical, updating the roadmap, identifying the next
  highest-leverage problem, and recording why it was not started.

## Structure and dependencies

- Organize code by coherent responsibility, not line counts or symmetry.
- A crate boundary should represent a real trust or unsafe boundary, an independently useful
  library, a supported target, measured compile isolation, or a low-coupling subsystem.
- Merge crates that mainly exchange internal descriptors, re-exports, or adapters.
- Remove numbered shards, include-only facades, one-child directory ladders, artificial tiny
  modules, redundant models, and conversion layers when recombination improves ownership and
  retrieval.
- Split a large module only when the split establishes ownership and reduces change coupling.
- Use mature dependencies when they remove substantial machinery or risk.
- Keep owned code when it is smaller, clearer, safer, easier to audit, or measurably better.
- Do not add a dependency for functionality clearer as local code.
- Do not reorganize unrelated files during a focused semantic or performance correction.

## Tests

- Tests protect intended semantics and public invariants, not accidental topology.
- Cover relevant type, generic, trait, effect, capability, ownership, control-flow, and cleanup
  laws.
- Cover completeness and explicit incomplete state.
- Cover stable identity, namespaces, generations, revisions, deletion, and deterministic
  ordering.
- Cover malformed, stale, foreign, wrong-kind, wrong-owner, duplicate, missing, and invisible
  input.
- Cover transaction and artifact failure atomicity.
- Cover exactly-once effects and generic/specialized equivalence where specialization exists.
- Cover cancellation, resource failure, host error, and cleanup when relevant.
- Cover deep input, deep destruction, scale, and checked representation boundaries when
  user-controlled depth is involved.
- Cover machine-output decoding, determinism, stdout, stderr, and exit behavior for machine
  interfaces.
- Cover effect-free checking and real product integration.
- Add a focused regression test for each root cause.
- Use generated fixtures for scale.
- Keep fast default tests separate from ignored locked-release stress while exercising the same
  algorithm at smaller size.
- Use differential, property, model, or test-only reference implementations when an independent
  oracle is cheap.
- Delete tests that preserve provisional syntax, old bytes, obsolete APIs, deleted machinery,
  arbitrary limits, private topology, or accidental details.
- Never weaken a test merely to make a redesign pass.
- A convergence test compares semantic outcomes, not only text.
- A failure-atomicity test verifies the prior snapshot and allocator remain unchanged.
- A stack-safety test covers construction, transformation, query, projection, and destruction
  where user depth matters.
- A machine test decodes output as a consumer would; do not validate JSON by substring matching.
- A quiet-success test asserts both streams empty.
- A no-effects check test uses a program whose execution would be observable.
- Timing and developer-machine RSS do not belong in default correctness gates.

## Documentation

- `README.md` owns product introduction and first successful use.
- `docs/spec/` owns intended external semantics and target contracts.
- `docs/status.md` owns current implementation and known gaps.
- `docs/architecture.md` owns current responsibilities, flow, ownership, and trust boundaries.
- `docs/performance.md` owns method, workloads, compact evidence, limitations, and reversal
  conditions.
- `docs/roadmap.md` contains only Now, Next, and Later.
- `docs/decisions/` contains sparse durable decisions.
- Update the owning document and delete stale text in the same change.
- Do not add digests, global revisions, fact shards, copied tables, per-commit evidence records,
  transcripts, handoffs, prompt archives, completion capsules, or duplicate roadmaps.
- Write a decision only when a choice is durable, non-obvious, expensive to rediscover, and has
  a meaningful reversal condition.
- Do not describe target as current, hypothesis as measurement, private movement as public
  movement, planned systems as supported, or developer-machine observation as a product
  guarantee.
- Examples must use active APIs and semantics.
- Agent-facing documentation should be executable or mechanically checked where practical.
- Do not maintain a hand-written capability table when authoritative implementation facts can
  answer the same question.
- Raw benchmark samples and logs stay outside Git.

## Git and publication

- Inspect worktree and branch state before editing.
- Preserve unrelated tracked and untracked work.
- Do not use destructive reset, checkout, clean, history rewrite, or force push against work you
  did not create.
- Commit only coherent repository changes.
- Exclude prompts, raw logs, raw benchmark samples, temporary plans, generated scratch files,
  credentials, and unrelated changes.
- Use a commit message naming the semantic, architectural, or measured result.
- Push only when the active task requests publication.
- Never force push for convenience.
- After a requested push, verify the local branch, tracking branch, and pushed commit.
- If publication fails, preserve the verified local commit and report the exact failure.

## Verification

During iteration, run the smallest focused command that can disprove the current change. After
the final relevant edit, run at least the semantic equivalent of:

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

- Report the completed objective and root cause or evidence-based keep decision.
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
