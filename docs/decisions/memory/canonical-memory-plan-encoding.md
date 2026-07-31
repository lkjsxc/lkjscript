# Canonical Memory Plan Encoding

[Authority](authoritative-memory-plan.md)

## Status

**Current.** HIR memory-plan and exact-witness identities use the closed
platform-revision-bound canonical binary projection defined here. Rust `Debug`, display text,
addresses, map order, allocator state, and process state are not identity inputs.

## Encoding

The plan domain is `lkjscript.hir-memory-plan\0canonical-platform-contract`;
the witness domain is `lkjscript.memory-witness\0canonical-platform-contract`. A SHA-256 digest of the domain-framed
projection is the identity. Strings and byte sequences use an unsigned 64-bit
big-endian length followed by exact bytes. Vectors use an unsigned 64-bit
big-endian element count followed by elements. Options use tag 0 for absent and
1 for present. Booleans use 0 or 1. Integers use fixed-width big-endian bytes.
Closed enum variants use explicit stable one-byte tags; payloads follow in field
order. Dense IDs use their unsigned 32-bit value. Structural 256-bit identities
use one length-framed 32-byte field.

Every struct implementation exhaustively destructures all fields, so adding a
field without encoding it fails compilation. Every enum match is exhaustive, so
adding a variant without assigning a tag fails compilation. Plan records are
encoded in their dense table order. The plan's own ID is excluded; all other
plan fields, including complete work accounting, are included. Witness IDs hash
witness facts only; plans include both witness IDs and facts.

All length growth and encoder reservation is checked. Failure is a compile-time
resource error before verified HIR publication, never a fallback identity.
Changing a tag, field order, domain, or included fact is a public contract change
that requires the sole platform revision and regenerated package artifacts.
There is no prior textual compatibility mode.

## Evidence

Focused tests retain exact plan and witness digest vectors, derive the same IDs
from repeated production, change IDs when declaration facts change, and change
the plan ID when work accounting changes. The independent memory verifier
recomputes every witness and the complete plan from canonical records before SSA.
Repository search rejects the removed `Debug`-formatted identity domains.
