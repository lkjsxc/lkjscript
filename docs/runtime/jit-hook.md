# JIT Observation Hook

## Purpose

Describe the explicitly incomplete call-observation seam reserved for native
code experiments.

## Status

**PLACEHOLDER.** There is no native compiler, compiled-code object, execution
handoff, deoptimization path, or JIT performance claim.

## Current Behavior

The VM invokes `JitHook::maybe_compile` when calling a closure. The baseline
hook returns a Boolean that the VM ignores; execution always remains in the
interpreter. During the foundation cycle this API will be made explicitly
observation-only so its type cannot imply a usable handoff.

## Completion Contract

This placeholder may become current only when a typed IR feeds executable code
objects, calls can transfer to compiled code, failures and deoptimization have
defined behavior, and warmup plus steady-state measurements beat the retained
interpreter baseline without correctness regressions.
