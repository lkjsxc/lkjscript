# Ownership And Borrowing

## Purpose

Define the Rust-grade ownership direction and one canonical AI-authored syntax
without claiming that the complete borrow conformance matrix is implemented.
## Status

The **Initial Sound Slice** below is **Current** for fresh `Owned Buf` values,
whole-local moves, and non-escaping `Ref Buf`/`RefMut Buf` loans. `Handle` is
also a Current affine resource: acquired locals must be explicitly returned,
moved, or `drop`ped; double drop and use after move/drop are compile errors.
Handle parameters are borrowed unless an explicit `move/` transfers ownership.
Generic `drop` consumes owned handles through verified SSA and the existing
resource-table close operation; SQLite close/finalize are consuming operations.
Program teardown remains a deterministic safety net, not a substitute for
static local cleanup. Products and collections containing affine resources,
general regions, and full Rust-style borrow checking remain **Accepted
Targets**.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Ownership Categories](ownership-and-borrowing/ownership-categories.md)
- [Initial Sound Slice](ownership-and-borrowing/initial-sound-slice.md)
