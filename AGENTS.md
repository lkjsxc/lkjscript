# Agent Guide

## Purpose

Define the working contract for automated engineering in this repository.

## Product Direction

`lkjscript` is a typed, line-oriented language implemented by a small Rust
compiler and bytecode VM. The canonical accepted source extension is
`.lkjscript`; the repository is moving to it without `.lkjml` compatibility.
Linux x86-64 is the current acceptance platform. Portability is a design
constraint, not a current support claim.

## Non-Negotiable Rules

1. Update the authoritative documentation before changing a public contract.
2. Keep current behavior, accepted targets, experiments, deferred work,
   placeholders, and superseded contracts visibly distinct.
3. A placeholder is allowed only when its code, user-facing behavior, and
   documentation explicitly say `PLACEHOLDER`. Unmarked inert behavior is a
   defect and must be implemented or removed.
4. Backward compatibility is not required. Remove obsolete paths instead of
   retaining aliases, fallback behavior, or shims.
5. An lkjscript source directory may contain at most 16 immediate entries,
   counting files and subdirectories together. This is a language source-tree
   rule, not a rule for Rust crates, documentation, metadata, or generated
   build trees.
6. Keep pure compiler/runtime state separate from host effects. Unsafe Rust is
   confined to `lkjscript-sys`, whose safe API must uphold Rust safety for all
   callers.
7. Do not claim success for a command that did not run. Record the commit,
   environment, command, and result for durable evidence.
8. Prefer focused conformance and boundary tests. Delete redundant tests only
   when equal or stronger coverage replaces them.
9. Do not add a third-party Rust dependency without an accepted decision
   record and measured justification.
10. Use one build-artifact tree, monitor free space, and remove reproducible
    experiment artifacts after recording compact results.
11. Record rejected experiments as carefully as adopted ones, including
    combinations that may become useful under different conditions.
12. Keep commits coherent and include `Tested:` and `Not-tested:` trailers.

## Read Order

1. [docs/current-state.md](docs/current-state.md)
2. [docs/operations/architecture.md](docs/operations/architecture.md)
3. [docs/language/README.md](docs/language/README.md)
4. [docs/operations/verification.md](docs/operations/verification.md)
5. [docs/vision/README.md](docs/vision/README.md)
6. [docs/vision/experiments.md](docs/vision/experiments.md)

## Development Loop

1. Establish current evidence.
2. Write the accepted contract.
3. Define hypotheses and adoption criteria.
4. Implement complete alternatives, not mocks.
5. Run focused correctness gates.
6. Measure multiple candidates and combinations.
7. Adopt or reject from evidence.
8. Synchronize current-state and historical records.
9. Remove obsolete code and tests.
10. Commit, integrate, and continue with the next highest-value risk.

## Verification

The canonical local gate is:

```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Runtime and Docker acceptance are separate gates. See
[docs/operations/verification.md](docs/operations/verification.md).
