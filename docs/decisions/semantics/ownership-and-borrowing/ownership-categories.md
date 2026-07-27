# Ownership And Borrowing: Ownership Categories

[Authority](../ownership-and-borrowing.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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
aliases are rejected and are not accepted syntax. `byte-slice` is shared and `byte-slice-mut`
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

**Accepted Target overall; only the whole-local `byte-vector` subset below is
Current.** HIR records a `PlaceId` and move path for every local, product field, indexed
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

**Accepted Target overall; Current NLL is the conservative same-block subset
below.** The checker rejects use/double-move, moving a borrowed owner, dangling or
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

**Accepted Target; not Current.** The compiler will elaborate deterministic drops in reverse initialization order at
normal scope exit, early return, structured trap/exit cleanup edges, and branch
joins. A moved value is not dropped. Every initialized owned value is dropped
exactly once. Explicit `drop` consumes its place and prevents later use.

Drop operations are explicit in ownership-resolved HIR and SSA. Collector
tracing never runs source `Drop`; finalization is deterministic program control,
not GC policy. Partially initialized or partially destroyed values are not
source-visible.
## Verification Boundary

Ownership analysis runs after name/type resolution and before ordinary SSA
construction. The Current slice retains explicit place initialization/end,
move/borrow, owner transport, loan identity, and source origin. Verification
runs after elaboration and every pass and independently checks this subset.
General region facts and deterministic Drop verification remain an **Accepted
Target**; they are not inferred from Current `PlaceEnd` root-liveness facts.
