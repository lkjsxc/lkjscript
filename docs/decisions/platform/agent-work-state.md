# Agent Work-State Contract

## Purpose

Define atomic, reviewable task state for one or more agents without confusing
intent, attempted work, generated context, or historical evidence with Current
repository behavior.

## Status

**Accepted Implementation Contract.** No persisted work-state store or command
surface described here is Current. The contract follows bounded topology and
repository-graph identity. Ordinary Git commits remain publication authority.

## Identity And Scope

The schema identity is `lkjscript.agent-work-state`, version `1`. Each task has
a stable length-framed task key, revision-scoped update ID, base repository
revision, optional parent task, owning agent identity, lease epoch, exact scope,
policy/profile identities, and creation/update sequence. Wall-clock time is
metadata, never ordering authority.

Scope is a closed set of repository graph node identities plus allowed operation
classes. A path glob alone cannot grant ownership. Overlap is diagnosed before
activation; an explicit parent coordinator may partition or serialize it.

## States

The closed lifecycle is:

```text
planned -> active -> blocked -> active -> completed
                    \-> abandoned
planned ------------> abandoned
active  ------------> abandoned
```

`completed` means the declared deliverables and required evidence were
atomically published at the named repository revision. It does not mean all
product gates passed. Every required but unrun gate remains in `not_tested`.
`blocked` names an actionable blocker and owner. `abandoned` retains reason and
all attempts; it does not erase history. Unknown or illegal transitions fail.

## Work Record

A V1 record contains bounded, deterministically ordered:

- objective, deliverables, non-goals, and accepted authority links;
- base revision, scope nodes, read set, precondition fingerprints, and lease;
- planned and completed semantic operations;
- touched paths/entities and old/new identities;
- diagnostics, failed mutations, repair attempts, and blockers;
- command evidence with commit, environment, command, exit/result, and artifact
  identity;
- explicit `tested` and `not_tested` gates;
- generated context profile/revision/charges and inclusion reasons; and
- publication revision, review decision, and superseded task identities.

Free-form notes may supplement but cannot replace closed status, scope,
operation, evidence, or outcome fields. Secrets, credentials, raw private
prompts, and unbounded command output are rejected.

## Atomic Update Semantics

Each update names the exact task revision, lease epoch, base repository
revision, and precondition fingerprint. The service validates schema/version,
limits, transition, scope, graph freshness, and all preconditions before
staging. It applies all record and semantic-edit operations in declared order,
revalidates policy and diagnostics, and publishes one new task revision only if
every operation succeeds. Failure publishes neither partial files nor partial
work state.

Repository publication is compare-and-swap against the exact base revision.
File changes use the Semantic Source transaction for semantic entities and
same-directory atomic replacement for supported authored files. A commit hash
is attached only after Git reports the exact staged tree. Lease expiry prevents
new publication but cannot roll back an already committed tree.

Concurrent disjoint tasks may publish only after rebasing and rechecking their
complete read/precondition sets. Overlapping scope requires coordinator order;
last-writer-wins is rejected. Rename preserves an explicit old/new relationship
and transfers task scope atomically.

## Commands

The accepted conceptual surface is:

```text
lkjscript agent-state snapshot --profile handoff
lkjscript agent-state begin request.json
lkjscript agent-state update request.json
lkjscript agent-state publish request.json
lkjscript agent-state abandon request.json
```

Strict request/response envelopes use stderr or a dedicated protocol stream;
normal program stdout is never mixed with state data. Generated snapshots and
audits are written under `target/agent-state/`, not tracked as authored files.

## Aggregate Budgets

Pre-allocation categories include request/response bytes, tasks, scope nodes,
read/write/precondition entries, operations, diagnostics, command records,
artifact references, context nodes/edges/bytes, history depth, transition work,
and publication bytes. Checked arithmetic and deterministic profile ceilings
apply before staging. Exhaustion reports category, limit, charge, profile, and
responsible task/operation and publishes nothing.

## Policy Coverage

Before publication, policy evaluation covers topology/provenance, manifest and
link correctness, Semantic Source structural and requested analysis checks,
repository graph freshness, scope ownership, generated-output location,
required focused tests, documentation status honesty, and immutable-evidence
integrity. Code/build/runtime gates not required or not run remain explicit.
Policy results identify rule/version and cannot be replaced by agent assertion.

## Acceptance Gates

V1 becomes Current only after illegal-transition, stale-base, stale-lease,
overlapping-scope, atomic rollback, crash-recovery, rename, expression-edit,
budget, malformed-version, deterministic snapshot, and disjoint-concurrency
fixtures pass. A retained multi-agent task must show exact scope, complete
attempt history, atomic Git publication, and truthful tested/not-tested fields.

## Deferred And Rejected

Remote coordination, distributed leases, autonomous merge resolution, and task
scheduling are **Deferred**. Hidden state, mutable unversioned logs, partial
publication, erased failed attempts, path-glob authority, last-writer-wins,
claiming unrun gates, and treating `completed` as a product capability claim are
**Rejected**.
