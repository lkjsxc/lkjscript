# Agent Guide

## Purpose
Define the stable working contract for autonomous engineering in this repository.

This file contains durable policy, not Current capability, the next implementation
target, a commit identity, or historical evidence. Put changing facts in Current
State, Agent Handoff, status authority, decision capsules, and retained evidence.

## Authority Order
1. Machine-enforced repository rules and exact contract registries are hard
   constraints.
2. The user's task defines intended scope and may authorize breaking changes. It
   is not evidence that a capability already exists.
3. `docs/current-state.md` and capability status authority define factual Current
   behavior.
4. Accepted Contract, Accepted Target, and Accepted Selection records define
   intended architecture for their exact scope.
5. Focused evidence and Git history support claims only for their named commit,
   environment, command, and result.
6. Historical, Superseded, Rejected, and Experimental material never provides a
   compatibility alias or fallback.

When authorities disagree, resolve actual `HEAD`, preserve the newer valid
machine and Current authority, repair stale documentation, and continue toward
the user's goal without inventing completion.

## Stable Product Invariants
- Build one AI-primary, statically typed, memory-safe language and platform.
- Use one resolved typed semantic authority from source through HIR, verified
  SSA, validated bytecode, evaluator, VM, baseline JIT, and proof JIT.
- Keep the runtime daemon-first. Standalone execution is a bounded bootstrap,
  recovery, diagnosis, CI, or development path.
- Use explicit typed capabilities and effects. Ambient host authority is absent.
- Keep ordinary source free of lifetime names, region names, retain/release,
  general `free`, raw pointers, allocator selection, and memory-engine switches.
- Preserve the unconditional no-tracing runtime. Analysis failure is a compile
  error, never a collector, dynamic box, or unchecked ownership fallback.
- Use canonical lowercase ASCII kebab-case source names and word operations.
- Use `.lkjscript` only. Removed spellings, editions, compatibility modes, and
  subsystem versions have no aliases.
- Use one nonzero monotonic `meta/platform-revision` and exact content digests.
- Linux x86-64 is Current until executed evidence promotes another native target.
- A native claim requires a real synchronous generated entry. Emission,
  disassembly, compilation, or observation alone is insufficient.
- Source values do not expose allocation, root, region, witness, process, or
  storage identity.

