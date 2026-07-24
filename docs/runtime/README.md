# Runtime

## Purpose

Describe the current VM and explicitly incomplete native-code boundary.

## Status

The validated, bounded bytecode VM and structured execution outcomes are
**Current**. Native JIT execution is **Placeholder** and must not be described
as implemented. Runtime-JIT-first tiering is an **Accepted Target**; offline PGO
is rejected.

## Table of Contents

- [vm.md](vm.md): validation, outcomes, limits, values, frames, heap, and host boundary
- [jit-hook.md](jit-hook.md): explicitly labeled observation-only placeholder
- [../decisions/callable-baseline-jit.md](../decisions/callable-baseline-jit.md): required Linux x86-64 callable-baseline cycle
- [../decisions/runtime-jit-instead-of-offline-pgo.md](../decisions/runtime-jit-instead-of-offline-pgo.md): accepted tiers, OSR, code-object, and no-PGO contract
