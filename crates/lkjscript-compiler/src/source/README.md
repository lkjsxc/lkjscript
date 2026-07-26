# Semantic Source authority capsule

This directory owns the single canonical source parser, loader, formatter,
identity model, diagnostics, and validated-tree authority.

The public entry points in `api` return only an opaque `ValidatedSourceTree`.
Its sole private authority constructor runs after exact-byte parsing, contained
loading, bounded import traversal, revision and stable-identity construction,
and declaration validation. Parsed syntax, mutable construction state, tokens,
and retained source files remain private.

Responsibilities are separated as follows:

- `model`: immutable retained syntax and private compiler projection;
- `diagnostics`: exact origins, spans, categories, and renderings;
- `parse`: line, lexing, token-tree, atom, limit, and declaration phases;
- `load`: descriptor containment, bounded reads, imports, and directory checks;
- `validate`: logical paths, aggregate budgets, and authority construction;
- `identity`: contract-framed source, tree, revision, declaration, and node IDs;
- `format`: structural canonical formatting;
- `tests`: diagnostics, identity, formatting, limits, loading, and Linux safety.

There is one parser authority and one import traversal authority. Removed source
generations have no marker, migration API, alias, or fallback path.
