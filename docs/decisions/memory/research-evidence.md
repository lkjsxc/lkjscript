# Collector-Free Memory Research Evidence

## Status

Research record for the accepted contract. It does not promote production
capabilities. Sources were checked through 2026-07-29; inaccessible details are
not assumed.

## Automatic Borrowing And Precise Counting

Brandon et al., *Fully-Automatic Type Inference for Borrows with Lifetimes*,
OOPSLA 2026, [DOI 10.1145/3798221](https://doi.org/10.1145/3798221), reports in
its accessible abstract a pure functional borrow/lifetime system with automatic
inference, count insertion when borrowing cannot type a program, a memory-safety
theorem, 75--100% fewer increments on affected benchmarks, and a 1.48x overall
geometric-mean speedup. The full paper was inaccessible through ACM and no
author manuscript was found. Constraint rules, inference completeness,
polymorphism, closures, and implementation proof scope remain uncertain.
Lkjscript adopts the source-level hypothesis, not unreviewed rules.

Reinking et al., *Perceus: Garbage Free Reference Counting with Reuse*, PLDI
2021, [DOI 10.1145/3453483.3454032](https://doi.org/10.1145/3453483.3454032),
uses owned/borrowed conventions, explicit `dup`/`drop` in a linear resource
calculus, uniqueness tests, reset, and constructor reuse. It establishes its
garbage-free result under functional acyclic-heap assumptions. It does not
reclaim arbitrary cycles; recursive decrements, synchronization, and physical
reuse costs remain. Lkjscript adopts checked ownership operations, count
coalescing, uniqueness specialization, and reuse, but not universal counting.
Lean's Perceus-derived pass decomposition is useful production evidence. The
review pins `InferBorrow`, RC insertion/coalescing, reuse, and compacted-region
code to Lean commit `a39eab69e1eee9ad38f4efe507907b1026a77808`.

## Modes, Isolation, And Concurrency

Lorenzen et al., *Oxidizing OCaml with Modal Memory Management*, ICFP 2024,
[DOI 10.1145/3674642](https://doi.org/10.1145/3674642), separates inferred
locality, uniqueness, and affinity. Locality supports stack placement and
unique-affine access supports safe destructive update. The work does not remove
OCaml's tracing heap. Lkjscript adopts independent internal mode axes and rejects
copying compatibility-driven surface complexity.

*Data Race Freedom a la Mode*, POPL 2025,
[DOI 10.1145/3704859](https://doi.org/10.1145/3704859), adds portability,
contention, capsules, ghost region keys, and lock authority. Its Iris/Rocq model
establishes data-race freedom for the modeled primitives, not deterministic
reclamation. Lkjscript adopts portability/contention facts and unique isolation
authority; shared capsules remain deferred.

Arvidsson et al., *Reference Capabilities for Flexible Memory Management*,
OOPSLA 2023, [DOI 10.1145/3622846](https://doi.org/10.1145/3622846), partitions
objects into isolated region forests with one active mutability window and
local policy. Lkjscript adopts isolation and explicit mutation authority, but
rejects arbitrary local tracing policies and wholesale adoption of Verona's
object model.

Jung et al., *Concurrent Immediate Reference Counting*, PLDI 2024,
[DOI 10.1145/3656383](https://doi.org/10.1145/3656383), combines safe memory
reclamation protection with prompt decrements and immediate linked-structure
reclamation. Atomics, epochs/hazards, cycle handling, scheduler interaction, and
non-deterministic concurrent timing make it unsuitable as the baseline. It is
deferred for an explicitly shared future facility only.

## Regions And Non-Local Control

Tofte and Talpin, *Region-Based Memory Management*, Information and Computation
1997, [DOI 10.1006/inco.1996.2613](https://doi.org/10.1006/inco.1996.2613),
infers region variables/effects and explicit bulk destruction for ML. Its
lexical/LIFO discipline is safe but can over-retain and poorly model arbitrary
lifetimes. Lkjscript adopts inference and bulk destruction, not lexical regions
as the only heap discipline.

Aiken, Fahndrich, and Levien, *Better Static Memory Management*, PLDI 1995,
[DOI 10.1145/207110.207137](https://doi.org/10.1145/207110.207137), strengthens
higher-order region analysis with constraints. Full primary review was not
completed. The adopted lesson is limited to treating region lifetime as a CFG
dataflow problem.

Hughes, Vollmer, and Batty, *Spegion*,
[arXiv 2506.02182](https://arxiv.org/abs/2506.02182), presents implicit
non-lexical, splittable regions and sized allocations with an effect system and
type-safety result. Input-dependent sizes, recursion, fragmentation, and
compiler maturity remain concerns. Lkjscript experiments with split regions and
capacity effects while retaining exact aggregate runtime budgets.

Mathiasen, Timany, and Birkedal, *Yarrow*,
[arXiv 2607.15876](https://arxiv.org/abs/2607.15876), supplies mechanized
separation-logic reasoning for regions with one-shot and multi-shot effect
handlers. It does not provide automatic region inference or collector removal.
Algebraic effects remain deferred; multi-shot capture must never silently retain
stack regions.

## Pure Borrowing And Destination Passing

Matsushita and Ishii, *Pure Borrow*, PLDI 2026,
[DOI 10.1145/3808259](https://doi.org/10.1145/3808259), accessible as
[arXiv 2604.15290](https://arxiv.org/abs/2604.15290), supports split/dropped
borrowers, affine mutable references, polymorphism, laziness, and parallel
mutation in Linear Haskell. Its wording describes metatheory toward safety,
leak freedom, and confluence rather than an end-to-end mechanized compiler
proof. Lkjscript adopts the requirement that non-local borrowing remain
AI-friendly, but proves it in its eager value model instead of importing the
library API.

Destination-passing style, FHPC 2017,
[DOI 10.1145/3122948.3122949](https://doi.org/10.1145/3122948.3122949), and
*Destination Calculus*, PLDI 2025,
[DOI 10.1145/3720423](https://doi.org/10.1145/3720423), construct results in
caller storage under linearity and scope conditions. Partial initialization,
aliasing, failure cleanup, size, and alignment remain hazards. Lkjscript adopts
DPS only inside verified SSA with unique destinations, initialization facts,
capacity, and cleanup obligations.

## Typed Pools And Sparse Deletion

Typed generational arena and ECS documentation was checked for
[`generational-arena`](https://docs.rs/generational-arena/0.2.9/),
[`slotmap`](https://docs.rs/slotmap/1.1.1/), and
[Bevy ECS entity identity](https://docs.rs/bevy_ecs/0.19.0/bevy_ecs/entity/struct.Entity.html).
The adopted mechanism combines runtime pool identity, typed class, slot, and
nonwrapping generation. Lookup creates checked temporary borrows; stale IDs
fail; exhausted slots retire; cyclic links are non-owning IDs. Raw indices,
untyped keys, raw pointers, silent wrap, and cross-world identity assumptions
are rejected.

Berger, Zorn, and McKinley, *Reconsidering Custom Memory Allocation*, OOPSLA
2002, [DOI 10.1145/582419.582421](https://doi.org/10.1145/582419.582421),
introduces reaps to combine region allocation with individual deletion. It is
retained as evidence that bulk-only regions may over-retain sparse long-lived
workloads. General `free` and untyped reap semantics are rejected; split/reset
regions or typed pools must be tested first.

Lean's compacted regions retain contiguous immutable graphs without per-object
retain/release and record cross-region dependencies explicitly. Lkjscript adopts
that region-level shape but rejects caller-enforced unsafe `free`, erased read
types, and unchecked dependency lifetimes. Safe typed domain/root identities
and exact release ledgers remain mandatory.

## Place-Sensitive Borrowing

[Polonius](https://github.com/rust-lang/polonius) models CFG propagation,
issuance, invalidation, killing, and place conflicts. Its 2026 Alpha project goal
shows that the stable Rust subset is still evolving. Lkjscript adopts bounded
place-sensitive dataflow over resolved SSA, not Rust surface syntax or a
wholesale solver dependency.

Tree Borrows models reborrow provenance and access-state transitions for unsafe
Rust/Miri. Its primary project page was inaccessible during research; the
[Miri implementation](https://github.com/rust-lang/miri/tree/master/src/tree_borrows)
was located. Lkjscript treats it as future FFI/unsafe-boundary oracle evidence,
not safe-source runtime metadata or a reclamation system.

## Adopted Synthesis

The accepted synthesis is inferred place-sensitive borrowing, separate mode
axes, bounded non-lexical regions, sealed immutable shared regions, typed pools,
verified destination passing and reuse, and explicit SSA ownership events.
Precise immutable acyclic counting is permitted only after retained no-RC
falsification. Concurrent counting and shared mutable capsules remain deferred.
No cited result justifies tracing fallback, unbounded release, source lifetime
syntax, or moving proof obligations to users.
