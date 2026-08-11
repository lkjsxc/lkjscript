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
identity allocator including tombstones, derived indexes, typed hole records, unresolved value-
reference records, diagnostics, and structured completeness blockers. The semantic program may omit
`main`; missing bodies and typed expression holes are actual `ExprKind::Hole` leaves with unknown
effects. An unresolved value reference is a distinct typed childless expression carrying one
validated requested-name hint and a fixed future copy-load intent. It has unknown effects, no
selected target, no reference or candidate dependency edge, and no executable load or fallback.
Source-free expressions have an honest semantic origin; imported source paths and digests do not
create logical identity.

The implemented unresolved lifecycle begins at an existing typed expression or body hole. A typed
introduction edit preserves the targeted root node identity, physically removes the displaced
subtree and hole record, and publishes one incomplete revision. Direct state inspection returns the
revision, requested name, expected type, owner, context, and stable visible-scope entities. A
revision-bound paginated candidate query scans that scope once and returns only visible parameter or
local identities accepted by the canonical copy-load type and copy-safety prefilters. Candidates
carry current name, kind, declared semantic type, exact case-sensitive requested-name equality, and
an explicit requirement for canonical resolution validation. Exact-name matches sort first; the
remaining order is current name then stable entity identity. Requested spelling is intent metadata:
renaming a candidate changes its current name and exact-match fact but never rewrites the hint.
Explicit resolution may select a differently named stable identity. A page containing one candidate
plus a continuation is sufficient to derive that multiple choices exist without adding snapshot
state. Even an exact-name candidate remains provisional and cannot resolve the node automatically.

Candidate multiplicity remains a revision-derived query fact rather than a second incomplete node
kind or stored candidate set. A finite stable-identity subset would be distinct semantic intent only
when an author deliberately constrains future resolution across a candidate-lifecycle change.
Reconsider that state only when a current site-preserving edit or durable multi-turn consumer needs
the constraint and explicit immediate resolution cannot preserve it.

Resolution is one typed structural edit from the unresolved root to the ordinary canonical load. It
reuses the existing draft load path for namespace/generation, visibility, value-binding kind, type,
copy safety, and storage, preserves the root node identity, removes the unresolved blocker and
diagnostic, and creates the normal reference/dependency facts. If that edit makes the staged snapshot
complete, the existing whole-program ownership check also runs before publication; otherwise the
later edit that reaches completeness runs it before that complete revision can publish. Failure
publishes nothing. Replacing the node or an ancestor and deleting its callable prune the unresolved
record through ordinary containment and tombstone removed identities; unrelated edits preserve it.
Old immutable snapshots retain their original unresolved state. Current text import remains
fail-fast on unresolved source names; it does not manufacture parser recovery state.

All construction and later editing uses one revision-checked atomic transaction path. Implemented
creation covers non-generic products and enums, generic and non-generic functions, and one
parameterless `main`; function and entry bodies begin as typed holes. One generalized function
creation operation accepts zero or more ordered type-parameter drafts. A declaration-local opaque
binder handle may occur at arbitrary depth in the creation-only `DeclarationType`; it has only that
edit's lifetime and never enters a snapshot, query, projection, or diff. Staging validates names,
handles, exact trait identities, bounds, and types, then allocates stable function, binder, and value-
parameter entities and lowers to the same canonical universal function and bound facts used by the
source importer. Public published type inputs and query results use one exact recursive
`SemanticType`, whose nominal and binder cases carry stable workspace identities rather than product
names, compiler enum/trait IDs, layout IDs, binder strings, or source identities. Prelude enum
constructors and core traits use explicit closed builtin identities. Recursive clone, equality,
debug/display where supported, validation, conversion, projection, and destruction of both public
type models do not recurse on native stack or impose a type-depth quota.

