# Syntax And Semantics: Files And Imports

[Authority](../syntax.md)

## Status

**Current.** Module and Linux runtime pathname rules are implemented.

## Files And Modules

Every source file ends in `.lkjscript` and is one module identified by its
exact package-root-relative UTF-8 path. `.lkjml`, extensionless source,
absolute module names, dot-relative names, parent components, malformed
separators, and path or symlink escape are rejected.

An executable target resolves one root with exactly one `main`. Its structured
signature carries the exact sorted typed capability parameters required by its
closed call graph. Imported modules cannot provide another root entry.

One bounded `imports/` envelope contains sorted records:

```text
import/
module/
src/std/fs/read-all.lkjscript
/module
declarations/
read-all
/declarations
/import
```

Each record names one exact module and a sorted declaration set. Wildcards,
ambient roots, environment lookup, installed fallback, private names,
collisions, transitive visibility, cycles, and unresolved imports fail closed.
Declarations are private unless they contain `public`.

## Text, Byte Data, And Runtime Paths

`string`, transitional `buf`, and `path` are disjoint. `string` owns valid
UTF-8. `path` stores immutable exact absolute Linux pathname bytes and is not
text. Current `buf` remains the bounded mutable byte representation while the
accepted `bytes` and affine `byte-vector` migration is incomplete; it is not an
alias for either destination type.

`convert-string-to-path` and `convert-buf-to-path` are the Current constructors.
They reject empty, relative, NUL-containing, and longer-than-4095-byte values.
`convert-path-to-buf` returns an independent exact copy;
`convert-path-to-string` performs strict UTF-8 validation. Filesystem and SQLite
pathname operations accept only `path`. No conversion normalizes, searches,
consults ambient state, or decodes with replacement.

The ownership foundation exposes direct `byte-vector`, `byte-slice`, and
`byte-slice-mut` spellings for the existing bounded whole-place affine slice.
Immutable `bytes` uses the exact `bytes-literal/` lowercase hexadecimal
projection and executes in the evaluator and VM. Ranged borrowed views and
borrowed text `str` remain non-Current and are rejected.
