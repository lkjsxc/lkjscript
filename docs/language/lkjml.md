# LKJML

## Purpose

Define the canonical, line-oriented source notation for `lkjscript2026`.

## Identity

LKJML is the language's attribute-less markup surface. Canonical source files
use the `.lkjml` extension. The runtime and semantic language remain
`lkjscript2026`; LKJML changes notation, not evaluation, typing, imports, or
runtime behavior.

## Invariants

- One structural marker or atom per physical line.
- Every structural line starts in column one. Nesting never uses indentation.
- Open markers are `name/`; matching close markers are `/name`.
- Tags never have attributes.
- Blank lines are ignored outside text blocks.
- A comment starts with `;;` in column one and occupies the whole line.
- Double quotes are not string delimiters.
- Existing nest, child, token, and top-level-form budgets still apply.

## Line Grammar

```text
file        := (blank | comment | expression-line)*
open        := tag-name "/"
close       := "/" tag-name
atom        := atom-name
comment     := ";;" text
text-block  := text-open newline text* text-close
text-open   := "str/" | "name/" | "import/"
text-close  := "/str" | "/name" | "/import"
```

A regular line cannot have leading or trailing whitespace, contain another
token, or combine an open marker, children, and close marker. Tag and atom
names retain the slash grammar's ASCII spelling. Slash remains structural;
the division function is named `div`.

The parser still requires matching close names. The semantic children of a
form are the expressions on the lines between its open and close markers.

## Text

Text is carried by a text tag instead of quote punctuation:

- `str/` accepts zero or more raw content lines and creates one `Str` literal.
  Multiple lines are joined with LF (`\n`).
- `name/` accepts exactly one non-empty raw line and names a definition.
- `import/` accepts exactly one non-empty raw line and carries an import path.
- Inside a text block, spaces, `;;`, quotes, and tag-looking text are data.
- A content line equal to the block's close marker is escaped with one leading
  backslash. For example, `\/str` represents the text `/str`.

This makes one-word strings unambiguous: `hello` is a symbol, while
`str/` + `hello` + `/str` is a string. Empty strings need no special escape:

```text
str/
/str
```

## Example

```text
import/
examples/hello/fact.lkjml
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

Every line starts in column one. Tree structure comes only from matched open
and close markers.

## Design Experiments

The pre-LKJML corpus contained 117 source files, 43,012 bytes, 2,316 physical
lines, and 365 quoted string occurrences. Three complete encodings were
simulated before selecting the surface:

| Encoding | Bytes | Lines | Result |
| --- | ---: | ---: | --- |
| One token per line, retain quotes | 38,023 | 5,653 | Migration-safe, but violates quote-free text. |
| Context text plus explicit `str/` | 37,573 | 5,709 | Adopted: unambiguous, tag-pure, and 12.6% fewer bytes. |
| Bare strings plus `@symbol` | 38,537 | 5,653 | Compact, but changes every symbolic atom and weakens the tag model. |

Bare-by-default strings without either a symbol sigil or a text tag were
rejected: 179 string occurrences were lexically indistinguishable from
symbols in the measured corpus. Indentation-based nesting was also rejected
because it contradicts the column-one invariant and makes whitespace carry
semantic structure.

The rejected encodings remain candidates for controlled future experiments.
In particular, schema-aware text inference could be combined with `str/`, and
an optional authoring frontend could test `@symbol` while lowering to canonical
LKJML. Neither alternative is part of the runtime contract until corpus,
parser, size, and readability measurements show a net improvement.

## Cutover

The old whitespace-separated `.lkjscript` slash surface is not accepted as
LKJML. The repository migration tool converts that syntax to canonical LKJML;
the checked-in standard library and examples are canonical `.lkjml` files.
