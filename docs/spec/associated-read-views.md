# Associated source-tied read-view families

Status: selected, independently reviewed design; unimplemented.
[The decision](../decisions/associated-read-views.md) owns the selection and custody
rationale. Existing [associated Owned types](associated-owned-types.md),
[borrowed results](owned-read-results.md) and
[structured parallel reads](structured-parallel.md) retain their behavior except
for the explicit extensions below. This is not an acceptance or release claim.

## Semantic inventory and authority

An associated member has one of two distinct kinds: an arity-zero `Owned` member,
or an arity-one `ReadViewFamily` whose argument is a read scope. Both retain the
exact package-qualified contract reference and stable AssociatedTypeMemberId.
Kind/arity is part of the complete inline inventory. Names, member ordinals,
matching output types and equal Self never select a member or witness.

A ReadViewFamily binding supplies an explicit owner-qualified scope formal and a
nominal read-view application template. Its type actuals use only that
implementation's admitted exact formals. Its scope actual is precisely the binding
formal. It supplies no associated equations, defaults, implementation search or
associated projections in stored RHS closure. Complete member coverage, authored
contract order, strict binding identity order and admission of unused bindings
remain mandatory. A projected view family application retains the complete exact
implementation, ordered actuals and prerequisite applications, including phantom
and unused actuals.

A nominal read-view declaration has one explicit scope formal, ordered type
formals with existing explicit Owned/ordinary-data constraints, and a complete
ordered field inventory. Its defining module owns
construction and field-operation authority. Public visibility can expose nominal
identity without exposing field layout or grants. A view's type identity includes
that exact declaration, all ordered type actuals and its symbolic scope actual.
Associated abstract applications additionally retain exact member/witness authority
until substitution. Representation/type equality is not access authority.

Read-scope formals have stable semantic IDs qualified by their binder: a named
function/view declaration or implementation binding, or the exact
contract-and-method/member coordinate for inline inventories. References to another
declaration do not import its binders. A lexical actual identifies its defining
callable and lexical loan binding. Runtime invocation IDs, leases, addresses and
timestamps are not canonical scope/type bytes.

Binder/member/field inventories use stable identities, bounded complete authored
order and independent uniqueness checks. Unused scope formals and phantom actuals
are admitted in their own exact owner context. A scope parameter is not a runtime
value, a capability grant or an ambient region imported by naming its declaration.

## Categories, fields and finite layout

`ReadView` is a distinct affine category. It is not `Owned`, ordinary data,
Transferable or CaptureSafe. Existing unconstrained ordinary or Owned type formals
cannot admit it. A view occurrence is legal only in explicit view-family/view
signature positions, view bindings, admitted read-reference fields and the scoped
operations below. Ordinary records, lists/maps/options/results, Owned aggregates,
capability descriptors, first-class callable signatures/values and phantom nominal arguments
do not acquire permission to contain a view. Complete occurrence admission follows
nested and unused actuals; erasing a visible field does not make a phantom escape
legal.

A read-view field is either admitted immutable ordinary Transferable data, or an
explicit `ReadReference(referent, scope)`. Ordinary field closure excludes views,
owners, resources, secrets and callables. A read reference targets an independently
admitted Owned type or nominal read-view type and has exactly the declaration's
scope actual after substitution. It conveys read rights only. Existing ordinary
or Owned fields cannot become read references.

Every reference field is constructed from a live admitted read of the selected
source or its accessible descendants/references. All fields must derive from that
one exact source parameter lineage. A private constructor may not store another
parameter merely because its type, scope or physical allocation matches. Ordinary
metadata conveys no new source rights. Validation includes fields not used by a
method and inactive syntax.

The nominal field dependency closure is finite and acyclic for this increment,
including view referents and phantom type actuals. General recursive nominal
Owned or view layouts are not introduced. Dynamic ancestor lease chains are
different: refining a view of the same nominal type may retain previous backing
without a recursive user-visible field. Their custody DAG and cleanup rules below
do not impose a type-depth bound on execution or prove program termination.

## Source scopes, binders and substitution