## Start Every Task
1. Resolve actual repository state before relying on a prompt or handoff.
2. Inspect commits after any referenced baseline.
3. Run topology and graph audits.
4. Build bounded context for Current State, Agent Handoff, and relevant capsules.
5. Create or resume one bounded Agent Work State task.
6. Run the inherited baseline appropriate to the changed boundary.
7. Record unavailable or intentionally omitted gates before implementation.

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
```

Use `structure context` for relevant capsules and crates. Do not read the entire
repository indiscriminately.

## Change Discipline
- Update accepted authority before changing a public contract.
- Implement complete vertical slices. A parser form, enum variant, metadata
  field, mock, Rust-only wrapper, or documentation claim is not a capability.
- [machine: LKJ-DOC-STATUS] Keep Current, Accepted Contract, Accepted Target, Accepted Selection,
  Experimental, Deferred, Rejected, Superseded, and Historical distinct.
- [machine: LKJ-DOC-PLACEHOLDER] Mark inert behavior `PLACEHOLDER` or implement/remove it.
- Remove obsolete paths, aliases, decoders, shims, and losing production
  candidates after a complete replacement passes.
- Replace or remove a test only with equal-or-stronger evidence.
- Do not weaken a Current limit before its checked replacement is Current.
- Make routine architecture decisions autonomously from evidence. Ask only for
  genuine external authority, secrets, legal consent, spending, or irreversible
  non-repository actions.
- If a full target cannot finish safely, push the strongest coherent
  intermediate and report exact remaining proof obligations. Never fake status.
- Do not expand into adjacent Deferred products merely because a mechanism could
  support them.
- Preserve source semantics across placement, scheduling, specialization, and
  backend changes.
- AI optimizer hints are proved, runtime-checked, or rejected; UB is never an
  optimization mechanism.

## Compiler and Runtime Discipline
- Backends consume verified typed authority; they never reinterpret source or
  synthesize ownership, capability, equality, or codec legality.
- Keep pure compiler and runtime state separate from all host effects.
- Producer and verifier may share schemas, encoders, hashes, tags, and limits.
  They must not share the policy decision being independently checked.
- Evaluator behavior is an independent semantic oracle, not accidental Rust
  ownership behavior.
- Proof optimization preserves or re-proves every semantic, ownership, resource,
  and contract fact.
- Forced native groups preflight before source effects and permit no fallback.
- Runtime JIT is the primary adaptive path; AOT, caches, and local PGO require
  shared verified identity and measured acceptance.
- Runtime keys, owner tokens, dense witness slots, addresses, loans, and provider
  identities never cross process boundaries.
- [machine: LKJ-UNSAFE-BOUNDARY] Unsafe Rust stays inside registered mechanism
  boundaries with reviewed safe-caller contracts.
- Add no third-party Rust dependency without accepted source, license, advisory,
  maintenance, unsafe-boundary, build, package, and runtime review.

## Repository Discipline
- `[machine: LKJ-REPO-*]` Authored files are at most 200 physical lines, 32 KiB,
  and 120 Unicode scalars per ordinary line.
- Authored directories have at most 16 immediate tracked entries and obey depth
  and provenance rules.
- [machine: LKJ-REPO-GENERATED-PROVENANCE] Use one artifact tree. Generated and
  reproducible output belongs under `target/`.
- Retain compact positive and negative evidence; remove temporary output.
- Do not preserve stale paths for an old checker. Update authority and checker
  atomically.
- Keep `AGENTS.md` stable and policy-only. Never put the current commit or next
  implementation target here.
- Machine registries are `meta/structure/{policy,provenance}.json`,
  `meta/unsafe/registry.json`, `meta/config/capability-status.json`,
  `meta/platform-revision`, and root or nested `capsule.json` manifests.

## Evidence and Status
- A command that did not run did not pass.
- Record exact commit, environment, command, result, and untested gates.
- Distinguish parsed, typed, compiled, built, linked, executed, measured, and
  accepted evidence.
- A build is not runtime support. A VM test is not native evidence. A
  specialization is not a residual ABI. A substrate test is not a language
  capability.
- Preserve failed and rejected experiments when they constrain future choices.
- Predeclare adoption thresholds and do not move them after observing results
  without a new recorded experiment.

## Verification
The canonical local integration gate is:

```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Runtime, release, Docker, retained-result, Miri, sanitizer, deterministic
property/fuzz, process-fault, cross-build, and performance acceptance are
separate gates in `docs/operations/verification.md`.

Before every commit:
1. Inspect the complete diff.
2. Run focused tests for the changed boundary.
3. Run `structure check`.
4. Update status, decisions, evidence, contracts, and locks.
5. Add exact `Tested:` and `Not-tested:` trailers.

## Definition of Done
A public capability is done only when all applicable layers pass:

```text
source and vocabulary
typing, effects, capabilities, and ownership
independently verified HIR
verified SSA and proof preservation
bytecode encoding and validation
evaluator
VM
forced baseline and forced proof
package and artifact provenance
process/provider boundary where applicable
resource limits and metrics
malformed and failure cleanup
documentation and status
```

The repository must be clean, topology-compliant, honestly documented,
coherently committed, and pushed when the task authorizes publication.

## Read Order
Always read:
1. `AGENTS.md`
2. `docs/current-state.md`
3. `docs/operations/status-authority.md`
4. `docs/operations/agent-handoff.md`
5. `docs/operations/verification.md`

Then use `structure context` for the exact decision, affected crates, evidence,
and graph slice. Read history only for a named question or retained experiment.

## Final Handoff
Report actual starting and final commits, revision changes, architecture,
implemented layers, commands and results, unavailable gates, negative evidence,
remaining defects, next highest-value risk, branch/upstream state, and whether
the integrated result was pushed.
