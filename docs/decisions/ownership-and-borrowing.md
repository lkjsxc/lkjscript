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
Owned T
Ref T
RefMut T
GcRef T
PinnedRef T

borrow/ place /borrow
borrow-mut/ place /borrow-mut
ref-read/ reference /ref-read
ref-write/ reference value /ref-write
move/ place /move
drop/ place /drop
```

Candidate B is **Selected**. The punctuation and `SharedRef`/`ExclusiveRef`
aliases are rejected and are not accepted syntax. `Ref` is shared and `RefMut`
is exclusive; mutability is therefore visible in both type and operation.

The first non-escaping slice infers every reference region and uses `Ref T` or
`RefMut T`. A later function that exposes a lexical reference declares region
names in one `regions/ ... /regions` child adjacent to its signature and writes
`Ref region-a T` or `RefMut region-a T`; that named-region form is an
**Accepted Target**, not Current syntax. Nested type parsing remains exact.

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

The first implementation slice is a complete safe island around `Owned Buf`:

- `owned-buf-new` creates a fresh `Owned Buf` that cannot have a pre-existing
  alias;
- whole local/parameter places only;
- explicit `move` for ownership transfer;
- `borrow` and `borrow-mut` create non-escaping `Ref Buf` and `RefMut Buf`;
- owned-buffer read operations require `Ref Buf`, and writes require `RefMut
  Buf`;
- last-use dataflow ends a local borrow before lexical scope end where proved;
- branch state joins are exact, while unsupported loop-carried loans,
  reborrows, field/index places, return/storage of references, and partial moves
  are rejected;
- SSA retains move, borrow, ownership, loan, and alias identities even though
  the VM representation remains the existing safe arena handle.

The initial owned-buffer operations consume typed references directly; general
`ref-read`/`ref-write` syntax is rejected until place projection is implemented.
This slice does not silently make legacy `Buf`, `Handle`, product, or collection
uses affine; those remain worker-local GC/capability values until migrated by a
later breaking contract. It establishes a sound ownership path without
modifying canonical Brainfuck source. Deterministic source `Drop`, resource
RAII, named regions, arbitrary `Owned T`, and borrow-aware existing host
operations remain **Accepted Targets**. Runtime session cleanup remains the
Current final backstop and is not called language `Drop`.

Native GC references are added as a separate ownership category rather than
pretending they are lexical borrows.

## Deferred And Rejected

Full partial moves, closure values, pinned source APIs, cross-worker transfer,
and collection element borrows are **Deferred** until their matrices pass.
Lexical-to-function-end approximation as the final model, implicit copies of
non-`Copy` values, raw safe pointers, conservative lifetime extension, and
source-asserted unsafe thread transfer are **Rejected**.
