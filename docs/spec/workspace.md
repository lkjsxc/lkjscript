# Semantic workspace contract

**Status: normative target contract, not current implementation.** Sections labelled **Target** define
the intended semantic-workspace behavior. The **Current implementation** section is descriptive and
is not a claim that every target exists. Current checkout status remains owned by
[`../status.md`](../status.md).

## Current implementation

**Currently implemented.** Text and path APIs are private-parser importer conveniences that create
one syntax-independent immutable `WorkspaceSnapshot`; they are not a sibling compiler or editing
authority. `compile_snapshot` consumes a complete snapshot directly. Parser-counter tests prove
direct compilation and semantic edits do not render or reparse text.

An in-process `Workspace` owns the current `Arc<WorkspaceSnapshot>`, one-revision atomic publication,
and generation-aware entity/node allocation. Snapshots own complete or typed-hole program state,
private typed HIR, deterministic post-edit provenance, optional source attachments, opaque
namespace/slot/generation IDs, semantic indexes, type facts, diagnostics, and hole contexts. IDs
survive rename, formatting attachment changes, attachment removal, unrelated edits, and meaning-
preserving descendant movement; removed identities are tombstoned.

Transactions atomically batch function/binding rename, flat typed expression replacement, and typed
hole introduce/refine/fill. They reject foreign namespaces, stale generations and revisions,
invalid or cyclic drafts, invisible references, type/arity/effect/ownership failures, and currently
unsupported storage/generic/match constructors before publication. Failure consumes no identity or
revision. Success returns deterministic semantic diff, diagnostics, and coarse invalidation domains.

Revision-labelled in-process queries cover paginated entity listing/search, definitions/references,
callers/callees, actual/expected node types, diagnostics, hole context, and legal constructors.
Continuations bind namespace, revision, and query. A fallible, iterative concise projection renders
selected entity, body, type, reference, and explicit `[HOLE]` headers without source attachments.
Projection labels and formatting never construct semantic identity.

The former syntax-shaped editing service, source-node protocol identities, stdio/session schemas,
text journal/publication path, CLI routes, and protocol descriptors are deleted. There is no
replacement wire service until a measured consumer justifies a process boundary and explicit host
policy.

General unresolved references, ambiguities, conflicts, recovery nodes, declaration
creation/deletion/movement, local-storage construction, generic-call construction, match
construction, complete source rendering, persistence, collaboration, and incremental recomputation
remain gaps. Everything below continues to define the target contract.

## 1. Snapshot authority

**Target.** One immutable typed semantic workspace snapshot is authoritative for program meaning.
It separates:

- semantic entities and their owned nodes;
- reference and dependency edges;
- presentation, comments, source, and span attachments;
- derived analysis and diagnostics;
- compiled artifacts; and
- cache state.

Containment follows language ownership; references and dependencies are explicit graph edges. Text
is an importer, renderer, review view, and interoperability projection, not a sibling authority.

Every response names the snapshot revision it describes. Deterministic local machinery, usable
offline, decides correctness; a model may propose operations but never participates in a correctness
gate.

## 2. Stable identity

**Target.** Mutable entities, bindings, and semantic nodes have opaque logical identities distinct
from snapshot-local dense indexes. Identity survives, where meaning permits, rename, movement,
formatting, file regrouping, projection changes, and unrelated edits.

Names, paths, spans, formatting, and source order are attributes unless the language explicitly
makes an ordering semantic. An identity carries or is checked against a workspace namespace,
generation, revision precondition, or equivalent stale-reference defense. Stale identity and stale
base revision fail before publication.

Content hashes may identify immutable snapshots, immutable definitions, artifacts, cache entries,
and transfer bytes. They do not replace logical identity for every mutable node.

## 3. Incomplete states

**Target.** A workspace may preserve typed holes, untyped holes, unresolved references, ambiguous
choices, type/effect/capability/ownership mismatches, missing fields, arms, parameters or
declarations, import failures, explicit conflicts, and recovery nodes.

Incomplete snapshots are valid editing states. Analysis preserves every sound fact available around
an error. Diagnostics attach to semantic identities rather than depending only on text spans. A
snapshot is executable only when all required completeness and language checks succeed.

## 4. Transactions

**Target.** Semantic edits are typed operations over identities. The operation family includes
create/delete/move/rename entity, insert/move/replace node, set a type/field/reference/effect or
capability, rewire a call or dependency, introduce/refine/fill a hole, apply a legal refactoring, and
resolve a conflict.

A transaction:

1. names a base revision and operation preconditions;
2. validates operation shape and all referenced identities before publication;
3. supports batching;
4. stages the complete change without mutating the published snapshot;
5. preserves the old snapshot on validation failure, cancellation, allocation failure, I/O failure,
   or resource-policy exhaustion;
6. publishes exactly one new revision on success; and
7. returns a semantic diff, deterministic diagnostics, and invalidation information.

A text patch may be imported into a transaction but is not the foundational edit representation.

## 5. Queries

**Target.** Queries are deterministic and revision-labelled. The useful core includes entity lookup,
qualified resolution, definitions/references, callers/callees, actual and expected types, typing
context, effects/capabilities, ownership/movement/borrowing, diagnostics, hole context, legal
constructors and edits, dependencies/impact, and search by name, type, operation, or capability.

Responses return compact headers and identities first and allow selective expansion. Large result
sets have stable ordering, filters, pagination, and continuation. A response never claims an
incomplete result is complete. Resource-policy failure is explicit and cannot publish a staged edit.

## 6. Projections and diffs

**Target.** The same snapshot can produce concise and verbose human-readable text, structured debug
or interchange data, semantic diffs, conventional text diffs, IDE/visual views, and compiled
artifacts. Rendering is deterministic for a selected projection but does not create semantic
identity.

A semantic diff reports entity creation/deletion/movement, rename, signature or type change,
reference rewiring, control replacement, effect/capability change, and incomplete-state introduction
or resolution. Text diffs remain available for human review.

## 7. Direct compilation

**Target.** The compiler consumes a complete semantic snapshot directly:

```text
semantic snapshot
    -> name, type, effect, capability, and ownership analysis
    -> canonical typed core
    -> verified executable representation
    -> selected production execution path
```

Text import constructs or updates the semantic snapshot. Direct compilation must not render and
reparse text, and tests must enforce that invariant. The selected snapshot revision, explicit target,
options, and capabilities determine compilation meaning; cache state and parallel schedule do not.

## 8. Persistence and collaboration

**Target sequence, not a persistence commitment.** Begin with in-memory immutable or copy-on-write
snapshots. Add a transaction log, embedded database, binary snapshot, CRDT, or distributed store only
when measurements establish a need for crash recovery, retained scale, concurrent writers, or
collaboration. Persistence and collaboration must preserve atomic publication, deterministic meaning,
and boundary validation.
