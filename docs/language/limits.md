# Language Limits

## Purpose

Define fixed source budgets for this language version.

## Status

Per-form and per-file limits are **Current**. The source-directory limit is an
**Accepted Target** replacing the current repository-wide eight-entry gate.

## Current Constants

Defined in `lkjscript-core/src/limits.rs`:

- `MAX_NEST_DEPTH`: 8
- `MAX_CHILDREN`: 16 expressions under one form
- `MAX_TOKENS_PER_FILE`: 384
- `MAX_TOPLEVEL_FORMS`: 8
- `MAX_DIR_CHILDREN`: currently 8 and incorrectly used as a repository-layout gate

`MAX_CHILDREN` and the source-directory rule are separate contracts even though
both accepted values are 16.

## Accepted Source-Directory Rule

An lkjscript source directory may contain at most **16 immediate entries**,
counting files and subdirectories together.

The rule applies to directories participating in an lkjscript source/package
tree. It does not constrain Rust crates, documentation, repository metadata,
`.git`, Cargo `target`, or other generated build trees. The in-tree gate checks
the complete `src` corpus, and compilation checks imported external source
directories.

Hidden source entries are not a loophole: if an entry is inside a language
source directory, it counts. Read failures are errors rather than silent
success. Symlinks cannot be used to evade counting or package containment.

## Policy

Limits are hardcoded language-version constants, not user configuration. A
change requires documentation, boundary tests, and an explicit language
contract update. Large raw strings and broad import graphs need aggregate byte
and import limits in a later resource-safety cycle because token count alone
does not bound them.
