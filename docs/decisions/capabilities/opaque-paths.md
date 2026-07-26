# Opaque Byte-Preserving Paths

[Authority](../../operations/status-authority.md)

## Status

**Current on Linux x86-64.** Runtime filesystem and SQLite pathname APIs accept
only the opaque immutable `Path` value. `Str` and `Buf` are not pathname values
and have no compatibility overload.

## Disjoint Foundations

- `Buf` is bounded mutable arbitrary bytes.
- `Str` is bounded immutable valid UTF-8.
- `Path` is bounded immutable native pathname bytes.

Runtime `Path` is unrelated to package and module identity paths, which remain
canonical package-root-relative UTF-8 strings.

## Construction And Observation

The closed operations are:

```text
path-from-str Str -> Result Path SystemError
path-from-buf Buf -> Result Path SystemError
path-to-buf Path -> Buf
path-to-str Path -> Result Str Utf8Error
```

A Current Linux `Path` contains 1 through 4095 bytes. Its first byte is `/`.
Empty and relative paths are rejected. Interior NUL and a 4096th byte are
rejected before FFI or an unchecked allocation. Constructors copy exact bytes;
they do not normalize, canonicalize, decode with replacement, consult the
current directory, inspect environment or home state, or search roots.

`path-to-buf` returns an independent exact byte copy. `path-to-str` performs
strict UTF-8 validation and returns the existing precise `Utf8Error` kind and
offset. `Path` has byte-value equality and immutable copy semantics; it has no
object-identity or mutation operation.

Absolute bytes remove current-directory authority but do not claim symlink
containment. Directory capabilities and `openat`-style sandboxing remain a
separate accepted target.

## Host API Cutover

These operations accept `Path` in place of `Str`, with no retained old arity or
overload:

- `sys-open-read`, `sys-open-write`, `sys-open-append`,
  `sys-open-create-new`, and `sys-open-dir`;
- `sys-path-exists`;
- both operands of `sys-rename`;
- the pathname operand of `sys-sqlite-open` and `sys-sqlite-backup`.

Their explicit provider capability parameter remains argument zero. Operations
on acquired `Handle` values remain unchanged.

Only `lkjscript-sys` constructs NUL-terminated host buffers and enters FFI.
Production and acceptance paths do not use lossy string conversion. Existing
errno translation, stale and wrong-kind handle rejection, hard deadlines,
resource bounds, and language `Result` values remain authoritative.

## Representation And Verification

`Path` is a distinct HIR, verified-SSA, bytecode, evaluator, and VM type. The VM
stores exact bytes in a traced immutable heap object. Path allocations and
copies charge existing allocation and heap-byte limits. Bytecode validation
rejects old `Str` pathname stacks and malformed conversion stacks.

Native tiers reject a `Path` operation before source effects unless that tier
implements its exact layout and operation contract. Forced tiers never fall
back. Auto execution keeps unsupported Path-bearing entries in the VM.

Language, typed-HIR, verified-SSA, bytecode, module/package, and native-layout
contract digests include this cutover. Stale source locks and artifacts fail
closed.

## Rejected

- pathname aliases from `Str`;
- replacement decoding or `to_string_lossy`;
- implicit current-directory, environment, home, or registry lookup;
- silent normalization or canonicalization;
- interior-NUL truncation;
- treating package/module paths as runtime `Path` values;
- claiming symlink containment from absolute bytes alone.
