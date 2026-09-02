# Exact-interface affine resources for durable-queue leases

Date: 2026-09-01 UTC.

## Status

Accepted and implemented by campaign `202609011246` for unreleased product snapshot 0.1.15.
Immutable public `v0.1.14` and the operational queue-data format remain unchanged.

## Problem

Graph meaning granted access to the exact `DurableQueue` interface, but claim returned a copyable
record containing attempt identity and terminal operations accepted reconstructed job, attempt, and
worker text. The store rejected stale tuples, yet accepted meaning could still duplicate or reuse a
transition right. Program authority and runtime transition authority were therefore split.

A queue-specific syntax rule would close only this adapter and leave the compiler, artifacts, and
runtime able to copy similar future rights. General linear ownership, broad resource-valued
function signatures, and affine containers would broaden the language without a measured
maintained workload.

## Decision

At acceptance, Graph 6 owned `CapabilityResource<ExactInterface>` and canonical operation-parameter use modes
`unrestricted`, `borrow`, and `consume`. Only an exact-requirement capability call may acquire a
resource. Validation follows language order, retains the acquiring requirement and interface,
permits ordered borrows followed by at most one consume, and rejects fabrication, aliases,
post-consume use, foreign authority, inconsistent branch joins, function transfer, and durable or
aggregate escape before publication.

The original slice was deliberately lexical and affine: dropping is allowed; must-use linearity is
absent. A nominal variant may carry one direct resource and matching transfers that payload to the
selected arm. Campaign `202609021736` subsequently admitted the narrower requirement-bound private
task-helper signature described in
[requirement-bound affine task handoff](20260902-requirement-bound-affine-task-handoff.md).
Resource results, records, structural containers, collections, streams, constants, tests,
captures, ports, generic values, and every broader function signature remain forbidden.

Compiler-unit 2 and bytecode 2 preserved borrow/consume local loads. Artifact 11 and normalized
runtime 2 revalidated those decisions. The successor handoff advances Graph 7, compiler-unit 3,
bytecode 3, Artifact 12, and normalized runtime 3 to carry the exact helper binding. A live entry is
bound to one task scope, resource kind, exact
interface, and acquiring requirement. Borrow preserves the entry; consume removes it before the
adapter call. Host cloning is not a second right.

The standard `DurableQueue` interface has exactly nine operations. Claim and heartbeat return
`QueueLeaseState = absent | live(CapabilityResource<DurableQueue>)`; `lease-info` borrows and returns
job ID, attempt number, lease-until time, and payload; heartbeat, complete, and fail consume. Raw
attempt and worker identity remain private inside the resource entry and queue engine. Claim and
heartbeat reserve handle identity/capacity before effects so a successful transition cannot lose
authority to avoidable allocation failure.

## Consequences

Maintained `lkjournal` claims, matches, borrows metadata, renews, and consumes terminally without
threading attempt or worker tokens. Package interfaces, summaries, relations, impact, compiler
units, artifacts, public inspection, and compact authoring retain the exact resource/interface/use
meaning. Predecessor graph roots, package interfaces, artifacts, parameter modes, list-lease
results, and raw terminal signatures reject; no compatibility reader or adapter remains.

Task cleanup drops a local handle and performs no implicit queue transition. Stale, cancelled, or
possible-visibility consumes do not resurrect it. This narrows transition authority but does not
provide exactly-once work, hostile-code isolation, must-use completion, cross-task transfer, or
cross-capability atomicity.

The physical `lkjscript-durable-queue-data-1` representation, ordering, attempt policy, backup, and
restore remain operational authority and require no migration. Reversal would require a maintained
consumer that cannot express its protocol lexically, a stronger independently proved ownership
model, complete consumer/artifact migration, and deletion of this resource contract; raw token
authority is not an acceptable fallback.
