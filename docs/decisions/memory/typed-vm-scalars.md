# Typed VM Scalar Representation

## Status
<!-- LKJ-F typed-vm-scalars current pq_CN6UwvrbtitwBC8w4qNKyazgupyvpmNq5l7GIYo0 -->


**Current.** Evaluator, VM, baseline JIT, and proof JIT scalar fixtures retain
complete-range i64 and exact-bit f64 values without scalar heap allocation.

## Decision

Validated bytecode supplies the static type of each constant, stack slot, local,
argument, block transfer, call, and result. The VM uses one safe closed `Value`
with a 64-bit payload and explicit metadata. Its 16-byte representation has
distinct categories for invalid storage, unit, bool, i64, f64, empty/segmented
list, capability, resource, structural root/view/destination,
invocation-region product, and opaque unique key.

I64 stores its two's-complement bits. F64 stores its IEEE-754 bits. Resource,
structural, invocation-region, and opaque-key payloads cannot be reinterpreted
as scalars. A
physically copyable Rust value does not make an affine source value copyable;
bytecode ownership verification still governs semantic copying.

## Exactness

All signed i64 values, including minimum and maximum, remain inline. F64
preserves every NaN payload, signed zero, infinity, constant, local, call,
return, evaluator conversion, and VM/native transition bit-for-bit.

## Aggregate Boundary

Constants, numeric operations, conversions, host adapters, VM/native
transitions, detached returns, runtime-value services, and aggregate payload
conversion create inline scalars. Aggregate storage cannot be reinterpreted as
an inline scalar.

Malformed bytecode with incompatible slot types rejects before effects. The
opaque unique-key category is implemented storage, not a claim that reusable
generation-safe resource slots or the complete collector-free island are
Current.

## Evidence

Focused core, VM, evaluator, baseline, proof, application, SQLite, equality,
and ownership tests cover complete i64 range, exact f64 bits, zero scalar heap
allocation, synchronous generated entry, and zero forced fallback.
