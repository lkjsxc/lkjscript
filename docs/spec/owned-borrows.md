# Scoped reads of owned children

An explicit lexical read scope can inspect an owned product field or an owned
choice payload while preserving the original owner. The accepted meaning graph
carries the operation, exact source type, child binding role and loan provenance.
A type annotation alone cannot confer read or ownership rights.

These expressions compose [products](owned-products.md), [choices](owned-choices.md)
and [first-order Owned parameters](owned-generics.md). They introduce no reference
type, mutable borrow, partial move or escaping lifetime. Existing pure borrow
helpers and exact borrowed implementation methods remain the read interface.

## Product field scope

```text
(borrow-owned-field
  (type PRODUCT)
  (local packet)
  (field payload (binding view (type T)))
  (in BODY))
```

`PRODUCT` must be the exact owned product type of a live owning or borrowed local.
The operation selects exactly one direct owned field by name. Its annotation must
be the exact field type: a concrete carrier, finite owned product/choice, or an
in-scope `T: Owned`. Ordinary fields retain the existing `field` read operation;
missing or ordinary fields cannot become owned views through this expression.
Owned temporaries must first be bound to explicitly typed locals.

The `view` binding has a distinct lexical identity and borrowed role. It is visible
only inside `BODY`. The source and every ancestor custodian remain read-loaned
throughout the body, including when the body never reads the view. Nested projection
creates another scope whose provenance retains that entire chain. The expression's
result is the body's result; it does not transfer the child.

## Borrowed choice scope

```text
(match-borrowed-owned
  (type CHOICE)
  (local outcome)
  (case accepted (binding value (type I64)) (in ACCEPTED_BODY))
  (case rejected (binding view (type T)) (in REJECTED_BODY)))
```

`CHOICE` must be the exact owned choice type of a live owning or borrowed local.
The arms cover every declared case exactly once, with no fallback, duplicate,
missing or extra case. Each annotation equals the corresponding payload type.
Arm bindings have distinct identities and are private to their own bodies;
separate arms may reuse a name. Every body has the same result type.

An owned payload is a read view. An ordinary payload follows the existing admitted
ordinary-value copying rules and may be returned. The whole source remains guarded
for the entire selected arm, including an ordinary arm. Only the selected body
executes. Complete admission still checks every arm and conservatively joins its
changes to surrounding ownership state.

Borrowed analysis does not consume the choice or change its selected case. After
scope exit, the original can be read again, moved or consumed by `match-owned`.
A selected ordinary value remains ordinary after that later consumption.

## Read rights, effects and escape

A view may be passed as an exact borrowed argument to an existing pure helper or
pure implementation method, reborrowed synchronously, or inspected with another
child scope. Product views may read ordinary metadata. Repeated parent and sibling
reads are legal while a nested view is active.

Neither a view nor a protected source/ancestor may be consumed through any alias.
A view cannot become an owner, an unrestricted argument, an ordinary container
member, a capture, a stored or returned reference, or a task-transfer operand.
`unpack-owned`, `match-owned`, consuming methods and owner-returning local reads
cannot transfer a loan. Branches and untaken syntax obey the same restrictions.
Borrowed task parameters and borrowed task Self remain unsupported.

The body may create or return an unrelated owner, consume an unrelated existing
owner, and perform effects allowed by its containing task. A lexical read scope
neither grants an effect nor changes a task into a pure callable. Memory loans and
capability-resource loans retain their separately checked contracts. An external
effect completed before a later failure remains completed; cleanup is not rollback.

## Authoring and independent admission

Public discovery, structural native authoring and canonical drafts expose both
expressions. Unchanged draft re-entry preserves accepted identities. Supported
identity-preserving edits retain the operation, exact type, selector/arm metadata,
source and borrowed binding identities while applying the supported body changes.
Other edits use the existing complete-body replacement and validation path.
Function extraction containing either new scope rejects before publication until
extraction can preserve its complete borrowing contract.

Symbolic generic validation checks the scope before a concrete implementation
exists. Substitution preserves exact child types, binding roles, ancestor custody
and selected implementation identity. Canonical source, package transport, compiled
metadata and instructions must agree. Loaders independently admit complete relevant
closures, including unused methods, annotations and untaken bodies. Recomputed
integrity hashes cannot authorize an erased guard, substituted source or counterfeit
owning binding.

Depth, field/case, proof-work, allocation and cancellation policies keep their
existing owners. Check and reserve traversal/storage work before growth. Finite
admission exhaustion is a resource result rather than invalid meaning. The new
expressions do not enlarge proof or execution limits.

## Runtime custody, tail calls and cleanup

Entering a scope validates the source's exact invocation origin, closed type, live
read/owner mode and selected field/case. Runtime code acquires checked whole-source
and child loans, then releases internal storage locks before evaluating user code.
No storage mutex may be held across the body. Repeated parent reads, nested
projections and sibling reads therefore cannot deadlock on the scope's own lock.
Child views preserve the original allocation; they do not copy its payload or
transfer custody.

An active lexical child-read scope prevents activation replacement by a tail call.
This barrier also applies when the source is a borrowed parameter and the frame
owns no storage. Checking only local owners with outstanding loans is insufficient:
the frame's scope guard must remain live through the callee. Outside these scopes,
existing eligible synchronous reborrow tail transfers retain their behavior.

Every exit finalizes active scopes innermost-first. Child loans end before ancestor
guards and custodians; frame unwind proceeds youngest-first. Normal results, traps,
cancellation, quota refusal, argument/result transfer failure and host unwind all
follow this order. A returned unrelated owner is protected during result transfer,
while scope-bound loans are discharged before the surrounding frame completes.
Cleanup releases owned storage and loans without invoking user methods or replaying
effects. A subsequent valid invocation must remain healthy.

Raw ingress, adapter results, retained values, JSON/data codecs, persistence and
callable capture cannot manufacture or export these views. Sealed inert clones
remain identity markers without read or ownership authority.

## Encodings and evidence

Graph/owner generation 24 adds child-scope operations and borrowed lexical binding
roles. Ordinary type forms, callable signatures, package-interface layout 13 and
witness manifest contract 9 remain unchanged. Scope-bearing authored intent selects
codec 28, `LKJACR28`; requests without these expressions retain their existing
canonical generation. Compact change contract 32, discovery registry 20 and CLI
observation 38 advertise the new surface. Function-definition projection 14 binds
the new expression and binding records, including continuation selection.

Compiler unit 24, bytecode 19 and artifact 31 select the new instruction and cleanup
contracts. Semantic validator 29 invalidates old derived acceptance for this
feature. Owner summary contract 10 uses `LKJSUM15` to retain the new borrowed roles.
Unsupported formats reject clearly; maintained derived assets are rebuilt through
their supported owners. No migration layer is added. Original source/publication
evidence keeps its original identity and obligations.

The [native guide](../guides/native-owned-borrows.md) and
[maintained example](../../examples/owned-borrows/README.md) explain ordinary use.
[Verification](verification.md#scoped-owned-child-read-obligations) owns independent
proof obligations; [status](../status.md) distinguishes source acceptance, tested
final bytes and public availability.
