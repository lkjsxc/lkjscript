# Structured parent ownership with ordinary graph state

- Status: accepted
- Date: 2026-09-03
- Decision owner: structured session contract 1

## Decision

An `interactive` target crosses callback boundaries only through one closed ordinary graph-owned
`State` value. A resident parent scope, not graph state or a callback result, exclusively owns the
RFC 6455 connection, transport children, mailboxes, timer, cancellation lineage, stream leases,
permits, and joins.

The target relation repeats the exact state type in `Option<State>` input and the canonical
`SessionDecision<State>` result. Runtime admission reconstructs that relation and rejects live,
callable, secret, or unresolved retained state. Phase validation and complete output reservation
make one transition the only state-installation point.

## Rationale

Ordinary HTTP and worker calls end with their task resources. Allowing a connection or mailbox to
escape through a general result would make lifetime, ordering, backpressure, and shutdown depend on
unstructured graph values. Host `send` and `receive` intrinsics would instead make application
transition policy a hidden Rust state machine. A structured parent retains generic operational
authority while the graph remains the sole editable owner of application state and behavior.

## Consequences

Callbacks remain finite resident tasks, live resources cannot enter retained state, children never
detach, and failed transitions cannot partially install state or output. Session lifetime can
exceed a callback deadline without weakening the resident task ceiling. Application reconnect
behavior derives from durable application data, not restored runtime cursors.

## Reversal condition

Reconsider general resource-bearing results only after a maintained workload demonstrates that it
must transfer a live capability across ordinary graph calls, the structured interactive runner is
insufficient, and an independently verified ownership model preserves bounded cleanup and a single
semantic authority.
