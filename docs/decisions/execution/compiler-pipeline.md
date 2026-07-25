# Typed Compiler Pipeline And Runtime JIT

## Purpose

Define one semantic pipeline shared by the reference VM, runtime JIT, minimal
native file-emission tests, and future Wasm so performance backends cannot
reinterpret the language.
## Status

**Current** for validated Semantic Source Foundation tree -> private checked
Edition 1 projection -> resolved typed HIR -> mandatory initial `Owned Buf`
ownership analysis with fixed-point function effects -> verified typed SSA ->
verified baseline normalization -> reference bytecode. The independent bounded SSA evaluator and
bytecode link metadata are
also Current. The selected owned Linux x86-64 scalar machine-plan encoder, safe W^X boundary,
narrow verified-SSA adapter, bounded code objects, forced/automatic callable
baseline tier, host-independent native references/allocation, and the forced
first proof-based optimizing slice are **Current**. Synchronous automatic
proof promotion is an **Accepted Implementation Selection**, not Current.
Handle/host native calls, native/VM reference transitions, loop OSR, minimal AOT
test emission, production AOT, content-addressed cache, and direct Wasm are
**Accepted Targets**. The backend is specified by [Linux x86-64 Native
Backend](linux-x86-64-native-backend.md). Optional explicit local PGO is a
**Deferred Optional Target**, not Current, under [Measured Execution
Portfolio](execution-portfolio.md).

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each
capsule preserves a cohesive part of the record. Current means implemented and
evidenced; accepted selections and targets are future work. Deferred and
Rejected remain non-current.

## Strict Capsule Manifest

- [Pipeline](compiler-pipeline/pipeline.md)
- [Wasm](compiler-pipeline/wasm.md)
