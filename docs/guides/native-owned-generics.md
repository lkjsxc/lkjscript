# Native owned generics and explicit witnesses

The public native declaration surface can author an owned generic library before
choosing a concrete representation. Use `lkjscript capabilities --section change`
for the current grammar and `change draft` for exact editable projections. The
[normative contract](../spec/owned-generics.md) defines supported combinations.
[Parameterized worklists](native-owned-worklists.md) extend this workflow with
ordered Owned contract arguments and structural method results.

The maintained literal [generic library](../../tests/fixtures/owned-witness-library.lkjc)
declares `Storage` with create, update and read methods and implements generic
producer, transformer, borrowed helper and consumer functions. It imports no
standard package and contains no ByteBuffer or OwnedI64Cell type, implementation,
or caller. Its opening contract is:

```lisp
(owned-contract create Storage (visibility public)
  (self Self) (type-parameter create Self (constraint owned))
  (method method_10000000000000000000000000000001 create
    (parameters (I64 unrestricted)) (returns Self))
  (method method_10000000000000000000000000000002 update
    (parameters (I64 unrestricted) (Self consume)) (returns Self))
  (method method_10000000000000000000000000000003 read
    (parameters (Self borrow)) (returns I64)))
```

The generic producer explicitly binds its witness and invokes a method:

```lisp
(function create produce (visibility public)
  (type-parameter create T (constraint owned))
  (implementation-parameter implparam_10000000000000000000000000000001
    ops Storage T)
  (parameter create n (type I64)) (returns T) (effect pure)
  (body (method-call
    parameter@produce@implparam_10000000000000000000000000000001
    Storage method_10000000000000000000000000000001 (local n))))
```

Method and witness parameter IDs are distinct semantic identity domains embedded
in their declaring owners. Preserve them in drafts when editing a contract or
function. A method mapping uses its exact method ID, not a name lookup at runtime.

The [cell implementation](../../tests/fixtures/owned-witness-cell.lkjc) supplies
ordinary graph wrappers around the closed scalar externals. `Scalar` and
`Alternate` have exactly the same Self type and signatures; Alternate's read
wrapper returns 99. The [buffer implementation](../../tests/fixtures/owned-witness-buffer.lkjc)
uses append and length. Neither needs execution grants. A concrete application
selects the implementation explicitly:

```lisp
(binding value (type OwnedI64Cell)
  (implementation-call abstraction::produce
    (types OwnedI64Cell) (implementations concrete@cell::Scalar) (i64 128)))
```

A generic forwarding call supplies
`parameter@CURRENT_FUNCTION@implparam_HEX` instead. That function and parameter
must be the exact current lexical witness, with the same contract and substituted
Self and every ordered additional contract argument. The Storage example has no
additional arguments. Memory arguments remain final exact locals; ownership is
never inferred
from a matching method name or an available implementation.

The [public integration test](../../tests/public_cli/native_owned_witnesses.rs)
contains the complete create/plan/apply/check/export/stage/build/run sequence. It
uses three fresh projects and literal fixtures, substitutes only observed package
locators, checks unchanged drafts, then removes the temporary source projects and
transport packs and runs the exact artifact with empty grants. The common input
sequence 0, 255, 128 yields length 3 for ByteBuffer and scalar 128 for the cell.
Both signed I64 extremes are retained by the cell. The alternate cell witness
returns 99, proving selection survives transport and detached execution.

`inspect owner owned_contract ID` exposes Self, additional parameters and every
method signature; `inspect owner owned_implementation ID` exposes the exact
contract, Self, ordered arguments and method map. Function inspection includes
static parameters, and function-definition
detail includes complete witness applications alongside its body operands.
`change draft --module NAME --output ABSENT_PATH` reconstructs these canonical
contracts; planning an untouched draft is unchanged.

The [recursive fixture](../../tests/fixtures/owned-witness-recursion.lkjc) forwards
both consume and read witnesses and restores an outer selection after a nested
call using another implementation. Ownership failures, missing or wrong witnesses,
callable-kind/effect mismatches, escaping loans and ordinary owned-element
containers reject before publication.
Explicit [structural owned products](native-owned-products.md) compose multiple
Owned payloads with closed metadata and complete consuming decomposition.
ByteBuffer, cell and product tokens cannot be JSON command inputs, persisted values,
callback captures or ordinary returned raw values. Expose ordinary results at
application boundaries.

## Task methods in development 0.1.66

A contract method may declare its task kind and the exact requirements it needs:

```lisp
(method method_80000000000000000000000000000002 finish
  (parameters (Self consume)) (returns I64)
  (effect (task (requirement authority::clock))))
```

Omitting `effect`, or writing `(effect pure)`, preserves the existing pure method.
`(effect (task))` is an empty-row task, not a pure method. The selected graph
function must match the method's kind and row exactly. A task Self argument must
consume; use a separate pure method for synchronous scoped reading. The caller
must independently declare the method's required effects, and deployment must
independently grant the actual operation. Selection alone authorizes nothing.

The literal [library](../../tests/fixtures/owned-task-method-library.lkjc),
[carriers](../../tests/fixtures/owned-task-method-carriers.lkjc) and
[consumer](../../tests/fixtures/owned-task-method-consumer.lkjc) cover both generic
witness forwarding and direct method calls. A scalar preserves the signed input;
a buffer reports length one. Each finishing operation performs one explicit clock
call. The [public test](../../tests/public_cli/native_owned_task_methods.rs) uses
three projects, exact exported dependencies, unchanged drafts, missing-grant
refusal and source-free artifact execution. It also exports a contract whose
requirement belongs to a private component and no public function mentions it.

Owner inspection reports each method's `kind`, `requirements` count and exact
`owned.method-requirement` records. An untouched task-method draft remains unchanged.
Cancellation releases remaining storage but does not undo or replay clock calls.
This is same-invocation composition, not a channel, scheduled child task or promise
of cross-task ownership transfer. Public availability remains owned by [status](../status.md).
