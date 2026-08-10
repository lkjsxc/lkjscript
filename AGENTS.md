# AGENTS.md

## Scope and autonomy

This file applies to the entire repository and to every agent that inspects, changes, tests,
documents, measures, commits, publishes, or reports on it.

Write code, comments, public APIs, diagnostics, tests, documentation, commit messages, and final
reports in English unless the active task explicitly requires another language for a user-facing
artifact.

The user authorizes autonomous technical judgment, incompatible changes, destructive
simplification, specification revision, representation replacement, file and crate reorganization,
and deletion of obsolete work. Backward compatibility is not a project objective unless the active
task names one current, externally consumed boundary that must remain compatible. Do not preserve
old syntax, serialized bytes, command shapes, internal Rust APIs, module layouts, package layouts,
cache formats, compiler or runtime representations, fixtures, or prose merely because they existed.

Historical requests, previous prompts, earlier assistant answers, old branches, and Git history are
context, not permanent requirements. Do not ask the user to choose among technical alternatives
when the checkout, accepted specifications, focused tests, measurements, or a reversible local
assumption can decide. Ask only when a genuinely external product requirement is missing and no safe
assumption can unblock the objective.

Preserve unrelated local work, credentials, host state, external data, and remote history.
Authorization to redesign this repository is not authorization to erase unrelated state.

The only permanently fixed property of the program file format is the `.lkjscript` extension. Every
other notation, grammar, byte layout, schema, CLI, package format, compiler representation, runtime
representation, and storage decision remains provisional unless an accepted specification fixes it.

## Mission

Build `lkjscript` into an AI-primary, statically typed, memory-safe, collector-free,
high-performance programming language and implementation.

AI-primary means that an agent can discover supported behavior, construct and modify programs,
inspect semantic facts, check without executing effects, compile and run intentionally, compare
outcomes, receive compact actionable failures, and verify changes through deterministic interfaces.
A model may propose an operation; deterministic implementation machinery decides whether it is
valid.

AI-primary does not mean:

- model inference inside a compiler, validator, optimizer, runtime, storage layer, or correctness
  boundary;
- optimization for one model, tokenizer, provider, benchmark, or prompting style;
- hiding meaning from humans or ordinary tools;
- wrapping display strings in JSON and calling the result structured;
- multiplying protocols, schemas, descriptors, registries, services, stores, or agent metadata;
- probabilistic validation where exact validation is available;
- emitting every internal fact merely because an agent could consume it; or
- building an agent platform before a complete local programming workflow exists.

The long-term direction is one syntax-independent mutable semantic authority, first-class
source-free construction, stable logical identity, explicit incomplete states, typed atomic
transactions, deterministic semantic queries and diffs, direct compilation without render/reparse,
one complete generic execution route, optional pre-effect specialization, a small local development
loop, low round-trip and irrelevant-output cost, human-reviewable projections, reproducibility, and
strong end-to-end performance.

## Authority and truth

Use `docs/authority.md` to resolve ownership by claim dimension. In practical order:

1. the active task owns the current objective;
2. this file owns repository-wide engineering procedure;
3. accepted files under `docs/spec/` own intended language and semantic-workspace contracts;
4. code, tests, manifests, schemas, and command definitions own behavior in the checkout;
5. `docs/status.md` summarizes current implementation and known gaps;
6. `docs/architecture.md` explains current responsibilities, flow, ownership, and trust boundaries;
7. `docs/performance.md` owns measurement method and compact reproducible evidence;
8. `docs/roadmap.md` owns planned ordering only;
9. sparse accepted decisions own durable rationale; and
10. Git history owns superseded implementation and prose.

A roadmap item is not an architectural commitment. A task prompt is not a durable authority
artifact. A previous answer is not a specification. A test proves only what it exercises. A
benchmark proves only its stated workload and protocol.

