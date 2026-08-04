# Agent Guide

## Purpose
Define the durable working contract for autonomous engineering in this repository.
Keep changing capability, revision, benchmark, and historical facts out of this file.
Put them in Current State, public facts, Agent Handoff, decisions, work state, and evidence.

## Authority Order
1. Machine-enforced contracts, registries, and repository rules are hard constraints.
2. The user's task defines intended scope and may authorize breaking changes.
3. Current State and registered public facts define factual Current behavior.
4. Accepted Contract, Accepted Target, and Accepted Selection bind only their exact scope.
5. Focused evidence supports claims only for its named commit, environment, and command.
6. Experimental, Deferred, Rejected, Superseded, and Historical text grants no fallback.
Resolve disagreement against actual `HEAD`. Preserve the newest valid authority, repair stale
projections, and continue without inventing completion.

## Stable Product Invariants
- Build one AI-primary, statically typed, memory-safe language and platform.
- Use one typed authority through HIR, verified SSA, bytecode, evaluator, VM, and both JIT tiers.
- Keep the platform daemon-first; standalone execution is a bounded support path.
- Use explicit typed capabilities and effects. Ambient host authority is absent.
- Hide lifetimes, regions, retain/release, `free`, raw pointers, and allocator controls from source.
- Preserve collector-free execution. Analysis failure rejects; it never selects tracing fallback.
- Use canonical lowercase ASCII kebab-case names and word operations.
- Use `.lkjscript` only. Removed spellings, editions, and compatibility modes have no aliases.
- Keep one nonzero monotonic platform revision and exact content-addressed contract identities.
- A native claim requires a real synchronous generated entry. Emission is not execution.
- Source values never expose allocation, domain, root, witness, process, or storage identity.
- Unsupported behavior returns a closed typed outcome; it never degrades silently.

## AI-Primary Engineering
- Use Semantic Source as the machine query and edit boundary wherever it is Current.
- Prefer stable semantic identities and typed transactions over byte-offset patching.
- Preserve incomplete programs through typed holes, obligations, and bounded legal actions.
- Build a complete revision-bound repository index, then derive small task-specific context views.
- One semantic fact has one authority. Indexes, projections, caches, and summaries are derived.
- Every context view states authority, revision, confidence, exclusions, budget, and omitted frontier.
- AI proposals, inferred facts, hints, tests, and proofs are untrusted until checked.
- Diagnostics identify the violated fact, origin, blocker, and bounded repairs without guessing.
- Optimize interfaces for weak models: closed schemas, stable order, local context, and exact errors.

## Limits And Scale
- Never fix a scale failure by increasing a magic number alone.
- Classify every limit as one of: semantic law, addressability maximum, safety maximum,
  resource quota, query/output budget, implementation geometry, or test fixture.
- Give each production limit one stable ID, unit, scope, authority, failure mode, and evidence.
- Coincidentally equal numbers are separate contracts; unrelated limits must not share authority.
- Implementation geometry, such as segment size, is measured and absent from source and wire rules.
- Resource profiles may lower quotas. Raising above validated safety maxima fails.
- Distinguish cumulative work from live capacity, retained bytes, peak usage, and output size.
- Reserve before allocation or publication. Failure is typed, deterministic, and mutation-atomic.
- A complete authority or index either publishes completely or fails; it is never truncated.
- Bounded views may stop only with an exact reason, frontier, and identity-bound continuation.
- Addressability maxima protect representation; ordinary workloads must be governed by profiles.
- Keep all arithmetic checked and make zero, exact-limit, plus-one, and overflow behavior testable.

## Documentation And Authority
- Documentation correctness is implementation correctness.
- A public claim has one fact identity, exact status, authority, exclusions, and suitable evidence.
- Current-facing prose is a checked projection of authority, not independent truth.
- Update authority, implementation, evidence, projections, indexes, and checks atomically.
- Entry documents must not retain stale commands, crate maps, limits, or removed mechanisms.
- Current text may not inherit claims from non-Current records.
- Historical text names its baseline and avoids unqualified present-tense Current claims.
- Examples and command blocks are interfaces; validate syntax, availability, class, and result.
- Derive facts from compiler, CLI, contracts, Cargo, capsules, and evidence; do not duplicate them.
- Generated projections belong under `target/`; tracked prose is authority or a checked projection.
- Hard documentation gates are deterministic and offline. LLM review is advisory only.
- Never use unchecked corpus-wide replacement to repair semantic claims.
- Remove obsolete active paths or move them to bounded history after replacement.

## Start Every Task
1. Resolve the actual branch, upstream, worktree, and commits after any named baseline.
2. Read this guide, Current State, status authority, Agent Handoff, and verification.
3. Run topology, repository-index, documentation, and public-fact audits.
4. Build strong bounded context for affected facts, capsules, crates, tests, and evidence.
5. Create or resume one bounded Agent Work State task.
6. Run the inherited baseline appropriate to the changed boundary.
7. Record unavailable and intentionally omitted gates before implementation.
Use at least:
```sh
git status --short --branch
git log --oneline --decorate -n 120
cargo run --locked -p lkjscript-xtask -- structure audit --json
cargo run --locked -p lkjscript-xtask -- structure graph --json
cargo run --locked -p lkjscript-xtask -- check-docs
```
Use the canonical replacement if accepted authority renames a command.

