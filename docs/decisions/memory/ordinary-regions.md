# Ordinary Owned Regions

## Status

<!-- LKJ-STATUS id=ordinary-regions status=accepted-contract -->

**Accepted contract; implementation is not yet Current.** Ordinary regions are
the selected bulk-lifetime domain, not a compatibility arena or collector.

## Ownership And Allocation

A region has one affine owner. Internal typed references are non-owning and may
form arbitrary cycles, but cannot escape the owner lifetime. A bounded chunked
bump allocator checks object count, bytes, alignment, arithmetic, and large
allocations before publication. There is no arbitrary per-object free.

Creation is inferred late and destruction early. Splitting or child regions
separate unrelated lifetimes; a whole-function region is not the default.
Reset requires proof that no root or view survives.

## Roots And Internal Graphs

Every root binds the region generation, root generation, layout, and semantic
type. Internal references validate the same live region and exact target root.
Destruction does not inspect payloads or follow internal references. Therefore
self-cycles and arbitrary internal cycles have graph-independent reclamation.

## Dependency Ledger

An outgoing owning dependency is registered when created. Strong domain
dependencies form an acyclic graph. Addition performs deterministic bounded
cycle detection and reports one canonical witness. Borrowing a longer-lived
owner is separate and does not add an owning edge.

The ledger, not graph tracing, discovers release work. Ordinary child ownership
is unique. Shared immutable dependencies use sealed-region ownership.

## Drop Ledger

Only exact supported nontrivial drops enter a bounded side ledger. Each entry
has stable kind, identity, and registration order. Destruction runs entries once
in reverse registration order, attempts all entries after a failure, and
reports deterministic ordered failures. Unsupported external resources inside
arbitrary regions remain rejected.

## Destruction

Destruction rejects live loans, executes side drops, releases dependencies with
a bounded iterative worklist, frees large allocations and chunks, invalidates
the generation, and records metrics. It performs no internal object traversal.
Cleanup applies to return, control transfer, trap, exit, allocation/resource
failure, deadline, host failure, and task cancellation where those outcomes are
Current.

## Resource Plane

A region is one data owner. Tasks may read, write, consume, or produce it under
verified accesses. Home transfer requires no live loan. Remote destruction uses
the existing bounded release queue.

## Acceptance

Current promotion requires stale-key rejection, exact bounds, internal-cycle
release, dependency-cycle witnesses, reset/split evidence, cleanup failure
injection, scheduler transfer tests, and leak-free completion. Backend and
source integration are separate family gates.
