# Lkjscript Source Format

## Purpose

Define the canonical physical notation accepted by the compiler.

## Status

**Current.** Canonical files use `.lkjscript`. `.lkjml`, edition markers,
compatibility modes, arrows, composite imports, quoted strings, and legacy
spellings are rejected.

## Invariants

- One structural marker or atom per physical line.
- Every structural line starts in column one; indentation is not syntax.
- Open markers are `name/`; matching close markers are `/name`.
- Markers have no attributes.
- Blank lines are ignored outside text blocks.
- A comment starts with `;;` in column one and occupies the whole line.
- Every language-owned name is exact lowercase ASCII kebab-case.
- Nest, child, token, top-level-form, and source-directory budgets apply.

## Physical Grammar

```text
file           := (blank | comment | expression-line)*
open           := identifier "/"
close          := "/" identifier
atom           := identifier | numeric-literal | "true" | "false" | "unit"
comment        := ";;" text
text-open      := "string-literal/" | "name/" | "module/"
text-close     := "/string-literal" | "/name" | "/module"
identifier     := [a-z][a-z0-9]*("-"[a-z0-9]+)*
```

A regular line cannot have leading or trailing whitespace or combine tokens.
Slash is structural; arithmetic uses word operations such as `divide`.
Matching close names are mandatory.

## Text Blocks

- `string-literal/` accepts zero or more raw lines and joins them with LF.
- `name/` accepts exactly one non-empty canonical identifier.
- `module/` accepts one exact package-root-relative module path.
- Spaces, `;;`, quotes, and marker-looking content are data in a string literal.
- A content line equal to `/string-literal` is escaped as
  `\/string-literal`.

`str/ ... /str` is removed. The word `str` is reserved for a future borrowed
UTF-8 view and is not a Current source type.

## Structured Signatures

```text
sig/
inputs/
i64
i64
/inputs
output/
i64
/output
/sig
```

`inputs/` may be empty. `output/` has exactly one structural type. The `->`
atom is rejected.

## Structured Imports

```text
imports/
import/
module/
src/examples/hello/fact.lkjscript
/module
declarations/
fact
/declarations
/import
/imports
```

Module paths are raw path data. Declaration children are sorted canonical
identifiers. Composite `path#names` imports are rejected.

## Example

```text
main/
sig/
inputs/
/inputs
output/
unit
/output
/sig
print/
stdio
string-literal/
hello
/string-literal
/print
/main
```

Alternative authoring frontends remain experiments. Canonical compiler input
changes only through an accepted contract and complete corpus migration.
