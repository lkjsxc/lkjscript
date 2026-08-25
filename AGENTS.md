# Repository Instructions

## Scope and precedence

This file applies to the entire repository unless a deeper `AGENTS.md` or
`AGENTS.override.md` supplies narrower instructions for its directory.

Follow instructions in this order:

1. the active user request;
2. the active campaign prompt explicitly supplied for the task;
3. the closest applicable agent-instruction file;
4. current executable contracts and tests;
5. current normative specifications;
6. current implementation and generated documentation;
7. historical decisions, campaign ledgers, prompts, and comments.

The active checkout outranks stale descriptions.

Historical prompts explain history only.
Do not treat them as current requirements unless the active task explicitly adopts them.

Read the applicable instructions before editing.
Inspect the actual repository before planning.
Do not infer current behavior from filenames, old contract numbers, or prior campaigns.

All first-party code, comments, diagnostics, command help, specifications, and maintained
documentation must be in English.

Do not record or expose private chain-of-thought.
Record decisions, evidence, alternatives, and uncertainty in concise reviewable form.

## Mission

lkjscript is an AI-first programming platform whose accepted typed semantic graph is the sole
authoritative representation of application meaning.

Ordinary application development must be possible through the released `lkjscript` CLI.

Editable text source must not become a second program authority.

A change file, command request, review projection, generated document, query index, compiler
artifact, runtime handle, cache, or validation receipt is not program meaning unless a current
normative contract explicitly makes it part of accepted semantic authority.

Human readability is useful for inspection and operations, but it is not a design objective that
may weaken machine precision, semantic locality, determinism, or performance.

The platform must remain general.
Do not place application-specific policy in Rust merely to complete one maintained application.

The long-term objective is one coherent system, not preservation of accumulated implementations.

## Architectural invariants

There must be one accepted semantic authority.

Do not maintain parallel editable representations.

Do not wrap a predecessor AST, source model, or graph as the permanent core of a new design.

Names are mutable namespace and presentation data.
Stable references must not depend on names when exact identity is available.

Identity has a cost.
Assign durable identity only when a demonstrated consumer needs continuity across edits, history,
merge, external reference, or precise selection.

Do not give every implementation detail a global identity by default.
Prefer scoped identity, immutable content identity, or no identity when those are sufficient.

Keep logical meaning independent from physical layout.

Do not make object boundaries, pack sizes, page fanout, cache shards, compiler units, or runtime
indexes language semantics merely because the current implementation uses them.

Derived relations, indexes, summaries, validation data, and compiler caches must be rebuildable
from accepted meaning unless a specification proves that they alter meaning.

Separate semantic revision identity from validation evidence and operational receipts.
A validator upgrade must be able to re-evaluate unchanged meaning without silently changing that
meaning's identity.

Publication may become visible only after its required semantic data and acceptance evidence are
durable and mutually bound.

Use exact references at semantic boundaries.
Lower exact references to dense runtime indexes before hot execution.

Keep host mechanisms generic.
Application routes, policy, queries, state transitions, authorization rules, and domain behavior
belong in graph meaning when the platform can express them.

Backward compatibility is not a default objective.

When replacing a contract, migrate every maintained consumer, reject predecessor inputs, remove
predecessor readers and writers, delete compatibility aliases, and update documentation in the same
dependency-closed campaign.

Do not create editions, legacy modes, fallback readers, or shadow authorities unless the active
user explicitly requires them.

## Decision discipline

Prefer deletion, consolidation, and reuse over adding another abstraction.

Prefer the smallest design that satisfies complete workflows.

Do not preserve a design because it is implemented, documented, tested, or recently written.

Do not replace a design merely to produce visible change.

Before a major architectural choice:

- state the actual problem;
- identify the current authority and consumers;
- list the smallest credible alternatives;
- compare correctness, complexity, locality, performance, migration cost, and deletion cost;
- identify what evidence can distinguish the alternatives;
- choose one path;
- define the reversal or deletion condition;
- remove losing prototypes.

Do not elevate arbitrary numeric thresholds into design principles.

Every maintained limit must be classified as one of:

- a format bound required for safe decoding;
- a deterministic request or execution budget;
- an operational default;
- a measured implementation ceiling;
- a temporary test parameter.

State the unit and owning boundary.
Do not use one scalar as a substitute for independent resource dimensions.

Do not flatten an inherently multidimensional problem merely to simplify an API or document.

Avoid project-specific terminology when an established technical term is accurate.

Introduce a new public term only when it names a real distinction that users or agents must reason
about.

Use one term for one concept.
Remove retired aliases and obsolete contract names after cutover.

Do not turn this file into an architecture specification.
Durable operating rules belong here.
Campaign design belongs in the active prompt.
Normative behavior belongs under `docs/spec/`.
Current facts belong in `docs/status.md`.
Measurements belong in evidence or performance records.

## Repository work

