# Syntax And Semantics: Files And Imports

[Authority](../syntax.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Files And Imports

Imported files contain only `import`, immutable function `def`, `product`,
marker `trait`, and exact marker `impl` declarations. An executable root
contains those declarations plus exactly one
`main/ sig/ -> T /sig body-expression /main`. Main has no parameters, its body
has exactly `T`, and `arg` remains the script-argument operation. A main in an
import, no root main, a duplicate root main, top-level `do`, and non-function
`def` are compile errors. All source files remain bounded by
`MAX_TOPLEVEL_FORMS`.

Current path resolution supports:

- `std/...`, `lib/...`, and `examples/...` from package `src` trees;
- `./...` relative to the importing file;
- installed fallback through `LKJSCRIPT_ROOT` when a local category directory
  is absent.

Parent path components and absolute imports are rejected. Canonicalized paths
must remain inside the project or installed root, so symlink escapes fail.
Cycles are rejected and repeated canonical files are deduplicated. Definitions
are not namespaced modules yet.

Every entry and import must end in `.lkjscript`. `.lkjml`, extensionless source,
and unrelated extensions are rejected before parsing.
## Strings And Bytes

Strings are UTF-8 host strings and many string operations index bytes. `Buf`
is the Current lossless bounded byte-storage path for file, socket, entropy,
SHA-256, and SQLite blob operations. Offset/length checks and partial-progress
counts are exact. `buf-slice` allocates and copies its selected range; it is not
the future borrowed `Slice T`. String APIs continue to reject invalid UTF-8
boundaries rather than imply character indexing.
