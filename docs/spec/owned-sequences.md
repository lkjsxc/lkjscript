# Runtime-sized owned sequences

`(owned-sequence T)` owns runtime-sized ordered storage containing values of exact
type `T`. An element may be independently owned or ordinary first-order data.
Container ownership is intrinsic: an empty sequence or a sequence of I64 values
remains affine. This complements ordinary immutable lists, fixed
[owned products](owned-products.md) and [owned choices](owned-choices.md).

Validate the complete element type using the existing Owned or ordinary-data
rules. A declared `T: Owned`, an admitted concrete carrier, or a finite owned
product, choice or sequence admits owned elements. Ordinary generic libraries use
the existing `T: Transferable` contract. Hidden resources, secrets, callables and
inadmissible phantom arguments do not become sequence data.

[Status](../status.md) records exact source acceptance and public executable
availability separately from this semantic contract. The
[owned example](../../examples/owned-sequences/README.md),
[owned guide](../guides/native-owned-sequences.md) and
[ordinary-data guide](../guides/native-data-sequences.md) provide literal product
inputs.

## Operations and custody

All graph expressions carry an explicit exact sequence type. A consuming or
borrowed source must be an exact live local; bind owned temporaries before using
them. Nonlexical operations also have ordinary generic library wrappers. Existing
`sequence-*` wrappers retain `T: Owned`. The `data-sequence-empty`,
`data-sequence-length`, `data-sequence-push`, `data-sequence-pop`,
`data-sequence-get`, `data-sequence-replace` and `data-sequence-discard` wrappers use
`T: Transferable` for ordinary
data. The standard library's discard wrappers consume a sequence using automatic
cleanup; neither adds a graph expression or runtime intrinsic.

| Expression | Contract |
| --- | --- |
| `(sequence-empty (type SEQUENCE))` | Create an empty owner. |
| `(sequence-length (type SEQUENCE) (local values))` | Borrow a sequence and return its length as I64. |
| `(sequence-push (type SEQUENCE) VALUE (local values))` | Evaluate the element, consume the sequence, append the element and return the same sequence owner. Owned elements require exact owning locals; ordinary elements are unrestricted values. |
| `(sequence-pop (type SEQUENCE) (local values))` | Consume the sequence and return the choice described below. |
| `(sequence-get (type SEQUENCE) (local values) (index INDEX))` | Return an ordinary immutable element under a short read loan. |
| `(sequence-replace (type SEQUENCE) (index INDEX) VALUE (local values))` | Consume the sequence and return an owned product containing the sequence and displaced element. |
| `borrow-owned-item` | Borrow one owned element in a lexical scope while retaining the sequence and ancestor custody. Ordinary elements use get. |

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
the sequence allocation and reusable capacity. An owned popped element can be
consumed, stored in an admitted owned composite or pushed back. Child handles move
between custodians without cloning their payloads. An ordinary popped element is
an unrestricted value inside the affine result envelope.

Push evaluates the element operand before the sequence operand. The sequence and
any owned element must possess exact owning rights. An active loan of the sequence
or an ancestor prevents its consumption. Length accepts owning and borrowed locals
without changing order, capacity or custody.

## Ordinary indexed reads and replacement

Get requires an admitted ordinary element type. Evaluate `INDEX` exactly once,
then recheck the source and take a short read loan. The returned immutable value
has no source loan and remains valid after later replacement, removal, disposal or
task transfer of the sequence. Nested immutable backing may remain shared; there
is no promise that all ordinary values are copied byte for byte. Reserve actual
clone storage when an ordinary value needs it.

Replace supports both ordinary and owned elements and returns exactly:

```text
(owned-product
  (field rest (owned-sequence T))
  (field value T))
```

Evaluate index, replacement value and source in that order. An owned replacement
must be an exact owning local. Check bounds and reserve result storage before the
swap. The sequence retains order, allocation and capacity; the result preserves
the displaced element's custody. Unpack the result to recover the sequence and its
previous value.

For get and replace, negative or out-of-range indices trap without wrapping,
clamping or an implicit optional result. Allocation failure and cancellation use
existing consuming-operation cleanup. They expose no successful partial result
and establish no retry permission.

## Indexed lexical reads

```text
(borrow-owned-item
  (type (owned-sequence T))
  (local values)
  (index INDEX)
  (binding view (type T))
  (in BODY))
```

The element type must satisfy Owned. The source annotation must equal the local's
sequence type, the view annotation must equal its element type and `INDEX` must
return I64. Evaluate the index exactly once before acquiring the lexical loan.
Negative and out-of-range indices trap; there is no wrapping, clamping or implicit
empty result. Index evaluation follows the containing callable's ordinary effect
and ownership rules. It does not receive the body's protected-source rights.

