# Restrict type-growth proof to exact callable cycles

Decision: discover the complete finite exact callable-context graph before
materializing prerequisite type-provenance paths. Keep independent admission of
all source operands. Materialize those paths and connect type-flow edges only
for calls whose endpoints are in the same strongly connected component.

## Why this boundary

The existing witness-shape interner already avoids copying duplicate selection
subtrees. Its type-growth proof nevertheless expanded every typed prerequisite
path at every callable context. An acyclic sequence of two-prerequisite adapters
could therefore exhaust work or metadata admission even though the selected
shape DAG remained small. Raising the budget would move that failure, not remove
the exponential expansion. Merging all occurrences of the same scheme would be
unsound: two identical shapes can carry different type arguments, and a closed
reset must not become a growing recursive path through another occurrence.

Every edge in the type-parameter flow graph belongs to a concrete caller-to-callee
edge. A cycle of type slots therefore projects to a closed walk of exact callable
contexts. All its context edges must lie within one context SCC. Conversely,
removing an edge between different context SCCs cannot destroy a type-slot cycle.
The retained induced type graph is sufficient for the original expanding-cycle
criterion. Singleton components need no witness slots unless they have a self-call.

Exact context identity includes the package/declaration, selected method and
ordered prerequisite shape IDs. Different methods or selected declarations cannot
be merged merely because signatures agree. The earlier conservative rejection of
potential recursive witness construction remains a separate prerequisite; this
decision neither relaxes it nor claims finite exact-context exploration without it.

## Implementation obligations

Discovery still visits all declarations, method mappings, function values and
untaken syntax. Each concrete witness and each call admits every ordered type
argument, its complete type-object closure and defining lexical scope before the
proof graph is pruned. Missing bodies, phantom types, wrong arities and foreign
parameters cannot become valid by residing on an acyclic edge.

Recorded calls bind source context, target context and source expression, or the
exact selected method mapping. Recursive connection retains the previous separate
witness-path slots and argument-flow rules. No process-global memo table or
cross-revision proof cache is introduced. The operation owns all temporary graph
storage and cancellation checkpoints; modeled work and metadata are admitted
before growth. Resource exhaustion remains distinct from semantic expansion.

## Evidence and remaining work

A retained failing baseline exercises a 24-layer typed duplicate-prerequisite
DAG. Focused tests compare small instances with an unpruned path mode. A separate
weighted transitive-closure oracle checks permutations, resets, phantom arguments
and nested constructors; neither production SCC implementation is its oracle.
Equal-shape/distinct-provenance and exact selected-method adversaries remain.
Budget exact/N-1 and late cancellation probes cover the new discovery/SCC boundary.

The [public workload](../../examples/compact-callable-proof/README.md) authors and
exports a generic library before concrete implementations exist, then checks exact
selected readers and detached execution through copied product bytes. Acceptance
and measured observations belong to [status](../status.md) and their original logs,
not to this design explanation.

This change does not establish polynomial preparation in general. Recursive
contexts still expand distinct prerequisite type paths; the number of exact
contexts can grow independently; concrete prepared witnesses retain their current
materialization costs. Demand-driven recursive provenance, proof-state sharing
with exact substitutions, and prepared-witness DAG storage are separate follow-ups.
No graph, type-object, interface or artifact encoding changes are required by this
derived proof selection. Existing accepted meaning remains unchanged.