A source scope is a checked upper bound on read validity, not the storage lifetime
of a descriptor variable. Named signatures universally quantify explicit scope
formals and explicit source relationships. Each source relationship names one
ordered borrowed parameter. A view parameter already carries its source scope;
an Owned borrowed parameter used as a view source is associated with a signature
scope formal. Functions use exact ParameterIds and methods use positions in their
complete ordered parameter vector.

At a call, an Owned owning local establishes the result's declared lexical loan
scope. An already borrowed Owned parameter may forward its certified source scope.
A view argument supplies its existing source scope. An explicit reborrow may use
that scope or a scope whose containment is already proved by lexical nesting or
declared source relationships. There is no arbitrary widening, inferred common
root, lifetime union or unrestricted region equation solver.

Containment `child <= parent` means every use/lease in child finishes before parent
ends. Scope equality is reflexive. Only explicit forwarding/containment evidence
can discharge a signature relationship. Returning a view constructed from a
borrowed parameter at that parameter's certified scope forwards existing validity;
it does not promote a local-owner loan to an enclosing scope. A new local Owned
value cannot become the source of a result tied to a different parameter.

Substitution simultaneously applies ordinary type actuals, exact witness operands
and owner-qualified scope actuals. Callee source positions/identities are mapped
to their exact caller arguments; matching IDs in another binder do not capture.
Mapped methods must match kind/arity, scope relationships, parameter order/use,
result relation/type and exact effects after substitution, including unused
methods. Ordinary type substitution being empty does not skip scope/witness
substitution. Parameter aliases to the same scope are allowed only with explicit
substitution evidence; they never merge their runtime source anchors.

Prepared type/application identity uses these finite symbolic coordinates and the
complete witness environment. It does not allocate a new type for every dynamic
invocation. Each invocation nevertheless mints fresh runtime access/return seals.
Shared prepared bodies/caches cannot store live source tokens, descriptor handles,
lease IDs or permissions from a previous invocation.

## Descriptor backing, moves, reads and release

Construction produces an affine view handle. Immutable backing owns its ordinary
field values, reference-field read tokens, an explicit selected-source custody
lease and ancestor lease links. That backing cell is their sole finalizer. The
source lease is required even for an empty or metadata-only view: it protects the
selected source independently of whether a reference field is read or exists.
Source authority must be supplied explicitly, not inferred from a first field.
Handles/read leases retain the cell but do not duplicate its stored cleanup owners
or the source's Owned token.

A `CustodyOnly` source/ancestor link protects validity and cleanup but grants no
user read path. No projection, ReadFrom result or task read can extract its token
as a readable value. A `ReadReference` field separately grants exactly its typed
read path. The two link classes are retained and independently checked; a digest
or matching source anchor cannot convert custody into access.

| Operation | Required behavior |
| --- | --- |
| Move a local view handle | Take its one cleanup obligation into the destination. The old local becomes unavailable. Backing/source payload does not move. |
| Implicit copy, unrestricted parameter or consuming parameter | Reject. A view cannot be ordinary data or acquire source consumption rights. |
| Borrow a live handle/read lease | Mint a separately checked, reserved read lease for the exact invocation and type/scope. Repeated/sibling reads are permitted. |
| Explicit `view-reborrow` | Produce a new affine handle retaining existing backing and source validity, with a fresh checked lineage seal. Reserve before acquiring its lease. No field/token owner is cloned. |
| Reuse after a read | Keep the original handle live; a read does not move it. |
| Discard/move a handle with derived readers | Remove/move that handle's obligation. Retained read leases keep immutable backing live. This does not permit consuming the Owned source. |
| Final handle/read lease release | Finalize backing exactly once, then release its stored tokens and ancestor links in the required order. |

The language does not expose a general descriptor-copy operation. Returning a
borrowed view parameter as a view result requires explicit checked reborrow;
loading it as an ordinary copied local does not mint an affine result handle.
Reborrow preserves the exact selected input's anchor and custody lease; it cannot
relabel another parameter's backing, including a zero-reference backing.
Local view moves and view-result handoffs are not Transferable movement into
another invocation/task. View parameters have borrow mode only in this increment.

