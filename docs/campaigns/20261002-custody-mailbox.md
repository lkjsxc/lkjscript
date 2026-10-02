# Runtime custody mailbox and structured-session adoption

Date: 2026-10-02 (Asia/Tokyo).
Product identifier: development `0.1.67`. This is not a public release or a
running-service deployment.

## Owner request

> lkjscriptについて進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

## Reconciled entry and selected boundary

Entry main and origin were independently fetched at
`ec11b77ee94595a27bba39d2ef3a764202dc1c89`, with tree
`b579691d7b46b3c5be7299243a08a5abae0d8336`. The accepted preceding task-method source
is `9e21bbbf6046fc524a0f806be7b45ccd769c758e`; the entry descendant records its
acceptance without becoming a new proof of different source.

Work uses the existing clean
`/home/coder/workspace/lkjscript-structured-handoff-20261002` checkout in
`lkjsxc/tomato-ocelot-73`, on the new branch `dev/custody-mailbox-20261002`.
The main checkout's two unrelated untracked files,
existing stash, other worktrees and running applications are outside this change.
The current priority is language-design quality and coherent shared execution,
not another application starter, benchmark-only cleanup or syntactic expansion.

The [structured-transfer decision](../decisions/20261002-structured-owned-transfer.md)
already identified that a library capacity permit does not establish logical
acceptance: a Tokio permit may send after receiver close. Its retained four-case
probe is evidence about the locked dependency, not a reproduced language bug.
The selected next increment makes the runtime own close/commit serialization and
uses it in the maintained session writer. This is a real runtime consumer, rather
than unused future channel code. Do not describe it as a native cross-task Owned
API: exact destination/type admission and nested origin adoption remain absent.

## Implementation

`src/platform/execution/mailbox.rs` and its sender/receiver modules provide a
bounded multiple-sender, single-receiver runtime mailbox. Queue slots are
preallocated before issuing endpoints; the count includes both queued entries and
outstanding reservations. A private non-clonable permit reserves capacity but
does not accept a payload. Commit and receiver close hold the same short custody
lock. A close that wins returns only the never-accepted value; an accepted value
can never be restored from a missing completion receipt.

The final producer lifetime includes outstanding permits, so a receiver does not
mistake a reserved but uncommitted sender for terminal disconnection. A cancelled
pending send drops its own unaccepted argument; dropping a permit releases its
slot. Capacity and terminal waiters register before checking their condition.
Receiver close detaches queued values under the lock, wakes waiters, then drops
payloads outside the lock. Neither surviving senders nor permits retain those
accepted queued values. A dequeued value is still owned by the active consumer
until its work finishes or is cancelled and joined.

The session writer now uses this mailbox for its single preallocated transition
batch slot. Capacity reservation still precedes effectful graph callbacks.
Receiver closure during a callback refuses the later batch commitment immediately;
a continuing transition fails rather than installing next state or retrying the
callback. Terminal close/peer-close transport cleanup retains its existing
best-effort semantics, not a delivery guarantee. A missing completion after
acceptance cannot establish that nothing was sent. Existing child cancellation
and abort-then-await joining remain the lifecycle owner.

The [session specification](../spec/structured-sessions.md#phases-and-atomic-transition)
now separates reservation, acceptance, transport delivery and completion. It also
removes the overly broad reading that a post-acceptance transport failure proves
no output was visible. Callback effects are neither rolled back nor replayed.

This increment changes no graph/type/effect semantics, origin checks, capability
grants, bytecode, artifact, authored-request or validator encoding. It adds no
dependency, unsafe code, Python requirement, ambient shared heap, detached task
API, network/durable channel, public endpoint or production deployment.

## Independent expectations and intended acceptance

The mailbox tests observe capacity separately from payload drop counters, exact
payload identities and per-message delivery counts. Cases include zero/overflowing
storage admission; mixed reservations and queued entries; actual refused-owner
retry; active versus queued cleanup; cancelled polled and unpolled send futures;
last-permit liveness; zero-sized messages; destructor execution outside the mutex;
cancelled receive/capacity waiters; and close notifications.

A four-producer workload sends 1,024 distinguishable values per capacity (1, 2 and
7), joins every producer, and checks exact per-value cleanup. A separate 128-run
two-thread close/commit race checks accepted versus actually refused custody.
These are bounded implementation observations, not exhaustive schedule checking,
fairness, lock-freedom, CPU-parallel language performance or memory-ordering proofs.
The original independent finite custody model remains unchanged.

Storage-level tests move nested OwnedChoice/OwnedProduct values containing both a
ByteBuffer and an independently sealed OwnedI64Cell. They check allocation
identity, existing foreign-origin and loan rejection, inert raw clones and
cancellation/receiver-failure cleanup while an unrelated owner survives. Test-only
type identities are not authored packages or a semantic admission certificate.

Session tests execute the actual `send_reserved`, `send_writer` and `join_child`
paths: closed reservations refuse immediately; cancelling completion observation
does not revoke or repeat an accepted batch; cancelled capacity waits clean their
unaccepted batch; and aborting then joining a consumer releases a dequeued batch.
The full-source suite and copied-executable native/session tests remain separate
acceptance obligations, not inferred from these focused cases.

Original development logs are retained under
`.artifacts/20261002-custody-mailbox/` in the implementation worktree.
Acceptance results and exact tested source identities are recorded below only
after their commands complete.

## Release decision and next language boundary

Public/latest remains immutable v0.1.64. Defer another binary publication until a
coherent public structured-owned-transfer milestone or a separately justified
runtime-maintenance release has its final-candidate evidence. Do not relabel
source integration, a copied development executable or a session unit test as
publication.

The next selected work is a sealed, bounded destination-admission envelope:
validate exact payload closure, sender/destination identity and absence of loans
before commitment, preserve nested allocation custody, and reject foreign or
mismatched admission without granting ambient authority. Then expose the boundary
through independent native producer/consumer packages with source-deleted
execution and cancellation at each acceptance phase. The mutex/notification
implementation is revisable; task semantics must not depend on a particular
scheduler or on weakening origin isolation to make transport appear complete.
