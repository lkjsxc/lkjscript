# Deterministic Drop Elaboration

## Status

<!-- LKJ-STATUS id=deterministic-drop status=accepted-contract -->

**Accepted contract with verified static/dead whole-place cleanup for exact
byte owners and owned typed resources. Full promotion still requires
conditional owners, every instruction-originated outcome, and the bounded
cleanup-failure policy.** Aggregate partial moves remain out of scope.

The implemented spine gives each direct affine SSA place an exact closed drop
identity, emits explicit loan-end and whole-place-drop events, rejects an
available owner at `place-end`, and verifies discharge before every explicit
SSA terminator. It elaborates exact byte-vector and owned typed-resource cleanup on normal
lexical exit and source-level return, break, continue, trap, and exit. Explicit
typed-resource close suppresses implicit close. The evaluator consumes exact
resource glue through its fake core table, and bytecode lowering selects generic,
SQLite-connection, or SQLite-statement close before place end in the reference
VM. Borrowed standard input is rejected as a guest-owned obligation.

Forced native byte-vector execution performs exact explicit drop and invocation
teardown release, including instruction-originated trap and resource-limit
cleanup, and proves zero final owners/loans. Native owned-resource glue remains
fail-closed at whole-group preflight. Conditional flags and typed-resource
cleanup after instruction-originated trap, deadline, resource-limit, host
failure, or propagated callee outcome are not Current. Cleanup-failure
attachment also remains absent; Current safe sys owners expose close through
infallible destruction.

## Obligations

Successful initialization of an affine whole place creates one obligation. A
move transfers it. Return transfers it to a result owner. Explicit drop or
resource close consumes it. Borrowing never consumes it. A failed constructor
creates no obligation.

`place-end` cannot erase an available owner. At each end the owner must already
be moved, returned, dropped, or absent, or control must enter an elaborated
cleanup block.

## Dataflow

Separate maybe-initialized and maybe-uninitialized CFG analyses classify each
potential drop:

- `static`: definitely initialized;
- `dead`: definitely uninitialized;
- `conditional`: wholly initialized or wholly uninitialized;
- `open`: partly initialized.

Static drops need no flag. Dead drops disappear. Conditional drops use an
internal typed boolean only when CFG facts cannot decide statically. Open drops
are a structured compile error in this slice. Resource-bearing or affine
aggregate fields remain rejected.

## SSA Events

Memory-complete SSA makes initialization, move, borrow, end-borrow, value drop,
resource drop, conditional flag update/test, and cleanup transfer explicit.
Each drop names a verified glue identity. Cleanup blocks are ordinary verified
CFG and are shared by evaluator, VM, and native tiers.

Optimizers preserve or reconstruct these facts. Dead-code elimination may
remove an allocation and its drop only together while retaining logical charges
and effects. External close is never removable.

## Ordering

Drops execute in reverse successful initialization order. Inner lexical scopes
precede outer scopes. A moved value is not dropped. A loan ends before owner
drop. Each loan and initialized obligation is discharged exactly once.

## Structured Outcomes

Cleanup covers normal completion, return, early return, break, continue, trap,
exit, resource-limit, deadline, host failure, and propagated callee outcome.
Cleanup is not VM teardown and does not depend on collector finalization.

## Cleanup Failure Policy

1. Preserve the primary execution outcome.
2. Attempt every remaining cleanup exactly once in deterministic order.
3. Collect failures into a profile-bounded structured list.
4. A cleanup failure never suppresses later cleanup.
5. When primary execution succeeded, cleanup failure becomes structured host
   failure while retaining the successful result or exit detail.
6. When primary execution failed, it remains primary and cleanup failures are
   attached.
7. An external close consumes ownership even if the provider reports failure.
8. Close is never retried and the resource is never made live again.

Borrowed standard streams are not guest-owned and are never closed by guest
drop.

## Supported Shape

Static, dead, and conditional whole-place drops are required for byte-vector,
owned dynamic bytes, owned dynamic path, and all typed resources. `open` drop,
partial moves, user destructors, unwinding, and resource-bearing aggregates are
not Current in this slice.

## Independent Verification

The verifier proves availability at each point, unique owner placement, move
consumption, loan containment and conflicts, exactly-once loan end and drop,
flag/dataflow correspondence, complete exit cleanup, non-reentry of cleanup
blocks, glue/type match, resource kind/provider match, and preservation of
logical charges.
