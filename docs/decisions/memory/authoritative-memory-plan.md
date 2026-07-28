# Authoritative Memory Plan

## Status

<!-- LKJ-STATUS id=memory-plan status=accepted-contract -->

**Accepted contract; implementation is not yet Current.** The existing SSA
memory inventory is diagnostic evidence, not this authority.

## Decision

Every executable program receives one deterministic memory plan after typed HIR,
effects, and capabilities and before backend lowering. The producer and the
independent verifier are separate implementations. Only an opaque verified
program crosses into SSA normalization, bytecode, evaluator, VM, or native
lowering.

The plan covers every value, constant, parameter, result, affine place, loan,
and relevant call edge. Missing analysis is a compile error. It never selects
legacy tracing because analysis failed.

## Plan Identity

`MemoryPlanId` is content-addressed by the memory-plan contract, source and HIR
identity, function signatures, use/escape facts, value plans, borrow scopes,
drop obligations, cleanup edges, and drop glue. Canonical encoding is ordered
by dense semantic identity and never by map iteration, address, time, thread,
or process state.

Plan facts participate in verified SSA, bytecode, native image, and package
identity.

## Value Axes

Each value plan records:

- multiplicity: `copy`, `immutable-value`, `affine`, or `borrowed`;
- aliasing: `unique`, `borrowed-shared`, `borrowed-exclusive`,
  `static-shared`, `legacy-traced-shared`, or `external`;
- escape: `local`, `caller`, `returned`, `captured`, `runtime`, `static`, or
  `legacy-unknown`;
- storage: `inline`, `static`, `stack`, `caller-destination`, `unique-slot`,
  `borrowed-view`, `external-slot`, or `legacy-traced`;
- destruction: `trivial`, `end-borrow`, `drop-glue`, `external-close`, or
  `legacy-traced`;
- identity: `value`, `external-resource`, or `legacy-object`;
- portability and contention;
- checked allocation-failure behavior;
- exact source and HIR origin.

Variable-sized immutable values are not bitwise `copy`. Migrated byte and path
values have value identity and cannot use object equality.

## Function Signatures

Each parameter is exactly one of `copy`, `borrow-shared`, `borrow-exclusive`,
or `consume`. Each result is `trivial`, `owned`, `borrowed`, or `external`.
Borrowed results remain rejected in this slice.

Direct calls consume the verified signature. Affine or borrowed indirect calls
remain rejected unless a complete callable signature exists. A backend cannot
invent a copy to satisfy a call.

## Analysis

The compiler constructs a bounded use-and-place graph from operands, calls,
branches, loops, block parameters, returns, traps, outcomes, slices, resources,
and runtime operations. CFG fixed points derive:

1. reachable last use;
2. maybe-initialized and maybe-uninitialized places;
3. escape and capture;
4. ownership availability;
5. shared and exclusive loan issuance, invalidation, liveness, and kill points;
6. exact drop obligations and cleanup paths.

Immutable transient call uses borrow where the signature permits it. Mutation
requires an exclusive loan. Loans end after their last reachable use, not at
function end.

## Legacy Tracing

`legacy-traced` is valid only for a family named by the tracing ratchet. An
island value cannot use `legacy-unknown`, `legacy-traced-shared`, or
`legacy-traced`. Unknown families and stale plan contracts fail before effects.

## Budgets

Profiles bound and precharge use edges, place projections, liveness work,
escape work, borrow work, plans, drop obligations, drop flags, cleanup blocks,
cleanup edges, drop glue, unique-slot metadata, and resource-slot metadata.
Checked overflow is a structured resource failure.

## Verification

The verifier independently reconstructs the plan and proves complete coverage,
function signature compatibility, availability, move and loan legality, escape,
storage eligibility, drop coverage, glue identity, cleanup routing, and tracing
registration. Diagnostic inventory is derived from the verified plan and cannot
override it.
