# Native structural owned products

Use an explicit owned product to move multiple memory owners together with closed
ordinary metadata. An ordinary record continues to reject these owners.

The maintained [producer](../../tests/fixtures/owned-products-independent.lkjc)
defines `pair<Left:Owned,Right:Owned>` and complete consuming `take-left`. Its accepted
meaning needs no concrete buffer or cell inventory. The
[transformer](../../tests/fixtures/owned-products-transformer.lkjc) retags a product,
forwards it recursively and borrows it opaquely. The
[consumer](../../tests/fixtures/owned-products-consumer.lkjc) combines ByteBuffer and
OwnedI64Cell, nests products, extracts ordinary results and drops unused children.

Each literal is a declarations request body. Start a minimal project, obtain its
base with `status`, prepend `request base=REVISION`, then use the ordinary
`change plan` and `change apply` operations. Between packages, export through
`package current export --kind transport`, stage the exact transport, add the exact
dependency, and bind native `use` aliases to the observed package and revision.
The [public CLI test](../../tests/public_cli/native_owned_products.rs) retains the
literal workflow, unchanged drafts, an identity-preserving edit and detached command
execution with empty grants.

```text
(function create wrap (visibility public) (effect pure)
  (type-parameter create T (constraint owned))
  (parameter create tag (type I64))
  (parameter create payload (type T) (use consume))
  (returns (owned-product (field payload T) (field tag I64)))
  (body (pack-owned (type (owned-product (field payload T) (field tag I64)))
    (field tag (local tag)) (field payload (local payload)))))
```

Pack expressions evaluate fields in authored order, even though the type sorts
fields by name. Owned operands are live locals. To destructure, give the exact type,
source local, all fields and explicitly typed bindings, followed by `(in BODY)`.
The parent is consumed; the new bindings exist only inside BODY. Unused owners are
released at exit. A borrowed product has no field projection or unpack operation.

Metadata must be closed first-order data, such as I64, Bytes, or a closed ordinary
nominal record. An open unconstrained or CaptureSafe parameter is insufficient.
Every product needs at least one owned field. General lists/maps of products,
ordinary record escape, task memory signatures, retained captures and persistence
remain unsupported. Products introduce no grants or user-defined destructors.

Exact product Self implementations use the existing explicit witness syntax; see
the [selection literal](../../tests/fixtures/owned-products-witness.lkjc). Semantics
and compatibility are specified in [owned products](../spec/owned-products.md).
