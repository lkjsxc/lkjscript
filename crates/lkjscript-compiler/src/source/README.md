# Semantic Source authority capsule

This directory owns the Edition 1 Semantic Source Foundation authority. It is
a structural capsule, not the future complete Semantic Source protocol. No
`capsule.json` is present because that integration schema is not yet accepted.

The public entry points in `api` return only an opaque `ValidatedSourceTree`.
Its sole private authority constructor is called after exact-byte parsing,
contained loading, bounded import traversal, revision and stable-identity
construction, and declaration validation succeed. Parsing syntax, projected
`Expr` values, mutable build state, tokens, and source files remain private.

Responsibilities are intentionally separated:

- `model`: immutable retained syntax and private compiler projection;
- `diagnostics`: exact origins, spans, categories, and renderings;
- `parse`: line, lexing, token-tree, atom, limit, and declaration phases;
- `load`: descriptor containment, bounded reads, imports, and directory checks;
- `validate`: logical paths, aggregate budgets, and final authority construction;
- `identity`: revision framing, dense node IDs, and declaration keys;
- `format`: structural canonical formatting;
- `tests`: diagnostics, identity, formatting, limits, loading, and Linux safety.

There is one parser authority and one import traversal authority. The split does
not add fallback paths, mutable public internals, runtime dispatch, or protocol
placeholders. Foundation maxima remain checked independently of configured
Edition 1 limits.
