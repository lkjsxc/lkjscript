# Collector-Free Deterministic Memory

## Status

<!-- LKJ-STATUS id=collector-free-deterministic-memory status=accepted-contract -->

**Accepted contract; production cutover is not Current.** The Current VM and
native tiers still use the stable-index, non-moving tracing `GcHeap`. The
Current inventory command records that fact. No collector-free claim is valid
until every production object family is migrated and the no-tracing gate is
clean.

## Decision

[Research evidence](research-evidence.md) records adoption and uncertainty;
[plans](authoritative-memory-plan.md), [drop](deterministic-drop.md), and the
[first island](collector-free-value-island.md) refine the initial executable cut.

The selected destination is one deterministic memory architecture with no
tracing liveness traversal or collector fallback. The compiler tries, in order:

1. inline or static representation;
2. stack or caller-owned destination;
3. unique affine ownership and inferred borrowing;
4. an inferred or first-class owned region;
5. a sealed immutable shared region;
6. a typed generational pool for mutable identity;
7. copying when measured cheaper;
8. precise immutable acyclic reference counting only for retained
   counterexamples to every earlier choice.

Tracing is not a fallback. The migration collector is a private differential
control and must be deleted at cutover.

## Terminology

Tracing collection decides liveness by traversing references from roots and
reclaims objects not reached. Mark/sweep, copying, compacting, generational,
local, cycle, concurrent, and reference-count cycle collectors are tracing.

Deterministic reclamation follows the end of an exact owner, count, region,
pool, scope, task, artifact, resource, or static lifetime. Reference counting
is deterministic ownership, but its metadata, traffic, release work, overflow,
and cycle restrictions remain explicit costs.

A terminating execution is leak-free when each non-static allocation is
reclaimed exactly once, transferred to an explicit returned owner, or retained
by a still-live owner with defined later destruction. Process exit is not
ordinary reclamation.

## Source Contract

Ordinary source has no lifetime names, retain/release, general `free`, raw
pointer, or collector selection. Immutable values keep value semantics; the
compiler selects copy, borrow, transfer, region share, sealed-region share, or
eligible precise sharing. Mutable unique values and external resources are
affine. Explicit `move`, `borrow`, `borrow-mut`, and `drop` remain only where
semantic intent differs.

Borrowed returns remain rejected until a separately checked interface contract
can prove them. Public interfaces carry canonical derived ownership, returned
owner, portability, and memory-ABI facts without exposing physical allocator
syntax.

## Derived Modes

Memory-complete HIR derives these independent facts:

- multiplicity: `copy` or `affine`;
- aliasing: unique, shared immutable, shared/exclusive borrow, weak, or ID-only;
- locality: local, escaping, or static;
- storage: inline, stack, caller destination, unique heap, region, sealed shared
  region, shared node, pool, static, or external;
- portability: worker-local, send, or shared-between-workers;
- contention: single-owner, synchronized, or atomic;
- destruction: trivial, field drop, bulk region, shared-region release,
  node release, pool-owned, or external close;
- identity: value, pool, external, or explicit shared identity.

The compiler infers parameter consumption/borrowing, result ownership, closure
capture, non-lexical loan liveness, region lifetime, sharing, storage, and exact
cleanup. Diagnostics expose owner/use paths, escape, cycle witness, storage
consequence, repair candidates, and certainty.

## Regions And Sealed Regions

An ordinary region is affine, bounded, and reclaimable without inspecting its
internal graph. Internal cycles are legal. Region dependencies are recorded as
an exact acyclic side ledger during construction; nontrivial drops use an exact
side ledger. Creation, split, reset, transfer, and destruction are explicit in
verified IR. Allocation is late and destruction is early according to dataflow,
not source nesting alone.

A sealed shared region is built uniquely, validated, and made immutable. One
region-level owner count or exact owner set replaces per-object counts. Sealing
enumerates outgoing dependencies from construction metadata and rejects a
strong inter-region cycle with a deterministic witness. Final release drops
side obligations, releases ledger dependencies, and frees chunks without
walking internal edges.

## Precise Sharing

Per-object counting is eligible only for a fully initialized immutable value
with an acyclic strong graph, no affine resource, no expiring borrow, and exact
field-drop and portability facts. Publication order makes strong cycles
unconstructible. Transient uses borrow; worker-local counts are non-atomic.
Cross-worker node counts require separate evidence.

