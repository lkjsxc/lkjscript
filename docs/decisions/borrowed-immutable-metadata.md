# Borrowed immutable metadata retains its allocation's admission

## Decision

Production field selection from a borrowed owned product retains the product's
already admitted ordinary immutable field, instead of treating that projection as
a new raw result. The sealed constructor lives in `vm_owned_metadata.rs`; the
production structural field operation is its consumer. The reference evaluator
continues independent recursive raw admission. This changes a runtime mechanism,
not the accepted meaning, source type, effect, borrow or artifact contract.

## Exact premise

A checked slot alone is insufficient. The constructor requires its original
prepared-program origin and memory classification, an actual owned-product token,
and that live allocation's currently valid admission for the same program. It
resolves the field name from the token's exact closed product type, not from a
caller-supplied field index or result type. The selected field must be in the
prepared ordinary-type set, including its complete type arguments. A declared
shape or matching allocation address cannot establish any of these premises.

The physical read independently requires a live Read placement in the invocation's
memory domain. A root owner, inert clone, foreign domain, revoked allocation or
invalidated admission fails. The loan excludes successful raw push, pop or
consuming adoption during extraction. Revocation either removes the fields before
the locked read and rejects, or follows a clone whose immutable backing is already
independently retained. No mutation-capable reference escapes the storage lock.

Raw mutation and interrupted adoption continue to invalidate allocation admission.
Restoring the old apparent shape does not restore its proof. A read transferred
into a scoped child retains the original prepared program but uses its admitted
child memory domain; program identity and current custody are separate checks.

## Consequence and resource boundary

The returned checked ordinary value shares the immutable backing and retains no
owner, loan, borrow provenance or execution grant. Its original container can be
released independently. Inline option/result/variant clone spines still require
the existing reserve-before-growth accounting and cancellation checks. Shared
record/list/map backing requires no fresh payload allocation merely for projection.
The projection adds no global cache, persistent certificate, pointer map, per-field
sidecar or universal reference-counting requirement.

This is not an unchecked constructor for arbitrary raw values. Raw invocation
inputs, adapter results, captures, consuming ownership transfer, pack and unpack
remain independently admitted at their existing boundaries. The producer's whole
product certificate is established only after its complete type and child checks;
this consumer does not mint, repair or change that certificate. Public malformed
input must still reject even when no field read is requested.

## Evidence and limits

The adversarial unit family checks current allocation admission, exact program and
field selection, domain adoption, revocation, raw mutation restoring the same
shape, interrupted adoption, reservation refusal and cancellation. Synthetic
internal corruption is not a demonstrated externally reachable vulnerability.
The detached `owned-metadata-costs` workload retains literal public authoring,
independent integer results, complete ingress and cleanup checks. Its borrowed
mode enforces at most N+2 raw-result admission nodes irrespective of read count;
packing still performs that one complete metadata admission. Modes for repeated
pack/unpack and pack/drop retain their separate costs.

The native dependency-component oracle exercises the same field operation within
flat and chunked generic worklists. It is a larger consumer, not a production
compiler replacement. Timing comparisons must use unchanged artifacts and inputs,
identify build profiles and retain unfavorable samples. Reduced admission counts
alone establish neither wall-time speedup, copied-byte reduction, bounded RSS nor
API-cost savings. Exact source acceptance and distribution remain status-owned.
