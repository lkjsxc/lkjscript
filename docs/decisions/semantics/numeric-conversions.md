# the canonical language: Numeric Conversions

[Authority](semantic-core.md)

## Purpose

Remove implicit mixed numeric operations and define the four exact I64/F64
conversion operations.

## Status

<!-- LKJ-STATUS id=numeric-conversions status=current -->

**Current for the four explicit the canonical language conversions and the canonical language rejection
of mixed numeric arithmetic and ordering.** Stable `NumericError` identity and
cases, generic prelude `Result` construction, Semantic Source and typed-hole
candidates, resolved HIR, verified SSA, the evaluator, validated
bytecode/reference VM, forced Linux x86-64 baseline, and forced proof-checked
JIT are included. the removed legacy source contract mixed I64/F64 arithmetic
and ordering remain Current only for the removed legacy source contract migration input.

## Mixed Operations

the canonical language rejects implicit I64/F64 arithmetic and ordering. Operands of ordinary
arithmetic and `lt`, `lte`, `gt`, and `gte` must have one exact numeric type.
Migration inserts a conversion only after resolved operand typing.

## Exact Operations

- `f64-from-i64-rounded I64 -> F64` accepts every I64 and applies IEEE-754
  binary64 round-to-nearest, ties-to-even. Loss of integer precision is allowed.
- `f64-from-i64-exact I64 -> Result F64 NumericError` returns `Inexact` exactly
  when conversion followed by exact integer interpretation would differ.
- `i64-from-f64-exact F64 -> Result I64 NumericError` rejects NaN and either
  infinity as `NonFinite`, finite values outside I64 as `OutOfRange`, and finite
  in-range non-integral values as `Fractional`.
- `i64-from-f64-trunc F64 -> Result I64 NumericError` rejects NaN and infinities
  as `NonFinite`, truncates toward zero, then rejects a result outside I64 as
  `OutOfRange`.

`NumericError` is the compiler-recognized prelude enum identity with cases
`NonFinite`, `OutOfRange`, `Fractional`, and `Inexact`. `Inexact` occurs only for
`f64-from-i64-exact`; `Fractional` occurs only for
`i64-from-f64-exact`. The other two cases occur only on F64-to-I64 operations.

## Boundaries

Binary64 represents every integer from `-2^53` through `2^53` exactly; outside
that interval representability depends on spacing and is tested, not assumed.
I64 is exactly `[-2^63, 2^63 - 1]`. Because `2^63 - 1` rounds to binary64
`2^63`, F64-to-I64 range checks compare mathematically against the I64 bounds,
not against an F64-cast maximum. Both F64 signed zeros convert to I64 zero;
I64 zero converts to positive F64 zero. NaN payloads and either infinity never
convert to I64. No operation depends on host casts or undefined behavior.

## Current Evidence Boundary

Focused tables cover signed zero, subnormals, fractional signs, infinities,
multiple positive and negative NaN payloads, values around `2^53`, the exact
`-2^63` result, the rounded `+2^63` boundary, and the largest representable F64
below `2^63`. Four-engine tests retain F64 bit evidence and require generated baseline/proof
entries and zero VM fallback. Rounded I64-to-F64 uses the verified native
scalar `cvtsi2sd` plan directly and has no heap-runtime site; its boundary table
checks exact result bits. The three fallible conversions retain exact generated
heap-runtime sites because they construct `Result` and `NumericError` values.
Malformed operand/result types and stale or malformed `NumericError`
identity, layout, and cases fail SSA verification.
