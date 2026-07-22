# Principles

## Purpose

Ranked invariants that shape every change.

## Ranked List

1. Honest gates beat aspirational claims.
2. Prefer shallow multi-def and multi-file over deep nests.
3. Keep the host thin and syscall-shaped; grow capability in `.lkjml`.
4. Prefer one resource-efficient runtime per OS user, or per Docker container,
   with isolated logical processes, shared immutable work, and explicit global
   budgets.
5. Backward compatibility is not a project constraint; replace obsolete
   contracts instead of carrying compatibility shims.
6. Avoid Python in project tooling. Use Rust or shell by default; allow Python
   only when an experiment or external comparison materially benefits from it.
7. No new third-party Rust crates without an ADR; prefer owned scratch code.
8. Do not add fat host “feature” opcodes when a script library could own it.
9. Limits are language constants / policy knobs, not sacred forever.
10. Dense bytecode and tagged values stay cache-friendly and JIT-ready.
11. Docs and code move together.
