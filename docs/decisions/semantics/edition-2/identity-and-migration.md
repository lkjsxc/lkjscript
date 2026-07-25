# Edition 2: Identity And Migration

[Authority](../edition-2.md)

## Purpose

Fix source-edition identity, closure agreement, and the only accepted Edition 1
to Edition 2 migration path.

## Status

**Accepted Target, not Current.** Edition 1 remains Current until every cutover
gate passes.

## Edition Identity

Every Edition 2 source unit begins with this first semantic form:

```text
edition/
2
/edition
```

No import, declaration, or other semantic form precedes it. Physical blank or
comment trivia may precede it and attaches as leading trivia to the edition
node; trivia is not a semantic form. All units in a loaded closure must declare
the same edition. An ordinary file
without the form is Edition 1; inference from path, package, CLI, or neighbors
is forbidden. Semantic Source uses `lkjscript.semantic-source/2`. ABI identity
changes only where changed public semantics require it.

## Migration

Migration is a semantic `check`, `diff`, then `publish` transaction. It pins the
old source/revision identities and produces exact new source/revision
identities. Publication is atomic across the closure. The compiler resolves
operand types before inserting `f64-from-i64-rounded`; spelling alone cannot
select a conversion. Nonpreserving or unresolved constructs are rejected with
structured diagnostics rather than guessed rewrites.

Ordinary Edition 1 compilation remains until all 125 tracked `.lkjscript`
source files, including all 121 under `src/`, migrate and evaluator, VM, and
forced-JIT differentials pass. Commit-scoped historical corpus counts remain
unchanged as historical evidence.

After cutover, normal compilation rejects Edition 1. Only explicit offline
migration and immutable Edition 1 fixtures remain. There is no hidden mode,
automatic fallback, package-default override, or compatibility parser.

## Source Authority

Both editions use one line projection, source parser, and validated tree.
Semantic Source is primary; edition adapters are deterministic checked
projections. Edition 2 cannot create a second AST or backend syntax path.
