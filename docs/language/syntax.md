# Syntax

## Purpose

Define the slash/whitespace surface (mandatory types elsewhere).

## Rules

- Open: `name/` … Close: `/name`. Atoms: bare tokens (no slash).
- Strings: `"…"` with `\` escapes (`\\`, `\"`, `\n`, `\t`).
- Comments: `;;` to end of line.
- Atoms: numbers, `true`/`false`/`nil`, symbols, strings.
- Calls are open/close with child args; empty body allowed (`flush/` `/flush`).
- Division operator name is `div` (slash is structural only).
- Comparisons may use `lt` `le` `gt` `ge` or `<` `<=` `>` `>=` as *names*
  (`</ a b /<`).
- Specials: `def`, `fn`, `sig`, `params`, `if`, `while`, `let`, `do`, `quote`,
  `import`, `type`.
- Top-level forms: `def`, `do`, or `import` (max `MAX_TOPLEVEL_FORMS`).
- Imports: package-root unless path starts with `.`; no `..` climbs.
