# Principles

## Purpose

Rank the invariants used when architectural goals conflict.

## Status

**Current engineering policy.**

## Ranked List

1. Truthful contracts and evidence outrank aspirational feature breadth.
2. Memory safety and semantic conformance precede performance claims.
3. Documentation changes define a public contract before implementation.
4. Backward compatibility is not required; obsolete surfaces are removed.
5. Placeholders are allowed only when explicitly labeled everywhere visible.
6. Prefer complete vertical slices over mocks and dormant interfaces.
7. Measure isolated candidates and multiple combinations before adoption.
8. Preserve rejected ideas with the conditions under which they may become useful.
9. Grow capability in lkjscript libraries over a small, safe host boundary.
10. Keep unsafe Rust isolated in `lkjscript-sys`; safe wrappers uphold safety.
11. Add no third-party Rust dependency without a measured decision record.
12. Keep language source shallow and at most 16 entries wide per directory.
13. Keep bytecode/value layouts cache-conscious, but redesign them when evidence
    supports a simpler or faster truthful contract.
14. Build portability seams honestly while accepting Linux-first delivery.
15. Delete redundant tests and generated artifacts only when stronger evidence
    or reproducibility makes them unnecessary.
