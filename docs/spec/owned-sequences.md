# Runtime-sized owned sequences

`(owned-sequence T)` owns a runtime-sized ordered collection of independently owned
values of exact type `T`. It complements ordinary lists, fixed
[owned products](owned-products.md) and [owned choices](owned-choices.md).
The element type must satisfy `Owned`, including a declared `T: Owned`, an admitted
concrete carrier, or a finite owned product, choice or sequence. The sequence
remains affine when empty. An ordinary element such as I64 belongs in an ordinary
collection.

[Status](../status.md) records exact source acceptance and public executable
availability separately from this semantic contract.
The [maintained example](../../examples/owned-sequences/README.md) and
[native guide](../guides/native-owned-sequences.md) provide literal product inputs.

## Operations and custody

All five graph expressions carry an explicit exact sequence type. A consuming or
borrowed source must be an exact live local; bind owned temporaries before using
them. Nonlexical operations also have ordinary generic library wrappers. The
standard library's `sequence-discard` consumes a sequence using ordinary automatic
cleanup; it adds no graph expression or runtime intrinsic.

| Expression | Contract |
| --- | --- |
| `(sequence-empty (type SEQUENCE))` | Create an empty owner. |
| `(sequence-length (type SEQUENCE) (local values))` | Borrow a sequence and return its length as I64. |
| `(sequence-push (type SEQUENCE) VALUE (local values))` | Consume an exact owning element local, then the sequence, append the element and return the same sequence owner. |
| `(sequence-pop (type SEQUENCE) (local values))` | Consume the sequence and return the choice described below. |
| `borrow-owned-item` | Borrow one element in a lexical scope while retaining the sequence and ancestor custody. |

For `SEQUENCE = (owned-sequence T)`, pop returns exactly:

```text
(owned-choice
  (case empty (owned-sequence T))
  (case item (owned-product
    (field rest (owned-sequence T))
    (field value T))))
```

An empty pop returns the original empty sequence. A nonempty pop detaches the last
element and returns the remaining sequence alongside it. Both outcomes preserve
the sequence allocation and its reusable capacity. A popped element is an owner
that can be consumed, stored in an admitted owned composite or pushed back. Child
handles move between custodians without cloning their payloads.

Push evaluates the element operand before the sequence operand. Both must possess
exact owning rights. An active loan of the sequence or an ancestor prevents its
consumption. Length accepts owning and borrowed locals without changing order,
capacity or custody.

## Indexed lexical reads

```text
(borrow-owned-item
  (type (owned-sequence T))
  (local values)
  (index INDEX)
  (binding view (type T))
  (in BODY))
```

The source annotation must equal the local's sequence type, the view annotation
must equal its element type and `INDEX` must return I64. Evaluate the index exactly
once before acquiring the lexical loan. Negative and out-of-range indices trap;
there is no wrapping, clamping or implicit empty result. Index evaluation follows
the containing callable's ordinary effect and ownership rules. It does not itself
receive the body's protected-source rights.

The view has a distinct borrowed lexical identity visible only in `BODY`. The
source and all ancestor custodians remain read-loaned throughout the body,
including a body that never reads the view. The expression returns the body's
result. Scope exit ends the child loan before its ancestor guards, so the original
sequence can subsequently be read, pushed, popped or consumed.

The rights match [other owned child reads](owned-borrows.md). A view may call exact
pure borrowed helpers and implementation methods, reborrow synchronously and enter
another child scope. Nested sequence, product and choice inspection retains the
complete provenance chain. Reading an ancestor's length or another sibling is
legal; no internal storage lock remains held across user code.

Neither a view nor a protected source or ancestor may be consumed through an
alias. A view cannot become an owner, an unrestricted argument, an ordinary
container member, a capture, a stored reference, or a task operand. A matching
[source-tied pure result](owned-read-results.md) can return the selected view with
its complete source and ancestor custody.
Borrowed task parameters remain unsupported. The body may consume unrelated owners,
return an unrelated new owner and perform effects authorized by its task. A scope
confers no additional effect or task authority.

Active indexed-read scopes retain their activation through calls. Tail-call
replacement cannot discard a scope guard merely because its source is borrowed
and the frame itself owns no storage.

## Generic, transport and boundary admission

Validate sequence element eligibility before constructing even an empty sequence.
Complete relevant closure admission includes unused substitutions, annotations,
methods and untaken syntax. An unconstrained type parameter cannot become an owned
element through an annotation. Exact implementation witnesses continue to select
element creation, observation and finishing behavior; sequence operations perform
no implicit implementation search and no user-defined destruction.

Substitution preserves exact element types, affine mode, source/index evaluation
order and lexical loan provenance across package interfaces, canonical requests,
compiled metadata and instructions. Production and reference validators admit
complete candidates independently. Integrity hashes cannot authorize an ordinary
element, erased guard, counterfeit view or changed source.

Structured task transfer is permitted only when the complete closed sequence and
its elements satisfy the existing transfer contract. Symbolic `T: Owned` alone
does not prove transferability. Joined parallel results may contain transferable
sequences inside owned products. Transferring the container transfers every child
under exact invocation-origin checks, including children added by a worker task.

Raw ingress, adapter results, retained values, JSON/data codecs, persistence and
callable capture cannot manufacture or export sequence owners or indexed views.
Ordinary results can report observations and consuming results. Sealed inert
clones remain identity markers without read or ownership authority.

## Growth, failure and cleanup

Reserve storage before modeled creation and geometric growth. Pop reserves its
result envelopes before detaching an element. Existing item, storage, type-depth,
preparation-work and cancellation policies remain authoritative. Reservation
refusal is an execution resource failure; it does not make otherwise valid meaning
invalid.

Finite structural nesting is intentional. Runtime length is dynamic, while element
type depth remains bounded. Cleanup traverses sequence breadth iteratively without
allocating a new teardown worklist or calling user methods. Every exit retains
custody until children are released: ordinary returns, traps, cancellation, growth
refusal, partially constructed results and interrupted task adoption. Scope guards
finish innermost-first before their custodians. A later valid invocation must remain
healthy after a failed one.

Completed effects remain completed after a later failure. Cleanup, cancellation or
missing output establishes neither rollback nor safe retry.

## Native consumers and verification

Discovery, native structural requests, canonical drafts and function-definition
projections expose the type and all five operations. Unchanged draft re-entry
preserves accepted identities. Supported literal edits preserve source, index,
borrowed binding, element type and surrounding operation identities. Function
extraction containing an indexed read rejects until extraction can preserve its
complete borrowing contract.

Sequence-bearing graph, interface, source, compiler and artifact encodings use the
coordinated current format cut. Maintained derived assets are rebuilt through their
supported owners. Old derived formats reject clearly; no experimental-format
migration layer is introduced.

The public `native_owned_sequence_` acceptance family authors and checks a generic
library before concrete carriers, then stages exact transports into two additional
packages. Its independent ByteBuffer and cell implementations, alternate witness
for the same cell Self, runtime inputs, nested reads, LIFO draining, empty reuse,
authorized task index and parallel transfer are retained literal witnesses. It
repeats execution after deleting the authoring projects and transports, checks
independent complete results and verifies joined cleanup. Public rejection and
bounds cases complement lower-level resource-failure and rehashed-artifact checks.
Passing acceptance and publication are distinct obligations.
