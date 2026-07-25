# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

<!-- LKJ-STATUS id=agent-foundation/1 status=current -->
<!-- LKJ-STATUS id=agent-work-state/1 status=current -->
<!-- LKJ-STATUS id=jit-auto-promotion/1 status=accepted-selection -->
<!-- LKJ-STATUS id=jit-proof-forced/1 status=current -->
<!-- LKJ-STATUS id=repository-graph-context/1 status=current -->
<!-- LKJ-STATUS id=repository-topology/1 status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler/1 status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation/1 status=accepted-target -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger/1 status=accepted-target -->
<!-- LKJ-STATUS id=semantic-session/1 status=accepted-target -->
<!-- LKJ-STATUS id=semantic-source-foundation/1 status=current -->
<!-- LKJ-STATUS id=semantic-source-schema/1 status=accepted-target -->
<!-- LKJ-STATUS id=typed-holes/1 status=accepted-target -->

**Current** only for implementation and evidence explicitly labeled Current in
the manifest capsules. Bounded Repository Topology, Repository Intelligence
Graph/context, externalized agent work state, the one-shot Agent Foundation V1
protocol, and the compiler Resource Profile V1 foundation are Current. Complete
Semantic Source Schema V1, daemon transport, typed holes, pre-allocation
profile charging, and cross-authority resource ledgers remain Accepted.
Deferred and Rejected work remains non-current.

## Current Baseline

Semantic Source Foundation V1, resolved typed HIR, verified typed SSA,
reference bytecode/VM, callable host-independent Linux x86-64 baseline code,
host-independent native allocation/collection, and forced proof-checked
optimization are Current at baseline `dd8fb316`. Exact boundaries and retained
historical command evidence are in the manifest below. Automatic proof
promotion, complete future Schema V1 operations, cross-authority aggregate
profiles, OSR, AOT/cache, and wider host-native transitions are not Current.
Structure, graph/context, work-state, one-shot protocol, and compiler-profile
commands/APIs are Current on the containing implementation revision.

## Implementation Sequence

1. [Bounded Repository Topology](decisions/platform/bounded-repository-topology.md) is Current.
2. [Repository Intelligence Graph And Context](decisions/platform/repository-intelligence-graph.md) is Current.
3. The bounded externalized [Agent Work State](decisions/platform/agent-work-state.md) service is Current.
4. The [Agent Foundation V1
   operations](decisions/platform/semantic-source-and-agent-protocol/first-current-candidate.md)
   are Current for the bounded one-shot protocol.
5. The [compiler resource-profile
   foundation](decisions/platform/resource-budget-profiles-candidate.md) is
   Current at its documented post-phase charging boundaries.

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
