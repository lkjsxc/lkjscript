# Exact I64 And F64 Semantics

## Purpose

Define the first numeric surface whose source grammar, static types, bytecode,
VM values, arithmetic, comparison, and host boundaries agree.

## Status

**Current.** Focused parser, type-vocabulary, bytecode, VM boundary, host
narrowing, and compiled source-to-VM conformance tests enforce this record.

## Canonical Types

The numeric type vocabulary is exactly `i64` and `f64`.

`I32`, `U32`, `U64`, `F32`, lowercase type aliases, `Int`, `Float`, numeric
cast names, float-prefixed operators, symbolic comparison aliases, `le`, `ge`,
and slash-named division are removed. A removed name is an error rather than a
compatibility alias. Future widths or conversions require complete source,
type, lowering, runtime, and boundary behavior before becoming public.

## Literal Grammar

Numeric literals are ASCII base-ten atoms:

```text
I64 := "-"? DIGIT+
F64 := "-"? DIGIT+ "." DIGIT+
```

Leading zeroes are accepted. A leading plus, exponent, digit separator,
hexadecimal form, missing digits around the decimal point, non-finite spelling,
or value outside the destination type is rejected with a numeric-literal
diagnostic. Numeric-looking malformed atoms do not silently become symbols.
Integer spelling is parsed directly to `i64`; it never passes through `f64`.
F64 source literals must be finite. IEEE arithmetic may subsequently produce
infinity or NaN.

## Operators

The canonical binary arithmetic names are `add`, `subtract`, `multiply`, and `divide`.

- Two `i64` operands produce `i64` using checked signed arithmetic.
- I64 `divide` truncates toward zero. Division by zero and `I64::MIN div -1` are
  runtime errors.
- If either operand is `f64`, the I64 operand is converted using normal IEEE-754
  round-to-nearest semantics and the result is `f64`.
- F64 division follows IEEE-754, including infinity, NaN, and signed zero.
- F64 results remain F64 even when mathematically integral.
- Arithmetic operators require exactly two operands.

`bit-and`, `bit-or`, and `bit-xor` accept exactly two I64 values and operate on
all 64 two's-complement bits.

Canonical numeric ordering operations are `less-than`, `less-than-or-equal`, `greater-than`, and `greater-than-or-equal`.
Ordering uses the same I64/F64 promotion rule as arithmetic.

Equality does not promote. `equal-value` requires identical supported operand
types: I64 equality is exact and F64 equality is IEEE equality, where NaN is
unequal to every value and positive/negative zero are equal.
`equal-f64-bits` accepts only F64 and compares complete bit patterns, so equal
NaN payloads match and signed zero differs. Use `not` around a positive
operation instead of a negative alias. Non-numeric equality categories are
specified by [Explicit Equality Families](equality-families.md).

## Representation

The VM keeps its tagged immediate fast path for I64 values in the signed 61-bit
payload range and stores other I64 values in a distinct heap integer object.
This is an implementation detail: both forms are one language type and every
I64 consumer must accept both. Bytecode constants preserve I64 values as I64.
No integer operation or literal is routed through F64.

F64 values are heap objects and never collapse into immediate I64 values.
Garbage collection traces both numeric object kinds as leaf objects.

## Host Boundaries

A primitive declared to consume I64 accepts the complete I64 range. A narrower
host domain, including bytes, u32 words, ports, backlogs, timeouts, buffer
indices, exit codes, and allocation lengths, performs an explicit checked
conversion and fails according to that primitive's documented error channel.
Implicit truncation, wrapping, saturation, and float coercion are forbidden.
`format-i64` accepts only I64 and emits the exact decimal value;
`format-f64` accepts only F64.

## Verification

Focused conformance covers:

- `I64::MIN`, `I64::MAX`, immediate/boxed boundaries, and values around `2^53`;
- literal overflow and every rejected numeric spelling;
- checked add, subtract, multiply, divide, zero divide, and `MIN / -1`;
- mixed arithmetic, integral F64 identity, signed zero, infinity, and NaN;
- exact I64 equality, IEEE F64 equality, exact F64-bit equality, and numeric
  ordering;
- high-bit bitwise operations;
- strict string conversions and checked host narrowing;
- rejection of every removed type, alias, cast, and operator;
- prelude/type/lowering/VM agreement through compiled source execution.

## Rejected

- **F64-backed integers:** loses exactness and changes runtime type.
- **A small-integer-only public type:** contradicts the `i64` name and host ABI.
- **Approximate F64 equality:** is scale-dependent and not IEEE equality.
- **Unimplemented advertised widths and casts:** typechecks code that cannot run.
- **Always-boxed I64:** makes common counters, indices, and bytecode constants
  allocate without a correctness benefit.
- **Wrapping overflow by default:** hides arithmetic failure and blocks a later
  explicit wrapping-operation surface.
