# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

<!-- LKJ-STATUS id=agent-foundation/1 status=historical -->
<!-- LKJ-STATUS id=agent-work-state/1 status=historical -->
<!-- LKJ-STATUS id=agent-work-state/2 status=current -->
<!-- LKJ-STATUS id=edition-2-identity-migration/1 status=current -->
<!-- LKJ-STATUS id=edition-2-enum-declarations/1 status=current -->
<!-- LKJ-STATUS id=edition-2-never-control/1 status=current -->
<!-- LKJ-STATUS id=edition-2-numeric-conversions/1 status=current -->
<!-- LKJ-STATUS id=edition-2-semantic-core/2 status=accepted-target -->
<!-- LKJ-STATUS id=jit-auto-promotion/1 status=accepted-selection -->
<!-- LKJ-STATUS id=jit-proof-forced/1 status=current -->
<!-- LKJ-STATUS id=repository-graph-context/1 status=current -->
<!-- LKJ-STATUS id=repository-topology/1 status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler/2 status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation/2 status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger/1 status=accepted-target -->
<!-- LKJ-STATUS id=semantic-session/1 status=current -->
<!-- LKJ-STATUS id=semantic-source-foundation/1 status=current -->
<!-- LKJ-STATUS id=semantic-source-schema/1 status=historical -->
<!-- LKJ-STATUS id=semantic-source-schema/2 status=current -->
<!-- LKJ-STATUS id=typed-holes/1 status=current -->

**Current** only for implementation and evidence explicitly labeled Current in
the manifest capsules. Bounded Repository Topology, Repository Intelligence
Graph/context, externalized Agent Work State V2 with semantic references,
complete Semantic Source Schema V2 with its preserved V1 base, typed holes,
legal actions, closed hole transactions, one-shot query/edit protocol, bounded
local stdio sessions, the Edition 2 identity/non-publishing migration slice,
generic enum declarations/resolved type facts, exact `variant-value`
construction, exhaustive source match, Never/structured control, the generic
prelude `Option`/`Result` cutover with closed typed errors, and the four explicit
numeric conversions through verified ordinary SSA, the evaluator, validated
bytecode, reference VM, and forced Linux x86-64 baseline/proof execution,
compiler Resource Profile V2, and the core hierarchical pre-allocation
foundation are Current. Agent Foundation V1 and Semantic Source Schema V1
identities are historical and rejected. Other Edition 2 changed semantics,
execution cutover, migration publication, whole-pipeline
pre-allocation, logical metering beyond enum construction, nonzero query
caching, and cross-authority ledgers remain Accepted Targets, not Current.
Deferred and Rejected work remains non-current.

## Current Baseline

Semantic Source Foundation V1, resolved typed HIR, verified typed SSA,
reference bytecode/VM, callable host-independent Linux x86-64 baseline code,
host-independent native allocation/collection, and forced proof-checked
optimization are Current at baseline `dd8fb316`. Exact boundaries and retained
historical command evidence are in the manifest below. Automatic proof
promotion, wider future Agent operations, Edition 2 surfaces beyond the Current
identity/migration, enums/match, and Never/control slices, cross-authority
aggregate profiles, OSR, AOT/cache, and wider host-native transitions are not
Current.
Structure, graph/context, work-state, one-shot protocol, and compiler-profile
commands/APIs are Current on the containing implementation revision.

## Implementation Sequence

1. [Bounded Repository Topology](decisions/platform/bounded-repository-topology.md) is Current.
2. [Repository Intelligence Graph And Context](decisions/platform/repository-intelligence-graph.md) is Current.
3. The bounded externalized [Agent Work State](decisions/platform/agent-work-state.md) service is Current.
4. [Complete Semantic Source Schema
   V2](decisions/platform/semantic-source-and-agent-protocol.md) is Current for
   the closed Edition 1 tree, typed holes/legal actions, transactions, and
   bounded one-shot/session protocol; its exact V1 base remains documented.
5. The [Profile V2 resource foundation](decisions/platform/resource-budget-profiles-candidate.md)
   is Current for core hierarchical reservation and the documented legacy
   compiler post-phase charging boundaries; whole-pipeline migration is not.

6. Explicit Edition 2 markers, homogeneous closure identity, edition-separated
   source/tree/revision/declaration identities, Schema V2 facts, and strict
   non-publishing Edition 1-to-2 migration check/diff are Current.
7. Generic nominal enum declarations, stable enum/variant/field identities,
   invariant resolved HIR type facts, bounded recursion/ownership validation,
   exact Profile V2 enum-shape preallocation, exact enum value construction,
   target-independent layout facts, verified SSA primitives/evaluator,
   validated bytecode, boxed active-payload GC representation, reference VM,
   exhaustive match planning/lowering, and forced Linux x86-64 baseline/proof
   JIT execution are Current. Typed prelude runtime boundaries are the Current
   native-host exception; broader transitions, publish, and corpus cutover remain targets.
8. Edition 2 Never is Current as a join-only HIR type with no value/storage/ABI
   representation. Typed loop and while control lower to verified SSA block
   parameters and ordinary terminators consumed by evaluator, bytecode/VM,
   forced baseline, and proof JIT; Semantic Source V2 and typed-hole operations
   expose the closed forms and exact control constraints.
9. Edition 2 numeric conversion is Current for exactly four canonical source,
   HIR, and SSA operations with stable nominal `NumericError`, host-independent
   bit/exponent algorithms, validated bytecode/VM, and forced baseline/proof JIT
   runtime calls with zero fallback. Edition 1 mixed numeric behavior remains
   migration input and is not available in Edition 2.
10. Prelude `Option T` and `Result T E` are ordinary stable-identity generic
   enums across HIR, SSA, evaluator, bytecode/VM, GC, and native plans. Closed
   `NumericError`, `Utf8Error`, and `SystemError` values use the same enum path;
   UTF-8 and host failures are classified once at their capability boundary,
   and dedicated option/result opcodes, heap objects, and native types are absent.

Current status applies only to each implemented bounded surface, not to broader accepted protocol designs.
Automatic baseline-to-proof promotion remains an Accepted Implementation
Selection after these repository-intelligence foundations; it is not enabled.

## Strict Capsule Manifest

### Current implementation and direction

- [resolved AI-native redesign baseline](current-state/resolved-ai-native-redesign-baseline.md)
- [compiler, semantic, and runtime implementation](current-state/current-implementation.md)
- [host capabilities and native runtime](current-state/current-native-runtime.md)
- [forced enum JIT evidence](current-state/forced-enum-jit-evidence.md)
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
