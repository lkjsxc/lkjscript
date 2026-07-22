# JIT Observation Hook

## Purpose

Describe the explicitly incomplete call-observation seam reserved for native
code experiments.

## Status

**PLACEHOLDER.** There is no native compiler, compiled-code object, execution
handoff, deoptimization path, or JIT performance claim.

## Current Behavior

The VM invokes `JitHook::observe_call` when calling a closure. The method
returns no compilation status and execution always remains in the interpreter.
The source and documentation both label this seam `PLACEHOLDER`, so its type
cannot imply a usable native execution handoff.

## Replacement Contract

This placeholder is replaced only when typed SSA feeds bounded executable code
objects, a forced mode proves calls transfer to native code, VM/native outcomes
and precise GC stack maps are exact, and failures have structured fallback or
forced-mode error behavior. Baseline JIT is non-speculative and does not require
deoptimization. Loop execution is not called OSR until live VM state transfers
through a verified loop-header mapping.

Current-process hotness counters are bounded, saturating, local, ephemeral, and
never telemetry. Offline PGO and persistent profiles/caches are not part of the
accepted plan. See
[Runtime JIT Instead of Offline PGO](../decisions/runtime-jit-instead-of-offline-pgo.md).