`ExpressionDraft` and `PatternDraft` are flat non-recursive trees whose physical node order is not
semantic. Expression drafts cover i64/f64/Boolean/unit/byte literals, selected canonical built-in
operations, exact generic and non-generic calls, conditionals, ordered sequences, lexical immutable
`let` bindings, explicitly typed mutable locals, assignment, `while`, explicitly typed `loop`,
`break`, `continue`, early `return`, copy-safe loads, byte-vector moves and shared borrows, product
construction and field projection, enum construction and enum-variant testing, and ordered exhaustive
matches over non-generic enum scrutinees. A sequence evaluates its children in listed order; an empty
sequence yields `unit`, and a non-empty sequence yields its final child's value. `while` requires a
Boolean condition, evaluates its body children in listed order, yields `unit`, and carries the
canonical divergence effect. A typed loop carries one source-independent `SemanticType`, resolves it
through the canonical workspace type boundary, rejects `never` at any depth, and has that exact
result type. `break` carries one non-divergent value exactly equal to the nearest lexical loop result;
a while therefore requires `unit`. `continue` carries no value. Both transfers have type `never`, and
an ordered control body rejects any later expression. A draft transfer may target a surrounding loop
already present in the immutable base snapshot; a nested draft loop or while shadows it and leaving
the nested body restores it. Private HIR loop identities are allocated during staging and are never
public workspace identities, references, projection fields, or diff edges. A return has one
non-divergent value exactly equal to the owning callable's declared result type, yields `never`,
preserves the value's effects, and exits through canonical ownership cleanup. An explicitly moved
affine payload transfers to the caller rather than being dropped by the callee. It
has no target identity: the draft's callable root supplies its lexical target. A generic call supplies
one structured type argument for every stable parameter entity; argument list order is
non-semantic, while the published instantiation follows declaration order.
The compiler validates substitution, value arguments, ownership/reference restrictions, and trait
bounds and derives auto or explicit implementation witnesses. Source inference is an importer
convenience that feeds the same exact resolver; workspace edits do not perform implicit inference.
Pattern drafts cover wildcards, enum variants with exact stable field identities, and named payload
bindings. Published lexical and payload bindings use stable entity identities; transaction-local
binding handles are a separate checked identity domain and cannot escape the draft. Each arm has its
own lexical binding scope. Mutable initializers are outside the declared binding's scope; the body is
inside it. Mutable storage uses the canonical source/HIR restrictions, and assignment requires one
visible mutable local kind and an exact non-`never` value type. One iterative transaction-local control walk seeds the
nearest loop from the target's immutable HIR ancestry, allocates private identities for draft loops
and whiles, and resolves every transfer in one pass. Its facts are discarded after staging. Affine
storage can be reinitialized only after its prior value has been moved or canonically consumed; live
affine overwrite is rejected and both successful and failed paths use canonical cleanup. Break and
continue clean iteration-local owners before their selected exit or backedge; an outer owner remains
live, and canonical HIR/SSA ownership validation rejects a changed loop-carried ownership or lexical
loan state. Compiler-hidden match scrutinee/projection locals are never workspace entities or legal
constructors. The canonical usefulness/exhaustiveness checker and complete staged HIR ownership
checker remain
authoritative for match validity, move/borrow legality, and cleanup. Generic pattern construction,
forwarding an unresolved caller type parameter, ownership/reference-bearing generic instantiation,
and non-enum source-free pattern spaces are not fabricated.

Transactions delete `main`, ordinary non-builtin functions, and user-defined product or enum
declarations; they also rename supported bindings, replace expressions, introduce/refine/fill typed
holes, and introduce or resolve unresolved copy-load value references. Callable deletion cascades
through the declaration's type parameters, value parameters, locals, payload bindings, body nodes,
holes, hidden match storage, match plans, and compiler layout participation.
Product deletion owns its fields and narrowly owns explicit trait implementations whose target is
that product. Enum deletion owns its type parameters, variants, and their fields. Direct member,
trait, or implementation deletion and fixed compiler-context deletion remain unsupported.

