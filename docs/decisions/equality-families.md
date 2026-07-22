# Explicit Equality Families

## Purpose

Replace universal dynamic `eq`/`ne` with statically selected equality operations
whose names expose value, identity, structural-list, and floating-bit semantics.

## Status

**Accepted Target.** The current implementation still exposes universal `eq`
and `ne`; the source-to-HIR-to-bytecode-to-VM cutover must land atomically before
this record becomes **Current**. No compatibility aliases are retained.

## Canonical Operations

The accepted source operations are:

```text
equal-value       T -> T -> Bool
same-object       T -> T -> Bool
list-equal        List T -> List T -> Bool
f64-bits-equal    F64 -> F64 -> Bool
```

The signatures above are descriptive. The compiler applies the exact static
constraints below; there is no general equality trait or inferred constraint
on a type parameter.

There is no negative equality operation. Use `not` around the appropriate
positive operation. The names `eq` and `ne` are removed rather than retained as
aliases.

## `equal-value`

Both operands have exactly the same static type. Mixed I64/F64 comparison is
rejected; equality does not perform arithmetic promotion.

Currently supported types are:

- Unit: the sole values are equal;
- Bool: canonical boolean equality;
- I64: exact signed 64-bit equality across immediate and boxed VM forms;
- F64: IEEE-754 equality, so NaN is unequal to every value and `+0.0` equals
  `-0.0`;
- Str: UTF-8 byte-content equality;
- Symbol: symbol-content equality, distinct from Str;
- `Option T` when `T` supports `equal-value`: none equals none, some equals some
  when payloads are equal, and none differs from some;
- `Result T E` when both payload types support `equal-value`: variants must
  match and matching payloads compare recursively.

Future immutable products and enums gain `equal-value` only when their types and
structural contracts are implemented. Bytes is not yet a current language type.

`equal-value` is rejected for List, Buf, Handle, Fn, unresolved type parameters,
and polymorphic schemes. In particular, closure allocation or duplication
remains unobservable.

## `same-object`

Both operands have exactly the same static type. Current supported types are:

- Buf: equality of the exact mutable buffer object, never byte-content equality;
- Handle: equality of the opaque capability token.

`buf-clone` produces a distinct object. Copies of one Handle token remain the
same identity even after close; operations on a closed token still fail through
the existing stale-handle contract. Integers cannot be compared to handles.

No source operation exposes closure identity. Future Cell/Ref identity requires
those types to be implemented first.

## `list-equal`

Both operands have exactly the same `List T` type, and `T` must support
`equal-value`. Nested List elements are not implicitly traversed; a future
recursive collection-equality contract would need an explicit operation rather
than hidden dispatch.

Comparison is iterative and structural:

1. two empty lists are equal;
2. empty and non-empty lists differ;
3. non-empty lists compare their heads with `equal-value` and then their tails;
4. the first unequal head or length difference returns false.

At most `MAX_LIST_EQUAL_STEPS = 1_000_000` pair-node comparisons are allowed in
one call. Reaching another pair after that bound is a runtime error, not false.
Improper lists or malformed runtime values are errors. Compiler-produced List
values are proper, but the VM boundary must remain truthful for malformed
chunks.

The operation reads memory and may trap on the fixed bound or malformed values.
It does not allocate language heap objects or mutate either list.

## `f64-bits-equal`

Both operands are exactly F64. Equality compares the complete IEEE-754 bit
patterns, equivalent to Rust `f64::to_bits` equality:

- equal NaN payload/sign bits compare equal;
- different NaN payloads compare unequal;
- `+0.0` and `-0.0` compare unequal.

Use `equal-value` when IEEE numeric equality is intended. I64 and mixed numeric
operands are rejected.

## HIR And Bytecode Contract

Resolved HIR records the exact canonical operation and operand/result types.
Backends never infer equality category from runtime values.

The bytecode cutover is intentionally incompatible because chunks are currently
in-memory and unversioned:

- byte 20 becomes `EqualValue`;
- historical byte 21 (`Ne`) becomes invalid;
- fresh opcodes represent `SameObject`, `ListEqual`, and `F64BitsEqual`;
- old `Eq`/`Ne` opcodes and source names are removed without shims.

VM operations validate runtime categories so malformed public chunks fail
rather than exposing raw tagged-value or closure identity.

## Effects

- `equal-value`, `same-object`, and `f64-bits-equal` conservatively read memory
  because current VM representations may be heap-backed;
- `list-equal` reads memory and may trap;
- none of the operations performs host IO, language allocation, mutation, or
  process exit.

## Verification

The complete cutover covers:

- Unit, Bool, immediate/boxed I64, IEEE F64, Str, and Symbol value equality;
- recursive Option and Result value equality;
- Str/Symbol category separation;
- same/different Buf and Handle identity;
- empty/equal/unequal/different-length List values;
- list bound and malformed/improper-list errors;
- F64 NaN payloads and signed zero under both F64 equality operations;
- rejection of mixed numeric equality, List under `equal-value`, scalar under
  `same-object`, non-F64 bit equality, closure equality, unconstrained generic
  equality, old `eq`/`ne`, and retired opcode byte 21;
- exact HIR operation identity, effects, bytecode lowering, disassembly, and VM
  outcomes;
- migration and runtime acceptance of the complete canonical source corpus.

## Rejected

- Universal dynamic equality.
- A negative alias instead of `not` plus a positive operation.
- Implicit I64/F64 promotion for equality.
- String/Symbol cross-category equality.
- Buffer byte-content comparison under object identity.
- Observable closure identity.
- Unbounded recursive list comparison.
- Returning false for a resource-bound or malformed-value failure.
- Retaining old source or opcode compatibility shims.
