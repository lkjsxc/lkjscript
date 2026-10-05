# Reuse owned implementation schemes across items

A generic implementation relates a storage shape to an item type once. Its
method map explicitly applies generic graph functions. A later package can select
that implementation for a new owned item without supplying storage wrappers.
The [specification](../spec/generic-owned-implementations.md) defines admission;
[status](../status.md) owns availability.

The [example](../../examples/generic-owned-implementations/README.md) has three
packages. First export the generic Worklist and IndexRead algorithms and the
Flat, Chunked and reader schemes. Then author independent element constructors,
observers and consuming finish methods. Finally select exact scheme applications
in a consumer.

```lisp
(owned-implementation create Flat (visibility public)
  (type-parameter create T (constraint owned))
  (contract owned-worklists::Worklist) (self (owned-sequence T)) (types T)
  (method method_88000000000000000000000000000011 std::sequence-empty (types T))
  (method method_88000000000000000000000000000012 std::sequence-length (types T))
  (method method_88000000000000000000000000000013 std::sequence-push (types T))
  (method method_88000000000000000000000000000014 std::sequence-pop (types T)))
```

The implementation's T differs from the contract's Item and each mapped
function's T. Explicit applications connect those scopes. Applying Flat to a
ByteBuffer substitutes its Self with `(owned-sequence ByteBuffer)` and its
Worklist Item with ByteBuffer, then applies each selected standard function to
that same item type.

```lisp
(implementation-call owned-worklists::build
  (types ByteBuffer (owned-sequence ByteBuffer))
  (implementations generic-elements::Buffers
    (implementation generic-storage::Flat (types ByteBuffer)))
  (local inputs))
```

Element and storage witnesses remain separate. A scheme never chooses how to
construct or observe an arbitrary item, and its selection grants no task effects
or adapter authority. Generic helpers can use `(implementation Flat (types T))`
with eligible local type parameters, while forwarding existing function witnesses
through their exact `parameter@` selectors.

The reader schemes map a generic `at` function returning T from the borrowed
storage parameter. Use a method result inside `borrow-call`; after the scope ends,
the storage can again be consumed. Scheme substitution preserves that source
relationship. The example also selects ReverseReader for the same flat storage:
it visits items from the last index first, so an equal greatest key chooses a
different original element. Both selections leave storage and drainage unchanged.

The independent product item contains a cell under two owned products and ordinary
tag/stamp fields. Its observer and finish add the metadata to the cell value.
The shared flat/chunked implementations consequently return product observations
offset by 12, without changing any storage method. This makes new-item reuse and
correct complete consumption externally visible.

Follow the example's exact dependency staging and proposal order. Check and export
the scheme library before creating the element project. Draft accepted modules
through `change draft`, review edits through `change plan`, apply the unchanged
input with its exact token and check the complete result. Build a standalone
artifact, then repeat the same independently expected outputs after removing
authoring projects and transports.

Scheme methods may select pure functions or named closed-row tasks when their
contract agrees. The example's SequenceTask consumes a sequence and reports its
length; joined generic children exercise distinct cell/buffer applications.
Borrowed results remain pure synchronous scopes, and transferability must still
be justified independently.

## Compose adapters with explicit prerequisites

The [composition example](../../examples/composable-owned-implementations/README.md)
exports reusable adapters before concrete element implementations exist.
`Maximum<T,W>` requires `Element<T>` and `IndexRead<W,T>`, then maps its borrowed
`best` method directly to the existing generic `select-max`. Each prerequisite
uses the same `implementation-parameter` form as a function, with the implementation
declaration as its exact scope. A method map supplies an ordered `implementations`
clause after its optional `types` clause.

```lisp
(method method_8e000000000000000000000000000001
  owned-read-results::select-max (types T W)
  (implementations parameter@Maximum@implparam_8e000000000000000000000000000001
    parameter@Maximum@implparam_8e000000000000000000000000000002))
```

`DelegatingReader<T,W>` takes an `IndexRead<W,T>` prerequisite and forwards both
length and source-tied reads. Applications nest their complete selected witnesses:

```lisp
(implementation composable-adapters::Maximum
  (types OwnedI64Cell (owned-sequence OwnedI64Cell))
  (implementations generic-elements::CellKeys
    (implementation composable-adapters::DelegatingReader
      (types OwnedI64Cell (owned-sequence OwnedI64Cell))
      (implementations
        (implementation generic-storage::ReverseReader (types OwnedI64Cell))))))
```

Changing the leaf from `ReverseReader` to `FlatReader` changes first-tie selection
while preserving the outer types and declarations. Borrowed results stay tied to
the original storage across every mapping; after their scopes end, that same owner
can be drained and reused. The example also maps a consuming empty-row task with
explicit element/storage prerequisites and executes it in joined children.

Method mappings forward a direct same-scheme prerequisite or use a parameter-free
concrete application tree. Constructing a nested witness from function prerequisites
is admitted only outside conservative potential recursive components. The
[specification](../spec/generic-owned-implementations.md#finite-preparation-and-independent-admission)
describes that restriction and distinguishes it from actual type expansion and
resource exhaustion. There is no inferred witness selection or added execution
authority.

Use `inspect owner owned-implementation ID` on accepted local meaning, or
`package dependency inspect owner owned-implementation ID --package-revision REVISION`
on an exact imported interface. Ordered prerequisite contracts, mapped operands
and nested operand paths are public observations. Drafting and unchanged re-entry
retain every exact scope, prerequisite identity and application. The example README
owns authoring order, independent expected outputs and detached execution.
