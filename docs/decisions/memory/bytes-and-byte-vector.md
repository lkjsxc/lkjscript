# Bytes And Byte-Vector Ownership

## Status

**Accepted contract; Current source still contains transitional `buf` and
`bytes` remains a PLACEHOLDER until the complete cutover lands.** No alias is
retained after migration.

## Byte-Vector

`byte-vector` is an affine mutable value in deterministic unique storage. New,
length, indexed read/write, fill, ranged copy, and bulk host operations use one
owner key. Reads infer shared loans. Mutation requires an exclusive loan.
Copying the owner is invalid.

## Views

`byte-slice` is a shared ranged non-owner carrying owner, start, length, and
verified lifetime. `byte-slice-mut` is exclusive. Bounds and overlap are checked
before effects. Views cannot be returned, captured, or stored in aggregates in
this slice. Their last reachable use emits `end-borrow`.

## Immutable Bytes

`bytes` has immutable value semantics:

- literals use static storage;
- dynamic values use unique immutable storage;
- transient uses borrow;
- multiple escaping owners use an explicit structural copy with plan and metric;
- no per-object reference count and no tracing are used.

Required operations cover length, indexed read, slicing, slice copy,
byte-vector freeze, bytes thaw, and explicit clone.

## Freeze And Thaw

Freeze consumes a vector and transfers compatible backing to dynamic bytes
without copy. Thaw consumes uniquely owned dynamic bytes and transfers backing
to a vector. Thawing static bytes performs one explicit copy. Failed operations
preserve exact ownership and establish no duplicate owner.

## Host Boundaries

File, socket, hashing, SQLite blob, path, protocol, and editor bulk operations
borrow slices. Host calls receive bounded ranges after validation. They do not
receive source object identity.

## Path

`path` is immutable exact Linux path bytes. Static or unique immutable storage
and borrowing follow bytes semantics. Strict UTF-8 conversion remains separate.
Filesystem and SQLite operations borrow path values.

## Transitional Removal

The cutover removes source `buf`, every `buf-*` operation, `HeapObj::Buf`,
identity semantics for buffers, native buffer heap sites, package uses, and
compatibility tests. Canonical operation names use `byte-vector`, `byte-slice`,
`byte-slice-mut`, and `bytes`; removed names have no aliases.

Package and contract identities are regenerated only after the corpus and all
backends migrate atomically.
