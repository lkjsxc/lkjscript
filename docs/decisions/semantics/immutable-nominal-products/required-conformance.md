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
9. products nested through Option, Result, List, and other products survive GC;
10. malformed bytecode descriptors, categories, identities, and indexes fail
    without panic;
11. product declarations add no runtime globals or initialization effects;
12. all canonical sources, runtime smokes, and bounded diagnostic performance
    comparisons remain accounted for.
## Follow-On Work

Once this contract is Current, Brainfuck, lkjedit, and terminal state can be
represented as immutable products. The later atomic semantic cutover still must
add function/main-local `var`/`set`, explicit executable `main`, effect-free
imported libraries, and removal of source mutable globals. Products do not make
that unfinished behavior Current by themselves.
