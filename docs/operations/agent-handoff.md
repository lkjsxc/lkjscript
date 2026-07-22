# Agent Handoff

## Purpose

Capture product intent and known sharp edges without preserving obsolete
implementation contracts.

## Status

**Current** for engineering policy. Foundation changes are **Accepted Target**.

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

- Imports merge definitions into one program-global namespace.
- Top-level definitions are installed in source order at runtime.
- `set` is heavily used by lkjedit and remains program-global despite exact
  target/type checking.
- Raw terminal redraw must emit CR+LF; LF-only output causes staircase display.
- lkjedit idle must wait without full repaint.
- Final cursor placement must be followed by a flush.
- Current string/file helpers may perform per-byte syscalls or quadratic string
  construction.
- VM host operations block and process exit is not process-safe.
- Bounded terminal operations, stale-safe handles, truthful Results, exact
  I64/F64 execution, resolved typed HIR, Unit/strict-if, and typed empty lists
  have landed. Option/no-nil is the next semantic slice, followed by equality
  and explicit main/local-state migration. SSA/AOT remain targets, not current
  capability.

## Host Boundary

Terminal, filesystem, network, and time policy belongs in the standard library
when a safe thin primitive can support it. Do not reintroduce removed fat host
features merely for convenience. Bulk operations are appropriate when they
are necessary for correctness or measured performance.

## Verification Discipline

Use [verification.md](verification.md). Record commands that actually ran,
including expected failures. Keep rejected experiment results in
[../vision/experiments.md](../vision/experiments.md).
