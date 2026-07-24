# Language Limits

## Purpose

Define fixed source budgets for this language version.

## Status

**Current.** All values are enforced through shared language constants.

## Constants

Defined in `lkjscript-core/src/limits.rs`:

- `MAX_NEST_DEPTH`: 8
- `MAX_CHILDREN`: 16 expressions under one form
- `MAX_TOKENS_PER_FILE`: 384
- `MAX_TOPLEVEL_FORMS`: 8
- `MAX_DIR_CHILDREN`: 16 files plus subdirectories in one source directory
- `MAX_PRODUCT_FIELDS`: 15 fields in one nominal product declaration
- `MAX_LIST_EQUAL_STEPS`: 1,000,000 pair-node comparisons in one `list-equal`
  call

`MAX_CHILDREN` and `MAX_DIR_CHILDREN` are separate contracts even though both
values are 16. The 15-field product limit leaves room for the product name plus
all constructor fields under the 16-child expression bound.
`MAX_LIST_EQUAL_STEPS` is a runtime semantic bound rather than a source-shape
bound; reaching another pair after the limit is an error.

The Current ownership safe island additionally defines
`OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES = 16_384` in
`lkjscript-compiler/src/ownership.rs`. It charges every HIR expression in every
function and main across the loaded program closure, and rejects the closure
before ownership place/loan analysis when exceeded. This deterministic compiler
analysis bound is distinct from the per-file source-shape constants above.
Typed-SSA ownership verification separately caps charged ownership dataflow
work and retained ownership-state cells at 131,072 each. General CFG
verification stores dense blocks in ID order, limits one function to 4,096
blocks, represents dominators as bitsets, and caps charged dominator work at
4,194,304 word operations. These verifier limits reject malformed or
adversarial public IR before unbounded path-state retention or quadratic
set-based dominance construction.

## Source-Directory Rule

An lkjscript source directory may contain at most 16 immediate entries,
counting files and subdirectories together. Hidden source entries count.

The rule applies to language source/package directories. It does not constrain
Rust crates, documentation, repository metadata, `.git`, Cargo `target`, or
other generated build trees. The repository gate recursively checks the in-tree
`src` corpus. Compilation checks every directory reached by an entry or import,
including external projects. Directory-read failures are errors, and symlinks
cannot escape package containment.

## Policy

Limits are language-version constants, not user configuration. A change
requires documentation, boundary tests, and a contract update. The aggregate
ownership-expression budget does not itself bound source bytes, raw-string
bytes, import depth/count, constants, globals, or bytecode; those still need
separate resource limits in a later safety cycle.
