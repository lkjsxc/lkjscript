# Lkjscript Source Format

## Purpose

Define the canonical physical notation of lkjscript source files.

## Status

**Current.** Canonical files use `.lkjscript`. The former `.lkjml` extension and
LKJML name are **Superseded** and are not accepted aliases.

## Invariants

- One structural marker or atom per physical line.
- Every structural line starts in column one; indentation is not syntax.
- Open markers are `name/`; matching close markers are `/name`.
- Markers have no attributes.
- Blank lines are ignored outside text blocks.
- A comment starts with `;;` in column one and occupies the whole line.
- Double quotes are not string delimiters.
- Nest, child, token, top-level-form, and source-directory budgets apply.

## Physical Grammar

```text
file        := (blank | comment | expression-line)*
open        := tag-name "/"
close       := "/" tag-name
atom        := atom-name
comment     := ";;" text
text-open   := "str/" | "name/" | "import/"
text-close  := "/str" | "/name" | "/import"
```

A regular line cannot have leading or trailing whitespace, combine multiple
tokens, or combine an opener and closer. Slash is structural; division is the
function name `div`. Matching close names are mandatory.

## Text Blocks

- `str/` accepts zero or more raw lines and joins multiple lines with LF.
- `name/` accepts exactly one non-empty line naming a definition.
- `import/` accepts exactly one non-empty canonical source path.
- `name/` and `import/` content cannot have leading or trailing whitespace.
- Spaces, `;;`, quotes, and marker-looking content are data inside `str/`.
- A content line equal to its close marker is escaped with one leading
  backslash; `\/str` represents `/str`.

## Example

```text
import/
examples/hello/fact.lkjscript
/import
do/
print/
str-from-i64/
fact/
10
/fact
/str-from-i64
/print
/do
```

## Historical Experiments

Earlier format comparisons tested quoted tokens, explicit `str/`, bare strings
with a symbol sigil, and indentation-based nesting. Explicit text blocks were
selected for unambiguous symbols and column-one structure. The original
measurements were not retained with a reproducible harness and are historical
context, not current benchmark evidence.

Alternative authoring frontends remain valid experiments. Canonical compiler
input changes only after corpus, parser, error-quality, size, and model-authoring
measurements are recorded under the experiment protocol.
