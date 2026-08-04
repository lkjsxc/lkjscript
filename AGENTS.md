# Agent Guide
## Purpose
Define the durable working contract for autonomous engineering in this repository.
Keep Current capability, next work, commit identities, measurements, and historical
evidence out of this file. Put changing facts in Current State, capability authority,
Agent Handoff, decision capsules, Agent Work State, and retained evidence.

## Authority Order
1. Machine-enforced repository rules and exact registries are hard constraints.
2. The user's task defines intended scope and may authorize breaking changes.
3. Current State and registered capability facts define factual Current behavior.
4. Accepted Contract, Accepted Target, and Accepted Selection bind their exact scope.
5. Focused evidence supports claims only for its named commit, environment, and command.
6. Experimental, Deferred, Rejected, Superseded, and Historical material grants no fallback.
Resolve disagreements against actual `HEAD`. Preserve the newest valid machine and
Current authority, repair stale projections, and continue without inventing completion.

## Stable Product Invariants
- Build one AI-primary, statically typed, memory-safe language and platform.
- Use one typed authority through HIR, verified SSA, validated bytecode, evaluator, VM, baseline JIT, and proof JIT.
- Keep the platform daemon-first; standalone is bounded bootstrap, recovery, diagnosis, CI, or development support.
- Use explicit typed capabilities and effects. Ambient host authority is absent.
- Keep ordinary source free of lifetime or region names, retain/release, `free`, raw pointers, and memory controls.
- Preserve no-tracing execution; analysis failure is a compile error, never a collector, dynamic box, or fallback.
- Use canonical lowercase ASCII kebab-case names and word operations.
- Use `.lkjscript` only; removed spellings, editions, and compatibility modes have no aliases.
- Use one nonzero monotonic `meta/platform-revision` and exact content digests.
- A native claim requires a real synchronous generated entry. Emission is not execution.
- Source values never expose allocation, root, region, witness, process, or storage identity.
- Unsupported behavior rejects through a closed typed outcome; it does not degrade silently.

## AI-Primary Engineering
- Use Semantic Source as the primary machine query and edit boundary where Current; text remains the Git projection.
- Prefer stable semantic entities and typed transactions over byte-offset patching.
- Preserve incomplete programs through typed holes, obligations, deterministic diagnostics, and bounded legal actions.
- Build bounded content-addressed graph context; do not read the whole repository when a precise slice suffices.
- One semantic fact has one authority and identity; indexes, projections, caches, and summaries are derived only.
- AI suggestions, inferred facts, hints, tests, and proofs are untrusted; prove, validate, or reject.
- Diagnostics identify the violated fact, origin, blocker, and bounded repairs without guessing authority.

## Documentation And Authority Discipline
- Documentation correctness is part of implementation correctness.
- A public claim has one fact identity, exact status, authority, explicit exclusions, and appropriate evidence.
- Current-facing prose is a checked projection of authority, not independent truth.
- Update authority, implementation, evidence, projections, indexes, and checks atomically when a fact changes.
- Entry documents must not retain stale commands, crate maps, limits, platform claims, status, or removed mechanisms.
- Current documents may not inherit claims from Historical, Superseded, Rejected, Experimental, or Deferred records.
- Historical text states its recorded baseline and never uses unqualified present-tense Current claims.
- Examples and command blocks are interfaces; deterministically validate syntax, availability, safety class, and result.
- Derive facts from compiler, CLI, contracts, Cargo, capsules, and evidence; do not copy them into another registry.
- Generated documentation projections belong under `target/`; tracked prose is authority or a checked projection.
- Hard documentation gates are deterministic and offline; LLM review is advisory or Experimental only.
- Never repair corpus-wide prose with an unchecked global replacement. Edit the
  semantic claim, inspect every affected context, and retain migration evidence.
- Remove obsolete active documents or move them to bounded history after replacement.
  Do not leave misleading active paths for compatibility.

## Start Every Task
1. Resolve actual branch, upstream, worktree, and commits after any named baseline.
2. Read this guide, Current State, status authority, Agent Handoff, and verification.
3. Run topology, repository graph, and documentation authority audits.
4. Build strong bounded context for affected facts, capsules, crates, and evidence.
5. Create or resume one bounded Agent Work State task.
6. Run the inherited baseline appropriate to the changed boundary.
7. Record unavailable and intentionally omitted gates before implementation.
Use at least:
```sh
git status --short --branch
git log --oneline --decorate -n 120
cargo run --locked -p lkjscript-xtask -- structure audit --json
cargo run --locked -p lkjscript-xtask -- structure graph --json
cargo run --locked -p lkjscript-xtask -- \
  structure context docs/current-state.md --profile strong
cargo run --locked -p lkjscript-xtask -- \
  structure context docs/operations/agent-handoff.md --profile strong
cargo run --locked -p lkjscript-xtask -- check-docs
```
Use the canonical replacement if a later accepted contract renames `check-docs`.

