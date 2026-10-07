# Owner-local worker dispatch and receipt completion

## Selection

Keep [idle-worker custody transfer](idle-worker-custody.md), but separate the
physical worker's current join owner from its receipt-joined availability. A
stable local dispatch must not take a process-wide lock merely because another
executor may later request that worker. This is a shared-runtime implementation
advance, not a new language form, channel, execution grant or fairness contract.
Actual acceptance and publication remain [status-owned](../status.md).

The predecessor uses the shared idle catalogue on both dispatch and receipt
completion. A controlled regression first warms a local worker, then deliberately
holds that catalogue while another caller uses the same owner. The actual
predecessor times out despite an available local worker. The test releases its
obstruction and joins the caller before reporting failure. Its original executable,
source fixture and failing log remain in
`.artifacts/20261007-local-worker-dispatch/`; this is not an invented baseline or
a general throughput benchmark.

## Custody directory, local availability

Each physical worker belonging to an open executor has one weak directory entry
naming its current owner and slot. The entry remains while the worker is active,
its result is unreceived, or its failed receipt is retained for shutdown. An entry
establishes custody only; it never grants availability or permission to execute.
It contains no program, capture, invocation control, grant or quota.

Availability lives with the mailbox, join handle and physical reservation in the
owner's locked worker state. Creation and claim make it unavailable. A successfully
received result, or successfully disposed unreturned result, may make it available
again. Failed result disposal and disconnected submission do not. Existing receipt
guards retain cleanup diagnostics and return active accounting even on unwind.

A local hit acquires only that owner's state, checks open dispatch and actual
availability, reserves the slot, and submits its intact job. A joined receipt also
updates only that owner. The shared directory is neither removed nor republished
on each local invocation. Before a local miss acquires the directory, it drops its
owner lock. Under the directory and reacquired destination state it rechecks both
closure and local availability before consulting foreign owners or starting a
physical worker. There is no state-to-directory lock inversion.

Cross-owner operations retain the ordering directory, destination state, source
state. Handoff rechecks the source's open dispatch, availability and thread state,
then moves the mailbox, join handle and unreleased physical reservation together.
It replaces the existing directory entry with the recipient and new slot while
those locks remain held. It neither leaves a stale source entry nor adds a duplicate.
No application code, result destructor or thread join runs under these locks.

Closing dispatch takes the directory before its owner state, marks that owner
closed and removes its still-owned entries. A preceding local claim remains an
accepted job to drain; a later claim executes inline. A preceding handoff belongs
to the recipient and is not cancelled or joined by the former owner. Receipt
completion after closure cannot reopen dispatch or restore a directory entry.
Stopping admission, joined service cleanup and thread reclamation remain distinct.

## Bounded state and tradeoffs

The directory has at most one entry per reserved physical worker, not one entry
per executor ever created or dispatch ever performed. Weak entries create no
permanent program or executor root. Shutdown removes the owner's entries before
joining its still-owned workers; physical reservations remain held until joins.
The existing per-owner vacant-slot reuse and physical ceiling remain unchanged.

A stable local dispatched pair removes the predecessor's two shared-catalogue
acquisitions: one at dispatch and one at receipt completion. This is a structural
lock-path claim. Owner mutexes, channels, allocation and operating-system scheduling
remain; this is not lock-free execution, a latency bound or a measured speedup.

A local miss now examines a bounded custody directory that can include active or
failed workers, rather than only advertised idle entries. It may acquire multiple
source locks and can still contend with handoff, creation or shutdown. The local
scan is bounded by its owner's retained slot inventory, not promised constant time.
The directory is not a task queue, and this selection introduces no work stealing,
CPU fairness, preemption, active-task migration or total root-plus-child CPU bound.
These costs require matched workload measurements before further queueing changes.

## Verification obligations

The controlled warm-local regression must complete while the directory stays
locked, returning the same physical thread and exact result. Two independently
warmed owners must each finish repeated jobs under the same obstruction without
exchanging workers. Local submission refusal must retain one unexecuted payload
and execute it once without taking the directory. Parent unwinding must dispose
its child and return accounting locally. A failed child-result destructor must
still retain its error and prevent donation despite an existing directory entry.

Repeated foreign attempts against an unreceived result must leave its custody
entry intact and refuse transfer; actual receipt completion must subsequently
permit one transfer. Concurrent local reuse, handoff and owner closure must keep
the physical ceiling, exact per-owner results, observation balances and eventual
weak-root reclamation. Existing late mailbox failure, shutdown races, exact-program
isolation, borrowed/owned parallel public cases and full fresh acceptance remain
required. Repetitions do not increase the count of unique tests.

CLI observation 43, shared-runtime observation 3, semantic graph, package, bytecode,
artifact and application-data encodings are unchanged. No data migration or change
to published assets follows from this implementation. The standard library's
[scoped mutex guard](https://doc.rust-lang.org/std/sync/struct.MutexGuard.html)
and [bounded channel](https://doc.rust-lang.org/std/sync/mpsc/) remain host mechanisms,
not independent evidence that this custody protocol is correct.
