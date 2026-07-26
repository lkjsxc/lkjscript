# Architecture

## Purpose

Map Current product behavior and implemented repository-intelligence services
to their authorities.

## Status

The compiler/runtime map in the capsules is **Current** at baseline `dd8fb316`.
Complete Semantic Source Schema V2 with its preserved V1 representation,
typed holes/legal actions, Bounded Repository Topology, Repository Intelligence
Graph/context, externalized Agent Work State V2 with exact semantic references,
the bounded one-shot semantic protocol, bounded local stdio sessions, and
compiler Resource Profile V2 and the Edition 2 generic enum declaration/type-
fact slice are Current. Agent Foundation V1 and Semantic Source Schema V1 are
historical rejected identities. Edition 2 enum construction/execution and the
remaining semantic core, whole-pipeline Profile V2 pre-allocation, nonzero
incremental query caching, and logical metering are **Accepted Targets**.

## Current Compiler/Runtime Boundary

Validated Semantic Source Foundation tree -> resolved typed HIR -> verified
typed SSA -> verified baseline normalization -> reference bytecode is Current.
HIR consumes one private mechanically checked projection from the validated
Edition 1 or Edition 2 tree; Edition 2 currently adds only resolved generic enum
declarations/type facts. No sibling parser or raw public AST enters analysis. VM,
baseline JIT, proof optimizer, future AOT tests, and Wasm must share verified
semantic IR rather than reinterpret syntax.

Linux x86-64 callable baseline and forced proof-optimizing claims require real
synchronous generated entry from verified SSA. Exact Current coverage and
historical evidence remain in [Current State](../current-state.md).

## Repository-Intelligence Flow

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
- Semantic Source/compiler boundary: source/entity/node/hole identity, derived
  checker facts, bounded legal actions, diagnostics, and atomic semantic edits;
- agent-state boundary: immutable task/base/scope preconditions, action and
  command history, blockers/risks, exact references, compaction, and atomic
  state publication;
- Git: reviewed authored-tree publication authority; and
- `target/`: disposable generated audits, graphs, contexts, and state snapshots.

The first four flow stages are Current in `lkjscript-xtask`; one-shot Semantic
Source transactions are Current in `lkjscript-compiler` and the app CLI.
Generated indexes and task assertions never enter compiler semantic authority.

## Aggregate Boundaries

The Semantic Protocol and compiler select the same five versioned profiles but
retain independent request-local ledgers. Topology, graph/context, and task
state use versioned bounded contracts without pretending to share that ledger.
Topology and graph bound traversal; task state bounds decode, retained Git
output, references, context, and publication; the protocol pre-bounds request,
source bytes/units, work, hole candidates, legal actions, transactions, staged
publication, and serialized output; compiler profile charges remain exact
post-phase guards before downstream phases not yet migrated, except enum
declaration/variant/field/recursion shape charges which precede enum HIR
allocation.
Exhaustion cannot publish partial authority. Edition 1 source limits remain
unchanged.

## Accepted Edition 2 Path

[Edition 2](../decisions/semantics/edition-2.md) preserves the same architecture:
Semantic Source -> resolved typed HIR -> verified SSA -> evaluator/VM/native/
proof consumers. Generic enum declarations and resolved type facts are Current
only before SSA; construction is deliberately unexposed. Match lowers to SSA CFG
only after plan verification; layout, roots, runtime facts, and logical charges
remain explicit. Every execution surface remains an Accepted Target until its
cross-engine acceptance gate passes.

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
