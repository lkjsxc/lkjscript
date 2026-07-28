# Typed VM Scalar Representation

## Status

**Accepted contract; Current VM may still box wide i64 and f64 values in
`GcHeap`.** Native scalar execution does not by itself satisfy VM acceptance.

## Decision

Validated bytecode supplies the static type of each constant, stack slot, local,
argument, block transfer, call, and result. The VM uses that authority to store:

- i64 as raw signed 64-bit bits;
- f64 as raw IEEE-754 64-bit bits;
- bool and unit inline;
- references and affine keys in separately typed forms.

A physically copyable Rust word does not make an affine source value copyable.
Bytecode ownership verification governs semantic copying.

## Exactness

All signed i64 values, including minimum and maximum, remain unboxed. F64
preserves NaN payloads, signed zero, infinities, constants, locals, calls,
returns, evaluator conversion, and VM/native transitions bit-for-bit.

## Removal

After every Current VM path uses typed scalar slots, delete `HeapObj::Int` and
`HeapObj::Float`, their allocation sites, trace classifications, returned
snapshot cases, and compatibility tests. Scalar metrics record inline
operations and avoided collector allocations.

Malformed bytecode with incompatible slot types rejects before effects.