When claims conflict, classify the claim, inspect its owning artifact and executable evidence,
decide which artifact is wrong, then update or delete stale material in the same coherent change.
Write a decision record only when a durable, non-obvious choice would otherwise be repeatedly
rediscovered. Use labels such as **Current**, **Target**, **Hypothesis**, **Historical**,
**Unknown**, and **Blocked** when ambiguity would remain.

Do not manufacture another authority layer from prompts, planning trees, digests, global revisions,
registries, ledgers, generated inventories, closure graphs, checkpoints, handoffs, or completion
capsules. Specifications may change. When a better design changes intended semantics, update the
owning specification and perform one direct cutover. Do not silently contradict a specification or
retain an obsolete compatibility path.

## Priority order

Unless current evidence establishes a more severe prerequisite, prioritize:

1. coherent language semantics;
2. memory safety, ownership, cleanup, failure atomicity, and determinism;
3. scale-safe algorithms and representations without arbitrary validity quotas;
4. a complete local semantic workspace and direct compiler input;
5. a small deterministic local development loop for coding agents;
6. representative edit, query, compile, and output measurements before incremental machinery;
7. one complete measured execution route before more specialization;
8. self-hosting and broader products only after lower layers are coherent; and
9. persistence, collaboration, daemonization, scheduling, and distribution only after a present
   consumer and measurements justify those boundaries.

Do not skip a lower layer because a later platform idea is more exciting. Do not optimize the
language around Brainfuck, one demo, one benchmark, or one synthetic workload. A benchmark may
expose a defect; it does not define the language.

## One active architecture and direct cutovers

Maintain one active language definition, mutable semantic authority, compiler path, generic
production execution route, ownership model, package model, documentation authority model, and
implementation for each current product boundary.

Do not create editions, permanent `v2` systems, `next` trees, legacy modes, compatibility layers, or
parallel canonical representations. A small independent evaluator may remain as a test oracle; it is
not automatically a second production engine.

When replacing a mechanism:

1. identify the intended replacement;
2. update the owning specification when needed;
3. update all active producers and consumers;
4. replace active fixtures;
5. delete the old contract and implementation;
6. delete adapters, aliases, feature flags, and compatibility tests;
7. remove stale exports, dependencies, and documentation; and
8. verify that one active route remains.

Names and current crate boundaries have no authority. Preserve, merge, split, rename, or delete
components according to cohesion, safety, real boundary ownership, independently useful APIs,
measured compile isolation, coupling, and current consumers. When architecture causes the defect,
replace it rather than surrounding it with bookkeeping.

## Semantic authority and identity

The authoritative semantic state must be able to exist without source text, files, formatting,
paths, spans, parser nodes, source hashes, or compiler-dense indexes. These may be importer,
presentation, provenance, cache, or boundary attachments. Text may construct semantic state;
compilation, editing, querying, validation, and correctness checks must not require rendering and
reparsing it.

Do not satisfy an invariant with dummy source files, placeholder paths, fabricated hashes, hidden
bodies beneath holes, synthetic entry points, fake declarations, reserved placeholder identities,
fallback executable meaning, or dense compiler values presented as stable public identity. Use an
honest source-optional origin model.

One representation owns mutable semantic facts. Every derived representation must have a current
producer and consumer, lifetime, invalidation rule, and deletion condition. Derived representations
are not coequal authorities. Dense IDs, vector positions, physical slots, code offsets, and layout
indexes should be compact, private, and replaceable.

Use opaque logical identity with namespace, generation, and revision defenses where meaning must
survive rename, movement, or private compaction. Tombstone identity when meaning does not survive.
Names, paths, spans, formatting, semantically irrelevant order, and hashes are not universal mutable
identities. Before adding an identity, state what it identifies, its uniqueness lifetime, what it
survives, who allocates and validates it, what tombstones it, how stale values fail, and why an
existing identity cannot serve.

## Public semantic and diagnostic APIs

