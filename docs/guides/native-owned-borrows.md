# Scoped reads of owned children

Use a lexical read scope to inspect an owned child without dismantling its
container. Once the body completes, the original product or choice can be read
again and then consumed. The view has read rights only within that body.

The maintained [example](../../examples/owned-borrows/README.md) composes a
[generic library](../../examples/owned-borrows/library.lkjc), independently authored
[carriers](../../examples/owned-borrows/carriers.lkjc), and an
[application](../../examples/owned-borrows/application.lkjc). The library is checked
and exported before any concrete implementation exists. The application combines
scalar cells, byte buffers, nested products, mixed choices and sibling reads through
exact package dependencies.

## Read one product field

With the example's carrier module imported, and `packet` a live local of the exact
annotated type, this expression reads its payload:

```text
(borrow-owned-field
  (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
  (local packet)
  (field payload (binding view (type OwnedI64Cell)))
  (in (record structural
    (field first (call owned-borrow-carriers::cell-read (local view)))
    (field second (call owned-borrow-carriers::cell-read (local view)))
    (field tag (field (local packet) (name tag)))))
```

The selected field must own memory. Ordinary metadata continues to use `field`.
`view` exists only inside `(in ...)`, and `packet` remains protected while that body
runs. Both owning locals and borrowed parameters may be inspected. Bind an owned
temporary to an explicitly typed local before inspecting it.

The generic library uses the same scope with a `T: Owned` payload. Its `Storage`
contract declares a pure borrowed `read` method. `read-packet` calls that method
and forwards the exact implementation parameter to `read-helper`; neither helper
needs knowledge of the concrete carrier.

```text
(implementation-call owned-borrows::read-packet (types OwnedI64Cell)
  (implementations concrete@owned-borrow-carriers::Scalar) (local packet))
```

Selecting `Alternate` instead produces the alternate read value, `99`, for the
same scalar Self. The stored scalar remains unchanged and a later consuming
operation returns its original value. Witness selection confers no capability
authority and performs no implicit global implementation search.

## Inspect a choice without consuming it

```text
(match-borrowed-owned
  (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
  (local outcome)
  (case accepted (binding value (type I64)) (in (local value)))
  (case rejected (binding view (type OwnedI64Cell))
    (in (call owned-borrow-carriers::cell-read (local view)))))
```

Every declared case needs exactly one arm with the exact payload annotation, and
all bodies return the same type. Only the selected arm executes. An ordinary
payload is an ordinary copied value; an owned payload is a scoped read view. The
whole choice remains guarded throughout either arm. Afterward, `match-owned` can
consume the unchanged choice and recover or finish its original payload.

Nested scopes can inspect a product inside a product or choice. The example also
reads the parent and another sibling while a child view is active. Ancestor custody
is retained until the nested reads finish, without holding a storage lock across
the body.

## Publish and run through the product

Each `.lkjc` file is a literal declarations request body. Create minimal projects,
read each current `status`, prepend `request base=REVISION`, then use ordinary
`change plan` and `change apply` with the reviewed token. Export the generic package
first through `package current export --kind transport`; stage exact transports,
add exact dependencies and bind aliases to the observed packages and revisions.
The [library guide](native-library.md) explains those operations.

Use canonical `change draft` output for unchanged re-entry and supported reviewed
edits. Build the application artifact with the public executable. The example's
command target is `owned-borrows`; its deployment grants no effects and accepts
an I64 and a Bool. Its complete result reports repeated reads, explicit alternate
selection, nested/sibling observations and later consuming results. The frozen
buffer is the single octet `73`, encoded as `{"$bytes":"SQ=="}`.

The maintained public acceptance workflow removes source projects and transports
before executing the artifact. Its literal requests and full expected results are
verification inputs; the guide alone is not evidence of a passing run.

## Rights and boundaries

A view may call borrowed pure helpers, reborrow synchronously and enter another
child scope. It cannot be consumed, returned, stored, captured, passed unrestricted
or transferred to a task. The source and its ancestors also cannot be consumed
until the scope ends. Borrowed task parameters remain unsupported.

A body may return an unrelated owner or perform effects authorized by its task.
Cancellation, traps and allocation refusal release child loans before ancestor
guards and remaining owners; they do not undo completed effects. Active scopes
retain their activation through calls, including when the source is itself borrowed.
Function extraction containing these scopes currently rejects before publication.

See the [specification](../spec/owned-borrows.md) for admission and cleanup semantics
and [current status](../status.md) for tested source and executable availability.
