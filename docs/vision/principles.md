# Principles

## Purpose

Rank the invariants used when architectural goals conflict.

## Status

**Current engineering policy.**

## Ranked List

1. Truthful contracts and evidence outrank aspirational feature breadth.
2. Memory safety and semantic conformance precede performance claims.
3. Documentation changes define a public contract before implementation.
4. Give one concept one canonical, explicit source form; AI convenience never
   authorizes implicit conversion, authority, mutation, or optimizer assumptions.
5. Backward compatibility is not required; obsolete surfaces are removed.
6. Placeholders are allowed only when explicitly labeled everywhere visible.
7. Prefer complete vertical slices over mocks and dormant interfaces.
8. Measure isolated candidates and multiple combinations before adoption.
9. Preserve rejected ideas with the conditions under which they may become useful.
10. Grow capability in lkjscript libraries over a small, safe host boundary.
11. Keep unsafe Rust machine-registered by coherent mechanism; safe wrappers uphold safety.
12. Add no third-party Rust dependency without a measured decision record that
    distinguishes runtime, build-time backend, and language-package impact.
13. Keep Current canonical source limits until aggregate checked replacements
    are Current; then measure semantic-cohesion and AI-context lints rather than
    treating 16-entry directory width as permanent language identity.
14. Keep the reference VM cache-conscious, but do not carry universal tagged
    values into typed native hot paths when static representation is available.
15. Use one typed semantic IR family for VM, runtime JIT, minimal AOT tests,
    and Wasm lowering.
16. Keep adaptive observations bounded, process-local, and distinct from
    telemetry. Runtime JIT is the primary adaptive strategy; optional explicit
    local PGO is deferred until common AOT/artifact identity and is never
    uploaded telemetry.
17. Build portability seams honestly while accepting Linux-first delivery.
18. Delete redundant tests and generated artifacts only when stronger evidence
    or reproducibility makes them unnecessary.