Public APIs expose semantic meaning, not parser nodes, display strings, private addresses, dense
indexes, debug formatting, or unvalidated compiler objects. Use one structured public model per
concept unless input and output have genuinely different semantics.

Transaction-local handles are acceptable before stable entities exist, but they must be scoped,
typed, validated, non-persistent, and impossible to confuse with stable identity. Do not proliferate
`Input`, `View`, `Ref`, `Descriptor`, `Resolved`, and `Wire` variants for conversion convenience.

A display message may accompany structured data. It must not be the only machine-readable meaning
when the producer already knows structured facts. Never parse a rendered diagnostic to reconstruct
facts its producer knew. Do not fabricate diagnostic codes, spans, semantic subjects, causes,
suggestions, or recovery actions. An honest broad category plus message is preferable when no richer
fact exists.

Machine-facing output must be deterministic, schema-explicit, completeness-explicit when partial,
stably ordered, bounded or paginated where necessary, free of private identities, actionable without
prose parsing when structured facts exist, and reviewable by a human. Never silently truncate.

Public recursive values must be safe to clone, compare, hash where required, project, validate,
convert, and destroy without unbounded native stack. Do not impose a nesting quota to compensate for
recursive implementation. Stable nominal and generic references use stable semantic or explicit
builtin identity.

## Incomplete states and transactions

Incomplete semantic state is valid editing state. Missing declarations, bodies, expressions,
references, choices, and conflict resolutions must be explicit blockers, holes, or recovery facts.
Never retain executable fallback meaning behind an incomplete node. Compilation of an incomplete
snapshot stops before ownership planning, memory planning, SSA, bytecode, native code, or execution.

Transactions publish one coherent final state or nothing. Validate revision and namespace before
mutation. Failure must not consume stable identities, mutate allocator state, change the published
snapshot, poison caches, or partially publish derived state. When a transaction contract promises
order independence, validate the intended final semantic graph rather than edit-list order.

Distinguish containment from dependency. Container deletion may remove facts whose semantic
existence it owns. Never silently delete independent declarations merely because they depend on the
removed object. Require explicit deletion, define one narrow ownership rule, or reject with an
actionable blocker.

Private relocation is not public movement. Surviving public identities remain stable when meaning
survives compaction. Old immutable snapshots remain valid. Foreign, stale, wrong-kind, wrong-owner,
duplicate, missing, and invisible identities fail deterministically. Derive dependency indexes from
semantic authority; do not maintain a second mutable dependency truth.

## Generic, ownership, and execution semantics

Generic declarations, instantiations, substitutions, bounds, and witnesses are semantic facts, not
parser decoration. Source import and source-free editing must converge on one exact instantiation
and trait-selection path. Inference is an authoring convenience; exact substitutions and
compiler-derived witnesses are the result. Draft binder handles must not escape publication.
Compiler-dense implementation IDs and binder names are not stable public identity.

Do not introduce a general higher-rank framework, implicit inference engine, or generic rewrite
system before a current language requirement needs it. Narrow restrictions must fail consistently
across source and source-free paths.

Ordinary execution is collector-free and non-tracing. Do not add tracing collection,
language-visible hidden reference counting, raw-pointer language surfaces, retain/release, general
`free`, or parallel GC and non-GC modes merely to simplify implementation. A memory-semantic change
requires a specification change and one complete cutover.

Preserve exact move and borrow laws, deterministic cleanup where promised, cleanup on normal and all
failure exits, no double release, stack-safe destruction, explicit host-resource ownership, and
failure-atomic publication.

Maintain one complete generic production route. Optional native or specialized execution may
decline only before effects and must leave the generic route intact. Once specialized entry begins,
its result or failure is final. Never re-execute effects through fallback.

Validate fail-closed at real untrusted boundaries. Inside one synchronous trusted typed pipeline,
validated wrappers and Rust ownership carry authority; do not repeatedly serialize, hash,
reconstruct, or independently revalidate them without a real boundary consumer.

