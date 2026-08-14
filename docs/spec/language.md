# Bootstrap language specification

## Types and values

The closed semantic types are `unit`, `bool`, and signed two's-complement `i64`. There are no
implicit conversions, null or dynamic values, casts, exceptions, generics, or nominal types.
Functions store ordered parameter Node IDs and one result type. Daemon entry invocation currently
supports only zero-parameter functions.

Scalar values have copy semantics. Operation contracts record `copy` operand use; ownership-bearing
values and borrow rules do not exist yet.

## Operations

`src/schema.rs` owns one exhaustive static operation contract used by graph validation, schema
queries, result typing, completeness, lowering, codecs, and machine description.

- `const_i64(value)` has no operands and produces one `i64`;
- `const_bool(value)` has no operands and produces one `bool`;
- `add_i64(lhs, rhs)` copies two exact `i64` values and produces one `i64`;
- `hole(expected)` produces one value of its exact expected type but remains incomplete and cannot
  lower;
- `return(value)` is the terminator and requires the exact function result type.

This campaign adds no language operation. `RefineHole` is a semantic graph edit, not an executable
operation. It is the sole identity-preserving constructor transition: a typed hole may become a
complete non-terminator operation with the same one-result contract while retaining its Node ID,
body position, owner, and uses. Refining to another hole, a terminator, a different result type, or
refining an already-complete operation rejects. There is no reverse transition or general operation
morph. Replacement operands still obey same-function scope, earlier-in-block order, result-index,
and exact-type rules.

Operations are pure. `i64` addition uses checked arithmetic and overflow is a structured runtime
trap. There are no host effects or capabilities.

## Compilation and execution

A selected entry may compile when its body exists and its dependency closure contains no holes.
Unused incomplete definitions do not block that entry. The single executable route is:

```text
immutable SPG snapshot -> completeness/type validation -> Core IR -> verifier -> interpreter
```

Core IR uses typed dense values and semantic origin Node IDs, is derived same-build state, and is
never serialized. The verifier rechecks definition, order/dominance, type, and return agreement
before interpretation.

Calls, branches, loops, recursion, aggregates, sums, patterns, generics, effects, host operations,
ownership-bearing values, and native execution are not implemented.
