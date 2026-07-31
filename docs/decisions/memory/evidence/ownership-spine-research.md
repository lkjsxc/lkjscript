# Ownership Spine Research Inputs

## Status

**Engineering evidence record.** External mechanisms constrain design but do
not establish lkjscript implementation correctness.

## Rust Drop Elaboration

The Rust Compiler Development Guide classifies MIR drops as static, dead,
conditional, or open using independent maybe-initialized and
maybe-uninitialized place analyses. lkjscript adopts those dataflow principles,
whole-place conditional flags, and generated cleanup. It does not adopt Rust
syntax, panic unwinding, unsafe rules, arbitrary user destructors, or partial
aggregate drop in this slice.

Source: <https://rustc-dev-guide.rust-lang.org/mir/drop-elaboration.html>.

## Swift Ownership SSA

The SIL ownership model makes owned and guaranteed conventions, borrow scopes,
destroy operations, and statically available versus unavailable values
explicit in SSA and whole-function verification. lkjscript adopts explicit
availability, moves, borrows, borrow ends, destroys, and ownership-preserving
optimization. It does not adopt Swift ARC or object semantics.

Sources: <https://github.com/swiftlang/swift/blob/040158599072ecfbfdf7fcc6fc2aafd3e1ab219e/docs/SIL/Ownership.md>
and the ownership/linear-lifetime verifier implementation at the same Swift
revision. The revision was checked on 2026-07-30.

## Perceus And Lean

Perceus derives consume/borrow conventions, precise release, and reuse from
liveness in a linear resource calculus; Lean exposes ownership and reset/reuse
operations in compiler IR. lkjscript adopts explicit consume/borrow facts,
last-use destruction, unique backing transfer, and later reuse opportunities.
It does not adopt universal reference counting or assume acyclicity for
unmigrated structures.

Sources: DOI `10.1145/3453483.3454032` and
<https://github.com/leanprover/lean4>.

## Automatic Borrowing And Place Dataflow

The automatic-borrow objective is ordinary source without lifetime names,
heavier sharing only after borrowing fails, and exact interface conventions.
Inaccessible paper details are not invented. Polonius informs place-sensitive
issuance, subset/liveness constraints, invalidation, kills, and CFG fixed
points; it is not a dependency or source-language memory system.

Sources: DOI <https://doi.org/10.1145/3798221>, its ACM supplement, and
<https://github.com/rust-lang/polonius>. The paper's reference-count insertion
fallback is explicitly rejected; inaccessible or pure-language-only details do
not establish mutable aggregate conformance.

## Destination Passing

Direct construction into final owner storage is permitted only when capacity,
initialization, failure cleanup, aliasing, and caller-destination escape are
verified. It is an optional optimization, not required to establish the first
island.

Sources: DOI <https://doi.org/10.1145/3720423> and author preprint
<https://arxiv.org/abs/2503.07489>. The linear hole/destination and complete-
initialization reasoning is adopted internally; first-class destination or age
syntax is rejected.

## Implicit Sized Regions And Lean Compaction

Spegion provides implicit non-lexical region and sized-allocation evidence, not
a replacement for affine drop or a production performance result. Lean's
compacted region implementation provides evidence for contiguous immutable
sealed graphs with explicit external dependencies, not proof that Lean's
runtime representation fits lkjscript.

Sources: <https://doi.org/10.4230/LIPIcs.ECOOP.2025.15> and Lean revision
`f696c4686b327b271b3488d757dc4fcd80f298ce`, especially
`src/Lean/CompactedRegion.lean` and `src/runtime/compact.cpp`.

## Tool Boundary

Miri borrow models and sanitizers are implementation oracles for unsafe or host
boundaries. They are not the source-language ownership proof or reclamation
system.