Backing remains stable independently of a local variable or departed activation.
No handle points into a frame's local vector. Parent backing is retained by an
explicit lease link where a derived view/read needs it. Thus a parent descriptor's
local binding may end while a subview retains that backing. Its source scope
must still remain valid. An Owned source's consume, mutation and reclamation remain
protected until all derived leases/loans discharge.

Backing links may target only already admitted live cells; fields/links cannot be
rewritten after construction. This creates an acyclic runtime custody DAG. Common
parents may have multiple read leases, but one finalizer owns their stored tokens.
Repeated refine-and-replace can make a deep DAG: final release uses reserved
intrusive cleanup work and iterative traversal, with no allocation, recursive host
destructor cascade or user callback. Ready-finalizer ownership transfers exactly
once, including concurrent last-reader release. Nominal type-depth limits do not
substitute for this runtime cleanup guarantee.

Releasing a parent lease enqueues a ready finalizer rather than invoking it
recursively. The same rule covers retained provenance/return-guard nodes, not only
user-visible descriptor fields. Cleanup links are reserved before publication;
quota exhaustion cannot require a new queue allocation to make teardown possible.

## Result relations and exact runtime source

The authoritative result model distinguishes `Value`, `ReadFrom(source)` and
`ViewFrom(source, scope)`. This semantic distinction is retained through public
inspection, canonical meaning, interfaces, compilation and execution.

`ReadFrom` has an Owned result type and returns a sealed read token. An Owned
source permits the exact input or a real Owned descendant. A read-view source
permits an Owned referent reached through a completely admitted path of
`ViewReference(field)` and `OwnedChild(field/case/index)` edges. A metadata field,
equal allocation or opaque descriptor identity is not such an edge. The result
packet retains descriptor leases and ancestor guards needed by that path. The
view is not classified as an Owned descendant, and no new owner is manufactured.
`CustodyOnly` links are never path edges: ReadFrom(view) cannot recover a declared
source merely from its retained lifetime/identity guard.

`ViewFrom` has a selected nominal/associated read-view result type. It moves a
newly constructed or explicitly reborrowed affine handle into one sealed return
packet with its source/scope and lease closure. It may return a descriptor that
was not previously an Owned descendant. Every reference field and retained read
path must derive from the selected source. Value/owning-result modes cannot carry
a view, and ViewFrom cannot justify an Owned owner or arbitrary local loan.

Both relations require a pure named graph function/method and one borrowed source
parameter of independently admitted Owned or ReadView category. Tasks and externals
cannot return either loan form. Borrowed-result functions remain unavailable as
first-class callable values or component entrypoints. Tail activation replacement
with a borrowed-result relation or live view custody is initially ineligible; it
requires a separate custody proof before later adoption.

Each call entry gives each ordered argument a separate semantic anchor containing
the exact prepared application, fresh invocation, parameter identity/position and
admitted ancestor path. Reborrowing/projection preserves that anchor. Even when
two arguments read the same allocation, the selected result must prove derivation
from its designated argument. The source scope establishes validity, not which
argument supplied the result.

Return validates that exact anchor and all paths, reserves packet/guard/handle
storage, then detaches custody. Caller adoption validates the exact application,
source argument, result kind/type/scope and sealed lease closure before rebinding
to its lexical source. Neither side trusts a producer's claimed source digest.
Failure before detach leaves callee custody; failure after detach leaves the
packet's one cleanup owner. No unsealed raw view value can cross this boundary.

## Subview and ordinary read composition

For a family `View<s>`, a pure refinement has the relationship
`refine(View<s> borrow) -> View<s> ViewFrom(input, s)`. It can explicitly narrow
the scope when required, but cannot extend it. The refinement may reconstruct the
private descriptor from checked fields and/or retain an ancestor descriptor lease.
Each reference still has the selected input's invocation anchor and original
source validity.

The following semantic shape is required, rather than unsupported:

```text
outer<s>(source: Self borrow) -> View<s> ViewFrom(source, s)
    a = range(source)                 // source scope s, local handle a
    b = refine(borrow a)              // retains a's required backing/guards
    return b                         // exact transitive source is source
```

After return, `a`'s local handle has been released but its retained backing remains
until no descendant needs it. Returning `b` through Value or tying it to another
source parameter rejects. A descriptor lease never rescues a newly created local
Owned source from an invalid escaping result.

