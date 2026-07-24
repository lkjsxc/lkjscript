# Decisions

## Purpose

Index active architecture decisions separately from superseded history.

## Status

**Current.** Individual records carry their own implementation status.

## Active And Accepted Decisions

- [allocation-capable-baseline-jit.md](allocation-capable-baseline-jit.md): exact native references, allocation, recursion, and host runtime-call target
- [bytecode-vm.md](bytecode-vm.md): dense Rust bytecode VM
- [callable-baseline-jit.md](callable-baseline-jit.md): current allocation-free scalar callable Linux x86-64 baseline-JIT cycle and later boundaries
- [compiler-pipeline.md](compiler-pipeline.md): typed HIR/SSA and runtime JIT pipeline
- [equality-families.md](equality-families.md): explicit value, identity, list, and F64-bit equality
- [immutable-nominal-products.md](immutable-nominal-products.md): named immutable aggregate state
- [isolates-and-structured-concurrency.md](isolates-and-structured-concurrency.md): ownership-transfer isolates and structured async direction
- [linux-x86-64-native-backend.md](linux-x86-64-native-backend.md): owned emitter selected for the future production baseline JIT
- [lossless-bulk-bytes.md](lossless-bulk-bytes.md): bounded exact byte boundary for files and sockets
- [durable-file-capabilities.md](durable-file-capabilities.md): append, sync, rename, and OS entropy boundary
- [sha256.md](sha256.md): fixed digest primitive for verifier and integrity use
- [sqlite-capabilities.md](sqlite-capabilities.md): generic owned SQLite handles
  and bounded statement operations for Candidate A consumers
- [runtime-jit-instead-of-offline-pgo.md](runtime-jit-instead-of-offline-pgo.md): runtime JIT tiers and rejection of offline PGO
- [modules-and-packages.md](modules-and-packages.md): explicit modules, reproducible locks, and capability manifests
- [native-references-and-gc-stack-maps.md](native-references-and-gc-stack-maps.md): exact native frames, roots, allocation, and barrier ABI
- [numeric-semantics.md](numeric-semantics.md): exact I64/F64 source-to-host contract
- [ownership-and-borrowing.md](ownership-and-borrowing.md): affine ownership, lexical references, NLL, and deterministic drop target
- [proof-based-optimizing-jit.md](proof-based-optimizing-jit.md): non-speculative verified optimizing tier and promotion contract
- [relational-database-roadmap.md](relational-database-roadmap.md): first-party B+tree/WAL/MVCC relational-server sequence
- [semantic-core.md](semantic-core.md): AI-first Unit/Option/control/mutation/equality contract
- [self-hosted-platform-roadmap.md](self-hosted-platform-roadmap.md): staged bootstrap-to-self-hosting boundary
- [traits-and-static-dispatch.md](traits-and-static-dispatch.md): coherent bounded traits, auto traits, and monomorphization
- [package-imports.md](package-imports.md): package-root source paths
- [resource-handles.md](resource-handles.md): stale-safe resources and bounded terminal ABI
- [system-results.md](system-results.md): truthful host failures as language values
- [web-platform-roadmap.md](web-platform-roadmap.md): ownership-first async Web framework and TLS-provider sequence
- [limits/essential-limits.md](limits/essential-limits.md): fixed semantic source budgets
- [source-tree-limit.md](source-tree-limit.md): 16-entry lkjscript source-directory rule
- [scratch-host.md](scratch-host.md): owned Linux sys layer and thin host policy

## Superseded

- [archive/xml-surface.md](archive/xml-surface.md): XML-like source surface
- [archive/slash-types-sys.md](archive/slash-types-sys.md): combined historical syntax/type/sys contract
- [archive/tunable-limits.md](archive/tunable-limits.md): configurable-limit proposal

The current physical source contract is documented under
[language/](../language/README.md). A superseded record may explain history but
must not be used as an implementation contract.
