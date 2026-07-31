# Byte And Text Ownership

## Status

**Accepted contract with an initial ownership foundation.** Owned text is
spelled `string`; `string-literal/` replaces the old `str/` marker. Direct
`byte-vector`, `byte-slice`, and `byte-slice-mut` spellings expose the existing
whole-place affine slice. Immutable `bytes` has one Current
`bytes-literal/` lowercase hexadecimal projection and the exact four-engine
operation subset defined by [bytes and byte-vector
ownership](../memory/bytes-and-byte-vector.md). Ranged borrowed views and
borrowed `str` are non-Current. Transitional buffer spellings are removed; no
old spelling aliases a destination type.

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

`string` owns valid UTF-8 text. Static literals are artifact values; dynamic
strings use deterministic unique, caller-destination, ordinary-region, or
sealed-region ownership selected by the verified plan. Equality compares UTF-8
bytes and ordinary observation borrows instead of cloning.

`str` is reserved for a borrowed valid-UTF-8 view. The first cutover keeps this
view internal rather than publishing an incomplete source type. It carries an
owner/root, byte range, UTF-8 boundary proof, and exact shared-loan lifetime; it
cannot return, enter an aggregate, be captured, or cross a process/task/host
ownership boundary. APIs distinguish byte length, scalar iteration, validation,
formatting, parsing, encoding, decoding, and slicing.

## Paths

`path` remains immutable Linux OS-native pathname bytes. It is not text.
Construction validates the existing absolute-path, NUL, and size contract.
Filesystem APIs consume or borrow `path`; explicit conversion to text validates
UTF-8 and returns a typed result.

## Operations and migration

Canonical names state ownership and units. The immutable family is
`bytes-length`, `bytes-byte-at`, `copy-bytes-slice`, `clone-bytes`,
`freeze-byte-vector`, and `thaw-bytes`. The mutable family uses
`byte-vector-length`, exact byte/u32 access, and checked shared or exclusive
views. Text conversion is `convert-string-to-bytes` and
`convert-bytes-to-string`; invalid UTF-8 remains data until explicit validation.

Bulk file, socket, random, hashing, terminal, SQLite blob, editor, Brainfuck,
and HTTP operations borrow `byte-slice` or `byte-slice-mut`. They receive a
validated range, never owner identity. The integrated cutover removes source
`buf`, every `buf-*` operation, buffer bytecode/native helpers and metrics, and
all `HeapObj::Buf` allocation. The removed spelling has one exact diagnostic and
no alias or forwarding path.

The migration preserves existing aggregate limits, byte bounds, logical
charges, exact path bytes, and failure-before-mutation behavior. Package and
contract identities change atomically with the corpus and backend cutover.
