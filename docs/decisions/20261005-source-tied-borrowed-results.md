# Compose read-only library results with explicit source relationships

Date: 2026-10-05 (UTC).

## Decision

Add a pure function result relationship `ReadFrom(source)` and a matching pure
owned-contract method relationship by ordered parameter position. Callers adopt
these results through an explicit lexical `borrow-call`. Keep the existing owned
value type and read rights; preserve exact parameter provenance and ancestor
custody across calls. The [specification](../spec/owned-read-results.md) owns
semantics, and [status](../status.md) owns acceptance and availability.

## Rationale and witness

Parameterized worklists can construct, transfer and consume generic storage, but
an indexed reader could only observe a child internally. A generic selector needs
to return that child to its consumer. One explicit source relationship supplies
this missing composition without choosing a general reference type or inferring
lifetimes from incidental representation.

The existing explicit Item argument suffices for `IndexRead<Item>` because every
maintained reader returns the same element representation. Associated view types
should follow a concrete workload that needs a different representation.
Storage ownership stays independent of physical address, allocator and packet
implementation. Aliased arguments remain different semantic origins.

The native witness exports a generic first-max selector before concrete readers
exist. Four readers cover flat and chunk32 storage with cells and buffers. An
alternate key observer creates distinguishable ties; the returned value is then
read with an identity observer. Both views end before the original collection is
fully drained and its empty owner reused. The scan retains an ordinary winning
index and closes each read scope before recursion, avoiding one live loan per
examined element.

## Boundaries and next triggers

One source root, pure named functions and lexical read scopes bound this increment.
Task transfer, mutable views, nullable views and alternative source roots need
additional lifetime contracts. Tail-call activation replacement across a borrowed
result is disabled until transfer can preserve its complete contract independently.
Sealed packets and reserve-before-detach transfer make failures have one cleanup
owner; independent reference and loader admission remain required.

Keep loan metadata separate from ordinary value payloads. Materialize and charge
it only where memory provenance or custody requires it, preserving established
ordinary workloads under their existing allocation limits. A lexical choice arm
can move an ordinary payload and still return its borrowed root; only remaining
loan guards need to accompany that root.

Associated views should follow differing reader representations. Regions should
follow measured working-set and reclamation needs. Shared auxiliary worker capacity
and bounded production self-hosting remain separate milestones with explicit
runtime and host-boundary contracts. Representation variety and borrow identity
do not establish performance or API-cost savings; use complete matched workloads
and actual usage evidence for those claims.
