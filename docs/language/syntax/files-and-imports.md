# Syntax And Semantics: Files And Imports

[Authority](../syntax.md)

## Status

**Current.** Module and runtime pathname rules are implemented for the Current
Linux x86-64 platform.

## Files And Modules

Every source file ends in `.lkjscript` and is one module identified by its
exact package-root-relative UTF-8 path. `.lkjml`, extensionless source, absolute
module names, dot-relative names, parent components, malformed separators, and
path or symlink escape are rejected before publication.

Imported modules contain explicit declarations. An executable target resolves
one root with exactly one `main`; its signature carries the exact sorted typed
capability parameters required by its closed call graph. No imported module may
provide another root entry.

One bounded `imports/` envelope contains sorted `import/` records of the form:

```text
import/
src/std/fs/read-file.lkjscript#read-file
/import
```

Each record names an exact module and sorted declaration set. Wildcards,
ambient roots, environment lookup, installed fallback, private names,
collisions, transitive visibility, cycles, and unresolved imports fail closed.
Declarations are private unless they contain the explicit `public` field.

## Strings, Bytes, And Runtime Paths

`Str`, `Buf`, and `Path` are disjoint. `Str` stores valid UTF-8. `Buf` is the
Current bounded mutable byte value for file contents, sockets, entropy,
SHA-256, and SQLite blobs. Runtime Linux `Path` stores immutable exact absolute
pathname bytes; it is unrelated to UTF-8 module identity.

`path-from-str` and `path-from-buf` are the only constructors. They reject
empty, relative, NUL-containing, and longer-than-4095-byte values.
`path-to-buf` returns an exact independent copy; `path-to-str` performs strict
UTF-8 validation. Filesystem and SQLite pathname operations accept only
`Path`. No conversion normalizes, searches, consults ambient state, or decodes
with replacement.

String operations continue to reject invalid UTF-8 boundaries rather than
imply character indexing. Buffer offset/length checks and partial-progress
counts remain exact.
