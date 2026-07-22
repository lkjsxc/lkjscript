# Current LKJML Baseline And Accepted Source-Format Cutover

## Purpose

Record the currently implemented physical notation while defining its approved
rename to canonical lkjscript source.

## Status

- **Current baseline:** the implementation and 117-file corpus use `.lkjml`.
- **Accepted Target:** the language is named `lkjscript`, source files use only
  `.lkjscript`, and the term LKJML becomes **Superseded**.

No `.lkjml` compatibility, extension inference, symlink alias, or conversion
command is part of the accepted product behavior.

## Physical Format

The physical format itself remains line-oriented during the naming cutover:

- One structural marker or atom per physical line.
- Every structural line starts in column one; indentation is not syntax.
- Open markers are `name/`; matching close markers are `/name`.
- Markers have no attributes.
- Blank lines are ignored outside text blocks.
- A comment starts with `;;` in column one and occupies the whole line.
- Double quotes are not string delimiters.
- Nest, child, token, top-level-form, and source-directory budgets apply.

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
function name `div`.

## Text Blocks

- `str/` accepts zero or more raw lines and joins multiple lines with LF.
- `name/` accepts exactly one non-empty line naming a definition.
- `import/` accepts exactly one non-empty import path.
- Spaces, `;;`, quotes, and marker-looking content are data inside `str/`.
- A content line equal to its close marker is escaped with one leading
  backslash; `\/str` represents `/str`.

Current lexer behavior rejects leading or trailing whitespace in single-line
`name/` and `import/` content. This is part of the current contract.

## Accepted Example

After the cutover:

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

The prior corpus comparison evaluated quoted tokens, explicit `str/`, bare
strings with a symbol sigil, and indentation-based nesting. Explicit `str/`
was selected because it preserved unambiguous symbols and column-one structure.
Those measurements were not stored with a reproducible harness and therefore
remain historical evidence, not a current benchmark result.

Alternative authoring frontends may be tested later, but canonical compiler
input changes only after corpus, parser, size, error-quality, and model-authoring
measurements are recorded in the experiment registry.
