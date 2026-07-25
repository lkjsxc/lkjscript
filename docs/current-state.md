# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

**Current** only for implementation and evidence explicitly labeled Current in
the manifest capsules. Bounded Repository Topology, Repository Intelligence
Graph/context, and the externalized agent work-state service are Current.
Complete Semantic Source/Agent Protocol V1 and resource profiles remain
**Accepted Implementation Contracts**, not Current code. Deferred and Rejected
work remains non-current.

## Current Baseline

Semantic Source Foundation V1, resolved typed HIR, verified typed SSA,
reference bytecode/VM, callable host-independent Linux x86-64 baseline code,
host-independent native allocation/collection, and forced proof-checked
optimization are Current at baseline `dd8fb316`. Exact boundaries and retained
historical command evidence are in the manifest below. Automatic proof
promotion, complete protocol operations, aggregate profiles, OSR, AOT/cache,
and wider host-native transitions are not Current. Structure, graph/context,
and work-state commands are Current on the containing implementation commit.

## Implementation Sequence

1. [Bounded Repository Topology](decisions/platform/bounded-repository-topology.md) is Current.
2. [Repository Intelligence Graph And Context](decisions/platform/repository-intelligence-graph.md) is Current.
3. The bounded externalized [Agent Work State](decisions/platform/agent-work-state.md) service is Current.
4. The [first Semantic Source/Agent
   operations](decisions/platform/semantic-source-and-agent-protocol/first-current-candidate.md)
   and [aggregate resource-profile candidate](decisions/platform/resource-budget-profiles-candidate.md) remain next.

Current status applies only to each implemented bounded surface, not to broader accepted protocol designs.
Automatic baseline-to-proof promotion remains an Accepted Implementation
Selection after these repository-intelligence foundations; it is not enabled.

## Strict Capsule Manifest

### Current implementation and direction

- [resolved AI-native redesign baseline](current-state/resolved-ai-native-redesign-baseline.md)
- [compiler, semantic, and runtime implementation](current-state/current-implementation.md)
- [host capabilities and native runtime](current-state/current-native-runtime.md)
- [accepted platform direction](current-state/accepted-platform-direction.md)
- [automatic proof-promotion selection](current-state/accepted-target-automatic-baseline-to-proof-promotion.md)

### Historical evidence

- [Semantic Source Foundation V1](current-state/semantic-source-foundation-evidence.md)
- [SQLite](current-state/sqlite-evidence.md)
- [optimizer and allocation](current-state/optimizer-and-allocation-evidence.md)
- [ownership through native foundation](current-state/ownership-through-native-foundation-evidence.md)
- [callable baseline](current-state/callable-baseline-evidence.md)

### Status boundaries

- [accepted next work, rejected work, and deferred work](current-state/accepted-next-work.md)

Historical evidence remains command- and commit-specific. It does not make an
Accepted Implementation Contract Current on a later revision.
