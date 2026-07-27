# Opaque Byte-Preserving Paths

[Authority](../../operations/status-authority.md)

## Status

**Current on Linux x86-64.** Runtime filesystem and SQLite pathname APIs accept
only the opaque immutable `path` value. `string` and `buf` are not pathname values
and have no compatibility overload.

## Disjoint Foundations

- `buf` is bounded mutable arbitrary bytes.
- `string` is bounded immutable valid UTF-8.
- `path` is bounded immutable native pathname bytes.

Runtime `path` is unrelated to package and module identity paths, which remain
canonical package-root-relative UTF-8 strings.

## Construction And Observation

The closed operations are:

```text
convert-string-to-path: fn inputs string output result path system-error
convert-buf-to-path: fn inputs buf output result path system-error
convert-path-to-buf: fn inputs path output buf
convert-path-to-string: fn inputs path output result string utf8-error
```

A Current Linux `path` contains 1 through 4095 bytes. Its first byte is `/`.
Empty and relative paths are rejected. Interior NUL and a 4096th byte are
rejected before FFI or an unchecked allocation. Constructors copy exact bytes;
they do not normalize, canonicalize, decode with replacement, consult the
current directory, inspect environment or home state, or search roots.

`convert-path-to-buf` returns an independent exact byte copy. `convert-path-to-string` performs
strict UTF-8 validation and returns the existing precise `utf8-error` kind and
offset. `path` has byte-value equality and immutable copy semantics; it has no
object-identity or mutation operation.

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

Their explicit provider capability parameter remains argument zero. Operations on acquired `file-reader`, `file-writer`, `file-appender`,
`directory`, and SQLite resource values require their exact declared kinds.

Only `lkjscript-sys` constructs NUL-terminated host buffers and enters FFI.
Production and acceptance paths do not use lossy string conversion. Existing
errno translation, stale and wrong-kind resource rejection, hard deadlines,
resource bounds, and language `result` values remain authoritative.

## Representation And Verification

`path` is a distinct HIR, verified-SSA, bytecode, evaluator, and VM type. The VM
stores exact bytes in a traced immutable heap object. `path` allocations and
copies charge existing allocation and heap-byte limits. Bytecode validation
rejects old `string` pathname stacks and malformed conversion stacks.

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
