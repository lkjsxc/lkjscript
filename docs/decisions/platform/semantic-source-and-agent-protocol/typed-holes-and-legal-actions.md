# Semantic Source: Typed Holes And Legal Actions

[Authority](../semantic-source-and-agent-protocol.md)

## Purpose

Define incomplete development nodes, exact hole context, bounded candidates,
legal actions, and checked hole transactions.

## Status

**Current.** Semantic Source Schema V2 implements typed expression holes,
`hole_context`, `legal_actions`, and the four closed hole transactions over the
Current Edition 1 expression subset plus Current Edition 2 enum, match,
Never, and structured-control forms. Schema V1 input is historical and rejected.

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
category counts when known, and a typed unsupported/truncation reason. Candidate
rank is the closed category, effect cost, ownership cost, construction cost,
canonical source, and candidate identity tuple. Every emitted candidate carries
one complete canonical snippet and an exact `replace_hole` semantic edit; a
context commonly carries several complete alternatives.

Exploration reserves Profile V2 `hole_search_work` and `hole_candidates` before
candidate construction. `legal_actions` reserves its category before action
construction. Budget exhaustion cannot be presented as an empty complete
result. Unsupported contexts return explicit unsupported, then ordinary
authoring may use bounded compiler validation. Edition 1 has exact import paths
but no qualification syntax, so no import or qualification edit is invented;
that absence is an explicit blocker. Match, return, break, continue, and Never forms remain explicit Edition 1
blockers. In Edition 2, checker-valid loop/return/break/continue/trap/exit
candidates and legal child kinds are emitted only at exact legal sites; context
reports the function return, nearest-loop result/depth, available forms, and
whether Never is admissible.

## Transactions

The exact hole operations are `insert_hole`, `fill_hole`, `refine_hole`, and
`delete_hole` where deletion leaves a structurally legal node. Each atomic
transaction pins source/schema/profile/root/revision,
checks node and semantic preconditions, then reruns exact type, effect,
ownership, control/divergence, incompleteness, and resource validation before
staging publication. Stale identity/revision, mismatched expected type, added
forbidden effect, invalid ownership transfer, or invalid control edge rejects
the whole transaction. No failed operation mutates source or caches. Transaction
staging reserves Profile V2 transaction, operation, impact-node, staged-node,
and staged-byte categories through the typed `transaction` authority before
cloning or rebuilding source state.

Insertion and refinement may retain an incomplete tree only when every hole has
an exact expected type and one bounded checker-valid completion for surrounding
type, effect, ownership, and control validation. Filling rejects another hole as
a concrete value. Deletion is Current only for hole children of `do` and `while`
where removal leaves a structural expression collection; rebuilt analysis still
must pass before publication.
