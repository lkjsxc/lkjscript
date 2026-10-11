# Preserve immutable field admission during consuming extraction

## Decision

Production `unpack-owned` consumes a checked owned product or nominal record
through a sealed extraction owner. Ordinary immutable fields retain the exact
parent allocation's admission. Owned fields retain their independent exact-type,
current-domain, live-custody and allocation-admission checks. The reference
evaluator continues to admit extracted ordinary fields recursively.

This extends the [borrowed metadata boundary](borrowed-immutable-metadata.md) to
consumption. It changes neither program meaning nor the source ownership rules.
Packing, raw invocation input, captures and cross-invocation ownership transfer
still own their existing admission. In particular, a checked slot alone cannot
certify arbitrary physical storage or a caller-supplied field value.

## Exact extraction authority

The consuming operation requires the original prepared-program origin and Memory
classification, a live owning product token in the current memory domain, and that
allocation's currently valid admission for the same prepared program. The selected
closed type must equal the token's type. Its complete structural or nominal layout
is resolved from that program; equal field shapes or nominal names confer no
authority. Complete field count and order must match the prepared layout.

A borrowed placement, active loan, inert clone, foreign domain/program/type,
revocation or interrupted adoption cannot satisfy these premises. Raw mutation
invalidates admission even when later operations restore the old apparent shape.
The extraction operation cannot repair or establish the parent certificate.

The sealed owner retains the extracted field vector and exact layout while
yielding fields in layout order. No external raw child or asserted field type can
enter this path. Only fields classified as ordinary immutable data by the prepared
program become checked ordinary values without another descendant walk. Their
retained backing is independent of the consumed parent and carries no loan,
custody, capability or execution grant.

## Completion and resource behavior

Consumption transfers the original vector; it does not clone payloads or construct
another result vector. Cancellation and custody checks still precede extraction
and field delivery. A failed field check, local installation, trap or cancellation
drops every undelivered child through the existing ownership cleanup. Previously
installed children remain owned by normal frame cleanup. Reclamation is neither
effect rollback nor retry authority.

The operation performs work proportional to the field count, independently of
ordinary descendants already proved by parent admission. This is not a claim of
whole-program zero copying, bounded RSS, faster preparation or API-cost savings.
Independent raw input admission and output encoding still scale with their values.

## Evidence obligations

Direct adversaries exercise exact program, type, nominal identity, field count,
placement, active loans, raw mutation, revocation and failed adoption. They also
exercise cancellation and failures after some fields have transferred custody.
Large immutable descendants must survive parent destruction without descendant
admission or hidden payload copies at extraction.

The copied-product metadata workload requires mode 2's raw-result admission to
stay at or below `K*(N+2)`: each pack still admits its metadata, while extraction
does not repeat that work. Complete input admission remains `N+3`. Mode 3 provides
the pack-and-drop control and mode 4 retains its borrowed-projection obligation.
The independent cost predicate must reject the predecessor's doubled mode-2 work.

The mixed indexed-storage consumer retains complete displaced nested values after
destroying both Flat and Chunk32 owners, including across chunk boundaries and
after source deletion. Malformed unused input still rejects; traps and resource
refusal leave no successful partial result and permit a subsequent fresh call.
Matched measurements must retain identical artifacts, inputs and build profiles,
preparation separately from invocation, and unfavorable samples.

No meaning, interface, compiler, bytecode or artifact generation changes follow
from this derived runtime mechanism. [Status](../status.md) owns actual validation,
accepted source, mainline delivery and finalized-byte publication.
