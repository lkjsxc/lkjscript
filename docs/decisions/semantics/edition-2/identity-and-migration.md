# Edition 2: Identity And Migration

[Authority](../edition-2.md)

## Purpose

Fix source-edition identity, closure agreement, and the only accepted Edition 1
to Edition 2 migration path.

## Status

<!-- LKJ-STATUS id=edition-2-identity-migration/1 status=current -->

**Current** for explicit per-unit and closure edition identity, the exact
Edition 2 marker, edition-separated identities, Schema V2 projection, exact
`check_edition2_migration`, `diff_edition2_migration`, and
`publish_edition2_migration` compiler APIs, atomic closure publication, the
125-file canonical corpus migration, and the ordinary-compilation cutover.
Edition 1 is accepted only by explicit source-validation and migration APIs.

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
the same edition. An explicitly validated migration input without the form is Edition 1 unless
it uses an Edition 2-only source form, which is rejected as a missing marker. A present
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

The executable migration surface is exact `check`, `diff`, then `publish` at
the compiler-owned source host boundary. It accepts one homogeneous validated
closure, an exact pinned revision, and a Profile V2 selection. It pins old
source, tree, declaration, and node identities and reports their exact new
identities plus per-file and aggregate old/new bytes. Its `_with_ledger` APIs require and reuse one caller-owned
Profile V2 ledger
through check/diff/publish, semantic validation, transaction amplification, and
staging. They reserve transaction, operation, impact, staged-node, and
staged-byte categories before staging; no migration phase creates a nested
ledger. Existing profile-taking APIs create one outer operation ledger. Typed
migration diagnostics retain any `BudgetError` and deterministic prefix.

Edition 1 migration inserts the exact marker after leading physical trivia in
every closure unit. The compiler first resolves operand types and inserts only
`f64-from-i64-rounded` around an I64 operand at a mixed arithmetic or ordering
site that Edition 2 rejects. Spelling alone cannot select conversion. Unknown,
nonpreserving, mixed, or unresolved input rejects rather than guessing. Edition
2 check and diff are idempotent and produce no replacements.

Publication acquires one repository publication lock, recovers any prepared or
committed journal, reloads and checks every pinned identity and exact old byte,
stages the complete closure, and installs every source through the no-replace
journal protocol. Stale, concurrent, ancestor, and leaf conflicts reject.
Failure rolls every installed file back; crash recovery runs before the next
check or publication. There are no partial writes.

All 125 tracked `.lkjscript` files, including all 121 under `src/`, carry the
exact marker. Normal compilation rejects Edition 1. Only explicit migration,
source-validation APIs, and immutable Edition 1 Rust-string/noncanonical
fixtures accept it. There is no hidden mode, automatic fallback, path or CLI
inference, package-default override, alias, or compatibility parser.

## Source Authority

Both editions use one line projection, source parser, and validated tree.
Semantic Source is primary; edition adapters are deterministic checked
projections. Edition 2 cannot create a second AST or backend syntax path.
