# Edition 2: Identity And Migration

[Authority](../edition-2.md)

## Purpose

Fix source-edition identity, closure agreement, and the only accepted Edition 1
to Edition 2 migration path.

## Status

<!-- LKJ-STATUS id=edition-2-identity-migration/1 status=current -->

**Current** for explicit per-unit and closure edition identity, the exact
Edition 2 marker over existing declarations, edition-separated identities,
Schema V2 projection, and the non-publishing `check_edition2_migration` compiler
API. Edition 1 remains ordinary compilation and migration input. ADTs, matches,
changed execution semantics, semantic migration publication, corpus migration,
and cutover remain Accepted Targets.

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
the same edition. An ordinary file without the form is Edition 1 unless it uses
an Edition 2-only source form, which is rejected as a missing marker. A present
marker must be first, unique, and byte-exact apart from physical leading trivia;
misordered, duplicate, or malformed markers reject. Inference from path,
package, CLI, or neighbors is forbidden. The marker is a retained structural
source node and does not consume the top-level declaration limit.

Every validated source unit and tree carries `Edition1` or `Edition2`.
Source-unit, tree, revision, node, and declaration identities frame the edition,
so equal paths or declaration spellings cannot collide across editions.
Semantic Source remains `lkjscript.semantic-source/2`, represents both
editions, and exposes edition-marker nodes and exact source-unit edition facts.
ABI identity changes only where changed public semantics require it.

## Migration

The first executable migration surface is the strict non-publishing compiler
API `check_edition2_migration`. It accepts a homogeneous validated closure, an
exact pinned revision, and a Profile V2 selection. It reserves transaction,
operation, impact, staged-node, and staged-byte categories before staging. For
Edition 1 it inserts only the exact marker after leading physical trivia in
each unit and returns deterministic replacement bytes, insertion offsets,
old/new byte counts, source identities, tree identities, and revisions. It
rejects stale revisions and mixed closures. For Edition 2 it is idempotent and
returns no replacements. It never writes or publishes.

The later semantic operation remains exact `check`, `diff`, then `publish`. It pins the
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