Unsafe code belongs in a narrow named mechanism with explicit invariants, a documented safe-caller
contract, focused malformed-input tests, and appropriate Miri, sanitizer, fuzz, or property
coverage.

## Scale and resource policy

Language validity follows semantic laws, not project-selected size quotas. Do not reject a trusted
program because it exceeds an arbitrary number of bytes, tokens, nesting levels, declarations,
fields, variants, parameters, arguments, locals, functions, files, modules, blocks, IR nodes,
identities, values, diagnostics, handles, or analysis steps.

Do not disguise a limit by raising it, widening an integer, moving it, renaming it, or calling it a
safety profile. Use checked arithmetic and checked narrowing. User-controlled depth must not consume
unbounded native stack; use iterative traversal or a justified heap-backed work stack.

An untrusted product may impose explicit coarse host policy for input, memory, output, elapsed time,
cancellation, and concurrency. Resource exhaustion is a typed host result, not a semantic error. Do
not design detailed untrusted policy before such a product exists.

## Agent-facing local development

The near-term product requirement is a local workflow through which a coding agent can:

1. learn the supported language;
2. create or modify a program;
3. check it without executing effects;
4. inspect an actionable result;
5. run it intentionally; and
6. verify the outcome.

Meet it with the smallest complete local boundary. Prefer tested examples and authoritative
authoring documentation, then a compile-only command, structured diagnostics, and one-shot batch
operations over existing in-process APIs. Add a long-lived process only after those are insufficient
and measurements show startup or repeated import dominates.

Do not infer that an agent workflow requires a daemon, database, journal, session broker, scheduler,
network protocol, CRDT, persistent semantic store, or broad agent framework. Do not expose a broad
semantic editing protocol before one concrete workflow needs it. Do not serialize internal types
merely because a CLI has a machine mode.

A check command must not execute program effects. A run command is intentional execution. Human and
machine rendering may share structured facts but must not parse each other. Command names,
arguments, exit behavior, stdout, and stderr are deterministic and tested. Successful
high-frequency validation should be quiet by default; explicitly requested data such as
disassembly, projections, and inventories may be verbose.

A one-shot command must not pretend identities remain valid across invocations unless a real
namespace and snapshot lifetime preserve them.

## Agent attention, command output, and API cost

Model context, tool output, developer attention, wall time, and API spend are engineering resources.
Reduce irrelevant output without weakening correctness or hiding evidence.

During discovery, search before opening large files, read focused ranges, inspect focused diffs, and
retrieve full detail only for a failing or ambiguous portion. Do not repeatedly dump an unchanged
large file, repository-wide diff, generated IR, machine code, massive JSON, or complete projection
into model context without a current diagnostic need.

During iteration:

- run the smallest command that can falsify the current hypothesis;
- prefer a focused test over a crate test and a crate test over a workspace test;
- avoid release rebuilds after edits that cannot affect them;
- do not repeat an identical successful command when no relevant input changed; and
- reserve the complete required suite for the final relevant state.

Quiet success still requires an exact command, completed exit status, and known boundary. Never hide
a non-zero status, compiler error, test failure, policy warning, sanitizer or Miri finding, fuzz
failure, malformed-output failure, or environment error. Do not use `|| true`, broad filtering, or
redirection to make a failing command look successful. A deliberate failing probe must record its
expected and actual status.

For noisy successful commands, prefer a native quiet flag that preserves failures. Otherwise capture
the complete log outside Git, report a bounded success summary, and on failure expose the command,
status, relevant diagnostic section, explicit omitted count when truncated, and full-log path. Do
not commit raw logs or benchmark samples.

Machine commands should emit one deterministic document or documented stream without progress
chatter. A successful high-frequency command normally emits no output or one explicitly requested
bounded summary. Failure remains actionable. The objective is low irrelevant output, not low
information.