A pure `project(View<s> borrow) -> OwnedT ReadFrom(view)` is also required. It
returns an admitted referenced Owned value or its descendants, with the view's
lease closure and exact parameter/path seal. That token can feed existing normal
synchronous borrowed helpers, including source-tied ReadFrom helpers. A higher
pure caller may forward it through its matching ReadFrom relationship when the
transitive chosen-source lineage is proved. The caller cannot obtain layout
authority, a different input's reference or an owning token by this forwarding.

## Guarded reference and ordinary field access

Construction validates provenance as well as private field authority. Access
independently validates a live backing/lease, exact instantiated nominal/family
type, read scope, destination invocation permission and admitted field/path.
These checks are required even if construction once validated the same cell.

Acquire a short checked access guard, read the selected field, and establish the
standalone result's admission before releasing that guard. Ordinary immutable
results retain their existing value admission. Reference results receive a sealed
read token/lease retaining the referent and necessary ancestors. No raw host
reference may outlive a storage lock or the checked guard. Storage locks never
remain held during a user method/helper/task or nested read.

Stored field token origins are immutable. Access in another admitted invocation
delegates a fresh read handle from the retained source lease and destination
permission; it does not rebase shared backing's stored tokens in place. Exact
source/path proof and live access validation are separate checks. Private layout,
same physical backing, an integrity digest or a reader's guessed pointer cannot
replace either.
That delegation requires an accessible ReadReference path; a custody-only source
lease never becomes a destination read token. Per-handle invocation anchors rebase
the immutable stored path under the exact argument mapping, preserving distinct
anchors when two parameters alias the same backing.

## Synchronous tasks and joined reads

A named synchronous graph task may accept a view in borrow mode and return an
ordinary value or an independently admitted owner. Its allowances, effects and
deployment grants remain independently checked; borrowing adds none. It may use
pure ViewFrom/ReadFrom helpers inside its active scoped invocation but cannot
return/retain/capture/serialize a view or loan.

A joined child may receive a view read only with independent `ReadShareable`
evidence for that exact closed or explicitly constrained view application. This
proves all accessible reference-field closures and metadata safe to read across
the boundary, including inactive and phantom actuals. A reference to Self requires
Self sharing proof. A passive ancestor custody guard grants no read access to
unrelated fields; merely retaining it neither grants nor requires a blanket
Shareable proof for those inaccessible fields. Existing Shareable requirements
still apply to every Owned read token actually delegated/projected. An Owned or
abstract ReadViewFamily bound alone is insufficient. ReadShareable is not
Transferable, capture permission or a capability grant.

A genuinely zero-reference, metadata-only declaration still retains its selected
source's custody-only lease and refuses source consume/mutation. If no operation
can recover that source through a declared read-reference path, ReadShareable
checks the descriptor's complete accessible inventory/metadata and custody protocol;
it does not grant a child source read or demand Shareable for inaccessible source
payload. Its guard remains source-origin custody. By contrast, a range descriptor
with a Self reference/source-projection method requires full Self sharing even
when its runtime range is empty. Empty bounds or an untaken method do not erase
the declared read authority. A source-recovery operation for a zero-reference view
must reject rather than secretly promote its custody guard into access.
An empty declaration does not waive any phantom type-argument obligation. The
metadata-only control may use a view with no source-type argument; adding a
forbidden/nonsharing phantom actual must still fail complete category/sharing
admission independently of its zero field count.

Each child envelope binds exact program/application/family/type/scope, source
lineage, source invocation and destination child. It retains parent backing and
source guards, and mints invocation-local read handles. Two children reading one
descriptor have separate delegated permissions; its stored tokens keep their
original origins. Nested groups may delegate only these admitted read rights.
Children cannot transfer affine handles or return views/loans. Their results prove
ordinary/Owned transferability independently.

An Owned token projected with ReadFrom(view) may be passed as an existing borrowed
task argument, under its exact Shareable proof and retained descriptor path. Do
not infer sharing from the enclosing view name or confuse its passive guards with
access rights. Parent protection lasts until every started descendant has completed
cleanup and joined, including trap, cancellation and result-adoption failure.

