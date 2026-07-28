# Authoritative Memory Plan

## Status

<!-- LKJ-STATUS id=memory-plan status=current -->

**Current for the complete HIR accepted by the Current compiler pipeline.** The
closed plan is produced and independently verified before SSA lowering; the
existing SSA memory inventory remains derived diagnostic evidence rather than
semantic authority.

## Decision

Every executable program receives one deterministic memory plan immediately
after typed HIR, effects, ownership, and capabilities and before SSA lowering.
The producer and the independent verifier are separate exhaustive traversals.
Only an opaque memory-verified HIR wrapper can enter SSA construction; no
backend-facing path accepts unchecked HIR.

The plan covers every value, constant, parameter, result, affine place, loan,
and relevant call edge. Missing analysis is a compile error. It never selects
legacy tracing because analysis failed.

## Plan Identity

`MemoryPlanId` is content-addressed by the plan schema, source and HIR identity,
function signatures, use/escape facts, value plans, borrow scopes, drop
obligations, and drop glue. Canonical encoding is ordered by dense semantic
identity and never by map iteration, address, time, thread, or process state.

`ExecutableProgram` retains the exact ID and opaque verified plan authority gates
SSA construction. Incorporating the plan ID into serialized SSA, bytecode,
native-image, and package artifact identities remains accepted follow-on work;
no artifact identity claim is made for that integration yet.

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

The Current pre-backend slice uses checked hard bounds for expression plans,
use edges, loans, obligations, constants, calls, functions, and verifier work;
all arithmetic is checked and exhaustion is a structured compile failure.
Profile-owned precharges for future place projections, liveness fixed points,
escape fixed points, drop flags, cleanup blocks/edges, unique-slot metadata,
and resource-slot metadata remain part of the accepted complete deterministic
cutover. Existing compiler profile ceilings are not weakened.

## Verification

The verifier independently traverses HIR and proves dense identities, complete
expression/parameter/result/place/loan/constant/call coverage, exact function
and direct-call memory signatures, use accounting, origin/type/effect agreement,
storage-axis eligibility, drop-glue/type agreement, allocation-failure facts,
and exact legacy-tracing registration. Producer failure or verifier mismatch is
a compile error and never selects tracing.

The plan now drives a verified static/dead SSA drop spine. Exact closed glue
identities reach affine place metadata and explicit loan-end/drop events;
`place-end` rejects an available owner. Byte-vector and owned typed-resource cleanup is elaborated on normal lexical
and source-level structured terminator paths. Explicit typed-resource close
receives a matching resource-drop event and suppresses implicit close. Borrowed
standard input never receives guest-owned glue.

Conditional flags, instruction-originated all-outcome resource cleanup, and
bounded cleanup-failure attachment remain governed by
[Deterministic Drop](deterministic-drop.md) and are not Current. The diagnostic
SSA inventory remains derived evidence and cannot override the plan.
