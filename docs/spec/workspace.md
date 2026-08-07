# Semantic workspace contract

**Status: normative target contract, not current implementation.** Sections labelled **Target** define
the intended semantic-workspace behavior. The **Current bootstrap** section is descriptive and is
not a claim that the target exists. Current checkout status remains owned by
[`../status.md`](../status.md).

## Current bootstrap

**Currently implemented.** Text files remain persistent import authority, but product compilation
imports them exactly once into a syntax-independent immutable `WorkspaceSnapshot`. An in-process
`Workspace` now owns the current `Arc<WorkspaceSnapshot>`, revision publication, and generation-aware
entity/node allocation. A snapshot is either complete or has a private typed-hole overlay over
non-executable backing HIR. It owns captured or deterministic post-edit development provenance,
optional source attachments, opaque namespace/slot/generation entity and node IDs, semantic indexes,
type facts, diagnostics, and hole contexts.

Transactions atomically batch function/binding rename, flat typed expression replacement, and typed
hole introduce/refine/fill. They reject foreign namespaces, stale generations and revisions,
invalid or cyclic drafts, invisible references, type/arity/effect/ownership failures, and the
currently unsupported storage/generic/match creation cases before publication. Stable IDs are
reconciled through private entity/node addresses: edited roots and unchanged descendants retain
identity, removed descendants are tombstoned, and newly created descendants receive allocator IDs.
A successful transaction publishes one revision and returns deterministic semantic diff,
diagnostics, and coarse invalidation domains; a failed transaction consumes no ID or revision.

Revision-labelled in-process queries cover paginated entity listing/search, definitions/references,
callers/callees, actual/expected node types, diagnostics, hole context, and legal constructors.
Continuations are bound to the namespace, revision, and query. `compile_snapshot` directly lowers a
complete snapshot and returns a typed incomplete error containing stable hole IDs otherwise. Parser
counter tests prove rename, replacement, hole fill, and direct compilation do not parse, render, or
round-trip text.

The older Semantic Source service remains temporarily internal for commit 3 of the same cutover.
It exposes revision-labelled source nodes, selected entity/node/hole queries, diagnostics, preview or
publish transactions, and a local JSON/stdio session. It can rename declarations, replace
expressions, and insert, fill, refine, or delete typed holes with base-revision and file
preconditions. Publication stages and validates source files before replacement. Its schema still
mirrors physical sources, spans, marker-shaped expressions, canonical rendered subtrees, and source
fingerprints. Its `NodeId` is a revision plus a dense `u64` index and does not preserve identity
across an edit. Transactions ultimately rewrite text; a later compile imports the resulting text
into a new compiler snapshot. Unresolved references, ambiguities, general conflicts, and recovery
nodes are not first-class workspace states. Query coverage and result pagination are not the target
interface.

It is not the authority for the in-process vertical above. General unresolved references, ambiguities,
conflicts, recovery nodes, declaration creation/deletion/movement, local-storage construction,
generic-call construction, match construction, and deterministic text rendering remain target-only.

Everything below is the target contract.

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
