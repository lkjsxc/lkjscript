# Ownership And Borrowing

## Purpose

Define the Rust-grade ownership direction and one canonical AI-authored syntax
without claiming that the complete borrow conformance matrix is implemented.

## Status

Current lkjscript has immutable values, function-local `var`/`set`, worker-local
traced heap handles, and runtime-owned resource handles, but no source borrow
syntax or ownership checker. The model below is an **Accepted Target**. It
becomes Current only in separately recorded sound slices. “Full Rust-style
borrow checking” remains an invalid claim until the declared matrix passes.

## Ownership Categories

The language distinguishes:

- affine uniquely owned non-`Copy` values;
- lexical shared references;
- lexical exclusive mutable references;
- worker-local GC references;
- immutable cross-worker shared values;
- explicitly pinned native-facing values.

A GC reference is not a lexical borrow. It is traced, worker-local by default,
and cannot expose a raw object address. A pinned reference is explicit and
bounded. Ordinary safe source has no raw pointer type.

Unit, Bool, I64, and F64 are `Copy`. Other types are `Copy` only through the
resolved core-trait rules. Runtime handles, unique buffers/containers, exclusive
references, and values with `Drop` are non-`Copy`. A consuming use moves a
non-`Copy` value; copying it silently is a compile error.

## Syntax Candidates

Candidate A used punctuation modeled after Rust:

```text
& T
&mut T
& place
&mut place
* reference
```

It is compact for humans but overloads punctuation and line parsing, makes
machine-generated region annotations less visible, and gives `*` both numeric
and dereference meanings.

Candidate B uses exact operation and type atoms:

```text
SharedRef region-a T
ExclusiveRef region-a T
GcRef T
PinnedRef T

borrow-shared/ place /borrow-shared
borrow-exclusive/ place /borrow-exclusive
ref-read/ reference /ref-read
ref-write/ reference value /ref-write
move/ place /move
drop/ place /drop
```

Candidate B is **Selected**. The punctuation candidate is rejected and is not
an alias.

A function that exposes a lexical reference declares region names in one
`regions/ ... /regions` child adjacent to its signature. Region names use the
`region-` prefix. Locally inferred borrows do not require a source region name.
Nested type parsing is exact, so `SharedRef region-a List I64` has one meaning.

A place is not an arbitrary expression. Canonical place forms are a local name,
`field-place/ owner field /field-place`, `index-place/ owner index
/index-place`, and `deref-place/ reference /deref-place`. Field/index places
must retain the owner and exact element layout.

## Places, Moves, And Initialization

HIR records a `PlaceId` and move path for every local, product field, indexed
container element where statically representable, and dereferenced reference.
A place has initialized, moved, shared-borrowed, or exclusively-borrowed state
at each CFG point. Uses in consuming positions require explicit `move` for
non-`Copy` source values; `clone` is a resolved trait method, not an implicit
copy.

The first sound implementation may reject partial moves of products and indexed
containers. It must diagnose “partial move unsupported” before changing state;
it cannot copy or invalidate the entire owner silently. Full field-sensitive
partial moves remain an **Accepted Target**.

## Borrow Rules And NLL

The checker rejects use/double-move, moving a borrowed owner, dangling or
escaping borrows, overlapping exclusive borrows, an exclusive borrow
concurrent with any shared borrow, invalid reborrow, cross-function mutable
aliasing, and shorter-region storage in a longer region.

Borrow liveness is computed over HIR/SSA control flow from creation through the
last reachable use, including branch joins and loop fixed points. A borrow does
not last to function end when dataflow proves an earlier last use. Reborrowing
an exclusive reference temporarily suspends the parent reference; the parent
becomes usable only after every child region ends.

Returning or storing a reference is accepted only when its region is a declared
input region that outlives the destination. References to locals and temporary
fields cannot escape through return, products, collections, closures, GC
objects, or runtime calls whose signature lacks the exact region.

Closure capture is checked as move/shared/exclusive capture. Mutable capture is
rejected unless one exclusive region covers the closure lifetime and call
contract. Cross-worker transfer additionally requires compiler-derived `Send`.

## Drop

The compiler elaborates deterministic drops in reverse initialization order at
normal scope exit, early return, structured trap/exit cleanup edges, and branch
joins. A moved value is not dropped. Every initialized owned value is dropped
exactly once. Explicit `drop` consumes its place and prevents later use.

Drop operations are explicit in ownership-resolved HIR and SSA. Collector
tracing never runs source `Drop`; finalization is deterministic program control,
not GC policy. Partially initialized or partially destroyed values are not
source-visible.

## Verification Boundary

Ownership analysis runs after name/type resolution and before ordinary SSA
construction. SSA retains ownership kind, move/drop operations, region facts,
place/alias class, and source origin. Verification runs after elaboration and
after every pass and rejects a transformation that invents an alias, crosses a
region, loses/doubles a drop, exposes uninitialized storage, or moves a live
root illegally.

## Initial Sound Slice

The first implementation slice is deliberately narrower than this full model:
locals and whole immutable product fields; explicit moves; shared/exclusive
borrows with CFG last-use liveness; reborrow of locals; no closure escape; no
partial move; and deterministic drop for compiler-known resource values. Each
unsupported place or escape is rejected explicitly. Native GC references are
then added as a separate ownership category rather than pretending they are
borrows.

## Deferred And Rejected

Full partial moves, closure values, pinned source APIs, cross-worker transfer,
and collection element borrows are **Deferred** until their matrices pass.
Lexical-to-function-end approximation as the final model, implicit copies of
non-`Copy` values, raw safe pointers, conservative lifetime extension, and
source-asserted unsafe thread transfer are **Rejected**.
