# Immutable Nominal Products

## Purpose

Define the first user-defined aggregate type needed to replace mutable singleton
state with explicit values passed through helpers.

## Status

**Current.** The compiler, resolved typed HIR, bytecode, disassembler, precise
GC, and reference VM implement this contract. Product state has not yet replaced
current mutable singleton globals; that follow-on remains an **Accepted Target**
in [current-state.md](../current-state.md).

## Decision

The first aggregate is a nominal, monomorphic, immutable product with named
fields. A product declaration creates a type identity and field layout but no
runtime value, global slot, initializer, or host effect.

This slice deliberately does not add tuples, anonymous records, mutable fields,
methods, inheritance, structural subtyping, row polymorphism, default fields,
field inference, or product equality.

## Canonical Declaration

A declaration has exactly one `name` form and one `fields` form:

```lkjscript
product/
name/
Point
/name
fields/
field/
name/
x
/name
type/
I64
/type
/field
field/
name/
y
/name
type/
I64
/type
/field
/fields
/product
```

The declaration contract is:

- a product name begins with an ASCII uppercase letter and then contains only
  ASCII letters, digits, or hyphens; lowercase-leading and underscore names are
  rejected;
- product names share the program-global declaration namespace with function
  and value names;
- a name is unique across the complete import closure and cannot collide with
  a built-in operation, contextual form, or built-in type constructor;
- fields are ordered by declaration, and each field name is unique within its
  product;
- a product has from zero through 15 fields;
- every field has an explicit type and contains no unbound type parameter;
- under the Current ownership safe island, every field rejects direct or
  nested `Owned`, `Ref`, and `RefMut`, including occurrences inside
  List/Option/Result;
- product declarations may refer to products declared later in the import
  closure;
- the annotation `Product Point` denotes this exact nominal type;
- two separately declared products are different types even when their field
  names and types are identical.

The 15-field limit is independent of general child-count limits. It permits one
product type name plus 15 canonical constructor fields within the current
16-child expression budget. Raising it requires a later explicit limit change.

## Construction

Construction names the product and supplies every field exactly once in
**declaration order**:

```lkjscript
product-value/
Point
field/
x
1
/field
field/
y
2
/field
/product-value
```

Declaration order is canonical. Reordering fields is rejected rather than
silently normalized because evaluation order must remain visible in source.
Unknown, duplicate, missing, extra, or out-of-order fields are compile errors.
Each value must be exactly assignable to the declared field type. Values are
evaluated eagerly from left to right. The result has type `Product Point`.
A zero-field product is constructed with only its product name.

## Access And Immutable Update

Named access is:

```lkjscript
field/
point
x
/field
```

The first operand must have a concrete product type. The second child is a
literal field name in declaration context, not a runtime Symbol value. The
result has the declared field type.

Immutable replacement is:

```lkjscript
with-field/
point
x
new-x
/with-field
```

The original product is evaluated before the replacement. The replacement must
be exactly assignable to the field type. The operation allocates and returns a
new value of the same nominal product type; the original value is unchanged.
There is no syntax for mutating a product field in place.

`product`, `fields`, `field`, `product-value`, and `with-field` are contextual
forms. Declaration-only forms are rejected in expression position, and
expression-only forms are rejected where a declaration is required.

## Type And Effect Contract

Resolved HIR gives every declaration a deterministic `ProductId`, assigned in
dependency-first source/form order. Product expressions carry that identity and
a resolved numeric field index; backends never reinterpret source names.

The effects are:

- product declaration: none and no runtime code;
- `product-value`: child effects plus `Allocates`;
- `field`: value effects plus `ReadsMemory`;
- `with-field`: original/replacement effects plus `ReadsMemory | Allocates`.

Product values are GC-managed. Tracing visits every field. Recursive product
types are allowed through existing indirection such as `Option` and `List`, but
all runtime values remain finite and resource-bounded.

Products are not accepted by `equal-value`, `same-object`, or `list-equal`.
Product equality begins only after a separate recursive-comparability decision;
identity is not silently substituted for value equality.

## Bytecode And VM Contract

The shared typed pipeline lowers products to:

- immutable chunk product metadata with product and field names;
- `MakeProduct` with a resolved product identity;
- `LoadProductField` with a resolved product/field descriptor;
- `WithProductField` with a resolved product/field descriptor.

Product metadata is not part of the mutable VM global table. A declaration must
not consume a global value slot or execute an initializer.

The VM validates descriptor indexes, product identities, field indexes, operand
categories, and constructor arity even for malformed public chunks. Construction
and replacement allocate through the ordinary GC heap path. The later general
heap/resource-limit phase remains pending. Field access does not allocate.
Display may use a stable opaque product marker until a
separate formatting contract exists.

## Required Conformance

The implementation is not Current until focused tests prove:

1. zero- and 15-field boundaries pass and 16 fields fail;
2. forward references and nested `Product Name` annotations resolve;
3. duplicate/colliding declarations and duplicate fields fail;
4. missing, extra, unknown, duplicate, and out-of-order constructor fields fail;
5. access and replacement preserve exact types and evaluation order;
6. replacement leaves the original product unchanged;
7. same-shaped declarations remain nominally distinct;
8. product equality is rejected;
9. products nested through Option, Result, List, and other products survive GC;
10. malformed bytecode descriptors, categories, identities, and indexes fail
    without panic;
11. product declarations add no runtime globals or initialization effects;
12. all canonical sources, runtime smokes, and bounded diagnostic performance
    comparisons remain accounted for.

## Follow-On Work

Once this contract is Current, Brainfuck, lkjedit, and terminal state can be
represented as immutable products. The later atomic semantic cutover still must
add function/main-local `var`/`set`, explicit executable `main`, effect-free
imported libraries, and removal of source mutable globals. Products do not make
that unfinished behavior Current by themselves.
