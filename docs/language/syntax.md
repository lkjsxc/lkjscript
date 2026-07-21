# Syntax

## Purpose

Define the slash/whitespace surface (mandatory types elsewhere).

## Rules

- Open: `name/` … Close: `/name`. Atoms: bare tokens (no slash).
- Strings: `"…"` with `\` escapes (`\\`, `\"`, `\n`, `\t`).
- Comments: `;;` to end of line.
- Atoms: numbers, `true`/`false`/`nil`, symbols, strings.
  Integer literals (`2`) type as `I64`; a `.` in the source (`2.0`) types as `F64`.
- Calls are open/close with child args; empty body allowed (`flush/` `/flush`).
- Division operator name is `div` (slash is structural only).
- Comparisons may use `lt` `le` `gt` `ge` or `<` `<=` `>` `>=` as *names*
  (`</ a b /<`).
- Specials: `def`, `fn`, `sig`, `params`, `forall`, `if`, `while`, `let`, `do`,
  `quote`, `import`, `type`.
- Mandatory `sig/` / `params/` on every `fn`. Sized types: `I32` `I64` `U32`
  `U64` `F32` `F64` (plus `Int`/`Float` aliases). No `Any`.
- Parametric fns: `forall/ T /forall` then `List T` in `sig/`, `List/ T /List`
  in `params/`.
- `print` takes `Str` only (`str-from-i64` for numbers).
- Top-level forms: `def`, `do`, or `import` (max `MAX_TOPLEVEL_FORMS`).
- Imports: package-root unless path starts with `.`; no `..` climbs.
