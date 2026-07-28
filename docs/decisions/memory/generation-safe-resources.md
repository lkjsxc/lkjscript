# Generation-Safe Typed Resources

## Status

<!-- LKJ-STATUS id=generation-safe-resources status=accepted-contract -->

**Accepted contract; monotonic Current tokens do not satisfy it.** Promotion
requires all eleven kinds, reusable slots, stale-key rejection, and exact drop.

## Key And Slot

An opaque resource key binds resource kind, slot index, generation, provider
identity, and ownership class. Source cannot construct or inspect it. A slot is
`vacant`, `owned-open`, `borrowed-open`, `closing`, `closed`, or `retired`.

A closed owned slot may be reused only after generation advances. Generation
never wraps to a previously valid key. Exhaustion retires the slot; aggregate
index or epoch exhaustion is a structured resource failure.

## Type And Provider Safety

Source type fixes one of the eleven registered resource kinds. HIR, SSA, and
bytecode reject wrong-kind operations before host effects. Each acquisition
records the exact capability provider. Filesystem, network, process, SQLite,
and terminal resources cannot cross provider domains.

Dispatch is a closed kind table, never a source string lookup.

## Ownership

Owned resources are affine. Shared and exclusive temporary borrows follow the
memory plan. Borrowed standard streams remain provider-owned. Explicit close
consumes ownership regardless of provider success. Implicit drop invokes the
same verified per-kind glue.

## Teardown

Ordinary successful execution reaches teardown with zero guest-owned resource
obligations. Teardown verifies that invariant. Emergency cleanup after an
internal host failure is a distinct reported safety net, not normal semantics.

## Failure

Primary and cleanup outcomes follow the deterministic drop contract. Every
remaining close is attempted once. A failed close leaves the slot closed and
stale; it is never retried or re-exposed.

## Native Contract

Forced native execution preflights the complete function group before effects.
Supported resource runtime calls are contract-digested and closed. Unsupported
operations reject preflight rather than falling back to VM or tracing.

## Verification

Tests cover all kinds, wrong-kind access, provider mismatch, borrowed versus
owned streams, explicit and implicit close, reuse, stale generations,
generation exhaustion, session isolation, close failure, cleanup order, and
emergency teardown distinction.
