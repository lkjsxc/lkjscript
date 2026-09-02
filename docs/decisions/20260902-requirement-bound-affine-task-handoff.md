# Requirement-bound affine task handoff

Date: 2026-09-02 UTC.

## Status

Accepted and implemented by campaign `202609021736` for unreleased product snapshot 0.1.17.
Immutable public `v0.1.16`, deployment authority, and durable queue data remain unchanged.

## Problem

The maintained `lkjournal` worker acquired, inspected, renewed, processed, and terminally consumed
one queue lease in a 48-record task body. Named calls could separate ordinary policy, but every
function resource signature was rejected, so the lease lifecycle and application work could not be
split without reconstructing a transition right or adding application-private host semantics.

The VM already moves consume locals into a direct call frame within one task resource scope. What
was missing was graph-owned authorization tying one callee parameter to the exact requirement that
acquired the handle, plus validation of the deliberately narrow call shape.

## Decision

One function parameter may canonically store an optional exact `resource_requirement` reference.
It is present only for one final, direct `CapabilityResource<Interface>` parameter with `consume`
use on a private, same-package, nongeneric task function. The requirement must occur in that
function's effect and own the same exact interface. Every preceding parameter and the result is
resource-free and unrestricted. The binding participates in canonical graph bytes, relations,
summaries, impact, package interfaces, compiler units, artifacts, runtime preparation, definition
inspection, and compact `add.parameter requirement=...` authoring.

Only a direct named call may transfer the resource. Arguments evaluate left-to-right; ordinary
arguments finish before evaluating the final resource argument, whose consume commits the move.
Caller and callee use the same exact requirement identity. Resource-bearing calls form an acyclic
same-package graph, so a helper may forward once to another admitted helper but cannot recurse.
Function values, invoke, public/package/cross-package signatures, multiple or nonfinal resources,
borrow parameters, resource results, generic resource functions, and resource containers remain
rejected.

Compiler lowering emits the final consume load into the call. Artifact validation and normalized
preparation retain and cross-check the exact function, parameter, requirement, interface, call
shape, and acyclic graph. Production and canonical-reference frames share the task resource scope
but independently validate that the transferred handle is live and has the exact scope, kind,
requirement, and interface. A failure, cancellation, or exhaustion after the consume does not
restore caller ownership. Unwinding drops remaining local authority and does not implicitly
complete, fail, or cancel a durable queue lease.

## Maintained cutover and consequences

The stable `lkjournal` worker entry continues to claim and dispatch absent/live. Its live arm now
makes one direct call transferring the lease to private helper
`decl_7f443401f4946c55fa239c5430e8ad93`. The helper owns processing, `lease-info`, renewal, renewed
dispatch, and terminal complete/fail. Entry and helper are respectively 15 and 36 body records,
both below the 40-record acceptance bound and the 48-record predecessor.

This contracts application policy authority without adding a closure, async task, resource result,
new public operation, alternate authoring format, host intrinsic, or queue adapter path. Graph 6,
unbound resource parameters, and predecessor compiler/artifact forms reject; the create-new Graph 7
migration tool was deleted after maintained consumers moved. Queue rows, attempt policy, backup
format, grants, descriptors, targets, and immutable release objects did not change.

Reconsider only from a maintained workload requiring a different lifetime or result protocol. Any
extension must name its evaluation, ownership, failure, cancellation, resource, recursion, package,
artifact, runtime, migration, deletion, and independent-oracle behavior. This decision does not
justify general linear types, closures, containers, detached tasks, or cross-package resource APIs.
