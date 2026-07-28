# Ownership And Borrowing

## Purpose

Define the Rust-grade ownership direction and one canonical AI-authored syntax
without claiming that the complete borrow conformance matrix is implemented.
## Status

The **Initial Sound Slice** below is **Current** under direct source spellings
`byte-vector`, `byte-slice`, and `byte-slice-mut`: fresh vectors move as whole
locals and views are bounded, lexical, and non-escaping. The former `owned
buf`, `ref buf`, and `ref-mut buf` source forms are removed.

Exact typed resources have an implemented affine foundation: acquired locals
must be returned, moved, or explicitly dropped; double drop and use after
consumption are compile errors. Parameters are borrowed unless `move` transfers
ownership. Resource-bearing aggregates are rejected. Compiler-inserted whole-place cleanup
is Current on lexical, branch, terminator, and instruction-failure edges in the
evaluator and VM, while native owned-resource host execution remains an
**Accepted Target**. The complete typed-resource contract is therefore not
Current.

`ExecutableProgram` retains the complete independently verified pre-backend HIR
memory plan and an independently recomputed SSA inventory for this direct affine
slice. The plan records every Current HIR result, exact call signature,
owner/place/loan, storage axis, tracing registration, and drop obligation. It
now drives closed affine-place glue metadata plus explicit verified SSA
loan-end/drop events. Static, dead, and statically decidable conditional whole
places are elaborated at normal lexical, branch-join, explicit source
terminators, and verified instruction-failure sites; active typed resources
still require move, return, or explicit close. Native owned-resource cleanup,
complete physical byte release, and deterministic storage remain governed by the
[collector-free memory contract](../memory/collector-free-deterministic-memory.md)
and are not Current.

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Ownership Categories](ownership-and-borrowing/ownership-categories.md)
- [Initial Sound Slice](ownership-and-borrowing/initial-sound-slice.md)
