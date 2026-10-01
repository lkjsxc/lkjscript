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
released at exit. A borrowed product cannot be unpacked or expose an owned child.

## Inspecting metadata without dismantling the owner

Development 0.1.63 adds metadata reads. The [continuation](../campaigns/20261001-owned-product-metadata.md)
records accepted source, copied-product public workflows and the separate binary
publication boundary. This feature is not in public v0.1.60.

```text
(function create tag (visibility public) (effect pure)
  (type-parameter create T (constraint owned))
  (parameter create packet
    (type (owned-product (field payload T) (field tag I64))) (use borrow))
  (returns I64)
  (body (field (local packet) (name tag))))
```

The result is an ordinary I64. Reading it preserves the packet; callers can read
again and then move or unpack the packet. The complete
[producer literal](../../tests/fixtures/owned-products-read.lkjc) also forwards the
borrow through a second generic helper. The separate
[consumer literal](../../tests/fixtures/owned-products-read-consumer.lkjc) exercises
an imported helper and later consumes the original payload. The added public test
specifies canonical draft re-entry, exact package transport and detached execution;
it passes through the copied optimized development executable.

`(name tag)` is a structural selector. A bare member is a nominal record selector.
Selecting `payload` here is rejected because it owns memory. Reading metadata from
an owned temporary requires first binding that temporary to an explicit local.
These reads are not general field borrowing or lifetime-polymorphic references.

Metadata must be closed first-order data, such as I64, Bytes, or a closed ordinary
nominal record. An open unconstrained or CaptureSafe parameter is insufficient.
Every product needs at least one owned field. General lists/maps of products,
ordinary record escape, task memory signatures, retained captures and persistence
remain unsupported. Products introduce no grants or user-defined destructors.

Exact product Self implementations use the existing explicit witness syntax; see
the [selection literal](../../tests/fixtures/owned-products-witness.lkjc). Semantics
and compatibility are specified in [owned products](../spec/owned-products.md).