Release uses a bounded iterative worklist, never unbounded native-stack
recursion. Count overflow is a structured resource failure. The
`ownership-regions-only` candidate must be measured on persistent lists, trees,
ropes, compiler snapshots, Web configuration, and game assets before precise
node counting becomes production authority.

## Pools, Weak Links, And Cycles

Typed generational pools own mutable identity. IDs contain pool identity, slot,
and generation, never raw indices. Lookup yields checked temporary borrows;
stale IDs fail safely; generation wrap retires a slot or epoch. Cycles are
non-owning IDs and pool destruction is bulk deterministic cleanup.

Weak links are non-owning class-and-generation references with checked upgrade.
Unique-owner cycles are rejected; precise shared strong cycles are
unconstructible; sealed-region dependency cycles are rejected; mutable graph
cycles use regions or pools. Recursive functions use static groups where known,
otherwise a region-owned closure graph or pool IDs.

## Resources And Structured Outcomes

Typed external resources remain affine and use exact generational runtime slots.
Compiler-inserted cleanup must run exactly once on normal return, early return,
break, continue, trap, exit, deadline, resource failure, host failure, and
future cancellation. No collector finalizer participates. Resource-bearing
shared or persistent aggregates remain rejected until field-sensitive ownership
and drop-ledger verification is complete.

## Compiler And Execution Contract

The single pipeline is typed HIR, effects, use graph, escape and mode inference,
ownership and borrowing, region and sharing analysis, cycle validation, storage
planning, cleanup elaboration, independent memory verification, verified SSA,
and shared evaluator/VM/native lowering. An unknown or collector-fallback plan
cannot reach executable publication.

Verified SSA makes move, borrow, region, sealed-region, share, weak, pool,
resource drop, value drop, reuse, release work, and cleanup blocks explicit.
The verifier proves initialization, lifetime containment, cycle restrictions,
count balance where applicable, generation safety, and cleanup on every outcome.
Logical resource charges remain stable across physical placement changes.

The evaluator models abstract ownership rather than host-language accidents.
The VM uses typed storage domains and validates ownership before effects. Native
frames retain owner, borrow, region, release, resource, deadline, cancellation,
deoptimization, and transition metadata. Polls remain for budgets, deadlines,
cancellation, tiering, OSR, and bounded release work; collecting polls do not.

## Inventory And Migration

`lkjscript.memory-obligations` identifies the inventory schema and closed
memory taxonomy. `lkjscript memory inventory [--json]` and
`lkjscript memory explain <identity>` expose derived Current evidence, including
collector dependencies and non-Current candidate plans. Inventory evidence does
not override semantic authorities.

**Current initial compiler slice.** `ExecutableProgram` retains an independently
recomputed SSA inventory for direct `byte-vector` owners, byte loans, and direct
typed resources. It labels traced buffer storage and incomplete cleanup rather
than pretending the target plan is Current. Aggregates and general storage
planning remain absent.

Migration first makes HIR and SSA memory-complete, unboxes scalars, then moves
static/byte values, ADTs, lists, closures, returned owners, compiler structures,
VM storage, and generated execution. Exact root guarantees become exact owner,
borrow, cleanup, region, pool, and resource guarantees. Root publication,
writeback, mark/sweep, collection policy, barriers, collector metrics, and
stable-handle indirection are removed only after stronger verification passes.

## Acceptance And Falsification

Acceptance requires evaluator, VM, forced baseline JIT, and forced proof JIT
execution with zero forced fallback; persistent, cyclic, compiler, Semantic
Source, Web, game ECS/frame/world, database, closure, destruction, failure, and
leak evidence; Miri/sanitizer/fuzz evidence where available; and zero Current
violations from `LKJ-RUNTIME-NO-TRACING-COLLECTOR`.

Any failed deterministic candidate retains its exact workload, command, result,
and violated invariant. A traced island requires a complete counterexample to
borrowing, destination passing, regions, sealed regions, pools, weak links,
precise immutable counting, and copying. No such counterexample is accepted.

## Rejected

Rejected designs are process-lifetime arenas, hidden root scans, cycle
collectors, universal atomic counting, strong count cycles, unbounded recursive
release, unbounded deferred release, collector finalizers, manual memory APIs,
source lifetime punctuation, untyped pool IDs, and separate backend memory
semantics.
