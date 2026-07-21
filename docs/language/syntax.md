# Syntax

## Purpose

Define the attribute-less XML-like surface.

## Rules

- Elements are `<name>…</name>` or empty `<name/>`.
- Attributes are forbidden.
- Empty tags with numeric names are number literals (`<1/>`, `<-2.0/>`).
- Other empty tags are symbols (`<n/>`).
- Text nodes are strings (spaces preserved).
- Specials: `def`, `fn`, `if`, `while`, `let`, `do`, `quote`, `import`.
- `<while><cond/>body…</while>` repeats body while cond is truthy; yields nil.
- Top-level forms must be `def`, `do`, or `import`.
- At most `MAX_TOPLEVEL_FORMS` top-level forms per file (default 8).
- Imports: package-root if the path does not start with `.`
  (`std/list/nth.lkjscript`, `lib/edit/loop.lkjscript`, `examples/...`);
  `./relative` for file-local; `..` climbs are banned.
  Prefixes `std/` and `lib/` map under `src/std/` and `src/lib/`.