## Complete admission, bounded proof and encoding cut

Authoring and discovery retain member kind/arity, complete bindings, nominal view
fields, scope binders/actuals, source/result relations and all typed paths. Canonical
owners and public interfaces independently check that meaning. Transport admission
does not infer support from an advertised capability or an associated supplier's
generation. Every actual consuming owner/interface and containing semantic revision
must use the selected supported contract. Compiler keys/summaries and full payload
admission bind the same complete meaning; no inventory/relationship can disappear
under lowering or cache reuse.

Source, interface, compiler and artifact readers independently reject wrong
member kind/arity, scope owner or source position, hidden view occurrences, missing
bindings/fields, forbidden provenance, private field access and invalid unused
content. Consistently recomputed hashes do not authorize a changed relationship.
Current proof seals must explicitly cover the new semantic admission; historical
seals keep their original identities and do not become current proof.

Finite preparation requires an independent term/binding/witness/scope interpretation
and growth oracle, plus a separate eager all-path comparison for demand completeness.
Include method mappings, function-value syntax, untaken branches, phantom actuals
and nested prerequisites even where an escaping callable itself rejects. Preserve
finite exact aliases/permutations and reject genuine positive type expansion.
Scope forwarding cannot collapse distinct witness paths or generate unbounded
prepared types from fresh runtime invocations. Scope proof is finite explicit
containment, not unrestricted solver search. Capacity exhaustion is Resource,
not invalid meaning or execution termination evidence.

Reserve work and storage before building types, scope maps, paths, descriptor
fields, cells, leases, seals and cleanup work links. Cancellation remains distinct;
failed admission cannot partially publish or poison healthy retry. Ordinary trusted
execution acquires no invented cumulative instruction budget.

A clean versioned cut may replace predecessor field shapes, including legacy
result_borrow metadata, with one explicit result contract. There is no requirement
to preserve feature-free .95 bytes if that obstructs a cleaner authoritative model.
Assign exact Graph/interface/compiler/bytecode/artifact/authoring/validator/reader
generations together at their maintained owners before implementation. Refuse
unsupported profiles before decoding new fields; do not erase scope/member/result
data or patch headers. Rebuild maintained assets through ordinary product owners
where required. Historical .95 evidence and immutable publication are preserved.

## Cleanup and failure order

Construction owns each newly minted field lease immediately. Failure before cell
publication releases partial fields/links through one cleanup owner. Published
cells own their stored tokens; handles/readers only own their respective leases.
Move/reborrow/return/adoption have reserved destinations before detaching a handle
or guard, including interruption between extraction and caller adoption.

Release projected child read tokens and derived descriptor leases before the
ancestor links/root guards they require. Last-lease finalization releases a cell's
field/read obligations before releasing its ancestor links; ready ancestors enter
the already reserved iterative cleanup worklist. The Owned source is not consumed
or reclaimed before these obligations and any started joined reads finish. Shared
parents finalize once after all dependent leases close. No user methods, effect
replay or fresh allocation occur in finalization, and cancellation/quota refusal
does not abandon cleanup. Deep dynamic lease chains must not overflow the host
stack. Failed construction/return/projection/adoption/child-result cleanup permits
a later valid invocation to remain healthy; it does not imply effect rollback.

## Required witness and acceptance

Author a RangeRead contract and generic reducer before suppliers, then a separate
generic subrange/forwarding transformer. Independent suppliers bind View to FlatSpan
and a private nominal ChunkedSpan. FlatSpan stores a checked source read and
start/length metadata; ChunkedSpan stores a checked private chunked-source read
and metadata for ranges genuinely crossing chunks. They are newly constructed
descriptors, not owned descendants or copied collections. The application imports
the consumer, transformer and exact independent suppliers; it needs no explicit
caller-selected View type argument or private field access.

The contract has pure range, view-length, view-get, subrange and source-projection
methods. Source projection is `source(View<s> borrow) -> Self ReadFrom(view)` and
must compose with an independently selected normal Owned reader/helper. This
exercises descriptor reference paths rather than only ordinary gets.
An implementation's method invariants do not widen field/provenance authority.

