# Exact owned task methods

Date: 2026-10-02 (Asia/Tokyo). Entry source: `1fe15ce864edce4fbb004e83dec36d962307abd9`.
Product identifier: development `0.1.66`. This campaign is not a public release.

## Selected language boundary

The owner delegates current technical decisions and prioritizes language-design
quality over AI development metrics or completion of a personal application.
The existing same-task transfer admits named task producers and consumers, but
its nominal Owned contracts could select only pure implementations. Consequently,
a representation-independent library could forward a witness yet could not state
an effectful implementation method. This is a compositional language restriction,
not a request for another host-specific buffer operation or application framework.

The selected extension makes a method either pure or a task with an exact closed
row of concrete requirements. A selected monomorphic graph function must have the
same callable kind, row, parameter modes and substituted signature. Task Self
arguments consume; pure methods retain scoped synchronous borrowing. An empty task
row is still a task. No implicit implementation search, effect inference, row
subtyping, method generics or blanket implementation scheme was added.

The three boundaries remain separate: static implementation selection, the
caller's declared effect allowance, and the deployment's exact operation grant.
A selected method does not manufacture any authority. Existing source checking,
canonical preparation and execution admission apply to unused signatures as well
as invoked methods. Contract effects join relation extraction, effect/capability
summaries and public package closure. Even a contract whose requirement lives in
a private component retains that exact requirement without a public function
accidentally carrying the dependency for it.

This increment does not implement a channel, child task, task scheduler, joined
cancellation or cross-invocation ownership transfer. The next runtime boundary
remains a bounded structured handoff with explicit acceptance, owner-returning
refusal and one cleanup custodian. A more general effect or trait calculus is not
a prerequisite for that experiment.

## Authoring and compatibility

Native methods accept an optional final `(effect pure)` or `(effect (task ...))`.
Omitted effects remain pure. Draft projections retain the method row, and owner
inspection reports callable kind and exact requirement records. Pure-only create
and set requests retain their previous canonical intent bytes. Task-bearing
contract requests use distinct tags and `LKJACR24`, including each method's effect.
A pure method cannot be confused with an empty-row task in a plan identity.

Compact change discovery is 28; authored intent is 24; semantic validator is 23.
The owned-implementation feature becomes 4, relation extraction 3 and summary
classification 3. The observed validator digest is
`validator_contract_d1516493a8999830f868d87baf8265f9cc30dd911c8275dfad67a84a9895d98f`.
Predecessor proof reuse is invalidated. The existing canonical method already
contains its effect field, so graph 20, interface 12, compiler 18, bytecode 14 and
artifact 25 retain their wire layouts. Product numbering confers no compatibility
promise. No dependency versions, proof limits or execution limits were increased.

## Independent observations

The literal library, carriers and consumer are maintained in
`tests/fixtures/owned-task-method-{library,carriers,consumer}.lkjc`.
The abstract library has no cell or buffer representation. Its scalar and buffer
implementations provide task factories and consuming clock-bearing finish methods.
The consumer uses both generic witness forwarding and direct concrete methods.

Runtime unit tests deliberately use a source-defined clock interface and controlled
adapter, not a replay of the live wall clock. They invoke the consumer function
with the authority component's explicit grant: local activation is not allowed to
borrow another local component's authority. Public CLI tests independently use the
actual builtin clock, imported port effects and real deployment binding.

Both evaluators are checked against independent expected records, not just against
each other: scalar results retain I64 minimum, -257, zero and maximum; buffer length
is one. Storage observers require exactly one scalar and one buffer allocation and
zero live owners or loans after completion. Exactly two physical adapter calls are
expected. A controlled cancellation immediately after the first external effect
requires cancellation, one completed call, no replay and no retained owned storage.
Pre-cancellation and allocation-quota sweeps retain their distinct failure classes.
Missing grants do not become successful execution.

Negative tests cover pure callers (including empty-row tasks), task borrowing or
unrestricted Self, wrong implementation kind, missing caller effects, same-kind
implementation row mismatch, open rows and dangling local/foreign requirement
identities in otherwise unused contracts. Independent memory and source-schema
readers separately reject forged implementation metadata. No body or compiler-only
shortcut certifies those contracts.

The public tests create three fresh projects, stage exact package transports,
verify untouched drafts, build an artifact, remove all three source projects and
both transport files, and run that same artifact again. Correct grants yield the
expected record and exactly two capability calls; missing grants fail without
writing a result. A separate contract-only export/import/build case covers the
private-component requirement closure. Executables are copied outside the checkout
and their subprocess environments are cleared by the maintained public harness.

## Development iterations retained

The first compilation exposed an authored decoder collection lookup and a test
closure lifetime mismatch; both were corrected. Initial focused tests then caught
the separate compact-edge allowlist missing the new effect field. Later fixture
failures correctly rejected a missing builtin dependency, an empty component port
inventory, and an attempted cross-component local activation. The fixtures were
corrected rather than weakening those language rules. Debug-profile logs remain
under `/tmp/lkjscript-owned-methods-*` on the authorized workspace.

Before broad acceptance, six focused unit tests passed, including both evaluators,
cleanup and canonical pure-input compatibility. Both public CLI cases passed in
45.92 seconds in the unoptimized development profile. A further independent
same-kind row-mismatch test was added for the full suite. These focused observations
are not full-source acceptance, optimized final-artifact proof or publication.
The authoritative broad result and exact tested source belong to the completion
record appended after the maintained verification owners finish.
