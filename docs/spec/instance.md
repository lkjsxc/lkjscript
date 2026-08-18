# Durable application-instance, grant, and host-adapter contract

This specification owns mutable application-instance authority, transition history, event
idempotency, typed command suspension, immutable grant bindings, host attempts and outcomes,
reconciliation, inspection, and tombstone deletion. Immutable application meaning is owned by
[application.md](application.md). Runtime topology and aggregate operational policy are owned by
[runtime-kernel.md](runtime-kernel.md). An instance is not a workspace, release, application,
filesystem path, process, cache, or deployment.

## Selected semantic model

Instance command and durable format version 2 bind one caller-selected 128-bit `InstanceId`, one
exact embedded application-format-4 artifact, one validated nominal state type, one immutable
instance policy, and exactly one immutable grant binding for every required application import
slot. The local store never allocates, reuses, or reinterprets an instance ID.

Creation supplies exact typed initial state and complete grant descriptors. Application and grant
validation precede any visible instance directory. Revision zero contains initial state and the
canonical grant bindings. Every committed event or resume evaluates a pure application transition
from exact current state and then publishes one complete next-state record. Rejection and
validate-only publish no revision. A successful semantic no-op is still one explicit accepted
event; duplicate delivery does not publish again.

Each instance is serial. A request names an exact base revision. Any nonduplicate stale or future
base rejects with `revision_conflict`. One process owns the store-wide exclusive lock, so competing
store opens reject with `authority_busy`. There is no cross-instance transaction.

## Pure suspension and resume

The application result is the exact typed sum specified by [application.md](application.md):

```text
(State, Event)   -> completed { state, response }
                 | suspended { state, response, command }

(State, Outcome) -> completed { state, response }
                 | suspended { state, response, command }
```

The instance owner validates next state, bounded response, exact command wrapper, import slot,
interface identity, operation, request type, and grant binding before publication. State and a
pending command are durably published before host work. Host execution never mutates semantic
state; it records one immutable typed outcome. Only a later pure resume may consume that outcome.
Semantic state publication and external visibility are deliberately not atomic.

At most one command is pending. A pending command contains its derived `CommandId`, slot, interface
and exact interface identity, operation, canonical typed request, immutable grant digest, and
adapter kind. Command identity is derived under `lkjscript.instance-command.v2` from the exact
instance/application/revision/routing/grant/value facts. It is correlation and equality evidence,
not authority. No application sees a runtime command ID.

## Grants and authority

A `HostGrant` contains contract version 2, canonical name, exact instance, exact import slot,
interface, adapter kind, and one matching closed descriptor. Its canonical JSON is hashed under
`lkjscript.host-grant.v2`. The descriptor must be resupplied exactly for host execution after
restart; a path or matching name does not substitute for its digest.

Creation requires exactly one grant for every import and no extras. Slots are sorted and unique.
The grant slot and interface must equal the application requirement, the descriptor must belong to
that interface, and the grant must bind the target instance. Application bytes contain no grants.

Retained descriptors are:

- `application_activation { source_directory, activation_slot }`; and
- `immutable_blob { namespace, maximum_objects, maximum_bytes }`.

Both require bounded absolute canonical paths with validated non-symlink parents and narrow
resource shape. Activation authorizes only one source namespace and slot. Blob authorizes only one
private immutable-object namespace plus object/count bytes. Neither is a general filesystem grant.
The retained grant is immutable for instance lifetime; grant rotation, revocation, or lookup is
absent because neither current application needs it. A new instance is the current authority-change
route.

`production` and `deterministic_fake` are disjoint adapter domains. Production execution rejects a
fake grant. Fake outcome injection rejects a production grant. A grant from another instance,
slot, interface, adapter, or descriptor rejects before host action.

## Host attempts, outcomes, and replay

A retained host outcome binds exact instance, application, pending command, interface, grant,
adapter, operation, infrastructure class, typed application outcome, bounded evidence, and its
canonical digest. The instance owner forms the application-visible outcome using the exact route in
the embedded application; adapters cannot invent semantic state, response, workflow intent, or an
unmapped outcome.

The closed infrastructure classes are:

- `succeeded` and `already_present`;
- `known_failure_before_visibility` and `outcome_unknown`;
- `reconciliation_present`, `reconciliation_absent`, and `reconciliation_indeterminate`;
- `cancelled_before_action` and `timeout_before_action`; and
- `timeout_after_possible_visibility` and `cleanup_failure`.

Only classes declared compatible with the exact operation can become a typed outcome. Expected
workflow results are ordinary application variants. Corruption, capability denial, resource
exhaustion, and inability to operate the store remain distinct errors.

Before `activate_application` or `put_blob`, the store publishes an attempt marker because
visibility may become unknown. If restart finds a matching attempt without an outcome, production
execution records `outcome_unknown` with exact evidence and does not repeat the action. Validation
and inspection operations do not publish visibility markers. Repeating an exact completed host
request returns its immutable receipt; a conflicting outcome rejects.

On every open, the store validates the record chain and reruns each event and resume transition
from embedded exact application bytes. Replay reproduces state, response, pending command, routing,
grant binding, and outcome compatibility. It never calls an adapter. Missing/corrupt bytes,
noncontiguous history, foreign authority, or a different transition result reject rather than
repair or guess.

## Activation interface and adapters

