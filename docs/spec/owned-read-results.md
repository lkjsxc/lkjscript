# Source-tied borrowed results

Status: normative.

A pure named graph function can return a read-only owned view tied to one exact
borrowed input. A pure owned-contract method declares the same relationship by
parameter position. The type describes the viewed value; the result relationship
confers read rights and preserves custody independently of physical addresses.

## Result contract

```lisp
(function create at (visibility public) (effect pure)
  (parameter create index (type I64))
  (parameter create storage (type (owned-sequence OwnedI64Cell)) (use borrow))
  (returns OwnedI64Cell (borrow-from storage))
  (body (borrow-owned-item (type (owned-sequence OwnedI64Cell)) (local storage)
    (index (local index)) (binding view (type OwnedI64Cell)) (in (local view)))))
```

`borrow-from` resolves the function parameter's lexical name to its semantic
identity. Its parameter must have `borrow` use and an owned type; the result must
also have an owned type. The relationship has semantic `ReadFrom(source)` mode.
Every successful path returns either that exact input or one of its owned
descendants. Branches may select different descendants of the same input.
Returning a new local owner, a descendant of another input, or an ordinary value
cannot satisfy the relationship. Equal types and aliased input allocations do
not merge parameter provenance.

An owned-contract method uses a zero-based position in its complete ordered
parameter vector, including ordinary parameters:

```lisp
(method method_8a000000000000000000000000000002 at
  (parameters (I64 unrestricted) (Self borrow))
  (returns Item (borrow-from 1)) (effect pure))
```

Exact implementation matching includes result type, result mode and source
position after substitution. A mapped graph function's source identity must
occupy the declared position. Unused methods receive the same admission.
Task functions, externals and task methods cannot declare this result mode.

Without `borrow-from`, the existing result type determines ordinary value or
owning-result semantics. Public inspection calls that mode `value`; the explicit
borrowed mode is `read-from`.

## Lexical call and forwarding

```lisp
(borrow-call
  (call at (i64 0) (local storage))
  (binding view (type OwnedI64Cell))
  (in BODY))
```

The invocation is one existing `call`, `implementation-call` or `method-call`,
with its existing exact type and witness operands. Its selected source argument
must be an exactly typed live owning or borrowed local. Owned temporaries first
need explicit owning bindings. Arguments retain their authored evaluation order.
An ordinary call cannot expose a borrowed result, and `borrow-call` cannot confer
a borrowed mode on an ordinary or owning result.

The new binding has `OwnedBorrow` rights and lexical identity. Its exact type is
required. Its scope protects the source and all ancestor custodians, even when
the body does not read the binding. Synchronous parent, sibling and nested reads
remain valid. Neither the view nor any protected custodian may be consumed or
mutated. An unrelated owner follows its usual affine rules.

The expression returns its body's ordinary or unrelated owning result after the
loan ends. A view can leave the scope only when returned through an enclosing
function's matching `ReadFrom` result. This rule also allows a borrowed result to
pass through multiple pure helper functions and through nested child scopes.
Returning a view through an ordinary owning signature remains invalid.

Borrowed results cannot cross entrypoints, first-class callable values, captures,
containers, serialization, raw-value ingress, retained values or consuming task
transfer. An adopted lexical view may be passed as a synchronous borrowed graph-task
input, or as a Shareable borrowed input to a joined child group. These read calls
retain the result packet's source and ancestor guards; tasks cannot return a loan.
Mutable views, multiple alternative source roots, nullable borrowed results and
general lifetime inference are outside this contract.

## Runtime custody and cleanup

Before callee cleanup, the selected read token and required ancestor guards move
into one sealed result packet. Provenance binds the exact selected argument and
invocation context. Equal allocation identity, equal types or shared invocation
origin cannot authorize a different parameter's result.

Packet and guard storage is reserved before custody detaches. Transfer and
adoption maintain exactly one cleanup owner, including failure between extraction
and caller adoption. The caller adopts the packet into its lexical scope.
Unrelated callee temporaries receive ordinary cleanup. Child read tokens release
before ancestor guards; a guard keeps its custodian alive until its descendants
finish. Storage locks do not remain held while user code executes.

Normal completion, trap, cancellation, finite storage refusal and interrupted
result adoption discharge all active loans and owners in this order. Cleanup
neither invokes user methods nor replays effects. A subsequent valid invocation
must remain healthy. Tail-call activation replacement into or out of a
borrowed-result function is disabled in this first contract; ordinary functions
retain their existing eligibility rules.

## Independent boundaries and witness

Canonical meaning, drafts, exact package interfaces, transport, compiler metadata
and artifact admission retain the relationship. Complete validation includes
untaken syntax and unused methods. The memory oracle, reference evaluator and
artifact loader independently check provenance, result mode and cleanup guards;
producer agreement or consistently recomputed hashes are insufficient.

The [native witness](../../examples/owned-read-results/README.md) exports a generic
`IndexRead<Item>` and `select-max` before its concrete readers exist. Flat and
chunk32 storage support both OwnedI64Cell and ByteBuffer. `select-max` returns
the first maximal element under its exact Element observer. An empty call traps
through `at(0)`; its consumer checks length and returns an ordinary empty result.
The consumer observes each view, exits the scope, drains the original storage in
unchanged LIFO order and reuses its empty owner. Different observers make tie
selection externally distinguishable without changing storage.

[Verification](verification.md#source-tied-borrowed-result-obligations) owns the
required proof, and [status](../status.md) owns accepted source and binary
availability. Encoding generations advance together at their maintained owners;
unsupported predecessors reject clearly and maintained bundles are rebuilt
through supported product operations. No migration layer is required.
