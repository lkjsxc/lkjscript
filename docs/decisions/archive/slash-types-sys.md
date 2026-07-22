# Historical Slash, Type, And Sys Contract

## Purpose

Preserve the combined decision that introduced slash markers, mandatory types,
opaque handles, precise GC, and a typed system prelude.

## Status

**Superseded.** Its physical notation was replaced by the line-oriented format,
and its type/sys claims overstate current runtime conformance. Active contracts
live in [../../language/](../../language/README.md),
[../../runtime/](../../runtime/README.md), and
[../../current-state.md](../../current-state.md).

## Historical Direction

The decision rejected optional typing, `Any`, traits, raw integer descriptors,
and a full JIT in the initial cut. It introduced mandatory signatures,
annotation-driven `forall`, sized numeric vocabulary, `Result`, opaque
`Handle`, and precise mark-sweep GC.

The GC and mandatory-signature portions landed. Numeric widths, conversions,
truthful system Results, and handle opacity did not land completely and must
not be inferred from this historical record.

## Continuing Principles

- Slash remains structural and division is named `div`.
- `Any` is not part of the language.
- Host failures should be explicit values at the language boundary.
- Raw operating-system descriptors must not be script-visible.
- A JIT claim requires an actual native execution handoff.