All deletion intent is collected before order-sensitive editing. Final staged signatures, field
types, expression types and aggregate operations, holes, generic substitutions and witnesses, and
match patterns/plans decide whether an independent dependency survives. A surviving dependency
rejects with the requested declaration and a deterministic surviving dependent; deleting every
dependent declaration or structurally removing every body dependency in the same batch succeeds
regardless of edit order. Product and implementation IDs and private product/enum/implementation
vector addresses compact once; every survivor reference is explicitly remapped while stable public
entity/node identities and stable nominal/member/layout identities remain unchanged. Semantic diffs
report every changed edge at aggregate sites that reference a declaration and multiple owned members;
private relocation alone emits no rewire. Deletion and same-name creation in one transaction is
invalid; a later recreation receives fresh public identities and generations.

Replacement and hole introduction may remove `let`, mutable-local, and semantic-match subtrees.
The targeted root retains its identity, and unaffected ancestors/siblings retain theirs; rebuilt
replacement descendants receive new identities even when their content coincides. A control
replacement iteratively recomputes derived sequence, conditional, local-body, and match result types
through the affected callable root; semantic match arm/result facts are updated with that same final
graph. Bindings and plans owned only by the removed subtree are compacted out of semantic state; no
removed expression or dead binding remains behind a hole. Transactions reject invalid, duplicate, or
reserved declarations and members; foreign, stale, or wrong-kind identity; stale revision;
disconnected, cyclic, reused-child, forward-binding, cross-arm, or out-of-scope expression/pattern
drafts; duplicate pattern handles/names/fields; overlapping or delete-owned structural edits;
duplicate deletion; rename-plus-delete; retained callable dependency; type/arity/field/variant
mismatch; empty, nonexhaustive, or useless match arms; and ownership failure before publication.
Failure preserves the exact published `Arc`, revision, allocator generations/free lists, future
allocation, diagnostics, and projection. Success publishes one revision and returns created and
deleted entities, member/local identities, holes, descendant changes, and graph rewiring through the
semantic diff. Multiple refinements of one hole in a transaction report one base-goal-to-final-goal
change; a refinement whose final goal equals the base goal reports no semantic change. A
call-instantiation diff includes exact old/new substitutions, instantiated
parameter/result types, witnesses, and effects, so a type-argument-only change remains reviewable.

Structured blockers currently cover missing entry point, missing declaration/entry body, typed
expression hole, and unresolved value reference. The unresolved diagnostic has stable code
`workspace.unresolved-value-reference` and is attached to the unresolved node. Every blocker is
revision-labelled through the snapshot or `IncompleteSnapshotError`. Incomplete compilation returns
before complete-HIR derivation, memory planning, SSA, bytecode lowering, or execution; independent
semantic-program validation also rejects a surviving unresolved leaf. A complete revision derives one
ephemeral source-optional HIR, validates it, and enters the existing compiler without rendering,
hashing, or
parsing source. Source-free selected paths invoke source loading and parsing zero times. Imported
and source-free scalar, product, enum, lexical-local, mutable counted-loop, typed-loop/break,
nested-continue, affine break/continue cleanup, borrow-then-move, early-return ownership-control, and
exhaustive enum-payload match fixtures have equal normalized entities/types, containment, references,
dependencies, node kinds/types/effects, canonical match shape, selected memory-obligation kinds, the
main bytecode stream, evaluator/VM results or traps, and cleanup behavior. Imported match plans carry
real source origin; source-free plans carry semantic origin; ordinary plans reject builtin or stale
provenance.

