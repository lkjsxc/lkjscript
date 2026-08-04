# Language Limits

## Purpose

Define fixed source budgets for this language version.

## Status
<!-- LKJ-F limit-inventory current c8woKdU31j7BiHTAlVYLEpwlLIPMMFnqmeL2JB0qL0c -->

**Current.** All values are enforced through shared language constants.

## Constants

Defined in `lkjscript-core/src/limits.rs`:

- `MAX_NEST_DEPTH`: 8
- `MAX_CHILDREN`: 16 expressions under one form
- `MAX_TOKENS_PER_FILE`: 384
- `MAX_TOPLEVEL_FORMS`: 8
- `MAX_DIR_CHILDREN`: 16 files plus subdirectories in one source directory
- `MAX_PRODUCT_FIELDS`: 15 fields in one nominal product declaration
- `MAX_LIST_EQUAL_STEPS`: 1,000,000 pair-node comparisons in one `equal-list`
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

## Structural Runtime Limits

A structural value has no 4,096-node language rule. The default production runtime accepts at most 65,536 semantic
nodes in one private flat-image publication; this is a lowerable resource-profile quota. The implementation safety
maximum is 1,048,576 nodes and the flat image's separate addressability maximum is `u32::MAX` local node IDs. Payload
bytes, fields per node, depth, live domains, roots, objects, and release work are independently classified limits.

Live domain/root/object capacity is released exactly once and can be reused. Cumulative construction, clone, and release
work never decreases; peak counters retain their high-water mark. Exhaustion before publication returns a typed outcome
and publishes no partial root. Runtime IDs and tuning values are not source or wire values.

The deterministic affected-limit inventory is:

```sh
cargo run --locked -p lkjscript-xtask -- limits --json
```

Its Current scope covers structural image/runtime and repository graph/query limits. Equal values elsewhere, including
4,096-block verifier safety maxima and 4,096-operation specialization work, remain independent authorities.

## Semantic Source Foundation Safety Maxima

Always-enforced implementation maxima in
`lkjscript-compiler/src/source/mod.rs` reject more than:

- 16 MiB of exact input bytes in one source file;
- 256 MiB of exact input bytes in one loaded source closure;
- 65,536 source units in one loaded closure; or
- 65,536 entries traversed by one complete source-tree check.

Opened regular files are read through the smaller remaining per-file/closure
allowance plus one sentinel byte; metadata/read changes, non-regular files, and
checked-arithmetic overflow fail before parsing. Import and source-tree
traversals use explicit stacks. These are implementation safety maxima, not
replacement language-shape limits or host profiles.

## Source-Directory Rule

An lkjscript source directory may contain at most 16 immediate entries,
counting files and subdirectories together. Hidden source entries count.

The rule applies to language source/package directories. It does not constrain
Rust crates, documentation, repository metadata, `.git`, Cargo `target`, or
other generated build trees outside language source directories. If any file or
directory, including `.git` or `target`, is placed inside a language source
directory, it counts. The repository gate recursively checks the in-tree `src`
corpus. Compilation checks every directory reached by an entry or import,
including external projects. Enumeration rejects as soon as entry 17 is seen;
directory-read failures are errors, and opened source descriptors cannot escape
package containment.

## Policy

Canonical-source shape limits remain fixed implementation contracts, not user
configuration, until aggregate profile replacements are Current. A change
requires authority updates, boundary tests, and one platform-revision cut. Foundation
maxima now bound exact source bytes and source-unit/tree counts; aggregate
import edges, source-schema nodes, type/trait/compiler work, compiler memory,
constants, globals, and other categories still need named resource profiles.
