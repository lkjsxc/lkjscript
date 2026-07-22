# Decisions

## Purpose

Index active architecture decisions separately from superseded history.

## Status

**Current.** Individual records carry their own implementation status.

## Active And Accepted Decisions

- [bytecode-vm.md](bytecode-vm.md): dense Rust bytecode VM
- [package-imports.md](package-imports.md): package-root source paths
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
