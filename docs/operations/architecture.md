# Architecture

## Purpose

Map Current product behavior and implemented repository-intelligence services
to their authorities.

## Status

The compiler/runtime map in the capsules is **Current** at baseline `dd8fb316`.
Complete Semantic Source Schema with its preserved legacy contract representation,
typed holes/legal actions, Bounded Repository Topology, Repository Intelligence
Graph/context, externalized Agent Work State canonical contract with exact semantic references,
the bounded one-shot semantic protocol, bounded local stdio sessions, and
compiler Resource resource profile and the the canonical source contract enum declaration/type-fact plus
construction/evaluator/reference-VM slice are Current. Agent Foundation legacy contract and Semantic Source Schema are
historical rejected identities. the canonical source contract exhaustive source match is Current;
automatic/host-native enum transitions, the remaining semantic core, whole-pipeline resource profile
pre-allocation, nonzero
incremental query caching, and logical metering beyond enum construction are
**Accepted Targets**.

## Current Compiler/Runtime Boundary

Validated Semantic Source Foundation tree -> resolved typed HIR -> verified
typed SSA -> verified baseline normalization -> reference bytecode is Current.
HIR consumes one private mechanically checked projection from the validated source tree.
That projection includes resolved generic enum declarations and exact `variant-value`
construction. No sibling parser or raw public AST enters analysis. VM,
baseline JIT, proof optimizer, future AOT tests, and Wasm must share verified
semantic IR rather than reinterpret syntax.

Immediately after typed HIR, effects, and ownership, separate exhaustive
producer and verifier traversals establish a dense content-addressed memory
plan. An opaque memory-verified HIR wrapper is the only SSA-construction input,
and `ExecutableProgram` retains the complete plan. The independently recomputed
direct-affine SSA inventory remains derived evidence. The public 62-record
inventory reports Current tracing layouts and Accepted deterministic candidates.
The first exact byte-vector family has explicit end-borrow/drop execution and
execution-owned `UniqueStore` backing in the evaluator, reference VM, and forced
native tiers. Native lowering assigns distinct unique/shared-loan/exclusive-loan
machine identities and selects an invocation-owned bounded unique/loan runtime
only after whole-group preflight. General regions, pools, broader unique storage,
immutable bytes/path, and collector removal remain Accepted Contract work.

Provider authority enters only through explicit closed capability parameters.
Package verification bounds grants; bytecode records exact main requirements;
VM entry validates them before source effects. Acquired handles remain object
capabilities and do not repeat provider parameters on every operation.
Capability-bearing functions remain VM-only where a native tier lacks the exact
value and operation contract; forced tiers reject before effects without
fallback. The sole Current native resource operation remains `standard-input` from exact
`stdio` capability to borrowed `input-stream`. It and the Current exact
byte-vector subset use the noncollecting invocation state; the former uses the
invocation-owned core resource table and the latter uses an invocation-owned
bounded `UniqueStore` and loan table. Mixed resource/unique groups reject before
effects until that composition is independently verified.

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
publication, and serialized output. Public compiler `_with_ledger` APIs borrow
one outer-owned compiler ledger. Validated source shape reserves enum/match HIR
work; immutable HIR reserves its charged input shape before SSA; immutable
normalized SSA reserves its charged input shape before bytecode. Parser-wide
preflight and exact bytecode-output categories are not Current, and protocol and
compiler ledgers remain separate. Exhaustion cannot publish partial authority.
the removed legacy source contract source limits remain unchanged.

## Accepted the canonical source contract Path

[the canonical source contract](../decisions/semantics/semantic-core.md) preserves the same architecture:
Semantic Source -> resolved typed HIR -> verified SSA -> evaluator/VM/native/
proof consumers. Generic enum declarations, resolved type facts, exact construction, independent
layout identities, enum SSA primitives, the SSA evaluator, validated bytecode,
reference VM execution, verified match planning/ordinary-CFG lowering, and
forced Linux x86-64 baseline/proof generated enum and match operations are
Current. Polymorphic entries, host enum operations, and automatic native/
VM reference transitions remain Accepted Targets.

## Authority Links

- [Collector-Free Deterministic Memory](../decisions/memory/collector-free-deterministic-memory.md)
- [Bounded Repository Topology](../decisions/platform/bounded-repository-topology.md)
- [Repository Intelligence Graph And Context](../decisions/platform/repository-intelligence-graph.md)
- [Agent Work State](../decisions/platform/agent-work-state.md)
- [Semantic Source And Agent Protocol](../decisions/platform/semantic-source-and-agent-protocol.md)
- [Resource Budget Profiles](../decisions/platform/resource-budget-profiles.md)

## Strict Capsule Manifest

- [Current crate graph, ownership map, and compile flow](architecture/crate-graph.md)
- [Current runtime flow and accepted execution direction](architecture/runtime-flow.md)

A capsule's historical or accepted text cannot promote a non-current capability.
