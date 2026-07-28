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

Source: <https://forums.swift.org/t/sil-ownership-model-proposal-refreshed/16872>.

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

Sources: DOI `10.1145/3798221` and
<https://github.com/rust-lang/polonius>.

## Destination Passing

Direct construction into final owner storage is permitted only when capacity,
initialization, failure cleanup, aliasing, and caller-destination escape are
verified. It is an optional optimization, not required to establish the first
island.

Sources: DOI `10.1145/3122948.3122949`; the 2026 arXiv source named by the
contract was unavailable during this cycle, so no additional mechanism is
claimed from it.

## Tool Boundary

Miri borrow models and sanitizers are implementation oracles for unsafe or host
boundaries. They are not the source-language ownership proof or reclamation
system.