The view has a distinct borrowed lexical identity visible only in `BODY`. The
source and all ancestor custodians remain read-loaned throughout the body,
including a body that never reads the view. The expression returns the body's
result. Scope exit ends the child loan before its ancestor guards, so the original
sequence can subsequently be read, pushed, popped, replaced or consumed.

The rights match [other owned child reads](owned-borrows.md). A view may call exact
pure helpers or synchronous borrowed graph tasks and implementation methods,
reborrow and enter another child scope. Nested sequence, product and choice
inspection retains the complete provenance chain. Reading an ancestor's length
or another sibling is legal; no internal storage lock remains held across user
code.

Neither a view nor a protected source or ancestor may be consumed through an
alias. A view cannot become an owner, an unrestricted argument, an ordinary
container member, a capture or a stored reference. A matching
[source-tied pure result](owned-read-results.md) can return the selected view with
its complete source and ancestor custody. Synchronous borrowed task inputs
require only Owned; a read input spanning joined parallel child invocations
additionally requires Shareable. Tasks cannot return borrowed results. The body
may consume unrelated owners, return an unrelated new owner and perform effects
authorized by its task. A scope confers no additional effect or task authority.

Active indexed-read scopes retain their activation through calls. Tail-call
replacement cannot discard a scope guard merely because its source is borrowed
and the frame itself owns no storage.

## Generic, transport and boundary admission

Validate element eligibility before constructing even an empty sequence. Complete
relevant closure admission includes unused substitutions, annotations, methods
and untaken syntax. An unconstrained parameter cannot acquire element eligibility
through an annotation. Exact implementation witnesses select element creation,
observation and finishing behavior; sequence operations perform no implicit
implementation search and no user-defined destruction.

Substitution preserves exact element types, affine mode, index/value/source
evaluation order and lexical loan provenance across package interfaces, canonical
requests, compiled metadata and instructions. Production and reference validators
admit complete candidates independently. Integrity hashes cannot authorize an
inadmissible element, erased guard, counterfeit view or changed source. An
entirely ordinary element closure never makes the container unrestricted.

Structured task transfer and sharing require the corresponding complete closed
sequence and element proofs. Symbolic `T: Owned` alone proves neither permission.
Joined parallel results may contain transferable sequences inside owned products.
Transferring the container transfers every owned child under exact invocation-origin
checks, including children added by a worker task.

Raw ingress, adapter results, retained values, JSON/data codecs, persistence and
callable capture cannot manufacture or export sequence owners or indexed views.
Ordinary results can report observations and consuming results. Sealed inert
clones remain identity markers without read or ownership authority.

## Growth, failure and cleanup

Reserve storage before modeled creation and geometric growth. Pop reserves its
result envelopes before detaching an element; replacement reserves its result
before swapping. Existing item, storage, type-depth, preparation-work and
cancellation policies remain authoritative. Reservation refusal is an execution
resource failure; it does not make otherwise valid meaning invalid.

An ordinary read's checked provenance binds the exact admitted sequence, element
type and program. Raw mutation invalidates admission. Checked insertion and
replacement may restore it only from a valid prior sequence and an independently
admitted replacement; untouched elements need no repeated prefix scan. Raw ingress
and retained-value admission remain complete and independent.

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
projections expose the type and all operations. Unchanged draft re-entry preserves
accepted identities. Supported literal edits preserve source, index, borrowed
binding, element type and surrounding operation identities. Function extraction
containing an indexed lexical read rejects until extraction can preserve its
complete borrowing contract.

Ordinary-element sequence closures and the new get/replace operations require
Graph 31, including unused signatures and annotations. Owned-element sequence
meanings in Graph 25–30 retain their existing semantics. The sequence type envelope
stays at version 1 because its shape is unchanged; the graph generation gates its
expanded element eligibility. Current exports use interface 19, compiler 31,
bytecode 26 and artifact 38. Maintained derived assets are rebuilt through their
supported owners. Old derived formats reject clearly; no experimental-format
migration layer is introduced.

The public `native_owned_sequence_` acceptance family authors and checks a generic
library before concrete carriers, then stages exact transports into two additional
packages. Independent ByteBuffer and cell implementations, an alternate witness
for the same cell Self, runtime inputs, nested reads, LIFO draining, empty reuse,
authorized task indices and parallel transfer remain literal witnesses. Execution
repeats after deleting authoring projects and transports, with independent complete
results and joined cleanup. Public rejection and bounds cases complement
lower-level resource-failure and rehashed-artifact checks.

The exact `native_data_sequences` family additionally checks a generic ordinary
library exported before its consumers, stable scalar and nested immutable reads,
owned displacement, complete type admission, active loans, bounds and joined
resource-failure cleanup. Its source-deleted executions and independent finite
model must agree on complete results. The native dependency planner uses ordinary
sequences for remaining counts, levels, visited flags and readiness indices while
retaining its independent checker and established output contract. Passing source
acceptance and publication remain distinct obligations.
