# Bootstrap language specification

## Types and values

The closed semantic types are `unit`, `bool`, and signed two's-complement `i64`. There are no
implicit conversions, null values, dynamic values, casts, exceptions, generics, or nominal types.
Function signatures store ordered parameter Node IDs and one declared result type. The current
daemon invocation supports only zero-parameter entry functions.

Scalar values use copy semantics. The operation contract records `Copy` for consumed bootstrap
operands. Ownership-bearing values and borrow rules do not yet exist.

## Operations

`src/schema.rs` owns one exhaustive operation contract used by graph validation, type derivation,
completeness, lowering eligibility, and result typing.

- `ConstI64(value)` has no operands and produces one `i64`.
- `ConstBool(value)` has no operands and produces one `bool`.
- `AddI64(lhs, rhs)` copies two exact `i64` values and produces one `i64`.
- `Hole(expected)` produces a typed semantic placeholder for graph construction but is always
  incomplete and cannot lower.
- `Return(value)` is the block terminator and requires the exact declared function result type.

Operations are pure. `i64` addition uses checked arithmetic; overflow is a structured runtime trap.
There are no host effects or capabilities in this slice.

An operation result may be used only later in the same ordered block. A parameter value must belong
to the owning function. The output index must exist in the producer's closed contract. The graph
validator enforces these facts before publication, and the private Core IR verifier checks dense
value definition, dominance, type agreement, and return type again before execution.

## Compilation and execution

A selected entry function may compile when its body exists and its containment closure has no
holes. Unused incomplete functions do not block that entry. The only executable path is:

```text
immutable SPG snapshot -> completeness/type validation -> Core IR -> verifier -> interpreter
```

Core IR has explicit functions, signatures, blocks, typed dense values, closed instructions, a
separate return terminator, and semantic origin Node IDs. It is derived, same-build, and not
serialized. The interpreter implements every current Core IR instruction and returns a typed
`RuntimeValue`.

Calls, branches, loops, recursion, aggregates, sums, patterns, generics, effects, host operations,
ownership-bearing values, and native execution are not implemented.
