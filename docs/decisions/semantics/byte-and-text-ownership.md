# Byte And Text Ownership

## Status

**Accepted contract.** Individual representations become Current only with
complete typing, lifetime, evaluator, VM, forced native, root, bounds, and
cross-engine evidence. Current `Str`, `Buf`, and `Owned Buf` spellings are not
aliases for this destination.

## Immutable bytes

`bytes` is an immutable arbitrary byte sequence with value semantics, exact
length and checked byte observation. It may share backing storage only where
sharing is unobservable and memory safe. No operation exposes mutable aliasing.
Its `copy`, `send`, and `sync` facts follow the selected immutable
representation rather than object identity.

## Unique mutable storage

`byte-vector` is affine, contiguous, growable mutable byte storage. It moves,
has exact length and capacity, and permits mutation only through unique access.
It is not implicitly shared or identified by a universal GC object identity.
Placement may be stack, region, or heap without changing source semantics.

## Borrowed views

`byte-slice` is a checked shared lexical range over a live owner.
`byte-slice-mut` is an exclusive checked lexical range. Neither owns storage,
escapes its lifetime, copies implicitly, aliases an incompatible borrow, or
crosses an unsupported worker, suspension, or host boundary.

## Text

`string` owns valid UTF-8 text. `str` is reserved for a borrowed valid-UTF-8
view. Current owned `Str` values migrate to `string`; they do not silently
acquire borrowed semantics. APIs distinguish byte length, scalar iteration,
validation, formatting, parsing, encoding, decoding, and slicing.

## Paths

`path` remains immutable Linux OS-native pathname bytes. It is not text.
Construction validates the existing absolute-path, NUL, and size contract.
Filesystem APIs consume or borrow `path`; explicit conversion to text validates
UTF-8 and returns a typed result.

## Operations and migration

Canonical names state ownership and units, for example `bytes-length`,
`bytes-byte-at`, `byte-vector-length`, `byte-vector-set-byte`,
`string-byte-length`, and `convert-bytes-to-string`. Bulk file, socket, hashing,
SQLite blob, editor, Brainfuck, and HTTP operations replace quadratic per-byte
default paths only after complete equivalents are Current.

The migration preserves limits and raw bytes, proves view lifetimes, updates
exact roots, and removes `buf` aliases after all live uses have complete
replacements. Invalid UTF-8 remains data until an explicit validating text
conversion.
