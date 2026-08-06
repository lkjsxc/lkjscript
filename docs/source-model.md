# Semantic program model

## Direction

The target source authority is an immutable, typed semantic snapshot edited through atomic
transactions. Text, structured interchange, IDE views, and human-readable diffs are projections
around that snapshot. The compiler must consume semantic data directly rather than render and
reparse text.

The current Semantic Source implementation is a useful bootstrap: it has deterministic,
revision-scoped opaque node access, transactions, typed holes, diagnostics, snapshots, and local
sessions. Source positions, spans, node indexes, and JSON transaction/query relations now preserve
`u64` values and perform checked host indexing, so values above `u32` do not saturate or alias.
That representation change does not make dense `NodeId` edit-stable. The schema still mirrors the
provisional text tree, spans, and rendering rules, and transactions ultimately publish source
files. Replacing revision-scoped dense nodes with stable logical semantic identity remains a Phase
5 gap; this bootstrap is not the permanent model.

## Identity

Mutable semantic entities and nodes need stable, generation-checked logical IDs. Rename, movement,
formatting, and an unrelated edit must not change identity. Names and projection locations are
attributes. Stale IDs and stale base revisions must fail before publication.

Content hashes are appropriate for immutable snapshot identity, compiled artifacts, cache entries,
and transfer integrity. They are not the identity of every mutable node.

## Partial programs

Workspace snapshots may contain typed or untyped holes, unresolved or ambiguous references,
missing branches or fields, parse-import errors, and explicit conflicts. Diagnostics attach to
stable entities. Such snapshots may be queried and edited but cannot be released as executables
until required completeness and semantic checks pass.

## Transactions

The in-process API should support typed operations including create/remove/move/rename entity,
insert or replace node, set a field or reference, rewire a call, introduce/refine/fill a hole, and
resolve a conflict.

A transaction must:

1. name its base revision;
2. validate operation shape and IDs before publication;
3. apply atomically without reparsing the entire workspace;
4. preserve the prior snapshot on failure or cancellation;
5. return a new revision, semantic diff, diagnostics, and invalidated query families.

Text patches may be imported into transactions but are not the foundational edit operation.

## Queries and diffs

Queries are deterministic and revision-labelled. The first useful set is entity lookup, qualified
name resolution, definition/references, callers/callees, type and expected type, diagnostics, hole
context/legal actions, and dependency/impact slices. Large results require stable ordering and
continuations rather than truncation.

Semantic diffs report declaration creation/deletion/movement, rename, signature/type change,
reference rewiring, control replacement, effect/capability change, and hole introduction or
resolution. Text diffs remain a review projection.

## First executable vertical

The first cut should cover one module with `main` and ordinary functions, primitive literals and
types, calls, local bindings, conditionals, and one incomplete-expression hole. It is complete only
when tests demonstrate stable IDs, stale-revision rejection, atomic rename and replace/fill
transactions, reference/type/hole queries, deterministic import/rendering, snapshot serialization,
and direct lowering to the existing typed core without text round-tripping.

Begin with in-memory immutable snapshots and measured copy-on-write revisions. Add an incremental
query framework or persistence only after comparing invalidation precision, edit latency, retained
memory, cycle handling, cancellation, compile-time cost, and debugging complexity.