Do not add a task runner, command registry, output broker, logging framework, cache, or service merely
to silence commands. A thin local script is justified only when it removes measured recurring cost,
preserves exact command semantics and complete failure evidence, has a current consumer, is smaller
than the duplicated shell usage, and has a deletion condition. Prefer existing Cargo, shell, and OS
mechanisms over a new orchestration crate.

When agent efficiency is an objective, measure relevant command count, round trips, stdout and
stderr bytes, line count, duplicate diagnostics, wall time, repeated work, and context needed to
choose the next action. Do not claim API-cost reduction from intuition when byte and line
measurements are available. Do not sacrifice deterministic failure detail merely to minimize tokens.

## Decision discipline and anti-overengineering

Start from a demonstrated defect or explicit current requirement. Correct the dependency-closed root
cause, not one visible symptom.

Before adding an abstraction, identify its current problem, producer, consumer, authority, lifetime,
invalidation, failure behavior, measured or structural benefit, why local code is insufficient, and
deletion condition. It should remove duplication or repeated work, make an invalid state
unrepresentable, isolate a real boundary, expose a current useful API, enable a measured property, or
materially simplify reasoning. It must be smaller than the problem and must not duplicate authority.

Prefer, in order: delete unused work; simplify semantics or representation; reuse an invariant;
replace repeated discovery with one local fact; improve layout and traversal; make invalid states
unrepresentable; add caching only after measured repetition; add parallelism only for large
separable work; and add target specialization only behind a complete generic route and measured
need.

Do not refactor unrelated code for symmetry, aesthetics, novelty, or theoretical completeness.
Investigate whether asymmetry is semantic, historical, or accidental. Prefer a small explicit
mechanism when only one current use exists. Prefer deletion over documenting machinery without a
consumer.

Do not build speculative daemons, services, sessions, process boundaries, persistence, journals,
databases, distributed stores, CRDTs, schedulers, resource topologies, process cells, custom
allocators, universal registries, descriptor systems, plugin frameworks, rewrite DSLs, general
incremental or cache frameworks, proof ecosystems, wire protocols, broad target matrices,
multi-tier JIT policy, deoptimization, PGO machinery, platform products, or self-hosting scaffolding.

A task may introduce one only with a demonstrated present boundary, an end-to-end current consumer,
measured need, explicit acceptance criteria, explicit ownership and failure behavior, and a reversal
condition.

Do not solve complexity with bookkeeping. First remove repeated scans, reconstruction, duplicated
facts, unnecessary boundaries, duplicate validation, duplicate commands, noisy success output, and
dead work. Do not create a universal graph engine for one walk, a rewrite framework for a few
identity remaps, an event system for one synchronous result, a cache for unmeasured work, a protocol
because an in-process API exists, a daemon for hypothetical tools, an interner because types are
recursive, an arena to avoid one traversal, a trait hierarchy to share two short functions, a
diagnostic framework to preserve one existing diagnostic, or a command framework for one command.

Do not impose numeric file-length, directory-width, directory-depth, module-count, plan-count, or
repository-shape rules. Split and merge by cohesion, ownership, retrieval quality, testability,
compile isolation, and real boundaries. Temporary planning belongs in agent working state, not
committed bureaucracy.

## Work selection and multi-turn execution

At the start of a substantial task:

1. inspect branch, worktree, upstream state, and recent history without destroying unrelated work;
2. read the authority documents relevant to the task;
3. trace producers, consumers, mutable authority, derived representations, trust boundaries, and
   failure paths;
4. characterize current behavior with focused tests or measurements;
5. identify the highest-leverage dependency-closed problem;
6. state a falsifiable hypothesis, completion criteria, reversal condition, and stop condition in
   temporary working state;
7. implement the smallest coherent root-cause correction;
8. delete the displaced path and stale claims;
9. add focused tests and update owning documentation;
10. run final verification after the final relevant edit;
11. commit cohesive changes when permitted; and
12. verify publication state only when publication is requested.

