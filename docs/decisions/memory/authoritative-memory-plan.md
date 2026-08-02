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
obligations, whole-place drop classes, and drop glue. The
[versioned encoding](canonical-memory-plan-encoding.md) uses exhaustive stable tags
and dense order, never Rust formatting, maps, addresses, time, or process state.

`ExecutableProgram` retains the ID and opaque verified authority through SSA.
Each concrete structural type carries its exact witness identity, canonical
semantic descriptor, and complete role-bearing dependencies through SSA and
validated bytecode. Package and residual-ABI export remains follow-on work.
## Value Axes

Each value plan records:

- multiplicity: `copy`, `immutable-value`, `affine`, or `borrowed`;
- aliasing: `unique`, `borrowed-shared`, `borrowed-exclusive`,
  `static-shared`, `region-shared`, `unresolved-shared`, or `external`;
- escape: `local`, `caller`, `returned`, `captured`, `runtime`, or `static`;
- storage: `inline`, `static`, `stack`, `caller-destination`, unique structural,
  ordinary/sealed region, borrowed view, external resource, or
  `unsupported-runtime`;
- destruction: `trivial`, `end-borrow`, `drop-glue`, `external-close`,
  `region-reset`, or `unsupported`;
- identity: `value`, `external-resource`, or `unsupported-value`;
- portability and contention;
- checked allocation-failure behavior;
- exact source and HIR origin.

Variable-sized immutable values are not bitwise `copy`. Migrated byte and path
values have value identity and cannot use object equality.

## Function Signatures

Each parameter is exactly one of `copy`, `borrow-shared`, `borrow-exclusive`,
or `consume`. Each result is `trivial`, `owned`, `sealed-shared`, or `external`.
Borrowed structural results remain rejected in this slice.

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

## Current Aggregate Cutover

The Current aggregate island records domain, root projection, mode, transitive
closure, destination, copy/share, borrow, drop, origin, and failure cleanup.
Domains are inline, static, stack, caller, unique, ordinary/sealed region,
borrowed, external, or unsupported. Structural roots project typed domain roots;
they are not storage classes. Destinations are private write-once state and
publish only after complete initialization; construction and abort must execute.

Mode comes from monomorphized fields. Copy products require copy fields;
`immutable-value` permits borrow, static identity, structural copy, or sealed
sharing; `affine` follows a unique/external owner or drop obligation. A nonrecursive product containing a selected
copy-leaf list transitively and only `unit`, `bool`, `i64`, `f64`, or already
selected acyclic region-product fields uses `OrdinaryRegion`, `RegionHandleCopy`,
no root, and a Current destination. Invocation records support exact calls and
bulk teardown but no process codec or entry result. Structural-image, symbolic,
owned, recursive, cyclic-region, or unsupported-list fields do not enter. VM copy-variable metadata is
not a native or unknown generic witness. Only active enum payloads initialize.

The first island is transitively closed. Structural images admit eligible nested
aggregates; ordinary-region products admit only the closed acyclic fields above. Missing
product list witnesses, captures, unknown arguments, or unsupported siblings
reject rather than select storage. Unsupported runtime values cannot own or
borrow deterministic roots. The verifier reconstructs closure from the type
graph.

Every Current `list<T>` uses a capacity-32 segmented session region and exact
witness. Handles need no root or per-value drop; prepends retain the allocation
charge. Selection accepts exact copy leaves and recursively nested selected copy
lists. Lists with structural owners remain `ListElementWitnessRequired`-blocked.

## Accepted Final Structural Extension

The Current first vertical extension carries each concrete structural witness
into verified SSA and validated bytecode and selects recursively nested lists of
already selected copy-list values. It does not claim a residual generic ABI or
lists of independently owned structural roots. Ultimately every exact type has
a content-addressed witness; residual polymorphism carries hidden static witness
parameters. Missing or mismatched witnesses are compile errors, never tracing.

A witness closes semantic/runtime-layout identity, mode, sizing, domain, routes,
portability, and contention. Revision 16 also authenticates a target-neutral root
and exact reachable product/enum closure. Nominal references use stable 32-byte
Semantic Source identities; fields, variants, source indices, and exact enum
arguments retain order. Unreachable declarations are excluded.

Contracts-owned dependency roles cover list elements, product fields, enum
variant fields, and nominal arguments. Targets are exact external witness IDs or
same-recursive-SCC local semantic identities authenticated by the closure.
Semantic recursion has no witness self-hash; the external graph is acyclic.
Package interfaces will export exact hidden requirements.

Product/enum declaration SCCs are recomputed after substitution. Their least
stable mode is derived from nonrecursive fields and exact arguments while
same-SCC fields become local structural-image edges. One recursive construction
uses one private destination/region plan and publishes only after complete
initialization. Ordinary values are finite and acyclic; a back edge to an
initializing node is rejected. Unsupported affine recursive ownership is a
structured compile error.

Lists use an exact element witness and a segmented immutable domain plan.
Accepted list elements are `copy` or `immutable-value`; affine resource or unique
mutable elements are rejected until a complete linear collection contract
exists. HIR records segment construction, source/tail ownership, nonescaping
borrow versus escaping owner, seal point, equality witness, and failure cleanup.

Separate producer/verifier HIR traversals construct the contracts descriptor,
identity, SCC facts, complete roles, local legality, external DAG, and ownership.
Only schema, encoder/hash, tags, and limits are shared. IR/core recompute
identities and reject malformed roles, targets, external cycles, or ambiguous
representations. Accepted plans contain no unresolved blocker variants.

This slice remains Experimental: it is not recursive executable witness groups,
compiler-selected sealed placement, package provenance, all-tier sealed
operation execution, or process rehydration.
## Unsupported Runtime Values

`unsupported-runtime`, `unresolved-shared`, and `unsupported` destruction are
blocker evidence only and never executable storage modes. Unknown families,
analysis failure, and stale plan contracts fail before effects.

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
and exact unsupported-runtime rejection. It independently requires byte-vector
owners to use deterministic `unique-slot` storage and capture-free function and
symbol artifacts to use static trivial storage; none may claim executable
storage without a valid witness.
Producer failure or verifier mismatch is
a compile error and never selects tracing.

The plan now drives a verified static/dead/conditional SSA drop spine. Exact
closed glue identities and independently recomputed whole-place classes reach
affine place metadata and explicit loan-end/drop events; `place-end` rejects an
available owner. Byte-vector and owned typed-resource cleanup is elaborated on normal lexical
and source-level structured terminator paths. Explicit typed-resource close
receives a matching resource-drop event and suppresses implicit close. Borrowed
standard input never receives guest-owned glue.

Conditional cleanup is emitted on the exact live predecessor and ends the place
on both edges; the verifier and bytecode unique-owner phi checks reject forged
mismatches. Interned instruction-failure plans are independently reconstructed,
preserved by normalization/proof optimization, lowered into atomic bytecode
ranges, and executed by the evaluator and VM for exact byte/resource owners and
by forced native tiers for byte owners. Native owned-resource cleanup remains
fail-closed. Bounded cleanup-failure attachment is Current. The diagnostic SSA
inventory remains derived evidence and cannot override the plan.
