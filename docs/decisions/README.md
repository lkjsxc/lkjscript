# Decisions

## Purpose

Index active architecture decisions separately from superseded history.

## Status

**Current.** Individual records carry their own implementation status.

## Active And Accepted Decisions

- [ai-native-platform.md](platform/ai-native-platform.md): overarching AI-primary language/platform
  identity, preserved invariants, superseded assumptions, and dependency order
- [semantic-source-and-agent-protocol.md](platform/semantic-source-and-agent-protocol.md): versioned
  validated source graph, stable/revision identities, semantic transactions, diagnostics, and
  typed-hole foundation
- [resource-budget-profiles.md](platform/resource-budget-profiles.md): implementation maxima, exact
  aggregate charge categories, host profiles, maintainability lints, and semantic metering migration
- [bounded-repository-topology.md](platform/bounded-repository-topology.md): authored bounds,
  provenance, semantic capsules, strict manifests, structure rule IDs, and audit JSON
- [repository-intelligence-graph.md](platform/repository-intelligence-graph.md): bounded nodes,
  edges, provenance, identities, queries, and context profiles
- [agent-work-state.md](platform/agent-work-state.md): versioned task lifecycle, scope, evidence,
  atomic updates, and policy coverage
- [execution-portfolio.md](execution/execution-portfolio.md): measured
  evaluator/VM/JIT/AOT/cache/optional-local-PGO/Wasm direction
- [allocation-capable-baseline-jit.md](jit/allocation-capable-baseline-jit.md): exact native
  references, allocation, recursion, and host runtime-call target
- [bytecode-vm.md](execution/bytecode-vm.md): dense Rust bytecode VM
- [callable-baseline-jit.md](jit/callable-baseline-jit.md): current allocation-free scalar callable
  Linux x86-64 baseline-JIT cycle and later boundaries
- [compiler-pipeline.md](execution/compiler-pipeline.md): typed HIR/SSA and runtime JIT pipeline
- [collector-free-deterministic-memory.md](memory/collector-free-deterministic-memory.md): inferred
  deterministic memory destination, Current inventory, migration, and falsification contract
- [equality-families.md](semantics/equality-families.md): explicit value, identity, list, and F64-bit equality
- [immutable-nominal-products.md](semantics/immutable-nominal-products.md): named immutable aggregate state
- [isolates-and-structured-concurrency.md](platform/isolates-and-structured-concurrency.md):
  ownership-transfer isolates and structured async direction
- [linux-x86-64-native-backend.md](execution/linux-x86-64-native-backend.md): owned emitter selected
  for the future production baseline JIT
- [lossless-bulk-bytes.md](capabilities/lossless-bulk-bytes.md): bounded exact byte boundary for files and sockets
- [durable-file-capabilities.md](capabilities/durable-file-capabilities.md): append, sync, rename,
  and OS entropy boundary
- [sha256.md](capabilities/sha256.md): fixed digest primitive for verifier and integrity use
- [sqlite-capabilities.md](capabilities/sqlite-capabilities.md): generic owned SQLite handles
  and bounded statement operations for Candidate A consumers
- [runtime-jit-instead-of-offline-pgo.md](jit/runtime-jit-instead-of-offline-pgo.md): Current
  runtime-JIT implementation/evidence; permanent JIT-only and PGO/cache rejection is superseded by
  the execution portfolio
- [modules-and-packages.md](platform/modules-and-packages.md): explicit modules, reproducible locks,
  and capability manifests
- [native-references-and-gc-stack-maps.md](jit/native-references-and-gc-stack-maps.md): exact native
  frames, roots, allocation, and barrier ABI
- [numeric-semantics.md](semantics/numeric-semantics.md): exact I64/F64 source-to-host contract
- [ownership-and-borrowing.md](semantics/ownership-and-borrowing.md): affine ownership, lexical
  references, NLL, and deterministic drop target
- [proof-based-optimizing-jit.md](jit/proof-based-optimizing-jit.md): current forced non-speculative
  tier and accepted synchronous automatic-promotion implementation selection
- [relational-database-roadmap.md](roadmaps/relational-database-roadmap.md): first-party
  B+tree/WAL/MVCC relational-server sequence
- [semantic-core.md](semantics/semantic-core.md): AI-first Unit/Option/control/mutation/equality contract
- [self-hosted-platform-roadmap.md](platform/self-hosted-platform-roadmap.md): staged bootstrap-to-self-hosting boundary
- [traits-and-static-dispatch.md](semantics/traits-and-static-dispatch.md): coherent bounded traits,
  auto traits, and monomorphization
- [resource-handles.md](capabilities/resource-handles.md): stale-safe resources and bounded terminal ABI
- [system-results.md](capabilities/system-results.md): truthful host failures as language values
- [web-platform-roadmap.md](roadmaps/web-platform-roadmap.md): ownership-first async Web framework
  and TLS-provider sequence
- [limits/essential-limits.md](limits/essential-limits.md): Current fixed source budgets during
  aggregate-budget migration; permanent semantic-limit policy is superseded
- [source-tree-limit.md](source-tree-limit.md): Current 16-entry source-directory rule during
  aggregate-budget migration; permanent language-rule policy is superseded
- [scratch-host.md](capabilities/scratch-host.md): owned Linux sys layer and thin host policy

## Superseded

- [archive/xml-surface.md](archive/xml-surface.md): XML-like source surface
- [archive/slash-types-sys.md](archive/slash-types-sys.md): combined historical syntax/type/sys contract
- [archive/tunable-limits.md](archive/tunable-limits.md): configurable-limit proposal

The current physical source contract is documented under
[language/](../language/README.md). A superseded record may explain history but
must not be used as an implementation contract.