The task owns the objective, not an unverified proposed mechanism. Change course when evidence
invalidates the suggestion. Fix a blocking correctness, safety, or authority prerequisite within the
same vertical, but do not use incidental findings as permission for an unrelated rewrite.

Every turn leaves the repository coherent, documented, tested, and usable. Never leave two active
architectures, a half-cutover, disabled correctness checks, an unfinished required migration, stale
prose presented as current, a compatibility layer hiding an incomplete replacement, or dependence
on a scratch artifact.

Prefer a smaller complete vertical over a larger partial program. One objective may cross layers
when they form one dependency-closed product vertical. Complete it, update the roadmap, identify the
next problem, and stop. Multi-turn progress is expected; the project need not decide every future
representation, runtime tier, platform, storage format, process boundary, schema, or cache now.

Do not commit task prompts, checkpoints, transcript summaries, copied context, handoff files, or
completion capsules. Remove a transport prompt from the intended commit.

## Performance and evidence

Profile before optimizing and measure the selected product path. Relevant evidence includes wall and
phase time, startup, throughput, edit and query latency, peak and retained memory, allocations and
bytes, copied or parsed bytes, rendered or serialized bytes, stdout and stderr bytes, line and
command count, agent round trips, traversal counts, scale behavior, generated-code size, and binary
size.

Before comparison, state the hypothesis, equivalent semantics, workload, environment, build and
cache state, sample protocol, selection criterion, and reversal condition. Prefer deterministic work
counters over noisy timing when they answer the question. Generated scale tests establish
correctness and complexity shape, not representative application performance.

Keep raw samples outside Git and commit only compact reproducible evidence. Do not turn
developer-machine noise into a correctness gate. Keep an optimization only when its end-to-end
benefit justifies compile time, memory, code size, complexity, tests, and maintenance. Do not add a
validity quota to hide a performance defect or a framework to avoid one local scan. Full
recomputation may remain correct until representative edits justify incrementality. Do not claim
latency, memory, allocation, output-volume, or throughput improvement without equivalent evidence.

## Structure and dependencies

Organize code by coherent responsibility, not counts or symmetry. A crate boundary should represent
a real trust or unsafe boundary, independently useful library, supported target, measured compile
isolation, or low-coupling subsystem. Merge crates that mainly exchange internal descriptors,
re-exports, or adapters. Remove numbered shards, include-only facades, one-child directory ladders,
artificial tiny modules, redundant models, and conversion layers when recombination helps.

Split a large module only when the split establishes ownership and reduces change coupling. Do not
shard by line number or reorganize unrelated code for aesthetics.

Use mature dependencies when they remove substantial machinery or risk. Keep owned code when it is
smaller, clearer, safer, easier to audit, or measurably better. Before adding a descriptor, registry,
witness, identity, plan, contract, cache, conversion layer, arena, interner, command wrapper, or
output collector, identify its producer, consumer, lifetime, boundary, invalidation rule, and
deletion condition. Do not add a dependency for functionality clearer as local code.

## Tests

Tests protect intended semantics and public invariants, not accidental topology. Cover as relevant:

- type, generic, trait, effect, capability, ownership, control-flow, and cleanup laws;
- completeness and explicit incomplete states;
- stable identity, namespaces, generations, revisions, deletion, and deterministic ordering;
- malformed, stale, foreign, wrong-kind, wrong-owner, duplicate, and missing input;
- transaction and artifact failure atomicity;
- exactly-once effects and generic/specialized equivalence;
- cancellation, resource failure, host error, and cleanup;
- deep input, deep destruction, scale, and checked representation boundaries;
- machine-output decoding, determinism, stdout/stderr, and exit behavior;
- effect-free checking; and
- real product integration.

Add a focused regression test for each root cause. Use generated fixtures for scale. Keep fast
default tests separate from ignored locked-release stress geometry while exercising the same
algorithm at smaller size. Use differential, property, model, or test-only reference
implementations when an independent oracle is cheap.

