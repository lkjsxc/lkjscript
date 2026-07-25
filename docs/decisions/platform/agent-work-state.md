# Externalized Agent Work-State Service

## Purpose

Define the Current bounded, reviewable handoff state used by autonomous work without treating generated state or
repository context as source authority.

## Status

**Current.** `lkjscript-xtask` implements this local service. Git remains publication authority. Scheduling, leases,
remote coordination, hidden reasoning, prompts, and autonomous merge resolution are not part of this service.

## Commands And Location

The exact command family is:

```text
cargo run --locked -p lkjscript-xtask -- agent checkpoint <request.json>
cargo run --locked -p lkjscript-xtask -- agent resume-context <task-id> [--profile weak|strong]
cargo run --locked -p lkjscript-xtask -- agent validate-state <task-id>
cargo run --locked -p lkjscript-xtask -- agent compact-state <task-id>
```

Live state is generated at `target/lkjscript/agent-state/<validated-id>.json` and is ignored by Git. Quarantined
input remains generated below the adjacent `quarantine/` directory under a task ID and SHA-256 content identity.
No generated task state is tracked.

## Closed V1 Snapshot

The persisted [work-state schema](../../../meta/agent-state/work-state.schema.json) has identity
`lkjscript.agent-work-state`, version `1`; the strict
[checkpoint envelope](../../../meta/agent-state/checkpoint.schema.json) carries its compare revision. Administrative
schema, version, task ID, and state revision fields accompany only these bounded facts:

- base and current repository revisions, goal, hard constraints, and selected capsule scope;
- accepted decision references and completed actions, including outcome, summary, supersession, and evidence;
- exact command results: command, exit status, summary, and evidence references;
- produced commits, open defects, risks, external blockers, and next actions;
- invalidated assumptions; and
- artifact and evidence references with exact path and SHA-256 content identity.

The schema has no prompt, chain-of-thought, secret, timestamp, host, model, owner, lease, scheduler, or free-form JSON
field. `deny_unknown_fields`, typed deserialization, duplicate-field rejection, and trailing-input rejection apply at
every request and state boundary. JSON `Value` is not an authority or intermediate schema.

## Checkpoint Semantics

A checkpoint request names an exact expected state revision and one complete next snapshot. Revision zero creates a
task; every later checkpoint increments by one. Task identity, base revision, goal, hard constraints, and capsule scope
are immutable. Ordered action and command history, decisions, commits, invalidations, and references are append-only
except for explicit compaction. Action and command sequence numbers are increasing and unique.

Before publication the service checks request bytes before typed decoding, then validates string, collection, history,
reference, output, and aggregate-work bounds immediately after that bounded decode and before repository work or
publication. It verifies task identity, current `HEAD`, base ancestry, monotonic repository history, produced commits
within the inclusive base-to-current ancestry range, capsule identities, exact normalized repository-relative paths,
ancestor containment, content hashes, and evidence references. Malformed, duplicate, unknown, trailing, stale, or
over-limit input publishes nothing.

Publication uses a same-directory create-new temporary file, `write_all`, file
synchronization where supported, atomic rename, and parent-directory
synchronization where supported. If rename succeeds but that synchronization
fails synchronously, the writer atomically restores the prior bytes (or removes
a first state) before reporting failure; a failed restoration is explicitly
reported as indeterminate. Every live read, validation, compaction, quarantine,
and write holds the same per-task local exclusion file. On the
Current Linux target a lock records its process ID; a lock whose process no
longer exists is removed before one bounded retry. The lock is synchronization only, not a lease or
task authority. No partial JSON becomes live.

Malformed or structurally corrupt live bytes within the quarantine byte ceiling are moved to a deterministic quarantine
name containing their full SHA-256. An existing different quarantine destination is never overwritten. A file above
that ceiling is rejected for explicit operator removal rather than scanned without a bound. The command fails after
quarantine; a later explicit revision-zero checkpoint is the deterministic recovery path.

## Resume, Validation, And Compaction

`resume-context` emits the exact snapshot first, then bounded repository-graph context in the graph directive order.
It reports current revision mismatch, stale references, unsupported facts, omitted scopes, and every truncation. Weak
and strong profiles change only work, byte, and output limits; they do not change authority or validation rules.

`validate-state` rechecks schema, all aggregate bounds, legal sequences, repository revision relations, commits,
capsules, paths, hashes, and evidence references. `compact-state` atomically removes only successful action detail
explicitly superseded by a retained action. It retains final facts, failed outcomes, tested and not-tested outcomes,
command results, commits, and all artifact/evidence references. A second compaction is byte-idempotent.

## Bounds And Failure Policy

V1 constants bound request and generated output bytes, quarantine bytes, each string, each collection, combined
history, combined artifact/evidence references, aggregate validation work, capsule enumeration, retained Git output,
and referenced artifact reads. Git pipes are drained while retaining at most the configured bound. Arithmetic overflow
is an error before repository work or publication. Repository traversal uses bounded Git and the bounded repository
graph; it never recursively walks untrusted state input.

## Deferred And Rejected

Distributed state, scheduling, semantic leases, uploaded state, and cross-repository coordination are Deferred.
Hidden state, prompts or reasoning, credentials, timestamps as ordering, host/model identity, unbounded output,
last-writer-wins, silent quarantine overwrite, partial publication, and inert command variants are Rejected.