## Change Discipline
- Update accepted authority before changing a public contract.
- Implement complete vertical slices. A parser form, enum variant, metadata field,
  mock, Rust-only wrapper, or prose claim is not a capability.
- Keep every status class exact and distinct.
- Mark inert behavior `PLACEHOLDER` or implement or remove it.
- Remove obsolete paths, aliases, decoders, shims, and losing production candidates
  after the complete replacement passes.
- Replace or remove a test only with equal-or-stronger evidence.
- Do not weaken a Current limit before its checked replacement is Current.
- Make routine architecture decisions autonomously from evidence. Ask only for
  external authority, secrets, legal consent, spending, or irreversible action.
- If the full target cannot finish safely, integrate the strongest coherent
  intermediate and state exact remaining proof obligations.
- Do not expand into adjacent Deferred products because a mechanism could support them.
- Preserve source semantics across placement, scheduling, specialization, and backends.
- Use impact analysis before broad code, contract, or documentation changes.

## Compiler And Semantic Discipline
- Backends consume verified typed authority; they never reinterpret source syntax.
- Producer and verifier may share schemas, encoders, hashes, tags, and limits.
  They must not share the policy decision claimed to be independently reconstructed.
- The evaluator is an independent semantic oracle, not accidental Rust behavior.
- Proof optimization preserves or re-proves every semantic, ownership, resource,
  effect, representation, and cleanup fact.
- Forced native groups preflight before source effects and permit no fallback.
- Bind executable routes to exact semantic, package, SSA, bytecode, contract,
  resource-profile, and native-layout identities as applicable.
- Runtime keys, owner tokens, witness slots, addresses, loans, and provider
  identities never cross process or persistence boundaries.
- Canonical codecs preserve semantic values independently of placement and tuning.
- Do not add reflection, source-string dispatch, universal value boxes, or
  backend-specific semantic compilers as shortcuts.

## Memory And Resource Discipline
- Prefer inline or static values, caller destinations, unique ownership and
  borrowing, regions, coarse sealed sharing, typed pools, and measured copying
  before precise node counting.
- Universal per-node reference counting is not an acceptable default.
- Mutable identity and cycles use typed generational pools or explicit regions.
- Verified SSA makes transfer, borrow end, destination initialization, sealing,
  release, reuse, resource close, and cleanup explicit.
- Cleanup runs exactly once on every implemented normal and failure outcome.
- Release and rollback use bounded iterative work, never unbounded native recursion.
- Unknown placement, ownership, cleanup, or codec legality rejects before publish.
- Physical placement changes performance, never semantics or logical charging.

## Performance And Experiment Discipline
- Final performance is a design constraint, not permission for special cases.
- Establish a reproducible baseline and predeclare workloads, metrics, thresholds,
  and falsification conditions before observing deciding results.
- Measure latency, throughput, tails, allocations, bytes, RSS, copying, ownership
  traffic, release work, compile time, and native code size where relevant.
- Include short, steady-state, adversarial, failure, sharing, and application workloads.
- Do not move thresholds after results without recording a new experiment.
- Retain compact positive and negative evidence. Remove losing production code.
- Runtime JIT remains the primary adaptive path. AOT, caches, local PGO, OSR,
  and background compilation require separate accepted contracts and evidence.
- Scheduling may consume verified locality facts but must preserve semantics and
  cooperate with ordinary Linux scheduling.

## Runtime, Safety, And Dependencies
- Keep pure compiler and runtime state separate from host effects.
- Unsafe Rust stays inside registered mechanism boundaries with reviewed safe callers.
- Add no third-party Rust dependency without accepted source, license, advisory,
  maintenance, unsafe-boundary, build, package, and runtime review.
- Platform-specific mechanisms are optional adapters behind exact capability and
  evidence boundaries, never semantic prerequisites.

## Repository Discipline
- Authored files are at most 200 physical lines, 32 KiB, and 120 Unicode scalars
  per ordinary line.
- Authored directories have at most 16 immediate tracked entries and obey depth,
  capsule, link, graph, provenance, and authority rules.
- Generated and reproducible output belongs under `target/`.
- Do not preserve stale paths for an old checker. Update authority and checker atomically.
- Keep `AGENTS.md` stable and policy-only.
- Machine authorities include structure and provenance policy, unsafe registry,
  capability facts, platform revision, contract registry, and capsule manifests.

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
A public capability is done only when every applicable layer passes:
```text
source vocabulary and Semantic Source
typing, effects, capabilities, and ownership
independently verified HIR
verified SSA and proof preservation
bytecode encoding and validation
evaluator and VM
forced baseline and forced proof
package, artifact, contract, and process provenance
resource limits, cleanup, metrics, and malformed inputs
documentation facts, projections, status, and retained evidence
```
The tree must be clean, topology-compliant, honestly documented, coherently
committed, and pushed when the task authorizes publication.

## Final Handoff
Report starting and final commits, revision changes, architecture, implemented
layers, commands and results, unavailable gates, negative evidence, remaining
defects, next highest-value risk, branch and upstream state, and publication state.
