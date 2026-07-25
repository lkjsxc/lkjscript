# Current State: Accepted Next Work, Rejected, And Deferred

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Current Foundations And Accepted Next Contracts

The immediate implementation sequence has progressed as follows:

1. bounded repository topology, strict manifests, rule IDs, and generated audit
   JSON are Current;
2. bounded repository graph/context nodes, edges, identities, and profiles are Current;
3. externalized agent work state with exact Git/state preconditions, action and
   command facts, content references, atomic replacement, and compaction is Current; and
4. bounded one-shot Agent Foundation V1 operations and the compiler Resource
   Profile V1 foundation are Current without changing an Edition 1 limit; and
5. complete future Schema V1, daemon/typed-hole operations, pre-allocation
   profile charging, and shared protocol/repository/task/runtime ledgers remain
   next.

The authorities are [Bounded Repository
Topology](../decisions/platform/bounded-repository-topology.md), [Repository
Intelligence Graph And Context](../decisions/platform/repository-intelligence-graph.md),
[Agent Work State](../decisions/platform/agent-work-state.md), [Semantic Source
And Agent Protocol](../decisions/platform/semantic-source-and-agent-protocol.md),
and [Resource Budget Profiles](../decisions/platform/resource-budget-profiles.md).
All five authorities now describe Current bounded slices. Their explicitly
broader Schema, profile, daemon, package, and runtime targets remain Accepted,
not Current.

The previously selected process-local synchronous automatic baseline-to-proof
promotion remains an **Accepted Implementation Selection** and valid later
experiment. Its threshold gate and default-disabled policy remain unchanged.
Broader proof passes, OSR, background compilation, guards, deoptimization, AOT,
caches, optional local PGO, and non-Linux targets remain non-Current.
## Rejected

Mandatory uploaded telemetry, hidden forced-engine fallback, incomplete artifact
cache keys, separate backend semantic compilers, unbounded optimizer search,
and optimizer-dependent deterministic semantic charges are rejected.
Current-process bounded JIT counters remain local, ephemeral, and not telemetry.
## Deferred

Production AOT, content-addressed native caches, optional explicit local PGO,
automatic optimizing promotion, OSR, package installation/update/manifests/
locks/registry, supervisor/scheduler, adaptive or generational GC, background
JIT compilation, guarded runtime specialization/deoptimization, non-Linux native
backends, browser, general HTTP server/framework, and GUI runtime are later
cycles. Local PGO is optional and is considered only after common SSA/AOT and
complete artifact identity; no uploaded telemetry is accepted. These documents
are decisions or experiments, not capability claims.
