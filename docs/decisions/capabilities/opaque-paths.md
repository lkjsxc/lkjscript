# Opaque Byte-Preserving Paths

[Authority](../../operations/status-authority.md)

## Status

**Current on Linux x86-64.** Runtime filesystem and SQLite pathname APIs accept
only the opaque immutable `path` value. `string`, `bytes`, and `byte-vector`
are not pathname values and have no compatibility overload.

## Disjoint Foundations

- `bytes` is bounded immutable arbitrary bytes.
- `byte-vector` is bounded affine mutable arbitrary bytes.
- `string` is bounded immutable valid UTF-8.
- `path` is bounded immutable native pathname bytes.

Runtime `path` is unrelated to package and module identity paths, which remain
canonical package-root-relative UTF-8 strings.

## Construction And Observation

The integrated public byte cutover provides:

```text
convert-string-to-path: fn inputs string output result path system-error
convert-bytes-to-path: fn inputs bytes output result path system-error
convert-path-to-bytes: fn inputs path output bytes
convert-path-to-string: fn inputs path output result string utf8-error
```

A Current Linux `path` contains 1 through 4095 bytes. Its first byte is `/`.
Empty and relative paths are rejected. Interior NUL and a 4096th byte are
rejected before FFI or an unchecked allocation. Constructors copy exact bytes;
they do not normalize, canonicalize, decode with replacement, consult the
current directory, inspect environment or home state, or search roots.

`convert-path-to-bytes` returns an independent exact immutable byte value.
`convert-path-to-string` performs strict UTF-8 validation and returns the
existing precise `utf8-error` kind and offset. `path` has byte-value equality
and immutable value semantics; it has no object-identity or mutation operation.

The accepted collector-free representation gives every dynamic path exactly one
`UniqueStore::PathKey` owner. A source-level copy must become an explicit
structural-copy event with one independently releasable publication and exact
allocation/retained-byte accounting. Whole-place move and return transfer the
existing owner. Observation, equality, filesystem calls, and SQLite calls
borrow exact bytes and never receive owner identity. Reference counting,
tracing, and aliasing owner keys are forbidden.

The safe core store implements exact path allocation, access, value comparison,
structural copy, release, stale/wrong-layout rejection, and returned-backing
transfer. Evaluator paths use generation-safe unique path keys and VM path
leaves use the bounded structural value runtime. `HeapObj::Path`, its wire tag,
and the `path` tracing entry are removed; no dual traced path representation is
permitted.

Generic constructor output `result path system-error` is governed by the
aggregate-affine contract. Its payload must transfer through an exact structural
or resource adapter with whole-value drop; a traced envelope containing a path
owner is rejected. Complete compiler-selected option/result execution through
all tiers remains separate acceptance evidence.

Absolute bytes remove current-directory authority but do not claim symlink
containment. Directory capabilities and `openat`-style sandboxing remain a
separate accepted target.

## Host API Cutover

These operations accept `path` in place of `string`, with no retained old arity or
overload:

- `open-file-reader`, `open-file-writer`, `open-file-appender`,
  `create-file`, and `open-directory`;
- `does-path-exist`;
- both operands of `rename-path`;
- the pathname operand of `open-sqlite` and `backup-sqlite`.

Their explicit provider capability parameter remains argument zero. Operations
on acquired `file-reader`, `file-writer`, `file-appender`, `directory`, and
SQLite resource values require their exact declared kinds.

Only `lkjscript-sys` constructs NUL-terminated host buffers and enters FFI.
Production and acceptance paths do not use lossy string conversion. Existing
errno translation, stale and wrong-kind resource rejection, hard deadlines,
resource bounds, and language `result` values remain authoritative.

## Representation And Verification

`path` is a distinct HIR, verified-SSA, bytecode, evaluator, and VM type.
Dynamic path owners use deterministic generation-safe storage, charge bounded
structural owner and byte limits, and export key-free snapshots. Bytecode
validation rejects old `string` pathname stacks, malformed conversion stacks,
and legacy aggregate routes that would contain a path owner. Exact path storage
mode, structural-copy metadata, evaluator/VM cleanup, borrowed host access, and
returned-owner transfer are independently checked; generic aggregate envelopes
still require their own explicit execution acceptance.

Native tiers reject a `path` operation before source effects unless that tier
implements its exact layout and operation contract. Forced tiers never fall
back. Auto execution keeps unsupported `path`-bearing entries in the VM.

Language, typed-HIR, verified-SSA, bytecode, module/package, and native-layout
contract digests include this cutover. Stale source locks and artifacts fail
closed.

## Rejected

- pathname aliases from `string`;
- replacement decoding or `to_string_lossy`;
- implicit current-directory, environment, home, or registry lookup;
- silent normalization or canonicalization;
- interior-NUL truncation;
- treating package/module paths as runtime `path` values;
- claiming symlink containment from absolute bytes alone.
