# Agent Guide

## Purpose

Define the working contract for automated engineering in this repository.

## Product Direction

`lkjscript` is an AI-primary, statically typed, memory-safe language and
platform implemented today by a small Rust compiler, verified typed SSA,
reference bytecode VM, callable Linux x86-64 baseline JIT, and forced
proof-checked optimizing JIT. The accepted destination is a versioned Semantic
Source system, explicit capabilities/effects, value semantics, hybrid
affine/region/traced memory safety, reproducible packages/components, and one
semantic IR family feeding a measured evaluator/VM/JIT/AOT/cache/Wasm
portfolio. Current line-oriented Edition 1 source remains the deterministic
text projection during migration; it is not the permanent editing identity.
Linux x86-64 tier evidence requires real synchronous calls from verified SSA;
code emission, disassembly, SSA scaffolding, or observation alone is
insufficient. The canonical accepted extension is `.lkjscript`; `.lkjml` is
rejected without an explicitly editioned migration mode. Linux x86-64 is the
current acceptance platform. Portability is a design constraint, not a current
support claim.

## Non-Negotiable Rules

1. Update the authoritative documentation before changing a public contract.
2. Keep current behavior, accepted targets, experiments, deferred work,
   placeholders, and superseded contracts visibly distinct.
3. A placeholder is allowed only when its code, user-facing behavior, and
   documentation explicitly say `PLACEHOLDER`. Unmarked inert behavior is a
   defect and must be implemented or removed.
4. Backward compatibility is not required. Remove obsolete paths instead of
   retaining aliases, fallback behavior, or shims.
5. Do not weaken any Current source or artifact limit before aggregate checked
   replacement bounds are Current. The Edition 1 16-entry source-directory
   rule remains enforced during that migration; its accepted destination is an
   AI-maintainability lint under versioned implementation maxima and host
   resource profiles, not permanent language semantics.
6. Keep pure compiler/runtime state separate from host effects. Unsafe Rust is
   confined to `lkjscript-sys`, whose safe API must uphold Rust safety for all
   callers.
7. Do not claim success for a command that did not run. Record the commit,
   environment, command, and result for durable evidence.
8. Prefer focused conformance and boundary tests. Delete redundant tests only
   when equal or stronger coverage replaces them.
9. Do not add a third-party Rust dependency without an accepted decision
   record and measured justification that distinguishes runtime, build-time
   backend, and language-package effects.
10. Do not trust AI-authored optimizer hints. Prove them, retain a runtime
    check, or reject them; undefined behavior is not a performance mechanism.
11. Keep VM, runtime JIT, minimal AOT tests, and Wasm semantics behind one
    resolved typed IR family; do not independently reinterpret untyped syntax
    in a backend.
12. Use one build-artifact tree, monitor free space, and remove reproducible
    experiment artifacts after recording compact results.
13. Record rejected experiments as carefully as adopted ones, including
    combinations that may become useful under different conditions.
14. Keep commits coherent and include `Tested:` and `Not-tested:` trailers.
15. Runtime JIT remains the primary adaptive-performance path, but not the only
    final execution strategy. AOT, content-addressed native caches, and optional
    explicit local PGO require the shared verified SSA/artifact-identity
    foundations, an accepted measured slice, and no uploaded telemetry or
    semantic divergence.

## Read Order

1. [docs/current-state.md](docs/current-state.md)
2. [docs/operations/architecture.md](docs/operations/architecture.md)
3. [docs/decisions/ai-native-platform.md](docs/decisions/ai-native-platform.md)
4. [docs/decisions/semantic-source-and-agent-protocol.md](docs/decisions/semantic-source-and-agent-protocol.md)
5. [docs/decisions/resource-budget-profiles.md](docs/decisions/resource-budget-profiles.md)
6. [docs/decisions/execution-portfolio.md](docs/decisions/execution-portfolio.md)
7. [docs/language/README.md](docs/language/README.md)
8. [docs/decisions/semantic-core.md](docs/decisions/semantic-core.md)
9. [docs/decisions/equality-families.md](docs/decisions/equality-families.md)
10. [docs/decisions/immutable-nominal-products.md](docs/decisions/immutable-nominal-products.md)
11. [docs/decisions/compiler-pipeline.md](docs/decisions/compiler-pipeline.md)
12. [docs/decisions/ownership-and-borrowing.md](docs/decisions/ownership-and-borrowing.md)
13. [docs/decisions/traits-and-static-dispatch.md](docs/decisions/traits-and-static-dispatch.md)
14. [docs/decisions/native-references-and-gc-stack-maps.md](docs/decisions/native-references-and-gc-stack-maps.md)
15. [docs/decisions/runtime-jit-instead-of-offline-pgo.md](docs/decisions/runtime-jit-instead-of-offline-pgo.md)
16. [docs/decisions/callable-baseline-jit.md](docs/decisions/callable-baseline-jit.md)
17. [docs/decisions/allocation-capable-baseline-jit.md](docs/decisions/allocation-capable-baseline-jit.md)
18. [docs/decisions/proof-based-optimizing-jit.md](docs/decisions/proof-based-optimizing-jit.md)
19. [docs/operations/verification.md](docs/operations/verification.md)
20. [docs/vision/README.md](docs/vision/README.md)
21. [docs/vision/performance-scorecard.md](docs/vision/performance-scorecard.md)
22. [docs/vision/experiments.md](docs/vision/experiments.md)

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
