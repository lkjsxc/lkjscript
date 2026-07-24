# Staged Self-Hosted lkjscript Platform

## Purpose

Fix the boundary between the trusted Rust bootstrap/native kernel and product
logic that should eventually be implemented in lkjscript.

## Status

**Current** only for the Rust compiler, verified typed-SSA pipeline, reference
VM, owned Linux x86-64 emitter, and audited host capabilities described by
[Current State](../current-state.md). Staged self-hosting is an **Accepted
Target**. No compiler stage written in lkjscript is Current.

## Decision

lkjscript adopts staged self-hosting. The minimum trusted native kernel may
retain raw syscalls, executable-memory ownership, generated-code entry, thread
creation, stack switching, signal handling, the lowest collector mechanisms,
cryptographic-provider boundaries, and bootstrap. Parser policy, type and
ownership rules, optimization, packages, framework behavior, database policy,
and ordinary application logic should migrate to lkjscript.

The accepted sequence is:

```text
Rust bootstrap compiler
  -> lkjscript lexer, parser, and formatter
  -> lkjscript resolver, type checker, borrow checker, and trait front end
  -> lkjscript typed-SSA construction and optimizer
  -> lkjscript compiler driver, package manager, test runner, and doc generator
  -> stage comparison
  -> reproducible self-hosting
```

Each stage consumes the same canonical source and emits an explicitly versioned
semantic artifact. Stage comparison must compare normalized HIR/SSA, diagnostics,
bytecode, or native object facts as appropriate; successful execution alone is
not equivalence evidence.

## Bootstrap Boundary

The bootstrap compiler remains sufficient to rebuild the first self-hosted
stage from a locked source graph. Native capabilities expose narrow typed and
versioned operations rather than Rust policy libraries. Adding a language
feature must not require parallel implementations in every runtime layer;
resolved typed SSA and versioned runtime calls remain the shared boundary.

## Reproducibility

Self-hosting acceptance requires exact package locks, content hashes, compiler
semantic and native ABI identities, bounded deterministic diagnostics, and
recorded host/target configuration. Native code bytes need not be identical
across explicitly different target modes, but every difference must be
accounted for by the selected target identity.

## Deferred

A self-hosted compiler, package registry, distributed builds, cross-platform
bootstrap, and Linux AArch64 code generation are **Deferred** until the
ownership, traits, modules/packages, allocation-capable JIT, and reproducible
stage-artifact contracts are Current.

## Rejected

A Rust rewrite of every ecosystem layer, a bootstrap that depends on an
unrecorded prior binary, and declaring self-hosting from a program that merely
invokes the Rust compiler are **Rejected**.
