# Agent Guide

## Purpose

Define the working contract for automated engineering in this repository.

## Product Direction

`lkjscript` is an AI-primary, statically typed, memory-safe language and
platform implemented today by a small Rust compiler, verified typed SSA,
reference bytecode VM, callable Linux x86-64 baseline JIT, and forced
proof-checked optimizing JIT. The language has one content-addressed semantic
contract, explicit capabilities/effects, and value semantics. Current execution
still uses an exact tracing heap during migration; the accepted destination is
collector-free inferred ownership, borrowing, regions, sealed sharing, and
pools. One semantic resource plane separates compiler-verified task legality
from measured topology, scheduling, and memory-home policy while Linux remains
the system-wide scheduler. Reproducible packages/components and one semantic IR
family feed a measured evaluator/VM/JIT/AOT/cache/Wasm portfolio. Current
line-oriented source is the deterministic text projection;
it is not the permanent editing identity.
Linux x86-64 tier evidence requires real synchronous calls from verified SSA;
code emission, disassembly, SSA scaffolding, or observation alone is
insufficient. The canonical accepted extension is `.lkjscript`; `.lkjml` is
rejected and no language edition or compatibility mode exists. Linux x86-64 is the
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
7. `[verified: Rust lint, crate graph, and unsafe-boundary review]` Keep pure
   compiler/runtime state apart from host effects. Every unsafe-containing Rust
   file must have one stable boundary identity and a reviewed safe caller
   contract before unsafe code may move beyond its Current `lkjscript-sys`
   locations.
8. `[verified: evidence review]` Never claim an unrun command. Record commit,
   environment, exact command, result, and explicit untested gates.
9. `[verified: focused tests]` Delete a test only when equal or stronger focused
   conformance or boundary coverage replaces it.
10. `[external: dependency policy]` Audited third-party Rust dependencies are
    allowed only with an accepted record, license/advisory review, and measured
    runtime/build/package justification.
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
16. `[machine: LKJ-SOURCE-*]` Source semantic names are lowercase ASCII
    kebab-case, operations use words, signatures/imports are structured, and
    removed spellings have no aliases.
17. `[information]` Runtime JIT is the primary adaptive path. AOT, native caches,
    and explicit local PGO require shared verified SSA/artifact identity,
    accepted measured slices, no telemetry upload, and no semantic divergence.
18. `[verified: memory authority]` Claim collector-free execution only after no
    tracing liveness traversal, collector fallback, collecting safepoint,
    collector barrier, or collection metric remains in Current production code.
19. `[machine: LKJ-MEMORY-TRACING-RATCHET]` Legacy traced object families form
    one closed non-increasing registry; analysis failure never extends it.

## Read Order

1. [Current state](docs/current-state.md)
2. [Capability status](docs/operations/status-authority.md)
3. [Architecture](docs/operations/architecture.md)
4. [Content-addressed contracts](docs/decisions/platform/content-addressed-contracts.md)
5. [AI-native platform](docs/decisions/platform/ai-native-platform.md)
6. [Bounded topology](docs/decisions/platform/bounded-repository-topology.md)
7. [Semantic Source and protocol](docs/decisions/platform/semantic-source-and-agent-protocol.md)
8. [Resource profiles](docs/decisions/platform/resource-budget-profiles.md)
9. [Semantic resource plane](docs/decisions/platform/semantic-resource-plane.md)
10. [Execution portfolio](docs/decisions/execution/execution-portfolio.md)
11. [Language](docs/language/README.md)
12. [Canonical lowercase vocabulary](docs/decisions/semantics/canonical-lowercase-word-vocabulary.md)
13. [Byte and text ownership](docs/decisions/semantics/byte-and-text-ownership.md)
14. [Typed affine resources](docs/decisions/capabilities/typed-affine-resources.md)
15. [Semantic core](docs/decisions/semantics/semantic-core.md)
16. [Equality](docs/decisions/semantics/equality-families.md)
17. [Products](docs/decisions/semantics/immutable-nominal-products.md)
18. [Compiler pipeline](docs/decisions/execution/compiler-pipeline.md)
19. [Ownership](docs/decisions/semantics/ownership-and-borrowing.md)
20. [Collector-free memory](docs/decisions/memory/collector-free-deterministic-memory.md)
21. [Authoritative memory plan](docs/decisions/memory/authoritative-memory-plan.md)
22. [Deterministic drop](docs/decisions/memory/deterministic-drop.md)
23. [Collector-free value island](docs/decisions/memory/collector-free-value-island.md)
24. [Traits](docs/decisions/semantics/traits-and-static-dispatch.md)
25. [Native roots](docs/decisions/jit/native-references-and-gc-stack-maps.md)
26. [Runtime JIT](docs/decisions/jit/runtime-jit-instead-of-offline-pgo.md)
27. [Callable baseline JIT](docs/decisions/jit/callable-baseline-jit.md)
28. [Allocation JIT](docs/decisions/jit/allocation-capable-baseline-jit.md)
29. [Proof JIT](docs/decisions/jit/proof-based-optimizing-jit.md)
30. [Repository graph](docs/decisions/platform/repository-intelligence-graph.md)
31. [Agent state](docs/decisions/platform/agent-work-state.md)
32. [Verification](docs/operations/verification.md)
33. [Vision and evidence](docs/vision/README.md)

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
