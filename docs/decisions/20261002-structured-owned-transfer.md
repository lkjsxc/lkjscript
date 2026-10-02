# Structured task-owned transfer

Date: 2026-10-02 (Asia/Tokyo).

## Status and authority

The first same-task call boundary is implemented and source-accepted in development
0.1.65. The subsequent structured channel remains an experiment with a finite
executable custody model, **not an implemented queue runtime or cross-task handoff**.
The [task-owned contract](../spec/owned-task-transfers.md) and
[accepted source](../campaigns/20261002-task-owned-transfer.md#accepted-source-and-mainline-delivery)
own the implemented behavior. This decision does not add another syntax authority,
select a scheduler or change running services. Public v0.1.64 and its original assets
are unchanged by this development. The meaning graph remains the sole program authority;
the [roadmap](../roadmap.md) records the revisable sequence.

The original decision inspected source `0048ae1ee2e4678b409c782e02044b038bf60052`.
That predecessor's task bodies can create and dispose of owned choices while independently
consuming exact capability resources, as witnessed by
[native task-resource cases](../../tests/public_cli/native_generic_resources.rs).
Its memory signature validator required pure owned parameters/results, a final
memory suffix and no mixed memory/resource signatures. Development 0.1.65 closes
that composition gap through the existing ownership system, not a parallel one.

## First language boundary: task-local composition

Development 0.1.65 implements direct named task helpers before a channel or detached
task API. The selected parameter order is ordinary unrestricted parameters, then
zero or more owned consume parameters, then the existing exact capability-resource
borrow/consume suffix. Require explicit use modes and exact resource requirements.
This preserves ordinary argument evaluation before transfer and keeps existing
resource suffixes recognizable. A task may return one direct owned value,
including an OwnedProduct or OwnedChoice; unrestricted containers remain unable
to hide owners.

Pure helpers retain synchronous whole-owner borrow/consume contracts. For the
first task-memory signature slice, owned parameters are consume-only: no
memory loan enters a potentially suspending helper. A task may still call a pure
borrowed helper; that loan ends before the task can suspend or transfer the owner.
Do not equate this memory restriction with existing task-local capability-resource
borrowing. Their authority and completion obligations are different.

Evaluate arguments in authored order. Failure after an argument moves does not
restore its caller-local owner. Already moved inputs are cleaned by the failed
call's custodian, and unrelated performed effects may survive. A rejected
application operation returns ownership explicitly through an OwnedChoice;
runtime cancellation or a trap is not silently converted to that case.
Generic substitution, exact implementation witnesses, package interfaces,
independent semantic checks and strict compiled admission must agree on this
boundary. Keep dynamic callable capture and raw entry/exit ownership escapes
rejected.

This first slice is same-task composition. It must pass public authoring,
cross-package forwarding, cancellation/failure cleanup and source-free execution
before cross-task handoff is advertised. Do not claim suspension safety merely
because a synchronous helper can move an input.

## Second boundary: one structured group within one instance

Begin with a bounded, typed in-process channel inside a joined task group belonging
to one admitted application instance. No cross-instance authority transfer,
durable queue, network transport, detached lifetime or supervisor is selected.
A receiver endpoint and a live task-group owner, not a type annotation or a name,
authorize delivery. Exact closed payload type and destination identity are checked
before live publication. Capability resources, secrets and raw host handles are not
generic Owned message payloads.

Keep custody distinct from processing. One payload has exactly one live custodian:
the sender operation, an accepted queue entry, or an active receiver. Its terminal
state has one completed cleanup responsibility, not a resurrectable token.
Dropping a future is not evidence that these responsibilities have been joined.

### Reservation and acceptance

Queue capacity includes both committed queued entries and outstanding reservations.
Reserve queue-slot and destination bookkeeping capacity before committing ownership.
A reservation is not acceptance. If the receiver closes before commit, the
reservation is released and an ordinary refusal can return the original owner.
A cancelled sender instead cleans that still-unaccepted owner; it cannot deliver a
normal refusal value to a task that no longer exists.

Acceptance has one linearization point: the reserved entry becomes queue-owned
and deliverable while sender ownership is irrevocably relinquished. Validation,
fallible allocation and cancellation-sensitive preparation precede this point.
The committing state transition cannot leave an owner between two custodians.
Locks, atomics, ticket representation and payload layout are implementation choices,
not additional semantic authority.

After acceptance, failure to deliver an acceptance receipt does not undo acceptance.
The receiver may already have dequeued or processed the payload. Cancellation of the
sender after commit leaves custody with the queue/receiver or its joined cleanup.
Do not return the original payload or report a retryable refusal on that path.
A sender can retry only a genuinely returned, never-accepted owner. Acceptance is
neither successful processing nor exactly-once application behavior.

### Receiver termination

Dequeue transfers custody to the receiver and releases the queue slot. The initial
shutdown policy closes admission and disposes of still-queued messages; an already
active receiver must complete or be cancelled and joined. Receiver failure releases
its payload exactly once. A queued item dropped during shutdown was accepted but
never processed; a sent item processed before the sender observes acceptance is
also valid. Neither outcome allows a second sender owner.

An active receiver's storage is not part of the queue-slot count. Per-instance
memory accounting, task limits and message-size accounting remain independent
requirements; a slot bound is not a bound on total heap or RSS.
Blocking operations require explicit wake-up and join paths. The finite model below
does not supply their implementation or prove scheduler fairness.

## Runtime origin and implementation boundary

Both current evaluators mint a fresh memory origin per invocation, and sealed
buffer/cell/product/choice tokens reject a foreign origin. Reusing a prepared program
does not authorize another invocation to consume those tokens. **Do not implement
handoff by disabling origin checks or giving every instance one ambient origin.**

Investigate a sealed transfer envelope owned by the runtime's handoff boundary.
It must validate the sender, destination, exact payload closure and absence of
active loans, then support a nonduplicating destination admission. Nested owned
products and choices must retain their original payload allocations where feasible;
ordinary immutable metadata may keep its existing sharing. Blindly changing the
outer origin leaves nested tokens inconsistent. Any recursive adoption must be
bounded, cancellation-aware before commitment, and have an unambiguous cleanup
owner on every failure. A larger common region domain is an alternative only with
independent task-custody admission, never as a substitute for it.

Do not select a permanent Arc-based heap, global collector or re-encode payloads
through JSON to solve this boundary. Physical origin/adoption representation remains
open until an actual native producer/consumer workload tests its costs and failures.
The first implementation must retain a separate reference expectation and allocation
identity observations, not certify itself solely with its own transition code.

## Executable design model and limits

[`tests/owned_transfer_model.rs`](../../tests/owned_transfer_model.rs) is a
dependency-free Rust test of the proposed abstract custody protocol, automatically
included by the maintained workspace test command. It does not import or emulate
the production executor. Two senders each own one distinguishable payload, one
receiver processes at most one at a time, and capacities 1 and 2 are explored.
Refused owners can be explicitly retried, so the reachable graph contains cycles.

The model enumerates reserve, commit, refusal, explicit retry, receipt observation,
sender cancellation, receiver close/dequeue, receiver success/failure and disposal.
It separately stores sender possession, queue membership, receiver possession and
disposal counts; the one-custodian invariant is not an enum that cannot represent
a duplicate. It also checks reservation-inclusive capacity, monotone acceptance
and at-most-one dequeue for each model payload. Reverse reachability checks that
every reachable state has a path to a closed, fully joined cleanup state.

The initial complete exploration visits 598 states at capacity 1 and 625 at capacity
2, with respectively 60 and 64 joined states. The deliberately incorrect alternative
`reserve -> commit -> cancel-and-return-original` creates simultaneous queue and
sender possession; the independent invariant rejects it. This is a design
counterexample, **not a reproduced bug in the current runtime**, which provides no
such channel.

The model assumes each listed transition is atomic and cleanup steps can execute.
It proves these properties only for that finite transition system. It does not
prove that every infinite execution terminates, fairness, wake-up correctness,
memory ordering, allocator cleanup, arbitrary queue sizes, lock freedom, CPU
parallelism, failure atomicity of a concrete implementation or a zero-copy claim.
There is no deduplication/transaction guarantee for independently repeated messages.

## Implementation acceptance before broader concurrency

Mixed task-local signatures are now accepted and integrated. Next refine the model
into the actual handoff boundary with two independent native producer/consumer packages
and copied-executable, source-deleted execution. Force cancellation before reservation,
between reservation and acceptance, after acceptance before observation, after dequeue,
and during receiver shutdown. Check borrowed/foreign-origin/wrong-type rejection,
capacity release, exact cleanup and preservation of an unrelated instance.

Use deterministic schedule controls to cover the model's critical interleavings,
then real multithreaded stress and a CPU-parallel workload. Distinguish abstract
model, implementation tests and public language evidence in every report.
Only after that evidence should scheduler work-stealing, multiple receivers,
cross-instance channels or dynamic instance supervision become the next experiment.

## Bootstrap channel reservations are not acceptance

An isolated probe against the repository's locked **Tokio 1.53.1**, built with pinned
Rust 1.98.0 and offline dependencies, identifies a concrete refinement obligation.
Tokio documents that an outstanding [Permit](https://docs.rs/tokio/1.53.1/tokio/sync/mpsc/struct.Permit.html)
can send after `Receiver::close`, and that [receiver termination](https://docs.rs/tokio/1.53.1/tokio/sync/mpsc/struct.Receiver.html#method.close)
waits for outstanding reservations to be sent or released. Thus a direct mapping
`reserve -> close -> permit.send` does not implement this decision's selected
close-before-commit refusal. This is a contract mismatch, not a claimed Tokio bug.

The four-case, clear-environment probe observes: a reserved send remains receivable
after close; an unused owned permit delays the disconnected observation; a late
permit send after dropping the receiver retains its payload until the remaining
sender is dropped; and dropping an unpolled ordinary send future disposes of its
captured payload without delivering it. The third case initially expected immediate
disposal and failed (observed zero drops, expected one). Both that original failure
and the corrected four-case passing observation are retained, rather than claiming
that receiver destruction alone proves cleanup. These observations cover only the
listed deterministic library operations, not arbitrary interleavings or fairness.

Original manifest, lockfile, source, failed assumption and both logs are retained at
`.artifacts/20261002-task-owned/reservation-probe/` in the implementation worktree;
the independently built original remains at `/tmp/lkjscript-reservation-probe-20261002/`.
The manifest selects exactly `tokio = 1.53.1`, with default features disabled and
only `sync`. Reproduction uses `cargo +1.98.0 build --locked --offline`, then
`env -i ./target/debug/lkjscript-reservation-probe`. This is an exploratory library
probe, not a maintained language test or evidence of an implemented channel.

The next implementation must own logical admission and close/commit serialization
independently of a library capacity permit. Reserve destination bookkeeping before
acceptance, then publish custody at one nonfallible commit point. A genuine refusal
may return the never-accepted owner; cancellation after commitment cannot do so.
Joined shutdown must also reclaim send operations and reservations, not merely drop
the receiver. Keep the original payload in a sealed custodian during fallible
preparation, and separately validate any nested destination-origin adoption before
claiming cross-invocation transfer. This does not authorize a global ambient origin,
ordinary data encoding of owners, an extra mutable program authority, or an
unobserved zero-copy/parallelism claim.
