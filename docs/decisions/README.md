# Decisions

## Purpose

Index active architecture decisions separately from superseded history.

## Status

**Current.** Individual records carry their own implementation status.

## Active And Accepted Decisions

- [bytecode-vm.md](bytecode-vm.md): dense Rust bytecode VM
- [compiler-pipeline.md](compiler-pipeline.md): typed HIR/SSA and runtime JIT pipeline
- [equality-families.md](equality-families.md): explicit value, identity, list, and F64-bit equality
- [immutable-nominal-products.md](immutable-nominal-products.md): named immutable aggregate state
- [runtime-jit-instead-of-offline-pgo.md](runtime-jit-instead-of-offline-pgo.md): runtime JIT tiers and rejection of offline PGO
- [numeric-semantics.md](numeric-semantics.md): exact I64/F64 source-to-host contract
- [semantic-core.md](semantic-core.md): AI-first Unit/Option/control/mutation/equality contract
- [package-imports.md](package-imports.md): package-root source paths
- [resource-handles.md](resource-handles.md): stale-safe resources and bounded terminal ABI
- [system-results.md](system-results.md): truthful host failures as language values
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
