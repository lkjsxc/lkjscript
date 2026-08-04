# Language

## Purpose

Define the physical source format, expression semantics, imports, and fixed
source budgets of `lkjscript`.

## Status

**Current** for the implemented canonical-language source, semantic, and execution
slices. Canonical source uses `.lkjscript`; the former LKJML name and `.lkjml`
extension are **Superseded** and unsupported. Removed language or edition marker
forms are rejected as ordinary syntax errors. Broader canonical-language surfaces retain their
[Accepted Target](../decisions/semantics/semantic-core.md) status.

## Table of Contents

- [source-format.md](source-format.md): canonical line-oriented notation
- [syntax.md](syntax.md): expressions, forms, types, and imports
- [limits.md](limits.md): fixed source budgets and source-tree width
- [semantic core](../decisions/semantics/semantic-core.md): Current slices and Accepted Targets
  migration, ADT, match, Never, conversion, error, and execution contracts

Historical notation records live only under
[decisions/archive/](../decisions/archive).
