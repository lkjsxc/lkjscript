# Native dependency-first component planning

This ordinary lkjscript tooling program extends the
[complete dependency-component analyzer](../dependency-components/README.md).
It condenses cycles into explicit units and returns dependency-first stages.
The [decision](../../docs/decisions/native-dependency-plan.md) owns correctness
and authority boundaries; [status](../../docs/status.md) owns actual acceptance.

## Edge direction and complete result

Here `node.successors` means **dependencies of that node**, not tasks that should
run after it. For A -> B, the component containing B must precede the component
containing A. Cyclic components are retained as mutually dependent units; this
program does not choose an internal execution order for their members.

The input is one proposal record inside the command's argument list:

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

Both maintained storage selections return:

```json
{"case":"valid","value":{
  "components":[
    {"cyclic":false,"members":[10],"dependencies":[20,30]},
    {"cyclic":true,"members":[20,40],"dependencies":[]},
    {"cyclic":false,"members":[30],"dependencies":[20]},
    {"cyclic":true,"members":[50],"dependencies":[]},
    {"cyclic":false,"members":[60],"dependencies":[]}
  ],
  "stages":[[20,50,60],[30],[10]],
  "reachable":[10,20,30,40],
  "unreachable":[50,60]
}}
```

A component is referenced by its first authored member. Each component and member
list retains authored node order. Dependencies retain the first distinct
cross-component reference encountered while scanning authored nodes and successor
lists. Internal edges disappear from that list but still determine cyclicity.
Repeated edges, including repetitions contributed by different members, appear
once. A dependency list is not sorted by numeric ID or component rank.

Stages contain representatives, never internal positions. Stage zero contains
components without dependencies; each other component belongs to one plus the
largest stage of its dependencies. Within one stage, representatives follow
component order, independently of queue traversal order. Every component appears
once, including unreachable ones. Empty graphs return empty components and stages.
Root changes affect reachability, not the all-component plan. To plan only the
root-required closure, a later consumer can select the returned reachable units;
that does not permit skipping complete proposal validation.

These stages describe graph precedence, not permission to run tasks concurrently.
They establish neither disjoint memory/effects nor execution or publication grants.
They do not make a cyclic component's member execution order valid.

## Author, check, build and detach

Follow the component guide to export the separately authored `owned-worklists`
and `worklist-carriers` packages against the exact builtin standard. In a fresh
minimal application, stage all three transports and bind the exact package IDs,
semantic revisions and package revisions in ordinary change requests.

Author `../owned-worklists/reachability.lkjc`, then the existing component modules
`order.lkjc`, `groups.lkjc`, `index.lkjc`, `application.lkjc` and `batch.lkjc`.
Author this directory's `edges.lkjc`, `stages.lkjc`, `presentation.lkjc`,
`application.lkjc` and `batch.lkjc` in that order. Each file is a separate request
with the current `request base=REVISION`; use `change plan` and its exact returned
plan token for `change apply`. Do not concatenate duplicate `use std builtin`
clauses into a single unit collection. No external semantic generator is needed.

Run `check`, then verify that canonical drafts of the five new modules plan
unchanged. Build the application beside the deployment descriptors:

```sh
./lkjscript build --output dependency-plan.lkja
./lkjscript run --deployment flat.deployment.json \
  --arguments-file arguments.json --result-file flat-result.json
./lkjscript run --deployment chunked.deployment.json \
  --arguments-file arguments.json --result-file chunked-result.json
```

Result paths must be absent. The descriptors select ordinary trusted foreground
execution with no configuration, secrets or grants. They do not invent a cumulative
instruction budget or establish hostile-code containment. Existing component
commands are unchanged and coexist in the authored application.

Batch descriptors take a single list of proposals, with argument shape
`[[PROPOSAL, PROPOSAL]]`, and return one independent Outcome per proposal in order.
Empty batches return `[]`. A runtime interruption does not become successful partial
batch output. Every proposal has its own complete validation and planning state.
The artifact remains executable after deleting its disposable authoring projects
and all staged/exported package transports.

## Refusals

The inherited inclusive capacities are 4,096 nodes, 4,096 roots and 16,384 successor
entries per proposal. Repeated entries count before condensation. Capacity refusals
precede structural invalidity in nodes, roots, edges order. Duplicate IDs precede
missing roots, then missing successors in authored order. Invalid unreachable edges
are never ignored. Signed I64 extremes and -1 are ordinary valid node identities.

Outcomes `capacity` and `invalid` retain the component analyzer's exact payloads.
`inconsistent` with payload `incomplete-component-order` reports a contradiction
in derived traversal state, not a normal classification of a cyclic input graph.
Counter, rank, duplicate and completion checks prevent a contradictory traversal
from yielding a valid partial plan. Internal helper signatures do not prove the
complete proposal/partition invariants; these helpers are not standalone validators
for arbitrary caller-supplied maps.

Malformed runner data and explicit instruction-policy exhaustion remain runtime
failures with no successful result file. They are neither graph-capacity outcomes
nor permission to retry live effects. This program is pure and executes no graph
node actions. Allocation failure and cancellation retain ordinary runtime behavior.

## Acceptance family and limits

The public regression independently expects 4,635 proposals: the existing 532-case
component suite, all 4,096 loop-free four-vertex graphs, and seven larger planning
and metamorphic cases. The existing suite includes all 512 three-vertex graphs
with self-loops, signed extremes, 257-node chains/cycles, wide worklists, inclusive
capacities and one-past-bound/structural refusals followed by valid proposals.

The oracle uses pairwise reachability for components and synchronous longest-path
relaxation for stages. The product uses two-pass component discovery followed by
remaining-dependency counts and a readiness queue. Complete results are compared
for both carriers before and after source deletion: 18,540 proposal comparisons.
The suite uses 19 batches of at most 256 proposals, making 76 batch executions.
Each argument file stays within the ordinary 1 MiB input-byte limit; that limit
is independent of graph capacities and is not increased for this workload.
Separate native fixed-result tests use both evaluators; the public matrix executes
production only. Do not describe it as 18,540 differential evaluator executions.

Public checks additionally cover single commands, empty batches, malformed final
unreachable data, resource refusals and recovery. They require unchanged copied
executable/artifact bytes, zero remaining local handles and joined cleanup. Exact
passing results and receipts must be read from status, not inferred from this test
inventory. Retained literal proposals and expected files are evidence only and
are not read by the detached program.

The native planner performs bounded node/edge/component visits, but persistent
collections, admission, preparation, allocation and encoding have separate costs.
This is not a linear wall-time, zero-copy, bounded-RSS or application-speedup claim.
It does not replace a production compiler pass or establish complete self-hosting.
A later production consumer needs its own checked admission and authority boundary.
