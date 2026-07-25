# Semantic Source: Typed Holes And Legal Actions

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Define incomplete development nodes, exact hole context, bounded candidates,
legal actions, and checked hole transactions.

## Status

**Accepted Target, not Current.** Holes and legal actions are unavailable until
this complete bounded contract is implemented.

## Typed Holes

A typed hole is a source-development node. It is never a runtime, bytecode,
proof, SSA, layout, ABI, or default value. It records stable declaration-local
identity, revision-scoped node ID, exact origin, optional user goal, containing
declaration/return type, and expected type or typed unavailable reason. Context
includes visible bindings/places and exact types; generic variables and trait
obligations; allowed and already-required semantic effects; ownership,
consumption, move/borrow/loan/region constraints; control target and return/loop
requirements; whether `Never` is admissible; and material incompleteness.

A hole satisfies only its expected position to continue analysis. It cannot
prove a trait, invent authority, bypass ownership, erase effects, establish
exhaustiveness, or provide layout. Every affected fact remains explicitly
Incomplete.

Executable, release, package, component, cache, AOT, VM, native, and proof
publication rejects every reachable hole. Independent unaffected declarations
may retain derived facts without granting completeness to a containing hole.

## Bounded Responses

`hole_context` returns deterministically ordered candidates from exact literals,
visible bindings, enum/product/Option/Result constructors, directly callable
functions, exact conversions, match skeletons, and legal return/break/continue
or Never forms. Bounded nested candidates state result type, effects, ownership,
capability facts when available, construction cost, exact semantic edit,
inclusion reason, and validating checker/proof.

`legal_actions` reports legal child kinds, constructors, required fields,
expected types, applicable bindings, and transaction kinds for its claimed
subset. Incomplete Edition/construct coverage is explicit. Neither service
claims full synthesis or full-language token masking.

Both responses include `supported`, `truncated`, charged category/count, omitted
category counts when known, and a typed unsupported/truncation reason. Budget
exhaustion cannot be presented as an empty complete result. Unsupported contexts
return explicit unsupported, then ordinary authoring may use bounded compiler
validation.

## Transactions

The exact hole operations are `insert_hole`, `fill_hole`, `refine_hole`, and
`delete_hole` where deletion leaves a structurally legal node. Each atomic
transaction pins source/schema/profile/root/revision,
checks node and semantic preconditions, then reruns exact type, effect,
ownership, control/divergence, incompleteness, and resource validation before
staging publication. Stale identity/revision, mismatched expected type, added
forbidden effect, invalid ownership transfer, or invalid control edge rejects
the whole transaction. No failed operation mutates source or caches.
