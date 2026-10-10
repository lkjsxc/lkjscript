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

Remaining dependency counts and levels use owned I64 sequences, and visited flags
use an owned Bool sequence. Ready component positions use an append-only I64
sequence with a scalar head; each component is queued at most once, bounding its
logical retained entries by the component count. Indexed reads return ordinary
immutable values; replacement explicitly returns the displaced scalar alongside
the sequence owner. The final sparse levels map is materialized after traversal,
with zero-level components omitted. Missing input counters mean zero, and unrelated
map keys do not introduce components.

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

Internal signed positions are checked before sequence indexing. Invalid endpoints,
duplicate releases, negative counters and stalled traversal return
`complete=false` with empty levels. Refusal releases every working owner; the
public outcome remains the established `incomplete-component-order` inconsistency.

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

The native planner performs bounded node/edge/component visits. Sequence capacity
can retain spare storage, and the append-only queue retains consumed positions
until cleanup. Persistent input/output collections, admission, preparation,
allocation and encoding have separate costs. Compare chains, wide frontiers and
diamonds through existing capacities; check bounded per-update admission without
rescanning untouched sequence prefixes, and retain slower measured outcomes.
This is not a linear wall-time, zero-copy, bounded-RSS or application-speedup claim.
It does not replace a production compiler pass or establish complete self-hosting.
A later production consumer needs its own checked admission and authority boundary.

## Matched storage measurements

The retained Host08 study compared the former map/list `plan-stages` from
`b9cacb40` with the owned-sequence implementation above. The former literal changed
only its module name. Both algorithms were authored through public requests into
one artifact, selected by one Boolean argument; component discovery, condensation,
presentation and dependencies were shared. The final executable admitted the
unchanged Host03-produced artifact independently.

All ten workloads passed complete independent expectations and joined cleanup in
20 detached preflight runs, 20 cache-warming runs and 140 measured runs. Each case
used seven pairs in alternating order, on CPU 20 of an AMD Ryzen 9 9955HX during a
coordinated build-free window. These are median invocation times, excluding
artifact preparation and result encoding:

| Workload | Former map/list (ms) | Owned sequence (ms) | Time ratio | Modeled byte ratio |
| --- | ---: | ---: | ---: | ---: |
| Stages, empty | 0.061 | 0.232 | 3.79 | 1.001 |
| Stages, single | 0.149 | 0.554 | 3.73 | 1.004 |
| Stages, chain 64 | 1.434 | 23.219 | 16.19 | 1.226 |
| Stages, chain 512 | 11.106 | 184.561 | 16.62 | 1.808 |
| Stages, wide 512 | 10.315 | 183.157 | 17.76 | 1.839 |
| Full plan, empty | 0.268 | 0.367 | 1.37 | 1.001 |
| Full plan, single | 0.555 | 0.947 | 1.70 | 1.004 |
| Full plan, chain 64 | 19.329 | 41.377 | 2.14 | 1.089 |
| Full plan, wide 128 | 42.202 | 84.681 | 2.01 | 1.077 |
| Full plan, chained four-node cycles, 128 nodes | 31.423 | 42.497 | 1.35 | 1.028 |

Ratios are owned sequence divided by former map/list. This implementation was
slower and reserved more modeled bytes in every measured case, despite reducing
persistent collection work. For stage chain 512, map nodes allocated fell from
14,309 to 5,101 and list element-handle copies from 7,936 to zero; allocation
charges rose from 104,434 to 206,316. The new chain's invocation time grew 7.95
times from 64 to 512 components; this finite sample does not establish asymptotic
complexity or identify the dominant runtime cost.

Each run started a separate foreground process and prepared the artifact again
(roughly 147–150 ms); cache warming does not imply a resident prepared runtime.
Modeled byte reservations include common proof/admission costs and are neither
live heap usage nor RSS. Small-case ratios are sensitive to sub-millisecond noise.
These measurements establish no general application speedup or API-cost saving.
Any subsequent optimization needs a separately identified comparison.

Local original evidence is retained at
`.artifacts/data-sequences/planner-cost-final/summary.json`, with every timing pair,
complete result, cleanup observation, input, literal source and operation log.
`artifact.json` and `authoring-lineage.json` bind execution binary SHA-256
`4712f702eac30ddb0376fbc3fe556a0b5ae370e38af1346c27c3aa0abb677779`
to unchanged artifact SHA-256
`58c9f7f09caaf84eedfe6001e625d2fac0cb6049caab5e9aac8c025105f66b81`;
the original public authoring receipts remain in
`.artifacts/data-sequences/planner-cost/`.

### Repeat with prepared type lookup

Host09 moves admitted composite type lookup into preparation. A separately retained
repeat used the identical artifact, literal inputs, expected results, measurement
harness, CPU affinity and 180-run protocol. All complete outputs and cleanup checks
passed again; modeled counters were deterministic across repeats. The original
Host08 evidence above remains unchanged.

| Workload | Former map/list (ms) | Owned sequence (ms) | Time ratio | Modeled byte ratio |
| --- | ---: | ---: | ---: | ---: |
| Stages, empty | 0.064 | 0.145 | 2.27 | 1.001 |
| Stages, single | 0.148 | 0.210 | 1.42 | 1.004 |
| Stages, chain 64 | 1.422 | 3.575 | 2.51 | 1.225 |
| Stages, chain 512 | 11.059 | 27.837 | 2.52 | 1.805 |
| Stages, wide 512 | 10.252 | 27.185 | 2.65 | 1.837 |
| Full plan, empty | 0.174 | 0.192 | 1.11 | 1.001 |
| Full plan, single | 0.397 | 0.452 | 1.14 | 1.004 |
| Full plan, chain 64 | 12.420 | 14.569 | 1.17 | 1.088 |
| Full plan, wide 128 | 27.771 | 32.104 | 1.16 | 1.077 |
| Full plan, chained four-node cycles, 128 nodes | 20.486 | 21.480 | 1.05 | 1.028 |

Both implementations benefit from the general runtime lookup change, including
their shared component discovery. The sequence planner still has higher median
invocation costs in every case: approximately 2.5–2.7 times the former stage cost
and 1.05–1.17 times the whole-plan cost on these nontrivial workloads. Its larger
modeled allocation totals also remain. This consumer demonstrates the language
capability and its actual costs; these results do not establish faster storage.

Instruction counts, calls, allocation-charge counts and value-work counters are
unchanged between Host08 and Host09 for each algorithm. Prepared type metadata
grew from 4,920,810 to 4,961,954 modeled bytes, and preparation type-derivation steps
from 121,418 to 122,187. The same 41,144-byte increase appears in each invocation's
modeled allocation total. Preparation medians remain approximately 147–150 ms;
the table excludes this preparation cost. Retained individual pairs show the
variation, including occasional favorable small-case samples.

The repeat's exact execution binary SHA-256 is
`168f65a7efaba572f30532cfef1affa05be29cd5bbd95593c096bef1ccfa38db`.
Its original evidence is
`.artifacts/data-sequences/planner-cost-prepared/summary.json`;
`comparison-lineage.json` binds the unchanged Host08 summary, artifact and harness.
The same process, memory-accounting and scope limitations apply to both tables.
