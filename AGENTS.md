# Agent Guide

## Purpose

Define the working contract for automated engineering in this repository.

## Product Direction

`lkjscript` is an AI-primary, statically typed, memory-safe language and
platform implemented today by a small Rust compiler, verified typed SSA,
reference bytecode VM, callable Linux x86-64 baseline JIT, and forced
proof-checked optimizing JIT. The language has one content-addressed semantic
contract, explicit capabilities/effects, value semantics, hybrid
affine/region/traced memory safety, reproducible packages/components, and one
semantic IR family feeding a measured evaluator/VM/JIT/AOT/cache/Wasm
portfolio. Current line-oriented source is the deterministic text projection;
it is not the permanent editing identity.
Linux x86-64 tier evidence requires real synchronous calls from verified SSA;
code emission, disassembly, SSA scaffolding, or observation alone is
insufficient. The canonical accepted extension is `.lkjscript`; `.lkjml` is
rejected without an explicitly editioned migration mode. Linux x86-64 is the
current acceptance platform. Portability is a design constraint, not a current
support claim.

## Enforcement Labels

- `[machine: RULE]`: enforced by the named deterministic rule.
- `[verified: command/review]`: covered by that named gate or required review.
- `[external]`: requires human authority because automation cannot decide it.
- `[information]`: product or status context, not an executable instruction.

## Non-Negotiable Rules

1. `[verified: contract review]` Update authority documentation before changing
   a public contract.
2. `[machine: LKJ-DOC-STATUS]` Keep Current, Accepted Target, experimental,
   Deferred, Rejected, and superseded status distinct.
3. `[machine: LKJ-DOC-PLACEHOLDER]` Inert behavior must say `PLACEHOLDER` in
   code, behavior, and documentation or be implemented/removed.
4. `[verified: change review]` Remove obsolete paths; do not retain aliases,
   fallback behavior, or shims merely for compatibility.
5. `[verified: resource-boundary tests]` Do not weaken a Current source or
   artifact limit before its aggregate checked replacement is Current.
6. `[machine: LKJ-REPO-*]` Authored files and directories obey the
   [bounded-topology contract](docs/decisions/platform/bounded-repository-topology.md).
7. `[machine: Rust lint and crate graph]` Keep pure compiler/runtime state apart
   from host effects; unsafe Rust is confined to `lkjscript-sys` behind a safe
   caller contract.
8. `[verified: evidence review]` Never claim an unrun command. Record commit,
   environment, exact command, result, and explicit untested gates.
9. `[verified: focused tests]` Delete a test only when equal or stronger focused
   conformance or boundary coverage replaces it.
10. `[external: dependency policy]` A third-party Rust dependency requires an
    accepted record, license/advisory review, and measured runtime/build/package
    justification.
11. `[verified: proof checker]` Prove AI optimizer hints, retain a runtime check,
    or reject them; undefined behavior is not an optimization mechanism.
12. `[verified: architecture review]` VM, JIT, AOT, and Wasm consume one resolved
    typed IR family; no backend independently interprets untyped syntax.
13. `[machine: LKJ-REPO-GENERATED-PROVENANCE]` Use one artifact tree; remove
    reproducible temporary outputs after retaining compact evidence.
14. `[verified: experiment schema]` Preserve adopted and rejected experiment
    evidence, including combinations useful under other conditions.
15. `[verified: commit review]` Commits are coherent and include exact `Tested:`
    and `Not-tested:` trailers.
16. `[information]` Runtime JIT is the primary adaptive path. AOT, native caches,
    and explicit local PGO require shared verified SSA/artifact identity,
    accepted measured slices, no telemetry upload, and no semantic divergence.

## Read Order

1. [Current state](docs/current-state.md)
2. [Capability status](docs/operations/status-authority.md)
3. [Architecture](docs/operations/architecture.md)
4. [Content-addressed contracts](docs/decisions/platform/content-addressed-contracts.md)
5. [AI-native platform](docs/decisions/platform/ai-native-platform.md)
6. [Bounded topology](docs/decisions/platform/bounded-repository-topology.md)
7. [Semantic Source and protocol](docs/decisions/platform/semantic-source-and-agent-protocol.md)
8. [Resource profiles](docs/decisions/platform/resource-budget-profiles.md)
9. [Execution portfolio](docs/decisions/execution/execution-portfolio.md)
10. [Language](docs/language/README.md)
11. [Semantic core](docs/decisions/semantics/semantic-core.md)
12. [Equality](docs/decisions/semantics/equality-families.md)
13. [Products](docs/decisions/semantics/immutable-nominal-products.md)
14. [Compiler pipeline](docs/decisions/execution/compiler-pipeline.md)
15. [Ownership](docs/decisions/semantics/ownership-and-borrowing.md)
16. [Traits](docs/decisions/semantics/traits-and-static-dispatch.md)
17. [Native roots](docs/decisions/jit/native-references-and-gc-stack-maps.md)
18. [Runtime JIT](docs/decisions/jit/runtime-jit-instead-of-offline-pgo.md)
19. [Callable baseline JIT](docs/decisions/jit/callable-baseline-jit.md)
20. [Allocation JIT](docs/decisions/jit/allocation-capable-baseline-jit.md)
21. [Proof JIT](docs/decisions/jit/proof-based-optimizing-jit.md)
22. [Repository graph](docs/decisions/platform/repository-intelligence-graph.md)
23. [Agent state](docs/decisions/platform/agent-work-state.md)
24. [Verification](docs/operations/verification.md)
25. [Vision and evidence](docs/vision/README.md)

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