Start significant work by recording:

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -20 --oneline
git remote -v
find .. -name AGENTS.md -o -name AGENTS.override.md
rustc --version
cargo --version
```

Inspect the released command surface and relevant focused tests before changing a public boundary.

Preserve unrelated work.

Do not use `git reset`, `git clean`, history rewriting, force push, blanket staging, or destructive
checkout commands.

Do not use `git add .` or `git add -A`.
Stage explicit paths.

Do not amend, rebase, merge, or switch branches unless the active task requires it.

Make coherent commits that compile and preserve the repository's intended intermediate state.

A normal fast-forward push is allowed only when the active user request or campaign prompt
authorizes it.

Do not ask the user to choose routine implementation details that can be resolved from evidence and
these priorities.

Ask only when an external fact, credential, irreversible action, or genuinely ambiguous product
decision cannot be resolved safely from the repository and active request.

Do not stop at a design document when implementation is requested.

Do not leave a new path private indefinitely while the predecessor remains public.
Complete a vertical public slice or remove the unfinished path.

## Rust and first-party tooling

Rust is the default language for production code, repository tooling, verification, fixtures,
benchmarks, migrations, and maintained generators.

Do not add new first-party Python tooling.

When an existing Python tool is in scope, replace it with a tested Rust implementation and remove
the Python path after parity.

Shell may coordinate standard commands, but substantial first-party logic must not hide in shell
scripts.

Application developers must not need Python, Cargo, a Rust toolchain, repository source, or network
access for ordinary binary-only lkjscript development.

Use the repository's active Rust edition and locked dependency graph.

Prefer explicit domain types over strings and untyped maps at authority boundaries.

Keep parsing, validation, canonicalization, storage, and execution boundaries typed.

Reject unknown fields, trailing bytes, duplicate keys, invalid order, foreign identities,
overflow, exhaustion, and malformed framing at their owning boundary.

Avoid `unwrap`, `expect`, `panic`, `todo`, and `unimplemented` on maintained runtime paths.

Safe Rust is the default.

First-party `unsafe` requires all of the following:

- a measured requirement that safe Rust cannot satisfy adequately;
- a narrow module boundary;
- documented invariants;
- focused adversarial tests;
- sanitizer or equivalent evidence when applicable;
- benchmark evidence for the claimed benefit;
- an explicit rollback condition.

Do not relax repository-wide safety merely for speculative optimization.

Add a dependency only when it removes more maintained complexity than it introduces.

Inspect source, maintenance state, feature flags, transitive cost, licensing, and security impact
before adding a production dependency.

Prefer private modules and narrow exports.
Split by ownership and dependency direction, not by arbitrary line counts.

Large files are evidence to inspect, not automatic proof that a split is correct.

Warnings are errors.
Formatting, Clippy, and tests must remain clean for touched code.

## CLI and protocol

The released `lkjscript` CLI is the normal application-development interface.

The CLI must be composable, deterministic, discoverable, revision-aware, bounded, and useful from
an empty directory with one copied release binary.

A fresh agent must be able to discover the current command and data model without reading repository
source or a giant generated schema.

Provide focused discovery by command, operation, semantic kind, and changed schema section.

Default output must be concise.
Return exact identities, revision bindings, status, essential counts, and the next useful handle or
command.

Write large payloads to explicit files and return their paths and digests.
Do not dump complete graphs, schemas, logs, or artifacts to ordinary stdout.

Raw recursive JSON must not be the normal way to author semantic changes.

JSON must not be canonical semantic storage, an artifact format, or a required control-plane
transport.

JSON may remain as an explicit optional projection and as ordinary application data at external
boundaries such as HTTP APIs when a real consumer justifies it.

Keep typed request and response models independent of any one transport.

A compact text change notation, direct CLI flags, optional machine projection, and any future
resident transport must lower to the same typed operation registry and semantic change engine.

Do not create a second source language, macro system, or hidden builder.

Change notation is an ephemeral request representation.
Accepted graph meaning remains authoritative.

Diagnostics must identify the failed operation, field, selector, expected form, observed revision,
and safe recovery action when those facts are available.

Dry-run and commit must use the same normalization, impact, validation, and publication preparation.

A prepared write must bind an exact base revision.
Publication must recheck that base before visibility.

Use request-local symbols for connected creation.
Return their allocated stable identities compactly.

Do not require a separate identity-allocation round trip.

Do not add a resident session, daemon, local socket, or custom binary protocol because it appears
agent-friendly.

Measure complete repeated workflows first.
Retain a session only when it materially reduces latency, output, retries, or provider cost without
weakening stateless correctness, revision binding, cancellation, recovery, or resource bounds.

## Correctness, storage, and evidence

Accepted revisions are immutable.

Publication must be atomic, fail closed, and crash consistent.

No rejected, stale, exhausted, corrupt, or interrupted operation may advance accepted state.

Never rely on a derived index for correctness without an independent path that can rebuild or
verify it from accepted meaning.

Incremental validation and compilation must agree with independent complete oracles.

Do not call a path incremental when it reconstructs, scans, clones, validates, compiles, or loads
the complete project or dependency closure.

Measure actual owner reads, edges, pages, objects, bytes, compiler units, writes, synchronization
operations, elapsed time, CPU time, and peak memory where applicable.

Semantic revision identity, validation evidence, verification receipts, and benchmark observations
must use separate typed domains.

If a command reports a durable receipt path, the receipt must already be complete, synchronized as
required, and atomically visible.

Retain failure evidence as carefully as success evidence.

A missing, partial, stale, or unverifiable receipt is not a pass.

Do not report generated files as authoritative when they are not mechanically verified against
their executable owner.

Backup and restore must operate on exact retained authority, verify integrity before visibility,
and never depend on disposable indexes.

Deletion, compaction, and garbage collection require exact roots, pins, active-reader protection,
registered backup roots, an independent reachability oracle, interruption tests, and conservative
failure behavior.

Never delete reachable authority.

## Performance and model-economy claims

Optimize end-to-end workflows, not isolated microbenchmarks.

Preserve asymptotic locality before tuning constants.

Benchmark maintained applications and representative synthetic topologies.

Distinguish semantic work from storage work, compiler work, runtime work, operating-system work, and
transport work.

Do not infer tokens or monetary cost from byte counts.

Provider token and monetary claims require exact provider telemetry or an explicitly identified
tokenizer with its limitations.

Byte counts, command counts, retries, latency, and context size may be reported directly.

Reduce agent cost through focused discovery, stable handles, revision-bound context, compact
diagnostics, quiet successful verification, reusable receipts, and avoiding repeated schema or
repository dumps.

Do not optimize cost by hiding required evidence or making failures ambiguous.

JIT, AOT specialization, memory mapping, custom allocators, SIMD, unsafe code, resident processes,
and new storage engines are evidence-gated options, not standing requirements.

Keep hot runtime representations replaceable so later evidence can support stronger optimization
without changing semantic authority.

## Security boundaries

Treat all external bytes, paths, archives, descriptors, requests, artifacts, store objects, and
environment-derived values as untrusted input.

Validate before allocation, traversal, publication, execution, or adapter binding.

Reject path traversal, symlink escape, ambiguous normalization, foreign identity, forged digest,
duplicate binding, oversized input, excessive nesting, integer overflow, and trailing data.

Secrets must remain host-owned and redacted.
Do not place secret values in accepted graph meaning, receipts, diagnostics, logs, artifacts, or
provider telemetry.

Capability grants must be exact, least-authority, typed, revision-bound where required, and checked
before effects.

Runtime resource handles must not cross scopes or durable boundaries accidentally.

Do not claim hostile-code sandboxing, multi-tenant isolation, encrypted transport, artifact
signatures, portability, or production readiness without retained evidence for that exact claim.

Do not broaden the campaign into unrelated security infrastructure.

## Verification

Run the narrowest relevant checks during iteration.

Run the repository's authoritative complete profile before final handoff when feasible.

A successful final verification must use the exact final content.
Reused evidence is not fresh evidence when the full profile requires freshness.

Tests must cover public behavior, not only private helpers.

For changed authority boundaries, include:

- valid round trips;
- malformed and truncated input;
- unknown forms;
- duplicate and noncanonical input;
- stale revisions;
- foreign identities;
- exhaustion;
- crash points;
- corruption;
- deterministic reproduction;
- independent-oracle agreement;
- predecessor rejection after cutover.

For performance claims, retain commands, inputs, toolchain, platform, revision, outputs, elapsed
time, CPU, peak memory when available, and artifact or receipt digests.

Successful checks should print a compact aggregate result and durable receipt location.

Child logs and detailed evidence belong in bounded files.

Do not suppress warnings or weaken tests to make a gate pass.

Do not classify a skipped, unavailable, timed-out, flaky, or partially executed gate as success.

Review the final diff for duplicated authority, compatibility residue, stale terminology, hidden
full-project work, unchecked resource dimensions, accidental application policy in Rust, and
documentation drift.

## Documentation

Keep normative specifications, current status, architecture, security boundaries, roadmap, and
performance evidence distinct.

Do not copy the same contract table or operation catalog into multiple handwritten documents.

Generate repetitive reference material from one executable registry and verify generated bytes.

Current-status documentation must describe implemented checkout reality and explicitly identify
what remains private, incomplete, unmeasured, or unsupported.

Decision records must state the decision, alternatives, evidence, consequences, and reversal
condition.

Campaign ledgers must be concise operational indexes, not transcripts or replacements for source
control.

Do not append long narrative checkpoints when a commit, receipt, and short status row carry the
same information.

Keep the root `AGENTS.md` below Codex's ordinary project-instruction budget.
Do not enlarge it with campaign-specific schemas, phase plans, field catalogs, or implementation
details.

## Completion

Work is complete only when the requested public workflow works through maintained interfaces.

Before handoff:

- inspect the final status and diff;
- confirm every maintained consumer uses the selected authority;
- remove superseded readers, writers, aliases, fixtures, generators, and documentation;
- run focused and complete verification as required;
- retain exact receipts;
- update current facts and limitations;
- record commits;
- report whether a normal push was performed;
- list any failed or unavailable checks honestly.

Do not describe a private prototype as a completed public capability.

Do not leave TODO text in place of required implementation.

Do not preserve dead architecture for possible future use.
Git history is the archive.
