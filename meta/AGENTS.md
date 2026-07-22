# AGENTS.md

## Purpose

Entry instructions for automated coding agents working in this repository
(`https://github.com/lkjsxc/lkjscript2026`). This directory is the repo root.

## What lkjscript2026 Is

A small functional language with the line-oriented, attribute-less LKJML
surface, a Rust bytecode VM, AI-friendly source budgets, and Docker-gated
verification.

## Non-Negotiable Rules

1. `docs/` is the implementation contract. Update docs with behavior changes,
   including [docs/current-state.md](../docs/current-state.md).
2. Prefer token / top-level-form budgets over line-count rules; keep files small by meaning.
3. Docs use ASCII prose, kebab-case filenames, one H1, then a Purpose section.
4. Prefer more shallow `def`s and more files over deep nests or fat files.
5. Limit numbers are hardcoded language constants for now; do not invent user-facing JSON limits.
6. Pure core, effects at the edges; no panic paths in product crates.
7. Honest state only: no fake success or unrun gate claims.
8. Commit small slices with `Tested` and `Not-tested` trailers.

## Read Order

1. [docs/current-state.md](../docs/current-state.md)
2. [docs/operations/agent-handoff.md](../docs/operations/agent-handoff.md)
3. [docs/vision/README.md](../docs/vision/README.md)
4. [docs/language/README.md](../docs/language/README.md)
5. [docs/runtime/README.md](../docs/runtime/README.md)
6. [docs/operations/verification.md](../docs/operations/verification.md)

## Verification

A gate that did not run did not pass. Prefer `cargo run -p lkjscript2026-xtask -- quiet verify`
and Docker when claiming completion.
