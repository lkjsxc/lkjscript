# Immutable Nominal Products: Required Conformance

[Authority](../immutable-nominal-products.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Required Conformance

The implementation is not Current until focused tests prove:

1. zero- and 15-field boundaries pass and 16 fields fail;
2. forward references and nested `Product Name` annotations resolve;
3. duplicate/colliding declarations and duplicate fields fail;
4. missing, extra, unknown, duplicate, and out-of-order constructor fields fail;
5. access and replacement preserve exact types and evaluation order;
6. replacement leaves the original product unchanged;
7. same-shaped declarations remain nominally distinct;
8. product equality is rejected;
9. deterministic products nested through Option, Result, List, and other
   products execute across evaluator, VM, baseline, and proof tiers with exact
   teardown;
10. malformed bytecode descriptors, categories, identities, ownership routes,
    and indexes fail without panic;
11. region-product keys reject at process boundaries, and malformed region
    identity or cross-domain graphs fail before publication;
12. product identities are content-addressed rather than declaration-order or
    allocator identities;
13. product declarations add no runtime globals or initialization effects;
14. all canonical sources, runtime smokes, and bounded diagnostic performance
    comparisons remain accounted for.
## Follow-On Work

Brainfuck, lkjedit, and terminal state already use immutable products. Follow-on
work may expand independently reconstructed generic witnesses, structural-image
region fields, immutable list elements, and product equality. It must not
reintroduce tracing, mixed ownership graphs, compatibility storage, or hidden
backend-specific product semantics.
