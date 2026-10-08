# Native dependency condensation and dependency-first stages

## Decision and scope

Extend the existing native complete dependency-component witness with an ordinary
lkjscript consumer that computes the condensation graph and dependency-first
stages. Keep the component analyzer's public shape unchanged. The new
`dependency-plan` commands interpret `node.successors` as that node's dependencies:
for a cross-component edge A -> B, B must appear in an earlier stage than A.
Both maintained flat and chunked worklist selections feed the same planner.

This is a native tooling capability over provisional proposals, not a replacement
for the production compiler, a new intrinsic, or a scheduler with execution grants.
The accepted meaning graph remains the only program-meaning authority. No Rust
runtime, type system, encoding, artifact contract or existing command changes are
required. Source and distribution acceptance remain separately status-owned.

## Complete input and identity

The existing analyzer admits all nodes, roots and edges before this pass starts.
Its inclusive capacities and diagnostic precedence are inherited unchanged. An
invalid unreachable edge still rejects the proposal; roots select the reported
reachability partition, not the input's admission closure. Cycles are valid graph
structure and remain explicitly marked, including singleton self-loops.

Each component retains its members in authored node order. Its representative is
its first authored member, including any signed I64 identity. Component rows and
members keep their existing ordering. Representatives are derived output references,
not newly allocated meaning identities or claims of stability across a changed
component partition. Internal nonnegative positions never escape as identities.

Traverse original nodes and each successor list in authored order. Eliminate
component-internal edges and retain the first occurrence of each distinct ordered
component pair. The output dependency list contains target representatives in that
first-reference order, including references contributed by different members.
Duplicating edges does not change the plan. Reordering successors can change the
reported dependency-list order, but not the stages or component partition.
Reordering authored nodes can change representatives and presentation order.

## Mechanism and obligations

The component partition indexes each node to a bounded component position. One
construction pass records distinct outgoing dependencies, reverse dependents and
remaining dependency counts. An operation-local nested map rejects duplicate pairs;
its entries are not global certificates. The planner does not re-run reachability
for each component, enumerate paths or repeatedly scan every edge for every stage.

Seed a queue with zero-dependency components. Each processed component decreases
its distinct dependents' remaining counts and proposes its own level plus one.
Retain the maximum proposed level. A dependent becomes ready only when its last
dependency completes. Components with no dependencies have level zero; others
have one plus their largest dependency level. Queue order is not semantic output
order: a final authored-component scan groups representatives by level.

A cycle among distinct strongly connected components would make all of them mutually
reachable, contradicting the complete maximal partition. Thus the constructed
condensation is acyclic. Inductively, a ready component has all dependency levels,
and the maximum rule computes its earliest dependency-respecting stage. Distinct
pair counting permits only one readiness transition per component. Completion
requires every component exactly once; range, duplicate, negative-count and stall
checks reject a contradictory traversal rather than exposing a partial plan.

The helpers require the exact validated proposal and complete component partition.
Their structural type signatures do not prove graph invariants. These internal
consistency checks are not a second general-purpose validator for arbitrary caller-
supplied maps; source construction plus independent result tests carry that claim.
The existing SCC pass still requires the selected worklist's LIFO law, which its
ownership/type signature alone does not prove.

`inconsistent("incomplete-component-order")` reports a derived-plan contradiction,
not malformed input or a claim that a legitimate cycle is invalid. Normal capacity
and invalid outcomes retain their existing classes. Runtime interruption, exhausted
execution policy and allocation failures remain runtime failures without successful
partial command output. The command never executes graph nodes or emits effects.

## Authority and cost boundaries

Stages cover the entire admitted graph, including unreachable components. Members
of a cyclic component are an unresolved mutually dependent unit; their internal
order is not a valid sequential execution schedule. Separate components in one
stage have no dependency path between them, but that fact alone does not establish
memory independence, disjoint effects, deployment grants, parallel safety or a
legal publication order. Those obligations remain at their existing owners.

Graph work visits bounded nodes, distinct edges and components. Persistent map/list
operations, ordinary value admission, retained working storage, preparation and
result encoding have separate costs. A queue with at most one entry per component
is not evidence of zero copying, bounded RSS or linear wall time. No performance,
API-cost or application speedup is inferred from this addition.

## Verification

Author every helper and consumer from literal requests with ordinary `change plan`
and `change apply`. Use separately exported worklist and carrier dependencies with
exact supplier bindings. Verify unchanged canonical drafts. Copy the executable
outside the source checkout, build an artifact, then compare complete results
before and after deleting all disposable authoring projects and transports.

The host oracle uses pairwise reachability for SCCs and synchronous longest-path
relaxation for stages, not the product's two-pass SCC and readiness-count algorithm.
Coverage includes every three-vertex directed graph, every loop-free four-vertex
directed graph, duplicate/cross-member edges, signed extremes, long chains, cyclic
diamonds, root changes, authored-order changes, inclusive capacities and capacity/
structural refusals. Internal derived-state adversaries exercise the contradiction
checks through both evaluators. Raw type and execution-policy refusals have absent
result files and subsequent valid recovery. Cleanup and unchanged artifact/executable
bytes are checked independently of graph equality. Passing test counts and exact
source receipts belong in status, not in this design's proof sketch.

The dependency-first edge convention is also used by the documented
[Boost topological sort output](https://www.boost.org/doc/libs/1_85_0/libs/graph/doc/topological_sort.html).
That reference is not a dependency, implementation oracle or performance guarantee.
