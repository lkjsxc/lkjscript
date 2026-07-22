# Syntax

## Purpose

Define expression and type rules above the LKJML line notation.

## Surface

[LKJML](lkjml.md) defines physical lines, matching open/close markers,
attribute-free text tags, comments, and the `.lkjml` extension. Whitespace does
not separate tokens: every structural marker or atom occupies its own line.

## Expressions

- Atoms are numbers, `true`, `false`, `nil`, or symbols.
- Integer literals such as `2` type as `I64`; a source decimal point such as
  `2.0` makes an `F64` literal.
- `str/` text blocks produce `Str` literals without quote delimiters.
- Calls use a matching open and close marker with child arguments between them.
  An empty body is allowed (`flush/` followed by `/flush`).
- Division is named `div` because slash is structural only.
- Comparisons may use `lt`, `le`, `gt`, `ge`, `<`, `<=`, `>`, or `>=` as tag
  names.

## Special Forms

Special forms are `def`, `fn`, `sig`, `params`, `forall`, `if`, `while`,
`let`, `do`, `quote`, `import`, and `type`.

Every `fn` has mandatory `sig/` and `params/` forms. Sized types are `I32`,
`I64`, `U32`, `U64`, `F32`, and `F64`; `Int` and `Float` alias `I64` and
`F64`. `Any` is not a type.

A parametric function declares type variables in `forall/`. `List T` in a
signature is three atom lines; `List/`, `T`, `/List` is the corresponding
nested parameter type form.

`print` accepts `Str` only. Numeric output uses `str-from-i64` or
`str-from-f64`; byte values use `str-from-byte`.

## Files And Imports

Top-level forms are `def`, `do`, or `import`, up to `MAX_TOPLEVEL_FORMS`.
Imports are package-root paths unless they start with `./`; `..` climbs are
forbidden. Canonical import paths end in `.lkjml`.
