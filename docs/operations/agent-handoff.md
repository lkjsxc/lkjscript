# Agent Handoff

## Purpose

Capture product intent, current sharp edges, accepted next contracts, and
verification discipline without preserving obsolete implementation priorities.

## Status

<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=semantic-core-target status=accepted-target -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=repository-graph-context status=current -->
<!-- LKJ-STATUS id=repository-topology status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger status=accepted-target -->
<!-- LKJ-STATUS id=semantic-session status=current -->
<!-- LKJ-STATUS id=semantic-source status=current -->
<!-- LKJ-STATUS id=typed-holes status=current -->

**Current** for the engineering policy and implementation boundaries linked from
[Current State](../current-state.md). Bounded Repository Topology, Repository
Intelligence Graph/context, Agent Work State canonical contract semantic references, complete
Semantic Source Schema with its exact legacy contract base, typed holes/legal actions,
closed hole transactions, one-shot protocol, compiler Resource resource profile, and
the core hierarchical pre-allocation plus deterministic journal foundation are
Current. Agent Foundation
legacy contract and Semantic Source Schema are historical rejected identities. Bounded
local stdio sessions serve canonical contract and are Current. the canonical source contract identity, homogeneous
closures, marker projection, exact atomic migration publication, the canonical
corpus, and ordinary-compilation cutover are Current. Other the canonical source contract changed
execution, whole-pipeline pre-allocation, logical metering integration, and a shared ledger
remain Accepted Targets.
Automatic proof promotion remains
an Accepted Implementation Selection, not the immediate priority.

## Product Intent

- Build the language, compiler, runtime, standard library, and ecosystem as one
  coherent product named `lkjscript`.
- Canonical accepted sources use `.lkjscript`; do not preserve `.lkjml` support.
- Keep the Rust host small and Linux-first; grow policy in lkjscript source.
- Keep unsafe Rust inside `lkjscript-sys`, with safe APIs sound for every safe
  caller.
- Add no third-party Rust dependency without an accepted measured decision.
- Remove stale aliases/contracts instead of preserving compatibility shims.
- Mark every placeholder in code, behavior, and documentation as `PLACEHOLDER`.
- Prefer complete vertical slices and focused conformance tests over mocks.

## Current Sharp Edges

- the removed legacy source contract still enforces depth 8, 16 form children, 384 tokens per file,
  8 top-level forms, 15 product fields, and 16 immediate source-directory
  entries. Repository topology contracts do not change those language limits.
- Imports still merge declarations into one loaded-closure namespace; modules,
  exports, package coherence, and general ownership are incomplete.
- `set` remains function-local and SSA joins use stable BindingId-ordered block
  parameters. Workload state is product-threaded.
- The independent SSA evaluator intentionally reports host operations as
  unsupported; it is not a host-runtime substitute.
- Compiler execution authority is verified normalized SSA plus validated
  reference bytecode. Do not restore an independent HIR-to-bytecode emitter or
  let a backend reinterpret source syntax.
- VM host operations block, and process-global terminal/stdio wrappers prevent
  concurrent VM supervision. Core exit remains a structured outcome.
- Current auto execution is baseline-only. Forced proof optimization is Current,
  but automatic promotion, OSR, deoptimization, and speculation are absent.
- Linux x86-64 callable-native claims require real synchronous generated entry
  from verified SSA. Emission, disassembly, or historical foundation scaffolding
  alone is not current tier evidence.
- String/file helpers may still perform per-byte calls or quadratic construction.
  Raw terminal redraw requires CR+LF, idle editor operation must not repaint,
  and final cursor placement requires a flush.

## Accepted Next Sequence

1. Continue moving the [resource profile](../decisions/platform/resource-budget-profiles.md)
   to whole-pipeline pre-allocation hierarchical request charging.
2. Implement the canonical source contract from the accepted ADT, pattern, control-flow, numeric,
   value, layout, and typed-error contracts.
3. Carry each the canonical source contract slice through HIR, verified SSA, the evaluator, VM,
   baseline native execution, and forced proof-optimized execution.

This order is an accepted implementation contract, not a capability claim.
Automatic proof promotion and its retained gate remain later measured work.

## Change Discipline

Update the authority before public behavior. Preserve Current, Accepted Target,
Deferred, Rejected, and historical evidence distinctions. Moves require
repository-wide link updates with no aliases. Generated outputs belong under
`target/`; immutable evidence bytes are not reformatted.

Use [Verification](verification.md). Record only commands that ran, with exact
commit/environment/result, and retain every failed or rejected experiment.
Code/build/runtime gates remain explicitly not tested for documentation-only
changes.