Revision-labelled queries cover deterministic paginated entity listing/search,
definitions/references, callers/callees, structured entity/function/node types, diagnostics, hole
context, unresolved value-reference state and copy-load candidates, exact lexical and match-arm
bindings, expected-type-filtered legal constructors (including typed loop, exact break payload type,
continue, early return, and selected canonical operation identities), structured function signatures
and call instantiations, node semantics, and structured match inspection. Loop is advertised with the
hole's exact representable non-`never` type. Break and continue are advertised only where a nearest
lexical loop exists and divergent replacement is admissible; break returns the exact stable semantic
payload type without exposing the private loop identity. Return is advertised only where a `never`
result is admissible; its payload expectation is the result in the owning callable's signature view,
not the hole's local expected type. Node semantics report stable node identity and kind,
actual/expected type, canonical built-in operation identity when present, and named effect bits.
Generic signature views report stable binder entities, bounds with stable or builtin trait identity,
value
parameters, and result types. Call views report canonical substitutions, instantiated parameter and
result types, derived witnesses, and named machine-readable effect bits. `MatchView` reports the
stable match/scrutinee/body nodes, result type, exhaustiveness, ordered arms, and deterministic typed
pattern nodes/fields with stable variant/field/payload-binding entities. Its flat stack-safe
pattern graph uses a distinct opaque response-local label solely for links within one arm; no raw
compiler-dense pattern/arm ID is exposed or accepted as identity. Nominal and generic type views
preserve stable semantic identity. Move and borrow candidates are labelled as requiring canonical
ownership validation rather than claimed legal from branch-insensitive filtering. Generic enum constructors and hidden match
storage are not advertised. Hole scope is recomputed after later declaration creation. Continuations
bind namespace, revision, and query. Public containment edges are emitted in semantic child order,
including sequence and loop evaluation order. A fallible iterative projection renders state,
blockers, selected entity/body/type/reference/hole/unresolved-reference/match sections, declared
entity types, local and
aggregate structure, operation identities, named node effects, ordered arms, patterns, explicit
holes, and explicit unresolved nodes without source attachments. The selected unresolved slice shows
its requested name, expected type, owner, context, and visible-scope count but does not dump
candidates. Projection labels never construct identity.

The former syntax-shaped editing service, source-node protocol identities, hidden-body hole overlay,
stdio/session schemas, text journal/publication path, CLI routes, unsupported reserved draft shapes,
and development semantic-digest surrogate are deleted. There is no replacement wire service.

Unresolved calls, moves, borrows, type names, nominal members, patterns, and imports; ambiguities,
conflicts, parser recovery nodes, direct nominal-member mutation, public semantic movement,
generic-pattern construction, Boolean/integer/product source-free patterns, source rendering,
persistence, collaboration, and
incremental recomputation remain gaps. Ownership/reference-bearing
nominal fields are also outside the current source-free declaration surface, while generic
ownership/reference instantiation is an explicit call restriction. Current payload-match ownership
coverage uses the strongest supported copy-safe field geometry. Everything below continues to define
the broader
target contract.

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
formatting, file regrouping, projection changes, unrelated edits, and private binding/function/plan
compaction. Compaction relocation is an implementation event, not a public move operation.

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
3. supports batching and validates callable and nominal deletion dependencies against the final
   staged semantic graph, including stored types, aggregate operations, patterns, holes, and generic
   witnesses;
4. applies containment-owned cascading removal without manufacturing an orphan;
5. stages the complete change without mutating the published snapshot;
6. preserves the old snapshot and allocator on validation failure, cancellation, allocation failure,
   I/O failure, or resource-policy exhaustion;
7. tombstones every removed public identity, increments the generation before slot reuse, and leaves
   old snapshots queryable; and
8. publishes exactly one new revision on success with a semantic diff, deterministic diagnostics,
   and invalidation information.

Delete-and-create replacement of the same declaration remains a contradictory batch; recreation is a
later transaction and follows ordinary tombstone/generation allocation. Product deletion cascades
only its owned fields and target implementations; enum deletion cascades only its owned variants and
fields. Independent callables and other nominals must be explicitly deleted or edited, never silently
removed as transitive dependents. Private dense relocation is not semantic movement and does not
produce survivor deletion/creation diff entries. A text patch may be imported into a transaction but
is not the foundational edit representation.

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
