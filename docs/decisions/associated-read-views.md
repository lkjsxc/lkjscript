# Associated source-tied read-view families

Status: selected, independently reviewed design for the next isolated language
increment. This document and [the semantic contract](../spec/associated-read-views.md)
define the intended behavior; implementation, version assignment and acceptance
are pending.
The base is `fce333fd500c23a1413c8b6b00c6a0153cba27e7`; its **0.1.95** proof and
publication work remain separate. This selection changes neither that source nor
its evidence.

## Why this boundary

[Direction](../direction.md) prioritizes composition of types, traits, ownership,
lifetimes and concurrency. [The roadmap](../roadmap.md#compose-ownership-and-generic-contracts)
selects associated views against differing storage representations. The implemented
[associated Owned types](../spec/associated-owned-types.md) can name a supplier's
real owned block and borrow it. They deliberately cannot construct a descriptor
containing a source read and offsets. A storage-independent range transformer
therefore cannot expose a transient Flat span and a private Chunked span through
one associated family.

Three alternatives were compared:

| Boundary | Compositional value | Why select or defer |
| --- | --- | --- |
| Associated source-tied read-view families | Connect exact trait selection, private representation, explicit source lifetime, generic transformation and joined reads. | Selected. A consumer-first range/subrange family exercises these contracts together. Its correctness has observable provenance, admission and cleanup obligations. |
| Prerequisite-forwarded associated bindings | Permit a generic DelegatingReader to bind its Block to Reader.Block. | A useful blocked adapter, but an alias still names an Owned type. It cannot express the constructed view or its lifetime. Stored RHS projection support remains separate. |
| General region custody | Permit roots, local aliases/cycles, closure freeze, transfer and reclamation policies. | Important later breadth. This workload needs a checked access contract rather than a collector or region-placement policy. Select those mechanisms against measured custody/reclamation needs. |

The selected increment is substantial. It is not chosen because it is the cheapest
change, and it does not require a speed win, personal application milestone or
immediate API-cost saving.

## Descriptor custody is a language contract

A read-view is an **affine handle to immutable descriptor backing**. It is neither
an Owned source value nor unrestricted ordinary data. One handle has one cleanup
owner. Moving that handle changes its cleanup location, without copying a source
or granting source ownership. Implicit copying is invalid. Borrowing and an
explicit checked reborrow create separately accounted read leases/affine handles;
they do not duplicate the backing's stored token owners.

The backing cell is the sole finalizer of its field read tokens and ancestor
lease links. Each handle or admitted reader retains that cell. Finalization starts
only after its last handle/read lease closes. An affine handle may be discarded
or moved while a derived view retains a read lease: descriptor storage is immutable
and remains live. This differs from consuming an Owned source, which remains
forbidden while its source loans exist. A host reference count may implement the
cell, but shared control is not shared source ownership or a selection of universal
per-object ownership for the language.

This separates two lifetimes:

* The **source scope** bounds the admitted referents and every handle/lease derived
  from them. A descriptor lease cannot extend that scope.
* The **descriptor lifetime** lasts until its backing cell's final lease closes.
  It may exceed the local binding or activation that constructed the descriptor.

Consequently, `outer(source) { a = range(source); return refine(borrow a); }` is
valid under an exact source-tied return contract. The returned subview retains the
necessary parent backing lease and source guards before `a` is discarded. It
does not point into a departed stack frame. The caller can reuse/read `a` while
it exists; discarding its handle does not revoke the subview. Neither view may
outlive the original source scope.

Immutable backing links form a DAG: construction may link only already admitted
live cells, and no field/link can be patched after publication. Repeated refinement
can create a deep dynamic chain despite finite nominal layout. Finalization must
therefore be allocation-free and iterative, with cleanup work links reserved as
part of cell creation. Recursive host destructors, an unmetered cleanup stack or
allocating a cleanup queue after quota exhaustion do not satisfy this contract.

## Source authority and scopes are different evidence

A symbolic read-scope argument participates in a view's type and substitution.
Its owner-qualified binder is canonical meaning. A fresh invocation, ordered source
parameter, lineage path and live lease identify runtime access. These identities
are separate: equal scopes/types/allocations do not authorize another argument,
and fresh runtime lease IDs do not create infinitely many prepared types.

Named signatures universally quantify explicit read-scope parameters. Owned source
borrows establish an actual scope at a caller's declared lexical loan, or forward
an already certified parameter scope. Borrowing a view forwards or explicitly
narrows its existing source scope. It does not bind validity to the temporary
descriptor variable's lexical lifetime, because the lease retains that backing.
There is no general lifetime inference, alternative-source union or implicit
scope widening.

Two result relations remain distinct:

* `ReadFrom(source)` returns a borrowed token of an Owned type. For an Owned source,
  it is that input or a real owned descendant. For a read-view source, it is an
  Owned referent reached through an admitted view-reference/owned-child path. The
  latter proof never classifies the descriptor as an Owned descendant.
* `ViewFrom(source, scope)` returns a newly constructed or explicitly reborrowed
  affine descriptor handle at the selected source scope. It retains the necessary
  lease closure. An ordinary owning/value return cannot carry it.

Both select exactly one borrowed parameter of a pure named graph function/method.
Entering each call rebases arguments to distinct invocation/parameter anchors;
return and caller adoption check/rebind those anchors under a sealed packet.
Module-private construction permission is not provenance proof.

Every backing retains an explicit selected-source custody lease, including an
empty/metadata-only view. That lease protects source consumption/mutation but is
not a readable field. Only declared ReadReference paths can project/read the
source. Thus the range witness's Self reference requires Self sharing even for
an empty range; a truly zero-reference descriptor may share its checked metadata
while retaining an inaccessible source guard. Its children receive no source read
permission, and ReadFrom(view) cannot recover the root through passive custody.
Complete declaration/phantom/unused admission decides this distinction, not a
runtime range length or a producer's claimed omission.

Read projection is checked under a short backing/access guard, including exact
type, source path, scope and invocation authority. It yields an admitted ordinary
value or a sealed read token/lease before the guard closes. No raw host reference
survives lock release. Joined children get separate destination-origin envelopes
and read handles; they never mutate the shared backing's stored token origins.

## Public composition and versioning

The selected witness has a consumer/contract package authored first, a separate
generic transformer, independent Flat and private Chunked suppliers, and an
application importing exact package/revision bindings. Its range views are real
constructed descriptors. Subrange forwarding and a `ReadFrom(view)` source
projection compose with normal borrowed helpers and nested joined reads. Expected
values, rejected proposals and fault cleanup are independent observations, as
specified in [the contract](../spec/associated-read-views.md#required-witness-and-acceptance).

Canonical meaning, public authoring/readback, interfaces/transport, compiler units,
preparation, both execution engines and independent admission must agree on the
complete contract. ReadView is a separate category throughout; being non-Owned
does not make it eligible for unconstrained ordinary generics, containers,
captures, serialization or transfer. Sharing is an additional proof of accessible
read referents and metadata, not an inferred grant.

A clean versioned encoding cut is permitted. Prefer one explicit authoritative
result/scope/member model over preserving obsolete field shapes through competing
paths when that would complicate admission. Feature-free byte identity with .95
is not an acceptance prerequisite for this increment. Exact new generation and
reader-profile values must be assigned at their maintained owners before encoding
implementation. Unsupported readers must refuse clearly; supported historical
proof identities remain historical and cannot be relabeled as proof of the new
semantics. Maintained native assets must be rebuilt/admitted through ordinary
product operations if the chosen cut invalidates their current encodings.

General regions, collectors, mutable views, multi-source views, stored associated
RHS projections, borrowed task results, detached loans, general higher-kinded
types and first-class higher-ranked callable values remain outside this increment.
This decision does not limit the checked source-tied composition selected here to
one stack frame or one storage representation.