## Change Discipline
- Update accepted authority before changing a public contract.
- Implement complete vertical slices. A parser form, enum variant, mock, or prose is not a capability.
- Keep every status class exact and distinct.
- Mark inert behavior `PLACEHOLDER`, implement it, or remove it.
- Remove obsolete paths, aliases, shims, and losing production candidates after replacement passes.
- Replace or remove a test only with equal-or-stronger evidence.
- Do not weaken a Current limit before its checked replacement is Current.
- Make routine architecture decisions autonomously from evidence.
- Ask only for external authority, secrets, legal consent, spending, or irreversible external action.
- If the full target cannot finish, integrate the strongest coherent intermediate and name the gaps.
- Do not expand into adjacent Deferred products merely because a mechanism could support them.
- Preserve semantics across placement, scheduling, specialization, and backends.
- Use impact analysis before broad code, contract, or documentation changes.

## Compiler And Semantic Discipline
- Backends consume verified typed authority; they never reinterpret source syntax.
- Producer and verifier may share schemas and encoders, not the policy decision being verified.
- The evaluator is an independent semantic oracle, not accidental Rust behavior.
- Optimization preserves or re-proves semantic, ownership, effect, resource, and cleanup facts.
- Forced native groups preflight before source effects and permit no fallback.
- Bind executable routes to exact semantic, package, IR, bytecode, profile, and layout identities.
- Runtime keys, owner tokens, witness slots, addresses, and loans never cross process boundaries.
- Canonical codecs preserve semantic values independently of placement and tuning.
- Do not add reflection, source-string dispatch, universal boxes, or backend-specific semantics.

## Memory And Calling Discipline
- Prefer inline/static values, caller destinations, borrowing, unique ownership, regions,
  coarse sealed sharing, typed pools, measured copying, and only then precise node counting.
- Universal per-node reference counting is not an acceptable default.
- Domains are coarse lifetime and ownership groups, not default one-per-node or one-per-copy units.
- Pass immutable nonescaping arguments by verified borrow; do not clone to satisfy a call boundary.
- Reuse or eliminate tail-call frames when semantics permit, and end dead owners before transfer.
- Use contiguous compact images for locality when measured best and segmented forms when required.
- Mutable identity and cycles use typed generational pools or explicit regions.
- Verified SSA makes transfer, borrow end, destination init, sealing, release, and cleanup explicit.
- Cleanup runs exactly once on every implemented normal and failure outcome.
- Release and rollback use bounded iterative work, never unbounded native recursion.
- Unknown placement, ownership, cleanup, or codec legality rejects before publication.
- Physical placement changes performance, never semantics or logical resource charging.

## Repository Intelligence
- The revision index is derived, deterministic, complete for its declared input, and non-authoritative.
- Preserve stable node and edge identities, provenance, confidence, exclusions, and exact contracts.
- Prefer streamable or sharded storage, forward and reverse adjacency, and incremental equivalence.
- Verify an incremental rebuild against a clean rebuild before making it Current.
- Queries are deterministic, budgeted, resumable, and bound to graph and command identity.
- A query may omit data explicitly; the underlying successful index may not omit accepted input.
- Generated indexes stay under `target/` and never become source, status, proof, or evidence authority.

## Performance And Experiments
- Final performance is a design constraint, not permission for special cases.
- Establish a reproducible baseline and predeclare workloads, metrics, and falsification conditions.
- Measure latency, throughput, tails, allocations, bytes, RSS, copying, owner traffic, and release work.
- Include short, steady-state, adversarial, failure, sharing, and application workloads.
- Measure small-value regressions as well as large-value scalability.
- Do not move thresholds after results without recording a new experiment.
- Retain compact positive and negative evidence. Remove losing production code.
- Runtime JIT is the primary adaptive path; AOT, caches, PGO, and OSR need accepted contracts.
- Scheduling may consume verified locality facts but must preserve semantics and Linux cooperation.

## Runtime, Unsafe Code, And Dependencies
- Keep pure compiler and runtime state separate from host effects.
- Unsafe Rust stays inside registered mechanism boundaries with reviewed safe callers.
- Add no third-party Rust dependency without source, license, advisory, maintenance, build,
  unsafe-boundary, package, and runtime review.
- Platform-specific mechanisms are optional adapters behind exact capability boundaries.

## Repository Discipline
- Authored files are at most 200 physical lines, 32 KiB, and 120 Unicode scalars per ordinary line.
- Authored directories have at most 16 immediate tracked entries and obey topology rules.
- Generated and reproducible output belongs under `target/`.
- Do not preserve stale paths for an old checker. Update authority and checker atomically.
- Keep `AGENTS.md` stable and policy-only.

## Evidence And Verification
- A command that did not run did not pass.
- Record exact commit, environment, command, result, and untested gates.
- Distinguish parsed, typed, compiled, built, linked, executed, measured, and accepted.
- A VM test is not native evidence. A substrate test is not a language capability.
- Preserve failed and rejected experiments when they constrain future choices.
The canonical local integration gate is:
```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```
Before every commit:
1. Inspect the complete diff and generated contract or projection changes.
2. Run focused tests for the changed boundary.
3. Run structure, status, documentation, and impact checks.
4. Update authority, evidence, contracts, locks, and platform revision atomically.
5. Add exact `Tested:` and `Not-tested:` trailers.

## Definition Of Done
A public capability is done only when every applicable layer passes: source and Semantic Source;
typing, effects, capabilities, ownership, independently verified HIR, verified SSA, bytecode,
evaluator, VM, forced native tiers, provenance, profiles, cleanup, malformed input, documentation,
status, and retained evidence.
The tree must be clean, topology-compliant, honestly documented, coherently committed, and pushed
when the task authorizes publication.

## Final Handoff
Report starting and final commits, revision changes, architecture, implemented layers, commands,
results, unavailable gates, negative evidence, remaining defects, next risk, upstream state, and
publication state.
