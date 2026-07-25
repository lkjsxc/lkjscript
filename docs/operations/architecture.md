# Architecture

## Purpose

Map Current product behavior to its authorities and record the accepted
repository-intelligence architecture before implementation.

## Status

The compiler/runtime map in the capsules is **Current** at baseline `dd8fb316`.
Semantic Source Foundation V1 is Current. Bounded Repository Topology,
Repository Intelligence Graph/context, agent work state, complete Schema/Agent
Protocol V1, transactions, typed holes, and aggregate profiles are **Accepted
Implementation Contracts**, not Current code.

## Current Compiler/Runtime Boundary

Validated Semantic Source Foundation tree -> resolved typed HIR -> verified
typed SSA -> verified baseline normalization -> reference bytecode is Current.
HIR currently consumes a private mechanically checked Edition 1 projection from
the validated tree; no sibling parser or raw public AST enters analysis. VM,
baseline JIT, proof optimizer, future AOT tests, and Wasm must share verified
semantic IR rather than reinterpret syntax.

Linux x86-64 callable baseline and forced proof-optimizing claims require real
synchronous generated entry from verified SSA. Exact Current coverage and
historical evidence remain in [Current State](../current-state.md).

## Accepted Repository-Intelligence Flow

```text
tracked authorities + strict manifest
    -> bounded topology/provenance validation
    -> deterministic repository graph
    -> versioned bounded context profiles
    -> atomic agent task/read/write/evidence state
    -> Semantic Source query/edit transactions
    -> policy checks and Git publication
```

Each arrow preserves authority identity, revision, provenance, aggregate
charges, and explicit omissions. Derived graph/context/task data cannot grant
source, compiler, proof, or evidence authority. Generated graph, context, audit,
and task snapshots are written under `target/`.

The accepted ownership boundaries are:

- `lkjscript-xtask`: topology policy, strict manifests, rule IDs, link checks,
  canonical audit/graph construction, and repository fixtures;
- Semantic Source/compiler boundary: source/entity/node identity, derived facts,
  diagnostics, and atomic semantic edits;
- agent-state boundary: task lifecycle, exact scope, read/precondition sets,
  attempts, blockers, command evidence, tested/not-tested facts, and publication;
- Git: reviewed authored-tree publication authority; and
- `target/`: disposable generated audits, graphs, contexts, and state snapshots.

These are accepted ownership selections only. Implementation may choose crate
placement after dependency-cycle review, but it must not put generated indexes
or task assertions into compiler semantic authority.

## Aggregate Boundaries

Topology, graph/context, task state, and Semantic Source share versioned resource
profiles but retain exact subsystem categories. Checked charges precede
allocation, traversal, staging, or publication. Exhaustion returns structured
category/limit/attempted-charge diagnostics and cannot publish partial authority.
Edition 1 source limits remain unchanged.

## Authority Links

- [Bounded Repository Topology](../decisions/platform/bounded-repository-topology.md)
- [Repository Intelligence Graph And Context](../decisions/platform/repository-intelligence-graph.md)
- [Agent Work State](../decisions/platform/agent-work-state.md)
- [Semantic Source And Agent Protocol](../decisions/platform/semantic-source-and-agent-protocol.md)
- [Resource Budget Profiles](../decisions/platform/resource-budget-profiles.md)

## Strict Capsule Manifest

- [Current crate graph, ownership map, and compile flow](architecture/crate-graph.md)
- [Current runtime flow and accepted execution direction](architecture/runtime-flow.md)

A capsule's historical or accepted text cannot promote a non-current capability.
