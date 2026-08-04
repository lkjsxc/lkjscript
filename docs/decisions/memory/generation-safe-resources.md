# Generation-Safe Typed Resources

## Status
<!-- LKJ-F generation-safe-resources accepted-contract y86E-iM-jpNexjLFzJuIoxysn5nwYtD0O-KdkfcEhvY -->


**Accepted end-to-end contract with Current core lifecycle and static/dead
owned-resource glue in the compiler, evaluator, and reference VM.** All eleven
kinds retain exact glue identities; borrowed `standard-input` is not guest-owned.
The evaluator dispatches a closed fake-provider slice for borrowed standard
input and terminal detection, file and directory acquisition, generic typed
close, SQLite connection and child-statement acquisition, and SQLite close or
finalize. Forced native tiers still implement only borrowed `standard-input`.
Statically decidable conditional whole-place close/drop, verified
instruction-originated cleanup, and bounded cleanup attachments are Current in
the compiler, evaluator, and reference VM. Promotion remains blocked by
complete evaluator operation dispatch and owned native resource execution.

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
runtime derives the same identities for fake providers and creates a fresh
abstract scope per execution without performing ambient host I/O. Evaluation
configuration can fail acquisition or close for one exact resource kind;
reservation cancellation, invalidation-before-provider-close, and parent-child
checks remain table-enforced on both successful and failed results.

## Ownership

Owned resources are affine. Shared and exclusive temporary borrows follow the
memory plan. Standard input is a borrowed table entry owned by its standard-I/O
provider, and guest close rejects it. Explicit close consumes ownership before
host payload destruction regardless of provider success. Implicit drop invokes
the same verified per-kind glue.

## Teardown

Static successful lexical paths now execute exact implicit glue before teardown
and therefore leave zero obligations for those owners. Instruction-originated
outcomes can still reach teardown with obligations. VM and evaluator teardown
record that unmet invariant distinctly and run reverse emergency cleanup before
removing borrowed standard streams. It remains a safety net, not ordinary
semantics.

## Failure

Primary and cleanup outcomes follow the deterministic drop contract. Every
remaining close is attempted once. A failed close leaves the slot closed and
stale; it is never retried or re-exposed. Cleanup reports retain exact resource
kind as their structured subject and distinguish ordinary close, emergency
resource teardown, terminal restoration, standard-output flush, and unique
storage release. The configured attachment ceiling never limits cleanup
attempts; excess records and message bytes are counted as omitted.

Conditional branch cleanup executes explicit close on the consuming edge,
implicit close on the live edge, and `place-end` on both; unknown or future
ownership rejects. Current safe Linux descriptor and SQLite owners report close only through
infallible `Drop`, so production VM teardown has no provider-close failure to
attach yet. Deterministic evaluator-provider and VM borrowed-stream failure
injection covers ordered attachment, truncation, later-cleanup continuation,
and retention of the primary outcome.

## Native Contract

Forced native execution preflights the complete function group before effects.
The Current closed native subset is `standard-input` with exact `stdio`
capability and `input-stream` result. It reserves one borrowed table entry,
reuses it within the invocation, removes it at teardown, and has no collector
state, roots, safepoints, heap dispatch, or fallback. Owned open/read/write and
explicit close remain unsupported and reject preflight before effects.

## Verification

Core, VM, and evaluator lifecycle tests cover all kinds, wrong-kind access
before effects, provider and scope mismatch, borrowed streams, explicit and
implicit invalidating close, reuse, stale generations, generation exhaustion,
failed acquisition, SQLite parent/child protection, reverse cleanup, and
emergency teardown. Compiler tests cover one exact implicit owned-resource event,
physical close opcode selection, and explicit-close suppression; an app smoke
executes that path in the reference VM. Evaluator source tests cover borrowed
standard input, successful and failed file acquisition, successful and failed
generic close, SQLite connection/statement acquisition, finalize, close, and
failed child acquisition without host I/O. A conditional resource-parameter
fixture covers explicit versus implicit close in the VM. Core and engine teardown
tests cover bounded UTF-8 messages, omitted counts and bytes, deterministic
reverse order, later cleanup, and unchanged primary outcomes. Evaluator and VM
fuel interruption tests prove ordinary instruction plans close a live resource
without emergency teardown. Complete evaluator host-operation coverage and owned
native resource support remain absent.
