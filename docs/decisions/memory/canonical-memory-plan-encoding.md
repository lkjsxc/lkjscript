# Canonical Memory Plan Encoding

[Authority](authoritative-memory-plan.md)

## Status

**Current for memory plans; Experimental for the revision-16 authenticated
semantic-closure witness slice.** HIR memory-plan and exact-witness identities
use the closed platform-revision-bound binary projections defined here. Rust
`Debug`, display text, serde, host discriminants, addresses, map/filesystem
order, allocator state, and process state are never identity inputs.

## Encoding

The plan domain is `lkjscript.hir-memory-plan\0canonical-platform-contract`;
the witness domain is
`lkjscript.memory-witness\0canonical-platform-contract`; the semantic descriptor
and product-field domains are separately framed contracts-owned constants. A
SHA-256 digest of a domain-framed projection is the identity. Strings and bytes
use an unsigned 64-bit big-endian length and exact bytes. Vectors use an unsigned
64-bit big-endian count. Options use tags 0/1, booleans use 0/1, and integers are
fixed-width big-endian. Closed variants have explicit one-byte tags. Every
256-bit identity is a length-framed 32-byte field.

Every struct implementation exhaustively destructures all fields, so adding a
field without encoding it fails compilation. Every enum match is exhaustive, so
adding a variant without assigning a tag fails compilation. Plan records are
encoded in their dense table order. The plan's own ID is excluded; all other
plan fields, including complete work accounting, are included. Witness IDs hash
witness facts only; plans include both witness IDs and facts.

The semantic descriptor has one root type and only its exact transitive nominal
declaration closure. References use stable declaration/member identities rather
than nested declaration bodies, making recursive closure cycle-free. Product and
enum declarations sort by declaration identity; declaration-order fields,
variants, type parameters, and type arguments are not sorted. Executable child
records encode a closed role followed by either an external witness identity or
an authenticated local semantic target. Role order and target kind are identity
inputs.

All counts, edge/work growth, and encoder reservation are checked against the
contracts-owned witness/type limits. Failure precedes verified HIR publication;
there is no fallback identity. Changing a tag, order, domain, or included fact
requires the sole platform revision and regenerated artifacts. No compatibility
decoder exists.

## Evidence

Focused tests retain exact digest vectors, deterministic recursive closure,
unrelated-declaration exclusion, field/variant/type-argument/list sensitivity,
role-swap rejection, malformed descriptor and target rejection, and external
cycle rejection. Producer and verifier independently construct the descriptor;
IR and core recompute descriptor and witness identities. Repository search
rejects `Debug`-formatted authority in this trust chain.
