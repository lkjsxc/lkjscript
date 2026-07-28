# Typed VM Scalar Representation

## Status

<!-- LKJ-STATUS id=typed-vm-scalars status=current -->

**Current.** Evaluator, VM, baseline JIT, and proof JIT scalar fixtures retain
complete-range i64 and exact-bit f64 values without scalar heap allocation.

## Decision

Validated bytecode supplies the static type of each constant, stack slot, local,
argument, block transfer, call, and result. The VM uses one safe closed `Value`
with a 64-bit payload and explicit metadata. Its 16-byte representation has
distinct categories for invalid storage, unit, bool, i64, f64, empty list,
capability, resource, legacy-traced reference, and opaque unique key.

I64 stores its two's-complement bits. F64 stores its IEEE-754 bits. Resource,
legacy-traced, and opaque-key payloads cannot be reinterpreted as scalars. A
physically copyable Rust value does not make an affine source value copyable;
bytecode ownership verification still governs semantic copying.

## Exactness

All signed i64 values, including minimum and maximum, remain inline. F64
preserves every NaN payload, signed zero, infinity, constant, local, call,
return, evaluator conversion, and VM/native transition bit-for-bit.

## Collector Boundary

`HeapObj` has no integer or floating family. Constants, numeric operations,
conversions, host adapters, VM/native transitions, detached returns, generated
heap services, and aggregate payload conversion create inline scalars. Only the
explicit legacy-traced metadata category can retain a `GcHeap` object.

Malformed bytecode with incompatible slot types rejects before effects. The
opaque unique-key category is implemented storage, not a claim that reusable
generation-safe resource slots or the complete collector-free island are
Current.

## Evidence

Focused core, VM, evaluator, baseline, proof, application, SQLite, equality,
and ownership tests cover complete i64 range, exact f64 bits, zero scalar heap
allocation, synchronous generated entry, and zero forced fallback.
