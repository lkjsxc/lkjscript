# Demand-driven handoff of idle structured workers

Decision: independently owned structured executors may transfer a reusable idle
worker, including its join handle and physical-capacity reservation, to another
open executor in the same process. Prefer an owner's own idle worker first. Do
not wait for active capacity, introduce a permanent process-global worker owner,
or share application authority. [Status](../status.md) owns acceptance and release.

## The boundary being corrected

The predecessor reserves physical auxiliary capacity until its owning executor
joins each worker. This bounds physical threads and avoids repeated starts within
one owner, but an open idle owner can retain the entire process ceiling. A later
independent owner then executes every child inline, even though a reusable thread
exists. Closing the earlier owner is not a reasonable prerequisite for using that
otherwise idle capacity.

Transfer idle worker custody on actual demand instead of stopping and recreating
threads, polling an idle timeout or retaining a universal immortal pool. An owner
still owns its active work and its remaining worker lifetimes. This is a scheduling
mechanism, not a new language channel, region transfer, permission to move an active
invocation, or cross-process sharing.

## Eligibility and atomic custody

A successful completion receipt must be received before a worker becomes eligible.
On caller unwind, the unreturned result is disposed of before eligibility. A body
that has finished while its parent still holds the unreceived result does not
release its slot. Scoped loans and resource cleanup retain the existing structured
join boundary. Accepted active work is never stolen or replayed.

A bounded catalogue contains only weak owner references and current idle slot
indices. It contains at most one entry per physical worker, not an ever-growing
list of runtime owners. The catalogue lock precedes destination and source state
locks; single-state observations and receipt waiters never acquire it while holding
a state lock. No user work, receipt wait or thread join runs under these locks.

Under that ordering, claiming an idle foreign worker removes it from the former
owner and moves the mailbox, thread join handle and still-held physical reservation
as one value into the new owner. No program, result, capture, grant, adapter, secret,
cancellation lineage or quota handle is added to the worker. Each subsequent job
continues to supply its own admitted immutable program and sealed invocation state.
Existing job-scoped observation guards remain responsible for thread-local counters.

Physical reservations are neither released nor reacquired during a handoff. The
process ceiling is unchanged. Repeated handoffs reuse empty owner slots rather than
retaining one historical slot per transfer. Runtime pointer identity only identifies
lifetime owners; it is not a package identity or a canonical semantic key.

## Refusal, failure and shutdown

If no eligible worker and no physical capacity exist, the right child remains
intact for its first execution on the caller. Nested groups therefore never wait
for their ancestor's worker. Submission refusal preserves that same closure and
its unique payload; it does not authorize retrying an accepted job.

An already observed disconnected mailbox, missing completion receipt or finished
thread is not donated. A failure first observed during submission after an atomic
handoff stays with the recipient as current join owner; it is not returned to the
donor or recirculated. The unsubmitted payload still executes intact on the caller.
Its current owner retains the thread's cleanup responsibility until shutdown. An ordinary caught child panic can return a valid infrastructure-failure
receipt while leaving a healthy reusable worker; its invocation cancellation does
not become the next owner's cancellation. This distinction does not hide the
original failure or claim successful cleanup after a missing receipt.

Closing dispatch and removing that owner's idle entries serialize against handoff.
No worker is taken from or delivered to that owner after closure. Shutdown waits
for its own accepted receipts, then stops and joins every still-owned thread before
releasing physical reservations. It neither waits for nor cancels a previously
handed-off worker now running another owner's job. A worker handed off before the
closure belongs to the recipient; one retained at closure belongs to the closing
owner. There is always one join owner, including races with shutdown.

A host-side unreturned-result destructor can unwind. Receipt disposal therefore
has an unwind guard: active accounting is returned, the worker is not donated, and
its current owner records an infrastructure failure while still joining the thread.
The original unwind is not relabelled as successful result cleanup, and subsequent
shutdown cannot wait forever for a receipt whose owner is already gone. This does
not recover process abort or erase an earlier cleanup failure.

The catalogue owns no executor or program strongly. After the last legitimate
owner or handle is released, there is no new global retention root. Retired/failed
workers and a closing owner's workers can still hold physical reservations until
that owner's joined shutdown; this change does not invent detached cleanup.

## Observation and limits

`workers_started` continues to count actual physical starts performed by that
owner. `workers_received` counts existing workers arriving from another owner;
`workers_handed_off` counts workers leaving for another owner. Before observational
counter saturation, custody obeys:

```
workers_started + workers_received
  = joined_workers + workers_handed_off + remaining_workers
```

A successful shutdown has zero active dispatches and remaining workers. It need
not join a thread whose custody was already transferred. After every participating
owner has completed shutdown, received and handed-off counts balance, and physical
starts equal joins.
Shared-runtime observation 3 and CLI observation 43 explicitly select this shape;
prior observation readers must not infer that starts always equal per-owner joins.
Meaning, package, bytecode, artifact and application-data encodings do not change.

The catalogue coordinates short scheduling operations, not fair CPU entitlement.
Local-first reuse and finite nonwaiting admission do not prevent starvation under
continuous competing demand. Resident roots and blocking adapters retain their
existing executors, so the auxiliary ceiling is not a bound on total process CPU
work. NUMA placement, active-work migration, dynamic service admission, idle timeout
retirement and root scheduling remain separate designs.

## Required evidence

Tests distinguish actual worker thread identity from invocation counters. They
cover idle handoff with a one-worker ceiling, repeated bounded-slot reuse, local
preference, unreceived-result exclusion, nested foreign-owner fallback, unique
payload execution on refusal, cancellation/failure isolation, missing receipts,
competing recipients and simultaneous handoff/shutdown. Runtime composition must
retain independently admitted exact programs and release their metadata after use.
Public copied-executable parallel/service cases remain required regressions; one
CLI service group still has one owner and is not evidence of cross-process sharing.
No speedup, RSS reduction, fairness or API-cost improvement follows from handoff
counts alone.