The activation interface accepts application-owned nominal requests routed to:

- `validate_application`, carrying one exact application digest plus an explicit source path at the
  adapter boundary;
- `activate_application`, carrying the digest and explicit source path; or
- `reconcile_activation`, carrying the digest and no source input.

The production adapter accepts a regular application file lexically below the granted source
directory, independently decodes and validates it, and requires exact digest equality. Activation
writes a private same-directory candidate, synchronizes it, renames it over the one granted slot,
and synchronizes the directory. Rename is the visibility point. Previsibility failure is known;
postvisibility failure is unknown and never silently retried.

Reconciliation reads only the granted slot, validates a regular non-symlink exact application, and
returns present, absent, or indeterminate evidence. It performs no semantic transition. The fake
adapter records only route-compatible exact evidence and performs no filesystem action.

## Immutable-blob interface and adapters

The blob interface owns two operations:

- `put_blob` carries bounded content bytes; and
- `inspect_blob` carries one exact 32-byte content digest.

The content identity is derived under `lkjscript.immutable-blob.content.v1`. It identifies exact
content in this object domain only; it is not provenance, authorization, or application identity.
The production adapter publishes content without replacement as `<digest>.lkjb` in the granted
private namespace. It validates the entire namespace layout and every existing object's canonical
name, regular-file type, bound, and digest. Same digest/same content returns `already_present`;
conflicting or corrupt retained bytes reject. Count and aggregate retained-byte limits are checked
before publication.

Inspection derives the canonical object path, revalidates content against its name, and returns
present or absent evidence. A put may return unknown after possible visibility; the application can
suspend on an exact later inspection. No arbitrary read path or mutable replacement operation is
exposed. The deterministic fake supplies only compatible evidence and never touches the namespace.

## Events and idempotency

A committed event or resume requires an instance-scoped event key of 1–96 ASCII alphanumeric,
underscore, hyphen, or dot bytes. Validate-only requests omit the key. Exact repeated key plus
canonical original input returns the retained receipt without reevaluation. Reusing a key with a
different base or value rejects with `idempotency_conflict`.

Host outcomes and attempts are separate durable evidence. Resumes name only exact instance/base and
event key; the store supplies the one compatible retained outcome to the pure application. A
foreign command, outcome, grant, interface, application, or instance cannot be replayed into this
history.

## Durable format version 2

The bootstrap retains one full canonical state record per revision and no compaction. A validated
HEAD selects one exact chain. Unreferenced immutable records left before a HEAD change are not
authority; a referenced gap or corrupt chain rejects.

Records use `LKJINS\0\x02`, outcomes `LKJOUT\0\x02`, attempts `LKJATT\0\x02`, and HEAD
`LKJIHEAD`. Each envelope contains format 2, checked little-endian payload length, strict canonical
JSON, and a domain-separated 32-byte BLAKE3 digest. Domains are:

- `lkjscript.instance-record.v2`;
- `lkjscript.instance-host-outcome.v2`;
- `lkjscript.instance-host-attempt.v2`; and
- `lkjscript.instance-head.v2`.

State, grant, and command domains are `lkjscript.instance-state.v2`,
`lkjscript.host-grant.v2`, and `lkjscript.instance-command.v2`. Digests provide their explicitly
named content/equality role only.

The decoder checks lengths before allocation and rejects wrong magic/version, duplicate or unknown
JSON fields, noncanonical identities/order/base64, wrong domain, foreign application/instance/
interface/grant/command, incompatible typed outcome, digest mismatch, truncation, trailing bytes,
and excessive counts/bytes. The embedded application, `records/`, `outcomes/`, `attempts/`, and HEAD
are all revalidated. Version 1 has no reader or migration path and rejects directly.

## Publication, deletion, and bounds

Creation synchronizes a private staging directory, embedded application, revision-zero record, and
HEAD before one directory rename and store sync. Later transitions publish an immutable record,
then replace and synchronize HEAD. Receipts are encoded and bounded before publication. Response
loss does not roll back authority; event-key or exact host retry returns the retained receipt.

Failure before visibility is known no-change. Failure after a link/rename may have become visible is
explicit unknown. No non-idempotent host action is retried after possible visibility. Tombstone
deletion requires the exact current base and no pending command. It retains history and permanently
forbids ID reuse.

Global maxima are 1 MiB each for state, event, and blob content; 64 MiB retained history; 10,000
transitions and replay records; 256 history items per page; 64 grants; 64 KiB host evidence; 96-byte
event keys; 64-byte grant/slot names; 4,096-byte paths; 10,000 blob objects; and 64 MiB per blob
namespace grant. Per-instance and per-grant limits may attenuate these maxima. Counts and aggregate
bytes are checked before corresponding allocation or publication. Logical accounting is not a
claim of exact RSS enforcement.

## Public commands and explicit absences

```text
instance create
instance validate-event | apply-event
instance execute-host | fake-outcome
instance validate-resume | resume
instance inspect | history | delete
```

All consume strict command-local version-2 JSON and the topology-neutral runtime kernel. The old
activation-specific command families and version-1 records are absent.

There is no mutable grant lookup, automatic retry, command batch, live handle, wall-clock semantic
value, durable runtime queue, worker, general filesystem interface, database, compaction,
cross-instance transaction, application migration, purge, or identity reuse. Process boundaries and
in-process adapters are not sandboxes.
