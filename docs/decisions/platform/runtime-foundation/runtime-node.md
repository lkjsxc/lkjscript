# Runtime Node

## Status

**Accepted Contract.** The runtime node and cells are not Current. This cycle
may implement only bounded lifecycle and immutable-image-sharing experiments.

## Current Inherited State

Linux remains the system-wide scheduler. Current evaluator, VM, and JIT paths
are process-local execution engines, not supervised cells. Existing semantic
resource-plane measurements remain authoritative; this decision does not add a
scheduler, daemon, cluster, or distributed consistency claim.

## Accepted Node Contract

A node is a host-owned coordinator for bounded cells. A cell has one immutable
runtime image, explicit capability grants, a resource profile, mutable local
state, and a closed lifecycle:

```text
prepared -> starting -> running -> stopping -> stopped
                         |              |
                         +--> failed <--+
```

Transitions are journaled before externally visible publication. Start and stop
are idempotent under exact cell identity. Failure is data, never an implicit
restart. Supervision policy is bounded, explicit, and separate from child
semantics. Parent shutdown cancels children in deterministic order and reports
all bounded cleanup failures while preserving the primary outcome.

This adopts lifecycle and supervision principles from official Erlang/OTP
references, not Erlang APIs, mailbox semantics, or restart defaults:

- [OTP applications](https://www.erlang.org/doc/system/applications.html)
- [supervision principles](https://www.erlang.org/doc/system/sup_princ.html)

## Immutable Sharing

Code, verified metadata, literals, and frozen runtime tables may be shared only
when immutable and content-addressed. Mutable heaps, capabilities, ledgers,
transactions, cancellation state, and metrics remain cell-owned.

The design principle is adopted from:

- Google V&#56;
  [embedded builtins](https://v&#56;.dev/blog/embedded-builtins), which move
  shareable builtins into an embedded read-only representation; and
- OpenJDK [Class Data Sharing](https://openjdk.org/groups/hotspot/docs/RuntimeOverview.html),
  which shares archived class metadata while retaining runtime isolation.

Their object models, collectors, archive formats, and process assumptions are
rejected as lkjscript contracts.

## This Cycle's Experiment

A successful probe demonstrates exact lifecycle validation, bounded supervision,
deterministic cancellation order, immutable image identity checks, zero mutable
cross-cell aliases, and complete resource-ledger discharge. It does not claim
multi-process distribution, automatic recovery, rolling upgrade, hot code
loading, or portability acceptance.
