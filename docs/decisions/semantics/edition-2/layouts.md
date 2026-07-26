# Edition 2: Layouts

[Authority](../edition-2.md)

## Purpose

Separate semantic type identity from layout facts, physical plans, and runtime
layout identity.

## Status

**Current for target-independent enum facts and the validated boxed reference-VM
plan.** Target-specific native/JIT representation plans, niches, unboxing, and
cross-tier adapters remain Accepted Targets.

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

The reference VM uses exact boxed tagged generic ADTs. Each value stores its
validated runtime layout identity, physical tag, and active payload. Recursive
ADT edges are physically indirect. Collection traces only the active variant's
initialized fields. Unknown identities, tags, fields, substitutions, or layout
facts fail closed before access.

A niche is used only with a compiler proof retained for independent validation.
No source sentinel or host ABI accident grants one. This contract does not
claim unimplemented scalar replacement, stack placement, region placement,
flattening, unboxing, or allocation elimination.
