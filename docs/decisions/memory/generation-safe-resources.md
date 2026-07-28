# Generation-Safe Typed Resources

## Status

<!-- LKJ-STATUS id=generation-safe-resources status=accepted-contract -->

**Accepted end-to-end contract with Current core, VM, and evaluator lifecycle
foundations.** They cover all eleven kinds, reusable nonwrapping slots,
provider/scope binding, stale-key rejection, reservations, invalidating close,
reverse emergency cleanup, and exact obligations. The evaluator uses fake
owners without resource-operation dispatch. Forced native tiers implement only
borrowed `standard-input` over an invocation-owned core table. Promotion remains
blocked by compiler cleanup on every outcome and owned native resource execution.

## Key And Slot

An opaque resource key binds resource kind, slot index, generation, provider
identity, execution scope, and ownership class. Source cannot construct or
inspect it. A slot is `vacant`, `owned-open`, `borrowed-open`, `closing`,
`closed`, or `retired`.

The reference VM projects a checked key into one `u32` guest token with 12 slot
bits and 20 nonzero generation bits. It decodes only through the core table
with an exact expected kind, provider, scope, and ownership. A closed owned
slot may be reused only after generation advances. Generation never wraps to a
previously valid key. Exhaustion retires the slot before token wrap; aggregate
index or generation exhaustion is a structured resource failure.

## Type And Provider Safety

Source type fixes one of the eleven registered resource kinds. HIR, SSA, and
bytecode reject wrong-kind operations before host effects. Each acquisition
records the exact capability provider. Filesystem, network, process, SQLite,
and terminal resources cannot cross provider domains.

Dispatch is a closed kind table, never a source string lookup. The VM derives
stable provider identities from exact capability origins. The evaluator
lifecycle harness derives the same identities for fake providers and creates a
fresh abstract scope per execution without performing ambient host I/O.

## Ownership

Owned resources are affine. Shared and exclusive temporary borrows follow the
memory plan. Standard input is a borrowed table entry owned by its standard-I/O
provider, and guest close rejects it. Explicit close consumes ownership before
host payload destruction regardless of provider success. Implicit drop invokes
the same verified per-kind glue.

## Teardown

Ordinary successful execution will reach teardown with zero guest-owned
resource obligations once compiler all-outcome cleanup is implemented. Current
VM and evaluator teardown record that unmet ordinary invariant distinctly,
observe exact emergency obligations, and run the core table's reverse owned
cleanup before removing borrowed standard streams. It is a safety net, not
normal semantics.

## Failure

Primary and cleanup outcomes follow the deterministic drop contract. Every
remaining close is attempted once. A failed close leaves the slot closed and
stale; it is never retried or re-exposed. Current safe Linux descriptor and
SQLite owners report close only through infallible `Drop`, so VM teardown can
count attempts but cannot attach provider close failures to `ExecutionOutcome`;
structured bounded failure attachment remains Accepted Contract work.

## Native Contract

Forced native execution preflights the complete function group before effects.
The Current closed native subset is `standard-input` with exact `stdio`
capability and `input-stream` result. It reserves one borrowed table entry,
reuses it within the invocation, removes it at teardown, and has no collector
state, roots, safepoints, heap dispatch, or fallback. Owned open/read/write and
explicit close remain unsupported and reject preflight before effects.

## Verification

Core, VM, and evaluator lifecycle tests cover all kinds, wrong-kind access
before effects, provider and scope mismatch, borrowed streams, explicit close,
reuse, stale generations, generation exhaustion, failed acquisition, SQLite
parent/child protection, reverse cleanup, and emergency teardown. Native tests
add exact borrowed reservation/reuse/removal and zero obligations in both forced
tiers. Evaluator resource operations, compiler all-outcome cleanup, and owned
native resource support remain absent.