Delete tests that preserve provisional syntax, old bytes, obsolete APIs, deleted machinery,
arbitrary limits, private topology, or accidental details. Never weaken a test merely to make a
redesign pass. Assert stable public meaning, not private dense indexes except in focused compaction
tests.

A convergence test compares semantic outcomes, not only text. A failure-atomicity test verifies the
prior snapshot and allocator remain unchanged. A stack-safety test covers construction,
transformation, and destruction on a small stack where user depth matters. A machine test decodes
output as a consumer would; do not validate JSON by substring matching. A quiet-success test asserts
both streams empty. A no-effects check test uses a program whose execution would be observable.

## Documentation

Keep active documentation small, non-overlapping, and truthful:

- `README.md`: product introduction and first successful use;
- `docs/spec/`: intended external semantics and target contracts;
- `docs/status.md`: current implementation and known gaps;
- `docs/architecture.md`: current responsibilities, flow, ownership, and trust boundaries;
- `docs/performance.md`: method, workloads, compact evidence, and reversal conditions;
- `docs/roadmap.md`: only `Now`, `Next`, and `Later`; and
- `docs/decisions/`: sparse durable decisions.

Update the owning document and delete stale text in the same change. Do not add digests, global
revisions, fact shards, copied tables, per-commit evidence records, transcripts, handoffs, prompt
archives, completion capsules, or duplicate roadmaps.

Write a decision only when a choice is durable, non-obvious, expensive to rediscover, and has a
meaningful reversal condition. Do not describe target as current, hypothesis as measurement, private
movement as public, planned systems as supported, or a developer-machine observation as a product
guarantee.

Examples must use active APIs and semantics. Delete examples for displaced contracts.
Agent-facing documentation should be executable or mechanically checked where practical. Do not
maintain a hand-written capability table when an authoritative implementation fact can answer the
same question.

## Git and publication

Inspect worktree and branch state before editing. Preserve unrelated tracked and untracked work. Do
not use destructive reset, checkout, clean, history rewrite, or force push against work you did not
create.

Commit only coherent repository changes. Exclude prompts, raw logs, raw benchmark output, temporary
plans, generated scratch files, credentials, and unrelated changes. Use a commit message naming the
semantic or architectural result.

Push only when the active task requests publication. Never force push for convenience. After a
requested push, verify the local branch, tracking branch, and pushed commit. If publication fails,
preserve the verified local commit and report the exact failure.

## Verification

During iteration, run the smallest focused command that can disprove the current change.

After the final relevant edit, run at least the semantic equivalent of:

```sh
cargo fmt --all -- --check
cargo clippy --quiet --workspace --all-targets --all-features --locked -- -D warnings
cargo test --quiet --workspace --all-targets --all-features --locked
cargo build --quiet --workspace --release --locked
```

Use verified quiet flags or the retained quiet wrapper when they preserve exact semantics and full
failure evidence. Run retained container verification when available:

```sh
docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify
```

Run additional focused release stress, differential, property, small-stack, deep-input,
malformed-boundary, cancellation, allocation-failure, Miri, sanitizer, fuzz, benchmark,
documentation, example, and machine-output checks when relevant.

Run full verification after the final relevant change. Do not rerun an unchanged complete suite only
to produce a different summary. If the environment blocks a command, report the exact command,
failure category, relevant output, whether the change caused it, successful remaining evidence, and
unverified risk. Never claim a command passed when it did not complete.

## Final report

Report the completed objective, root cause, principal design, replaced or deleted paths, tests and
measurements, output-volume evidence when relevant, exact verification commands and outcomes,
environment-limited checks, documentation changes, commit and publication state, next
highest-leverage problem, and why work stopped before beginning it.

Keep the report factual and compact. Do not reproduce the prompt, paste complete successful logs, or
claim future work is implemented.
