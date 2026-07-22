# Lkjscript Source-Tree Width

## Purpose

Keep language projects navigable without imposing arbitrary layout limits on
the compiler implementation or repository infrastructure.

## Status

**Accepted Target.** It supersedes the repository-wide eight-visible-child
policy still present in the baseline gate.

## Decision

Every lkjscript source directory may contain at most 16 immediate entries,
counting files and subdirectories together.

The language rule applies to source/package directories, including external
projects compiled by the CLI. It does not apply to Rust crates, documentation,
metadata, `.git`, Cargo `target`, or generated artifacts outside language
source trees.

All entries inside a source directory count; dot-prefixes and `LICENSE` are not
special exemptions. Directory-read errors fail verification or compilation.
The shared language constant is the sole numeric source of truth.

## Consequences

- Standard libraries and packages split broad categories into meaningful
  subdirectories before they exceed 16 entries.
- Repository infrastructure may use the layout most suitable for Rust and
  operations.
- The repository gate checks the complete in-tree source corpus.
- The compiler checks directories reached by external entries/imports so the
  rule travels with the language.

## Rejected

- The old repository-wide limit of eight visible entries.
- Ignoring hidden source entries or files named `LICENSE`.
- A user-configurable limit that makes source validity environment-dependent.
- Applying the rule to generated build trees.
