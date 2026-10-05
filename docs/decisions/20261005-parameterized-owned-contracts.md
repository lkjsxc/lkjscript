# Compose storage through explicit owned contract arguments

Date: 2026-10-05 (UTC).

## Decision

Extend the existing nominal Owned contract with ordered additional Owned
parameters and finite structural owned method signatures. Retain distinguished
Self, explicit static witness selection and monomorphic implementation targets.
The [specification](../spec/owned-contract-parameters.md) owns exact semantics;
[status](../status.md) owns acceptance and availability.

An element-removal method needs to return both the reusable storage owner and
the removed element owner. Direct-Self-only signatures could not express that
operation even though owned products, choices and sequences already provided
the required runtime carriers. `Worklist<Item>` composes those established
features through one application boundary.

## Rationale and witness

Explicit arguments make storage representation and element representation
independent while keeping all evidence at package boundaries. Exact witness
matching includes contract identity, Self and every ordered argument. Structural
substitution derives complete method types from the original signature; unused
arguments and unused method mappings remain checked.

Flat and 32-element chunked worklists implement the same contract for scalar
cells and buffers. Generic transfer exercises different source and destination
storage types. A native provisional dependency-graph validator uses owned cells
as work items and returns complete ordered reachability or typed diagnostics.
These are designed language witnesses, not a claim of production compiler
self-hosting or maintained application adoption.

The coordinated format cut removes unreachable predecessor compiler encoding
paths while retaining readers required by maintained accepted meaning and original
historical evidence. Compatibility layers are not an acceptance requirement for
owner-authorized experimental data.

## Next trigger and measurement

Associated projections and lifetime relationships become useful when a generic
algorithm must return a borrowed view tied to its storage owner. Require that
concrete workload before selecting scoped associated views, higher-ranked borrowing
or general regions. Region custody should follow measured working-set and
reclamation needs.

Measure preparation time, execution time, allocation and retained storage for both
representations on matched complete outputs. A difference in representation or
allocation identity is not a performance result. Retain unfavorable results and
claim API-cost savings only from actual usage measurements.
The [matched worklist and graph measurements](../performance.md#parameterized-owned-worklists-2026-10-05)
retain higher chunk32 invocation/allocation costs. Flat storage is the practical
baseline for those workloads; preparation dominates the flat small cases. Future
preparation sharing needs its own independently admitted, measured increment.

Idle executor retention of shared worker capacity is a separate runtime problem.
Its admission, fairness, cancellation and joined-cleanup contract should not be
coupled to this type-system change.
