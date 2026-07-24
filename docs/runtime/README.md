# Runtime

## Purpose

Describe the reference VM and the exact callable Linux x86-64 baseline tier.

## Status

The validated bounded bytecode VM, structured outcomes, and allocation-free
scalar callable baseline JIT are **Current**. The VM is the default and
full-language oracle. Native references, allocation, host operations, recursion,
OSR, optimizing compilation, and background work are not implemented.

## Table of Contents

- [vm.md](vm.md): validation, outcomes, limits, values, frames, heap, and host boundary
- [baseline-jit.md](baseline-jit.md): exact callable scalar coverage, engines, PollV1, code objects, W^X, and unsupported boundaries
- [../decisions/callable-baseline-jit.md](../decisions/callable-baseline-jit.md): Linux x86-64 callable-baseline contract
- [../decisions/runtime-jit-instead-of-offline-pgo.md](../decisions/runtime-jit-instead-of-offline-pgo.md): accepted later tiers, OSR, and no-PGO contract
