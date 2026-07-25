# Syntax And Semantics

## Purpose

Define expression, type, and import behavior above the physical line format.
## Status

**Current.** Executable roots use exactly one explicit `main`, imported files
are declaration-only, mutation is limited to typed function-local `var`/`set`,
and the exact initial `Owned Buf` safe island below is implemented. The exact
numeric contract is
[numeric-semantics.md](../decisions/semantics/numeric-semantics.md).

## Authority And Status Vocabulary

This page is the authority for the record and its capsule manifest. Each capsule preserves one
cohesive part of the accepted record. “Current” means implemented and evidenced; “Accepted Target”
or “Accepted Implementation Selection” is a contract for future implementation; “Deferred” and
“Rejected” remain non-current. A capsule cannot promote a capability beyond the explicit status in
its text.

## Strict Capsule Manifest

- [Expressions](syntax/expressions.md)
- [Files And Imports](syntax/files-and-imports.md)
