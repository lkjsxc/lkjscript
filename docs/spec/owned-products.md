# Structural owned products

Graph 19 adds explicit affine products with fixed named typed fields. A product is
a distinct type form, `OwnedProduct`, not an ordinary record or a nominal ownership
annotation. Its ownership does not depend on its instantiated fields. Empty and
data-only products are invalid: at least one direct field must be ByteBuffer,
OwnedI64Cell, another finite owned product, or an exactly scoped Owned parameter.

Other fields must prove ordinary first-order data. Development 0.1.71 also permits
exact in-scope [transferable ordinary parameters](transferable-types.md) in metadata.
The proof follows nominal members and every actual argument, including phantom
arguments and unused members. It excludes functions, task functions, streams,
secrets, capabilities and unconstrained open parameters. Neither None nor CaptureSafe
alone proves this property.
Ordinary nominal and structural records and ordinary containers remain unrestricted
and cannot contain products. Generic functions can abstract over one or more Owned
payloads. Product parameters must occur in their exact function scope. Pure helpers may borrow; named tasks may only consume under [same-task transfer](owned-task-transfers.md).

## Meaning and scope

```text
(owned-product (field payload T) (field tag I64))

(pack-owned (type (owned-product (field payload T) (field tag I64)))
  (field tag (i64 128)) (field payload (local bytes)))

(unpack-owned (type (owned-product (field payload T) (field tag I64)))
  (local packet)
  (field payload (binding data (type T)))
  (field tag (binding tag (type I64)))
  (in BODY))
```

The explicit product operand is checked semantic metadata, never an ownership
certificate. Type fields are unique and canonically ordered by name; all existing
field-count and depth bounds apply. Pack fields retain their authored expression
order. Each field appears exactly once and has exactly the annotated field type.
Depth covers the longest structural path, including shared type nodes and closed
generic substitutions. A shorter path to a previously visited type cannot hide a
deeper one. Both preparation derivations check materialized products before execution.
Owned field operands must be exact live owning locals. Ordinary metadata expressions
obey their existing evaluation and copying contracts. A trap stops later fields.

Unpack requires an exact live owning local of the exact annotated product type. It
consumes the parent, binds every field exactly once, and introduces simultaneous,
disjoint lexical identities visible only in its body. Binding annotations must
match the complete product shape. The consumed parent cannot be used again. Fields
left unused drop at lexical exit; nested children transfer without cloning their
storage. The operation has no partial-move form or borrowed owned-child projection.
Unpacking a loan remains invalid.

### Closed ordinary metadata reads (development 0.1.63)

`(field (local packet) (name tag))` reads a closed ordinary field through a short
whole-product read loan. The source must be an exact live owning local or a live
borrowed parameter of the exact product type. A generic product can contain scoped
Owned parameters while its selected metadata remains closed ordinary data. Direct
reads, helper calls and synchronous reborrows leave the original ownership rights
unchanged. A later complete move or unpack remains possible after the read returns.

The result is an ordinary value, not a child borrow. It can outlive consumption or
destruction of the parent under the existing immutable ordinary-value rules. This
does not expose an Owned field, introduce a partial move or give a lifetime-bearing
reference to product storage. Owned fields, missing fields, consumed locals and
non-local product temporaries cannot use this operation. Bind an owned temporary
explicitly before inspecting it. Ordinary record projections retain their behavior.

Accepted source, independent public witnesses and the separate distribution
boundary are recorded in the [metadata continuation](../campaigns/20261001-owned-product-metadata.md).

Unchanged native drafts retain accepted bodies. Same-kind literal-only edits traverse
pack fields in authored order and unpack source/body slots, then compare the complete
canonical intent, including the product operand and all binding annotations. Only
that complete equality proof permits preserving expression and binding identities;
other edits follow the existing complete-body replacement and validation contract.

Whole products follow the existing direct-memory consume/borrow parameter suffix
and synchronous borrow/reborrow contracts. Borrowed values cannot escape or unpack.
Owned parameters may instantiate to products; exact monomorphic owned implementations
may select a closed product as Self under the existing first-order witness rules.
There is no implicit witness search or capability grant. Named tasks may consume
and return products under [same-task transfer](owned-task-transfers.md). Indirect
callable signatures, capture, general owned-element containers, nominal owned declarations
and function extraction of product scopes remain unsupported. The separate
[structured parallel contract](structured-parallel.md) admits closed product inputs
and, in development 0.1.69, returns owned child results in a joined product. This
does not permit asynchronous borrowing or detached transfer.

## Runtime and boundaries

A separately sealed product token carries the exact invocation origin and closed
product type. It owns one fixed field vector and admits optional whole read loans.
Cloning the raw token yields an inert marker, with no copied children and no transfer
authority. Owner drop releases children even when inert markers survive. Consumption
requires owner mode, live storage, matching origin/type and no outstanding loans.
Construction and decomposition check ordinary child admission and exact memory-child
origin, type and ownership. Raw metadata cannot confer authority.

Storage accounting reserves field vectors, token and control storage before growth;
cancellation is checked before allocation and detachment. Metadata reads validate
the exact invocation origin, live read mode, selected ordinary type and output.
They reserve the inline Option/Result/variant box spine before cloning. Shared
immutable backing remains shared; no owned child storage is cloned. Cancellation
checks bracket reservation and copying, and a failed read does not move the owner.
Nested product cleanup uses a bounded iterative traversal. Cleanup never calls user-defined methods, and
does not roll back independent effects. Quota or proof-work exhaustion remains
distinct from semantic invalidity. No bound is increased for this feature.

Raw entry and exit, adapters, persistence, JSON/data codecs, ordinary containers and
retained callable captures reject products. The rejection includes unused arguments,
untaken branches, hidden nominal members and missing type metadata. Compiler units
retain all local annotation roots; strict loading checks canonical source, types,
ownership, package closure and exact instruction meaning independently of execution.

## Encodings and admission

The new disjoint `LKJPRD01` type envelope and appended owner-operation/binding tags
leave ordinary type bytes unchanged. Product-bearing meaning requires Graph 19 even
when a function body only uses an old Local operation. Owner, package-interface and
artifact type closures all enforce the generation boundary. Package interface 12
keeps its existing layout. Old source generations retain their exact readers.

Development 0.1.63 uses compiler unit 17, bytecode 13 and artifact
24 for metadata-read lowering and execution semantics. Graph 19, package interface
12, authored request 22 and compact request/discovery 26 keep their layouts.
Predecessor derived artifacts require rebuilding; frozen source acceptance is not
permission to execute them. Original hostile fixtures remain unchanged. The test-only
fixture owner can produce current controls from their exact retained source and
instruction forms. Authored request 22 and compact request/discovery 26 add explicit
type and operation forms; requests without this extension preserve earlier bytes,
including empty optional setters. Semantic validator 20 and structural-owned-product
feature revision 2 invalidate stale ownership proofs for metadata reads. This
source acceptance does not establish a public binary release.

Development 0.1.64 also admits [owned choices](owned-choices.md) as explicit owned
product children. Both composite forms share bounded mixed-depth validation and
iterative cleanup; ordinary record/Option/Result containment remains rejected.
The current compiler/bytecode/artifact cut is 18/14/25; the historical 0.1.63
encoding paragraph above describes that increment's exact source, not current
permission to execute its derived artifacts.

See the [native guide](../guides/native-owned-products.md) and
[campaign](../campaigns/20261001-owned-products.md) for literal programs, actual
verification and remaining work. These products are public in v0.1.64; its
immutable predecessor v0.1.60 remains unchanged.
