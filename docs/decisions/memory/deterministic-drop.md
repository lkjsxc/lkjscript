# Deterministic Drop Elaboration

## Status
<!-- LKJ-F deterministic-drop accepted-contract 3x9ef1XNMfeqD8J9zudNzZhRsKS_iaOJU3iwBS-pD_M -->


**Accepted contract with verified static, dead, conditional, and
instruction-originated whole-place cleanup for exact byte owners and owned typed
resources. Bounded structured cleanup attachments are Current.** Evaluator and
VM cleanup cover all structured instruction outcomes; forced native cleanup is
Current for collector-free byte owners. Native owned resources and aggregate
partial moves remain out of scope for the Current slice.

The implemented spine gives each direct affine SSA place an exact closed drop
identity, emits explicit loan-end and whole-place-drop events, rejects an
available owner at `place-end`, and verifies discharge before every explicit
SSA terminator. It elaborates exact byte-vector and owned typed-resource cleanup on normal
lexical exit and source-level return, break, continue, trap, and exit. Explicit
typed-resource close suppresses implicit close. The evaluator consumes exact
resource glue through its fake core table, and bytecode lowering emits the
dedicated typed `ResourceDrop` operation; the VM selects generic, SQLite
connection, or SQLite statement destruction from the exact live resource kind
without constructing an ignored language `result`. Borrowed standard input is rejected as a guest-owned obligation.

Forced native byte-vector execution performs exact explicit drop and verified
instruction cleanup for trap, poll-fuel/resource-limit, and propagated callee
outcomes, and proves zero final owners/loans without emergency release. Native
owned-resource glue remains fail-closed at whole-group preflight.
Branch-specific conditional cleanup is Current when each reachable edge proves
a whole-place move, explicit close, or live owner; it inserts no flag when edge
facts decide statically. Evaluator and VM typed-resource cleanup is Current for
instruction-originated trap, deadline, resource-limit, host failure, and
propagated callee outcome. Core, evaluator, VM, and forced native byte cleanup
retain bounded ordered cleanup records without replacing the primary outcome;
deterministic injected provider and borrowed-stream failures cover truncation
and attachment. Current safe sys owners still expose close through infallible
destruction.

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

Static drops need no flag. Dead drops disappear. Conditional drops execute on
the exact live predecessor and both predecessors end the place when CFG facts
decide statically. An internal typed boolean is reserved only for a future
shape whose CFG facts cannot decide at an edge; no such shape is Current. Open
drops are a structured compile error. Resource-bearing or affine aggregate
fields remain rejected.

## SSA Events

Memory-complete SSA makes initialization, move, borrow, end-borrow, value drop,
resource drop, conditional flag update/test, and cleanup transfer explicit.
Each drop names a verified glue identity. Lexical cleanup blocks are ordinary
verified CFG. Each fallible instruction with live cleanup work names one
bounded, function-local, interned failure-cleanup plan; absence is the canonical
empty plan. A plan ends live loans before it
drops caller-retained owners in reverse initialization order; it excludes an
unpublished result and any argument whose obligation transferred to a callee.
The independent ownership verifier reconstructs the exact plan at the failure
site and rejects missing, extra, duplicate, reordered, stale, or mismatched
actions. Evaluator, bytecode/VM, baseline JIT, and proof JIT consume that one
verified semantic plan; emergency teardown is not its representation.

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

The attachment records a closed cleanup phase and subject, a bounded UTF-8
message, retained message bytes, omitted message bytes, and omitted failure
count. Default execution limits retain at most 32 failures and 8192 message
bytes; evaluator and VM configurations may only choose bounded explicit
limits. A record whose message exceeds the remaining byte budget is retained
with an exact UTF-8 prefix and omitted-byte count. Attempts after the retained
record ceiling are counted but retain no message. Arithmetic saturates rather
than wrapping. Emergency runtime teardown is a distinct phase and never
masquerades as ordinary semantic drop.

An outcome with attachments is represented as one structured cleanup failure
containing the unchanged primary outcome and the bounded ordered attachment;
it is not flattened into a host-error string. Borrowed standard streams are
not guest-owned and are never closed by guest drop.

## Supported Shape

Static, dead, and conditional whole-place drops are required for byte-vector,
owned dynamic bytes, dynamic strings and paths, deterministic structural
aggregates, and all typed resources. Unrestricted partial moves, user
destructors, and unwinding are not Current. Resource-bearing aggregate
boundaries use the exact generation-safe adapter rather than structural or
tracing-heap storage.

## Independent Verification

The verifier proves availability at each point, unique owner placement, move
consumption, loan containment and conflicts, exactly-once loan end and drop,
flag/dataflow correspondence, complete exit cleanup, exact bounded
instruction-failure plans, non-reentry of cleanup blocks, glue/type match,
resource kind/provider match, and preservation of logical charges.
