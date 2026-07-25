# Agent Handoff

## Purpose

Capture product intent and known sharp edges without preserving obsolete
implementation contracts.

## Status

**Current** for engineering policy, the semantic/outcome foundation, verified
typed SSA, independent evaluation/baseline normalization, reference-bytecode
cutover, callable scalar and allocation-capable Linux x86-64 baseline tiers,
and forced certificate-verified optimizing execution. Ownership/traits,
Handle/host transitions, automatic optimizing promotion, and broader proof
passes remain **Accepted Targets**.

## Product Intent

- Build the language, compiler, runtime, standard library, and future ecosystem
  as one coherent product named `lkjscript`.
- Canonical accepted sources use `.lkjscript`; do not preserve `.lkjml` support.
- Keep the Rust host small, owned, and Linux-first. Grow policy in lkjscript
  source rather than host frameworks.
- Keep unsafe Rust inside `lkjscript-sys`, and require every safe wrapper to
  uphold memory safety for arbitrary safe callers.
- Do not add a crates.io dependency without an accepted decision record.
- Backward compatibility is not required; remove stale aliases and contracts.
- A source directory may contain at most 16 immediate files plus directories.
  This language rule does not constrain Rust/docs/infrastructure layout.
- Placeholders are allowed only when code, observable behavior, and docs all
  explicitly label them `PLACEHOLDER`.
- Prefer complete vertical slices and focused conformance tests over broad mock
  scaffolding.

## Layout

```text
src/std/          language standard library
src/lib/          reusable language packages
src/examples/     executable validation workloads
crates/           typed SSA, compiler, core, VM, sys, CLI, and gates
meta/             Docker, scripts, benchmark comparators, and configuration
```

## Current Sharp Edges

- Imports merge immutable function and product declarations into one program
  declaration namespace; there are not yet modules or exports.
- Function closures are still installed in internal VM global slots before
  source main, but those slots are not source values or mutable source state.
- `set` is local-only. It targets the nearest same-function `var` by stable HIR
  BindingId and is environment-renamed into SSA; branch and loop state joins use
  explicit block parameters in stable BindingId order. lkjedit, terminal, and
  Brainfuck state is product-threaded.
- `lkjscript-ir` is dependency-free and backend-independent. Its evaluator is
  not a host-runtime substitute: console, filesystem, sockets, terminal, time,
  and handle operations report explicit unsupported-evaluator outcomes.
- Compiler results are `ExecutableProgram` values retaining verified normalized
  SSA, deterministic function/prototype/main and bytecode-position links, and
  validated bytecode through `bytecode()`. Do not reintroduce a HIR-to-bytecode
  semantic emitter.
- Raw terminal redraw must emit CR+LF; LF-only output causes staircase display.
- lkjedit idle must wait without full repaint.
- Final cursor placement must be followed by a flush.
- Current string/file helpers may perform per-byte syscalls or quadratic string
  construction.
- VM host operations block; stdin/stdout, terminal guard, and some host
  wrappers remain process-global, so concurrent VM supervision is unsupported.
  Core exit is a structured outcome rather than process termination.
- Bounded terminal operations, stale-safe handles, truthful Results, exact
  I64/F64 execution, resolved typed HIR, Unit/strict-if, typed empty lists,
  Option/no-nil, explicit equality families, immutable nominal products,
  explicit main, declaration-only imports, local-only mutation, product-
  threaded workload state, fixed-point effects, chunk validation, structured
  process-safe outcomes, verified typed SSA, differential evaluation, baseline
  passes, measured backend selection, SSA-to-bytecode cutover, bounded W^X code
  objects, actually called synchronous Linux x86-64 baseline JIT, exact native
  host-independent roots/allocation/collection, and the forced first
  certificate-verified optimizing tier have landed. The active sequence is
  broader ownership/coherent traits and Handle/host transitions, then measured
  automatic optimizing promotion and broader proof passes. Forced first-tier
  performance is Adopted from the clean `cc967ff` run at 2.984780x after the
  retained `063668e` run was Rejected by its scalar native sentinel; preserve
  both records. Automatic promotion remains disabled and unmeasured, with no
  OSR, deoptimization, or speculation claim. Do not add inert
  engine flags or label baseline code optimizing. Loop OSR and a minimal AOT
  test emitter remain later targets, not current capability. Offline PGO is
  rejected by product decision. See [Allocation-Capable Baseline
  JIT](../decisions/allocation-capable-baseline-jit.md) and [Proof-Based
  Optimizing JIT](../decisions/proof-based-optimizing-jit.md).

## Host Boundary

Terminal, filesystem, network, and time policy belongs in the standard library
when a safe thin primitive can support it. Do not reintroduce removed fat host
features merely for convenience. Bulk operations are appropriate when they
are necessary for correctness or measured performance.

## Verification Discipline

Use [verification.md](verification.md). Record commands that actually ran,
including expected failures. Keep rejected experiment results in
[../vision/experiments.md](../vision/experiments.md).
