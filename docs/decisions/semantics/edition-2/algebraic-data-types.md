# Edition 2: Algebraic Data Types

[Authority](../edition-2.md)

## Purpose

Define nominal immutable algebraic data types, stable identities, and their
exact Edition 2 line projection.

## Status

**Accepted Target, not Current.** No generic enum machinery is implemented by
this contract.

## Declaration Projection

`enum` is the only declaration name; aliases are forbidden. An enum has a name,
an optional existing `forall/` parameter list, and `variants/`. Every variant
has a name and an explicit `fields/`; every field is a named
`variant-field/` with `name/` then `type/`. Zero fields are allowed, zero user
variants are rejected, and fields are always written in declaration order.

```text
enum/
name/
Result
/name
forall/
T
E
/forall
variants/
variant/
name/
Ok
/name
fields/
variant-field/
name/
value
/name
type/
T
/type
/variant-field
/fields
/variant
/variants
/enum
```

## Construction Projection

Every constructor names the fully instantiated enum type, variant, and all
named fields in declaration order:

```text
variant-value/
type/
Result/
I64
Utf8Error
/Result
/type
variant/
Ok
/variant
fields/
variant-field/
name/
value
/name
42
/variant-field
/fields
/variant-value
```

Missing, extra, duplicate, unknown, out-of-order, or wrongly typed fields are
errors. Nullary variants still contain empty `fields/` and `/fields` markers.

## Identity And Type Rules

Opaque stable `EnumId`, `VariantId`, and `VariantFieldId` derive from canonical
semantic identities; any collision is rejected. Backend tag plans map stable
`VariantId`s to physical tags. Variant source order is metadata and evaluation
order, never backend identity. Same-shaped enums are nominally distinct.
Generic parameters are invariant.

Ownership-bearing fields, directly or nested, are rejected in the initial
slice. Recursive immutable ADTs must be finite values, physically indirect in
the initial representation, exactly traced, and accepted only through bounded
recursion validation.

## Prelude Migration

`Option` and `Result` are compiler-recognized prelude enum identities, never
recognized by source names. Product remains its distinct nominal product
concept; Product does not become an enum. Dedicated Option/Result semantics are
removed only after generic ADT replacement passes complete differential
coverage. No compatibility machinery remains afterward.
