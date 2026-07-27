# the canonical language: Algebraic Data Types

[Authority](semantic-core.md)

## Purpose

Define nominal immutable algebraic data types, stable identities, and their
exact the canonical language line projection.

## Status

<!-- LKJ-STATUS id=enum-declarations status=current -->

**Current for declarations, resolved type facts, and exact value construction.**
the canonical language accepts generic nominal `enum` declarations, `variant-value`, and
exhaustive source `match`; validates exact named variant, field, and pattern
shape; resolves invariant explicit instantiations; and executes construction,
tag tests, active projection, and match CFG on the evaluator, VM, forced
baseline JIT, and forced proof JIT. The stable compiler-owned prelude enum
replacement is Current.

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

**Current.** Every `variant-value` names the fully instantiated enum type,
variant, and all named fields in declaration order:

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

Ownership-bearing fields and ownership-bearing explicit enum arguments,
directly or nested, are rejected in the Current declaration slice. Declaration
graph recursion is validated under exact depth/work bounds; stable source order
is retained separately from identity. The Current boxed VM representation makes recursive edges physically indirect,
constructs only finite runtime values, and traces exactly the active initialized
payload. These execution facts do not alter declaration identity.

## Prelude Migration

`option` and `result` are compiler-recognized prelude enum identities, never
recognized by source names. Product remains its distinct nominal product
concept; Product does not become an enum. Dedicated Option/Result runtime
semantics are removed. Prelude constructors and accessors resolve to stable
compiler-owned identities and lower only through generic enum construction,
tag test, active projection, and ordinary CFG. No compatibility machinery
remains.
