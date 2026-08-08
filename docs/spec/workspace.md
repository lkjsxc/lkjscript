# Semantic workspace contract

**Status: normative target contract, not current implementation.** Sections labelled **Target** define
the intended semantic-workspace behavior. The **Current implementation** section is descriptive and
is not a claim that every target exists. Current checkout status remains owned by
[`../status.md`](../status.md).

## Current implementation

**Currently implemented.** `Workspace::empty` creates a fresh namespace at revision 1 with no
source records, attachments, entities, entry point, or body. The empty revision is a valid
queryable/projectable editing state with a `MissingEntryPoint` blocker and diagnostic. Text and path
APIs are private-parser importer conveniences that produce the same immutable semantic program
authority; compiler-owned operations, prelude enums, and core traits are fixed language context, not
mutable program entities.

A `WorkspaceSnapshot` owns one partial-capable `SemanticProgram`, optional imported diagnostic
origins and presentation attachments, opaque namespace/slot/generation identities, the exact
identity allocator including tombstones, derived indexes, typed hole records, diagnostics, and
structured completeness blockers. The semantic program may omit `main`; missing bodies and typed
expression holes are actual `ExprKind::Hole` leaves with unknown effects. No executable expression
is retained behind a hole. Source-free expressions have an honest semantic origin; imported source
paths and digests do not create logical identity.

All construction and later editing uses one revision-checked atomic transaction path. Implemented
creation is one non-generic scalar function with scalar parameters/result and one parameterless
scalar `main`; both begin with typed missing bodies. Implemented flat child-before-parent expression
construction covers i64/f64/Boolean/unit literals, visible parameter loads, non-generic calls, and
`if`. Transactions also rename supported bindings, replace expressions, and introduce/refine/fill
typed holes. They reject duplicate/reserved global declarations or function renames, duplicate parameters at creation,
foreign or stale identity, stale revision, invalid or cyclic drafts, overlapping subtree edits,
invisible bindings, unsupported ownership
shapes, and type/arity failures
before publication. Failure preserves the exact published `Arc`, revision, tombstones, future
allocation, diagnostics, and projection. Success publishes one revision and returns created entity
and hole IDs through the semantic diff.

Structured blockers currently cover missing entry point, missing declaration/entry body, and typed
expression hole. Every blocker is revision-labelled through the snapshot or
`IncompleteSnapshotError`. Incomplete compilation returns before complete-HIR derivation, memory
planning, SSA, bytecode lowering, or execution. A complete revision derives one ephemeral
source-optional HIR, validates it, and enters the existing compiler without rendering, hashing, or
parsing source. Semantic edits and compilation invoke the parser zero times; imported and
source-free `identity(42)` fixtures have equal user declaration signatures, expression kinds,
references/calls, diagnostics, compiler outcome, and VM result.

Revision-labelled queries cover deterministic paginated entity listing/search,
definitions/references, callers/callees, actual/expected node types, diagnostics, hole context, and
expected-type-filtered legal constructors. Hole scope is recomputed after later declaration
creation. Continuations bind namespace, revision, and query. A fallible iterative projection renders
state, blockers, selected entity/body/type/reference headers, and explicit holes without source
attachments. Projection labels never construct identity.

The former syntax-shaped editing service, source-node protocol identities, hidden-body hole overlay,
stdio/session schemas, text journal/publication path, CLI routes, unsupported reserved draft shapes,
and development semantic-digest surrogate are deleted. There is no replacement wire service.

General unresolved references, ambiguities, conflicts, recovery nodes, declaration deletion/movement,
local-storage construction, generic-call construction, match construction, source rendering,
persistence, collaboration, and incremental recomputation remain gaps. Everything below continues
to define the broader target contract.

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

**Target.** A workspace may preserve an absent entry point, missing declaration bodies, typed holes,
untyped holes, unresolved references, ambiguous choices, type/effect/capability/ownership mismatches,
missing fields, arms, parameters or declarations, import failures, explicit conflicts, and recovery
nodes.

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
    -> structured completeness and semantic consistency witness
    -> complete source-optional compiler HIR
    -> effect, ownership, memory, and typed lowering validation
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
