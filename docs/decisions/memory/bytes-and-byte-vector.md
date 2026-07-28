# Bytes And Byte-Vector Ownership

## Status

**Accepted contract with executable byte-vector and immutable-bytes evaluator/VM families.** The exact
`new-byte-vector`, whole-place `move`, `borrow`, `borrow-mut`,
`byte-slice-length`, `byte-slice-byte-at`, and
`byte-slice-mut-set-byte` family uses deterministic unique storage in the SSA
evaluator and reference VM. The immutable-bytes source projection and six
operations below are also executable through typing, verified SSA, evaluator,
independently validated bytecode, and VM. Baseline and proof native execution,
ranged borrowed views, host byte operations, and the remaining byte-vector
operations are not Current. Transitional `buf` remains a
separate traced family; it is neither an alias nor a conversion path.

## Byte-Vector

`byte-vector` is an affine mutable value in deterministic unique storage. New,
length, indexed read/write, resize, fill, ranged copy, and bulk host operations
use one owner key. Reads infer shared loans. Mutation requires an exclusive
loan. Copying the owner is invalid.

Resize, fill, and ranged copy validate every range, arithmetic result, retained
byte change, and ceiling before mutation. Ranged copy within one vector is
memmove-like: overlap is accepted and reads the logical source bytes before
they are overwritten. Resize growth beyond capacity is privately constructed
and atomically published; shrinking length retains capacity and therefore does
not reduce retained/live-byte metrics.

## Views

`byte-slice` is a shared ranged non-owner carrying owner, start, length, and
verified lifetime. `byte-slice-mut` is exclusive. The first executable family
creates whole-owner bounded views; ranged source syntax remains non-Current.
Bounds and overlap are checked before effects. Views cannot be returned,
captured, or stored in aggregates in this slice. Their sole supported use ends
the loan deterministically; trap cleanup ends an established loan before owner
release.

## Immutable Bytes

`bytes` has immutable value semantics:

- literals use static storage;
- dynamic values use unique immutable storage;
- transient uses borrow;
- multiple escaping owners use an explicit structural copy with plan and metric;
- no per-object reference count and no tracing are used.

The one canonical literal projection is:

```lkjscript
bytes-literal/
00ff10
/bytes-literal
```

The payload is one line of lowercase hexadecimal with exactly two digits per
byte. The empty payload is represented by an empty line between the markers.
Whitespace, uppercase letters, odd digit counts, non-hexadecimal characters,
multiple payload lines, and decoded data over the active constant-data ceiling
are source errors before typing or effects. There is no quoted, escaped, text,
list, or compatibility spelling.

The exact operations and monomorphic signatures are:

- `bytes-length : fn inputs bytes output i64`;
- `bytes-byte-at : fn inputs bytes i64 output i64`;
- `copy-bytes-slice : fn inputs bytes i64 i64 output bytes`;
- `clone-bytes : fn inputs bytes output bytes`;
- `freeze-byte-vector : fn inputs byte-vector output bytes`;
- `thaw-bytes : fn inputs bytes output byte-vector`.

Length and indexed observation borrow their operand for the call. Slice copy and
clone borrow the input and publish one dynamic owner. Freeze consumes one
byte-vector owner. Thaw consumes one dynamic-bytes owner, or copies a static
literal once into a new byte-vector owner. A dynamic value passed to an
ownership-consuming operation uses explicit `move/`; a static literal is
copyable and needs no owner move. This slice has no borrowed `bytes` source type
and no zero-copy ranged `bytes` result; `copy-bytes-slice` is the checked owned
range operation.

Negative indexes, negative range components, `start + length` overflow, and
out-of-bounds access trap with the operation name and exact rejected values.
Allocation, object, byte, slot, generation, retained-capacity, and host storage
failures use the existing resource-limit or deterministic runtime-trap classes.
Every bound and allocation preflight completes before ownership transfer or
payload publication. Source-literal failures are structured source diagnostics;
there is no runtime attempt and no effect.

## Freeze And Thaw

`freeze-byte-vector` consumes a vector and transfers compatible backing to
dynamic bytes without copy. `thaw-bytes` consumes uniquely owned dynamic bytes
and transfers backing to a vector. Both preserve runtime-local packed
slot/generation identity. Thawing static bytes performs one bounded, accounted
allocation and copy. `clone-bytes` and `copy-bytes-slice` likewise publish one
independently releasable dynamic owner after complete preflight. Failed
operations preserve exact ownership and establish no duplicate owner.

## Host Boundaries

File, socket, hashing, SQLite blob, path, protocol, and editor bulk operations
borrow slices. Host calls receive bounded ranges after validation. They do not
receive source object identity.

## Path

`path` is immutable exact Linux path bytes. The accepted cutover uses static
artifact storage or one dynamic `UniqueStore::PathKey` owner. Ordinary
observation, equality, filesystem, and SQLite uses borrow. A source copy is an
explicit planned structural copy that publishes and accounts one new owner;
whole-place move and return transfer the existing owner. Strict UTF-8 conversion
remains separate. No accepted path owner is reference counted, traced, or
aliased.

The core store now provides bounded path construction, structural copy, exact
value comparison, release, and returned-backing transfer with stale and wrong-
layout rejection. Current evaluator/VM source paths still use `HeapObj::Path`.
The constructor's `result path system-error` envelope is copyable/traced today;
unique path bytes cannot enter it until whole-value aggregate transfer/drop is
implemented or the affected contract and corpus are atomically rewritten.

## Transitional Removal

The cutover removes source `buf`, every `buf-*` operation, `HeapObj::Buf`,
identity semantics for buffers, native buffer heap sites, package uses, and
compatibility tests. Canonical operation names use `byte-vector`, `byte-slice`,
`byte-slice-mut`, and `bytes`; removed names have no aliases.

Package and contract identities are regenerated only after the corpus and all
backends migrate atomically.
