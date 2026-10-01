# Structural owned products

Graph 19 adds explicit affine products with fixed named typed fields. A product is
a distinct type form, `OwnedProduct`, not an ordinary record or a nominal ownership
annotation. Its ownership does not depend on its instantiated fields. Empty and
data-only products are invalid: at least one direct field must be ByteBuffer,
OwnedI64Cell, another finite owned product, or an exactly scoped Owned parameter.

Other fields must prove closed ordinary first-order data under the existing owned
contract rules. The proof follows nominal members and every actual argument,
including phantom arguments and unused members. It excludes functions, task
functions, streams, secrets, capabilities and open ordinary parameters. Neither
None nor CaptureSafe proves this closed property. No Data constraint is introduced.
Ordinary nominal and structural records and ordinary containers remain unrestricted
and cannot contain products. Generic functions can abstract over one or more Owned
payloads. Product parameters must occur in their exact pure function scope.

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
storage. The operation has no partial-move form or borrowed-field projection.
Field access on a loan is invalid. Ordinary field projections do not apply.

Unchanged native drafts retain accepted bodies. Same-kind literal-only edits traverse
pack fields in authored order and unpack source/body slots, then compare the complete
canonical intent, including the product operand and all binding annotations. Only
that complete equality proof permits preserving expression and binding identities;
other edits follow the existing complete-body replacement and validation contract.

Whole products follow the existing direct-memory consume/borrow parameter suffix
and synchronous borrow/reborrow contracts. Borrowed values cannot escape or unpack.
Owned parameters may instantiate to products; exact monomorphic owned implementations
may select a closed product as Self under the existing first-order witness rules.
There is no implicit witness search or capability grant. Task signatures, indirect
callable signatures, capture, async transfer, general owned-element containers,
nominal owned declarations and function extraction of product scopes remain outside
this increment. A task body can create and dispose of products locally.

## Runtime and boundaries

A separately sealed product token carries the exact invocation origin and closed
product type. It owns one fixed field vector and admits optional whole read loans.
Cloning the raw token yields an inert marker, with no copied children and no transfer
authority. Owner drop releases children even when inert markers survive. Consumption
requires owner mode, live storage, matching origin/type and no outstanding loans.
Construction and decomposition check ordinary child admission and exact memory-child
origin, type and ownership. Raw metadata cannot confer authority.

Storage accounting reserves field vectors, token and control storage before growth;
cancellation is checked before allocation and detachment. Nested product cleanup
uses a bounded iterative traversal. Cleanup never calls user-defined methods, and
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

Current derived execution uses compiler unit 16, bytecode 12 and artifact 23.
Predecessor derived artifacts require rebuilding; frozen source acceptance is not
permission to execute them. Original hostile fixtures remain unchanged. The test-only
fixture owner can produce current controls from their exact retained source and
instruction forms. Authored request 22 and compact request/discovery 26 add explicit
type and operation forms; requests without this extension preserve earlier bytes,
including empty optional setters. Semantic validator 19 invalidates stale proof reuse.

See the [native guide](../guides/native-owned-products.md) and
[campaign](../campaigns/20261001-owned-products.md) for literal programs, actual
verification and remaining work. Public v0.1.60 is unchanged.
