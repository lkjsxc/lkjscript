# Agent Handoff

## Purpose

Capture product intent and known sharp edges without preserving obsolete
implementation contracts.

## Status

**Current** for engineering policy and the explicit-main/local-mutation
foundation. Later validation, outcomes, SSA, and native execution remain
**Accepted Targets**.

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
crates/           compiler, VM, sys, CLI, and gates
meta/             Docker, scripts, benchmark comparators, and configuration
```

## Current Sharp Edges

- Imports merge immutable function and product declarations into one program
  declaration namespace; there are not yet modules or exports.
- Function closures are still installed in internal VM global slots before
  source main, but those slots are not source values or mutable source state.
- `set` is local-only. It targets the nearest same-function `var` by stable HIR
  binding and slot; lkjedit, terminal, and Brainfuck state is product-threaded.
- Raw terminal redraw must emit CR+LF; LF-only output causes staircase display.
- lkjedit idle must wait without full repaint.
- Final cursor placement must be followed by a flush.
- Current string/file helpers may perform per-byte syscalls or quadratic string
  construction.
- VM host operations block and process exit is not process-safe.
- Bounded terminal operations, stale-safe handles, truthful Results, exact
  I64/F64 execution, resolved typed HIR, Unit/strict-if, typed empty lists,
  Option/no-nil, explicit equality families, immutable nominal products,
  explicit main, declaration-only imports, local-only mutation, and product-
  threaded workload state have landed. The active cycle next adds fixed-point
  effects, chunk validation, process-safe outcomes, verified typed SSA, one
  measured backend, W^X code objects, and an actually called synchronous Linux
  x86-64 baseline JIT. Loop OSR, optimizing JIT, and a
  minimal AOT test emitter remain later targets, not current capability. Offline
  PGO is rejected by product decision. See
  [Callable Linux x86-64 Baseline JIT Cycle](../decisions/callable-baseline-jit.md).

## Host Boundary

Terminal, filesystem, network, and time policy belongs in the standard library
when a safe thin primitive can support it. Do not reintroduce removed fat host
features merely for convenience. Bulk operations are appropriate when they
are necessary for correctness or measured performance.

## Verification Discipline

Use [verification.md](verification.md). Record commands that actually ran,
including expected failures. Keep rejected experiment results in
[../vision/experiments.md](../vision/experiments.md).
