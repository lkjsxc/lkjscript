# Edition 2: Layouts

[Authority](../edition-2.md)

## Purpose

Separate semantic type identity from layout facts, physical plans, and runtime
layout identity.

## Status

**Accepted Target, not Current.** Only representations explicitly evidenced by
a later implementation record may be called Current.

## Four Identities

The compiler keeps these domains distinct:

1. `SemanticType`: nominal type, substitutions, and source semantics;
2. `TargetIndependentLayoutFacts`: finite shape, variants, field liveness,
   recursion/indirection, trace categories, and proven niche candidates;
3. `TargetSpecificRepresentationPlan`: concrete ABI and engine placement; and
4. `RuntimeLayoutIdentity`: versioned identity checked by allocator, collector,
   artifacts, runtime calls, and cross-tier adapters.

Equality of any one domain does not imply equality of another. Same-shaped enum
semantic types remain nominally distinct.

## Representation Plan

A target-specific plan records exact size, alignment, tag encoding, per-variant
payload offsets, active-variant trace program, compiler-proven niches, VM/native
homes, root and barrier requirements, runtime-call sites, and materialization
rules. Backend physical tags are a mapping from stable `VariantId`; source order
is not tag identity. Public source cannot observe null, tags, padding, addresses,
boxing, or a niche choice.

The initial VM may use exact boxed tagged generic ADTs. Recursive ADTs are
physically indirect. Collection reads a validated runtime layout identity and
traces only the active variant's initialized traced fields. Unknown identities,
tags, offsets, trace metadata, or niche proofs fail closed before unsafe access.

A niche is used only with a compiler proof retained for independent validation.
No source sentinel or host ABI accident grants one. This contract does not
claim unimplemented scalar replacement, stack placement, region placement,
flattening, unboxing, or allocation elimination.
