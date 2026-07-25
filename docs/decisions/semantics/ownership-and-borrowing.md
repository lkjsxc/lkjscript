# Ownership And Borrowing

## Purpose

Define the Rust-grade ownership direction and one canonical AI-authored syntax
without claiming that the complete borrow conformance matrix is implemented.
## Status

The **Initial Sound Slice** below is **Current**. It is one deliberately narrow
safe island for fresh `Owned Buf` values, whole-local moves, and non-escaping
`Ref Buf`/`RefMut Buf` loans. Legacy `Buf`, handles, products, and collections
retain their previous semantics. The broader model remains an **Accepted
Target**; “full Rust-style borrow checking” is still an invalid claim.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Ownership Categories](ownership-and-borrowing/ownership-categories.md)
- [Initial Sound Slice](ownership-and-borrowing/initial-sound-slice.md)
