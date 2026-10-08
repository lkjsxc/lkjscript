# Native dependency components

This native compiler-tooling witness partitions a complete provisional dependency
graph into strongly connected components. The algorithm is authored in lkjscript,
not supplied by a host oracle, intrinsic or privileged meaning-graph writer.
[Status](../../docs/status.md) owns acceptance and availability; the
[decision](../../docs/decisions/native-dependency-components.md) owns its boundary
with later compiler integration.

## Complete observable result

The command accepts one record containing authored nodes and roots. For example:

```json
[{"roots":[10],"nodes":[
  {"id":10,"successors":[20,30]},
  {"id":20,"successors":[40]},
  {"id":30,"successors":[40]},
  {"id":40,"successors":[20]},
  {"id":50,"successors":[50]},
  {"id":60,"successors":[]}
]}]
```

Both storage implementations return the same result:

```json
{"case":"valid","value":{
  "components":[
    {"cyclic":false,"members":[10]},
    {"cyclic":true,"members":[20,40]},
    {"cyclic":false,"members":[30]},
    {"cyclic":true,"members":[50]},
    {"cyclic":false,"members":[60]}
  ],
  "reachable":[10,20,30,40],
  "unreachable":[50,60]
}}
```

Every node belongs to exactly one component, including unreachable nodes. Members
retain authored node order; components are ordered by their first authored member.
A component is cyclic when it has multiple members or its singleton has a self-edge.
Repeated roots and edges are permitted. Identities span the entire signed I64 range;
internal labels and traversal positions are not returned as identities.

## Author, build and detach

Use the [owned-worklist authoring sequence](../owned-worklists/README.md) to create
and export `library.lkjc` before creating the independent `carriers.lkjc` package.
Both use the exact staged builtin standard. In a fresh minimal application, stage
that standard and both exported packages; import the latter as `owned-worklists`
and `worklist-carriers` with their observed package IDs and package revisions.

Author the following literal inputs in order, each as its own ordinary change
request with the current `request base=REVISION`. Each request uses both exact
package aliases. Do not concatenate their `use std builtin` clauses into one unit
collection: supplier aliases must be unique within a request.

1. `../owned-worklists/reachability.lkjc` admits the complete proposal and owns the
   existing reachability partition and diagnostic precedence.
2. `order.lkjc`, `groups.lkjc` and `index.lkjc` provide finish order, reverse-graph
   traversal and deterministic presentation.
3. `application.lkjc` exposes the validated single-proposal API; `batch.lkjc` adds
   independently validated batches.

For each input, use `change plan --input-file FILE`, then `change apply
--input-file FILE --plan TOKEN` with the exact returned token. Run `check` and
verify that an unchanged canonical `change draft --module MODULE` plans unchanged.
No external semantic generator participates. The helper modules are internal
companions whose input preconditions are established by the validated application;
they are not independent complete-proposal validators.

Build beside the supplied descriptors:

```sh
./lkjscript build --output dependency-components.lkja
./lkjscript run --deployment flat.deployment.json \
  --arguments-file arguments.json --result-file flat-result.json
./lkjscript run --deployment chunked.deployment.json \
  --arguments-file arguments.json --result-file chunked-result.json
```

Use absent result paths. Both descriptors select the same immutable artifact with
no configuration, secrets or grants. Runtime and execution policy are omitted,
selecting ordinary trusted foreground execution rather than an invented cumulative
instruction budget. This does not establish hostile-code containment.

`batch-flat.deployment.json` and `batch-chunked.deployment.json` take one list of
proposals, so the arguments file has two array levels: `[[PROPOSAL, PROPOSAL]]`.
The result is one Outcome per proposal in order. Empty batches return `[]`.
Each proposal has its own validation, reachability and component state. Batching
shares invocation preparation, not graph state or acceptance authority.

## Refusals and capacity

Existing complete validation precedes component traversal. Inclusive per-proposal
bounds are 4,096 nodes, 4,096 roots and 16,384 successor entries; repeated entries
count. Capacity takes precedence in nodes, roots, edges order and returns
`{"case":"capacity","value":"nodes"}`, or the corresponding dimension.

Structural invalidity returns `{"case":"invalid","value":{"code":TEXT,
"owner":I64,"target":I64}}`. Duplicate IDs precede missing roots, then missing
successors, each in authored order. Invalid unreachable edges are not ignored.
A missing root has owner -1; the diagnostic code distinguishes that role from a
valid node whose identity is -1. This reuses the existing validator rather than
silently inventing a different admission contract.

Wrong argument types reject at the ordinary runner boundary. Traps, cancellation,
allocation failure or explicitly selected execution-budget exhaustion are runtime
failures, not successful typed graph outcomes. A batch does not turn an interrupted
invocation into a partial successful result.

## Mechanism and proof

The pass computes finish order using explicit enter/leave frames, then traverses
reversed edges in decreasing finish order. First-pass marking occurs on entry,
not when all siblings are queued. Edges between pending siblings therefore retain
the required DFS ordering. Frames carry bounded positions, never arithmetic on
signed IDs. Generic owned worklists are drained and their empty owners reused
between trees/components; graph depth does not become recursive call depth.

The first pass requires LIFO stack behavior. The maintained FlatCells and
ChunkedCells witnesses provide it, but the Worklist type signature alone does not
prove that algebraic law for an arbitrary implementation. The correctness scope
is these explicit selections and other implementations satisfying that law, not
every function with matching ownership/type signatures.

The public regression uses a test-only pairwise-reachability oracle independent
of this two-pass algorithm. It includes all 512 directed three-vertex graphs,
signed extremes, disconnected components, 257-node chains and cycles, broad
worklists, repeated references, inclusive capacities and one-past-bound refusals.
It requires equal complete results from flat and chunked storage before and after
removing authoring projects and transports, unchanged artifacts/executables,
zero remaining owned handles and joined cleanup. Empty batches are checked by
both native evaluators and by both detached public commands. Literal requests and input/output
files can be retained as evidence; they are not consulted by detached execution.

Graph visits are bounded by nodes and edges, but persistent collections, type
metadata, preparation and result encoding have additional costs. This is not a
linear wall-time, zero-copy, speedup or complete self-hosting claim. The pass does
not read, edit, validate on behalf of, or publish the accepted compiler meaning
graph. Integrating it into a production compiler requires its own checked boundary.
