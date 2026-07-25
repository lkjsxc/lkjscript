# Runtime

## Purpose

Describe the reference VM and the exact callable Linux x86-64 baseline tier.

## Status

The validated bounded bytecode VM, structured outcomes, callable baseline JIT,
host-independent native references/allocation/recursion in forced mode, and
forced proof-optimizing execution are **Current**. Auto remains baseline-only
with a scalar VM/native entry adapter, and the VM remains the cold full-language
oracle. Synchronous automatic proof promotion is an **Accepted Implementation
Selection**, not yet Current. Handle/host native calls, native/VM reference
transfer, OSR, and background work are not implemented.

## Table of Contents

- [vm.md](vm.md): validation, outcomes, limits, values, frames, heap, and host boundary
- [baseline-jit.md](baseline-jit.md): callable scalar/forced-reference coverage, engines, PollV1,
  code objects, W^X, and the selected automatic-promotion boundary
- [../decisions/callable-baseline-jit.md](../decisions/jit/callable-baseline-jit.md): Linux x86-64
  callable-baseline contract
- [../decisions/runtime-jit-instead-of-offline-pgo.md](../decisions/jit/runtime-jit-instead-of-offline-pgo.md):
  accepted later tiers, OSR, and no-PGO contract