Use literal [11,7,23,5,19,13] with chunk boundaries [11,7] / [23,5,19] / [13].
Range [1,5) yields length4, [7,23,5,19], sum54; relative subrange [1,3) yields
[23,5], sum28. Empty, whole and single-element ranges preserve their explicit
expected results. Invalid bounds trap through an explicit checked range/index
boundary. After all leases discharge, drain to [13,19,5,23,7,11] and reuse the empty
owner. Repeated refinement and parent-local scope exit exercise retained backing.

Acceptance additionally requires:

* Ordinary public create/set/readback/import/check/build/export through a copied
  CLI outside its checkout, with literal retained inputs and exact real package
  bindings. Execute attached and source-detached artifacts. Independent source,
  interface/compiler and artifact admission corroborate the contracts.
* Rejections for another same-typed/same-allocation parameter, invalid field/path,
  scope/member/witness swap, local Owned-source escape, hidden/phantom/unused view
  occurrence, private foreign field access and every prohibited copy/capture/
  serialization/container/entrypoint/task-result/transfer boundary. Rejected
  proposals leave accepted state unchanged; rehashed forgeries still reject.
* Independent memory/provenance interpretation and canonical reference execution
  for ViewFrom and ReadFrom(view), across packages and multiple activations. Cover
  exact argument aliases without merging anchors, guarded field reads, scope
  substitution and valid fresh invocations sharing prepared types.
* Explicit reborrow, move, reuse and drop tests, including dropping a parent handle
  while its subview/read remains live. Normal completion and errors must retain
  backing/source guards until the final lease and then release them exactly once.
* A legal independently ReadShareable closed view plus an Owned-only/abstract view
  refusal. Nested joined children read the same backing and projected Owned reads
  with distinct destination handles, return ordinary results and preserve source
  custody. Stored token origins remain unchanged.
* An empty-range control still requires sharing its declared Self reference. A
  separate metadata-only/zero-reference view retains exact source custody even
  when unused, can share only its admitted accessible metadata, and cannot project
  or delegate its passive source. Protected consume/mutation and forged source
  recovery reject independently of reference-field count.
* Faults during field/lease reservation, packet extraction/adoption, guarded read,
  child start/result adoption and cancellation. Deep refine-and-replace chains
  clean up iteratively without allocation after exhaustion or host-stack growth;
  a subsequent valid invocation is healthy. Retain unfavorable costs separately.
* An independent family/growth oracle and eager all-path comparison, complete finite
  layout/category admission, finite aliases and genuine expanding refusal. A
  finite execution prefix or production/reference agreement alone is insufficient.

Finalized-byte acceptance and publication are separate later verification/release
obligations. Descriptor allocation must be distinguished from copying source
payload; this contract makes no whole-program zero-copy or performance claim.

## Current implementation owners

The changes must be grounded in the existing owners rather than privileged adapters:

| Boundary | Current starting points |
| --- | --- |
| Meaning/category/result relationships | kernel/type_object.rs, owned_contract.rs, owner.rs, memory.rs, owned_associated.rs, owned_contract/associated_application.rs |
| Independent admission/growth | kernel/memory_reference.rs, memory_associated_reference.rs, memory_associated_closure_reference.rs, associated_flow_canonical_reference.rs, associated_flow_reference.rs |
| Authoring/discovery/interface | change/request/, control/change/, contract/, package_interface.rs, package_transport/ |
| Lowering/compiled admission | compiler/lower.rs, unit.rs, artifact_associated.rs, borrowed_result_admission_tests.rs |
| Prepared bindings and view/result execution | execution/normalized/prepare.rs, vm.rs, vm_checked.rs, reference_borrowed_results.rs, reference_checked.rs, reference_associated_signatures.rs |
| Sealed read storage and joined custody | execution/normalized/value.rs, owned_storage.rs, vm_transfer.rs, reference_transfer_substitution.rs, parallel.rs, vm_parallel.rs, reference_parallel.rs |

Paths above are relative to src/platform/. No general region runtime, collector,
mutable/multi-source view, stored associated RHS projection, borrowed task result,
detached loan or first-class higher-ranked callable is selected here.
