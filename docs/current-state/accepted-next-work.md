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
4. first Current-candidate Semantic Source operations and aggregate resource
   profiles remain next without changing any Current Edition 1 limit.

The authorities are [Bounded Repository
Topology](../decisions/platform/bounded-repository-topology.md), [Repository
Intelligence Graph And Context](../decisions/platform/repository-intelligence-graph.md),
[Agent Work State](../decisions/platform/agent-work-state.md), [Semantic Source
And Agent Protocol](../decisions/platform/semantic-source-and-agent-protocol.md),
and [Resource Budget Profiles](../decisions/platform/resource-budget-profiles.md).
The first three authorities now describe Current bounded implementations. The
Semantic Source operations and aggregate resource profiles remain Accepted, not Current.

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
