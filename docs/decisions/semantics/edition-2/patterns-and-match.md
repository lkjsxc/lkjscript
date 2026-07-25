# Edition 2: Patterns And Match

[Authority](../edition-2.md)

## Purpose

Define the closed pattern language, match semantics, usefulness algorithm, and
lowering boundary.

## Status

**Accepted Target, not Current.** Current Edition 1 has no general match form.

## Exact Projection

A match evaluates its scrutinee once and contains ordered arms:

```text
match/
SCRUTINEE
arms/
arm/
PATTERN
BODY
/arm
/arms
/match
```

Every pattern is one closed marker form:

- `wildcard/ /wildcard`;
- `binding/ name/ x /name /binding` for an immutable binding;
- `bool-pattern/ true /bool-pattern`;
- `i64-pattern/ 42 /i64-pattern`;
- `variant-pattern/ type/ INSTANTIATED-TYPE /type variant/ NAME /variant
  fields/ NAMED-VARIANT-FIELD-PATTERNS /fields /variant-pattern`;
- `product-pattern/ type/ INSTANTIATED-PRODUCT-TYPE /type fields/
  NAMED-PRODUCT-FIELD-PATTERNS /fields /product-pattern`.

Whole-value aliases are Deferred with guards and or-patterns; there is no inert
or partially implemented alias form.

A named variant field pattern is exactly nested as follows; product fields use
`product-field-pattern/` with the same children:

```text
variant-pattern/
type/
Option/
I64
/Option
/type
variant/
Some
/variant
fields/
variant-field-pattern/
name/
value
/name
binding/
name/
x
/name
/binding
/variant-field-pattern
/fields
/variant-pattern
```

All fields appear exactly once in declaration order. Enum patterns always state
the exact instantiated type. Nullary patterns retain empty `fields/` markers.
Each uppercase metavariable in the match grammar expands to exactly one source
expression or one of these closed pattern forms.

## Semantics

Arms are tested in source order; the first matching arm is selected. Arm types
join by exact equality, with `Never` only as the explicit divergence join.
Useless arms are compile errors. F64, string, range, equality, extractor, regex,
view, guard, and or-patterns are deferred. I64 coverage requires a wildcard or
binding remainder.

## Exhaustiveness

The compiler uses bounded Maranget pattern matrices over the closed constructor
space, with constructor specialization, default matrices, usefulness checking,
and deterministic witnesses. It does not use ad-hoc set subtraction. Matrix,
constructor, recursion, usefulness, and witness work is pre-charged. Budget
exhaustion is a resource error and never a claim of exhaustiveness.

One opaque verified match plan records the scrutinee type, constructor space,
source arm identities and order, stable tag tests, active payload projections,
binding assignments, default/unreachable edges, exhaustive witness fact, and
exact work/size charges. The verifier rechecks every identity, field type,
projection precondition, arm join, coverage result, and bound.

Only after plan verification does lowering create SSA CFG. Backends implement
verified ADT primitives and CFG terminators; no backend implements `match`.
