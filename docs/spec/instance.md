# Durable application instance and activation contracts

This specification owns mutable application-instance authority, transition history, event
idempotency, command suspension, activation grants, host outcomes, reconciliation, inspection, and
deletion. Immutable application meaning is owned by [application.md](application.md). An instance
is not a workspace, release, application, filesystem path, process, cache, or deployment slot.

## Selected semantic model

An instance binds one 128-bit `InstanceId`, one exact application-format-3 artifact and digest, one
nominal state type implied by that application's stateful profile, one immutable resource policy,
and one immutable activation-grant digest. The caller chooses a canonical 32-character
lowercase-hex instance ID; the local store will never allocate, reuse, or reinterpret it.

Creation supplies one exact typed initial state. The application validator checks the state before
any instance directory becomes visible. Revision zero contains the initial state. Every committed
event or host-result resume evaluates the pure application entry from the exact current state and
then publishes one complete next-state record. Rejection and validate-only publish no instance
revision. A successfully evaluated semantic no-op is still an explicit accepted event and therefore
publishes a revision; duplicate delivery does not.

Each instance is strictly serial. A request names an exact base revision. Any non-duplicate stale or
future base rejects with `revision_conflict`. One process holds the store-wide exclusive lock, so
competing stores reject with `authority_busy`; the retained bootstrap has no worker pool, scheduler,
or cross-instance transaction.

## Pure suspension and resume

The selected transition model is one-command suspension:

```text
(state, event) -> completed(next_state, response)
               | suspended(next_state, response, command)

(state, host_outcome, evidence) -> completed(...)
                                 | suspended(...)
```

State and the pending command are durably published before a host executor is called. Host
execution never mutates semantic state. It validates the exact pending command and grant, records
one immutable typed outcome, and returns. Only a later pure resume may consume that outcome and
publish the next state. Thus semantic state and external visibility are deliberately not claimed to
be atomic.

At most one command is pending. There is no command batch or opaque continuation. Command identity
is a derived BLAKE3 key over instance ID, next state revision, exact grant digest, application target,
and command kind under `lkjscript.instance-command.v1`. It is correlation, not authority. Command
content cannot change while retaining a key.

The closed command vocabulary is:

1. validate one exact application artifact;
2. activate that exact artifact in one granted slot; and
3. reconcile whether that exact artifact is active.

Applications encode these commands as the ordinary decision data specified by
[application.md](application.md). The runtime validates and derives the command key; the host may
not invent state, response, command kind, or target.

## Events, duplicate delivery, and replay

A committed event or resume requires an instance-scoped event key of 1–96 ASCII alphanumeric,
underscore, hyphen, or dot bytes. Validate-only requests must omit the key. The complete retained
history is the deduplication authority.

An exact repeated event key and canonical external input returns the original published receipt
without reevaluation. Reusing a key with different base or event bytes rejects with
`idempotency_conflict`. An exact repeated resume key and original base returns its retained receipt;
the immutable command outcome used by that transition remains in the instance authority. Keys from
another instance cannot match because stores and records are instance-bound.

On every open, the runtime validates the record chain and deterministically reruns every external
transition and host resume from the embedded exact application bytes. Replay must reproduce state,
state digest, response, command, ordering, and nominal identity. Replay never executes a host
action. Missing application bytes, missing linked history, corrupt records, or a differing semantic
result reject rather than repair or guess.

## State and history formats

The bootstrap retains one full canonical state record per revision and no compaction. A small
mutable HEAD selects one exact chain. Unreferenced immutable objects left by a pre-HEAD crash are not
authority and may be ignored; a referenced gap or corrupt chain rejects.

Records use magic `LKJINS\0\x01`, outcomes `LKJOUT\0\x01`, attempt markers
`LKJATT\0\x01`, and HEAD `LKJIHEAD`. Every envelope contains format version 1, checked
little-endian payload length, strict canonical JSON payload, and a domain-separated 32-byte BLAKE3
digest. The domains are respectively:

- `lkjscript.instance-record.v1`;
- `lkjscript.instance-host-outcome.v1`;
- `lkjscript.instance-host-attempt.v1`; and
- `lkjscript.instance-head.v1`.

The state digest hashes the exact application digest, canonical state byte length, and canonical
state bytes under `lkjscript.instance-state.v1`. It is integrity and equality evidence, not
continuity or authority. The record digest names immutable record content. The application digest
names immutable application content. None grants access.

The decoder checks envelope and declared lengths before allocation and rejects wrong magic/version,
unknown or duplicate JSON fields, noncanonical identities or JSON, invalid UTF-8/base64, wrong
domain bindings, foreign application/instance/command data, digest mismatch, truncation, trailing
bytes, excessive counts, and byte limits. State is revalidated against the exact nominal type before
execution. There is no older successful instance format.

The instance directory embeds the exact application bytes, `records/`, `outcomes/`, `attempts/`,
and HEAD. Every open rejects missing, permissive, non-directory, or symlink-substituted authority
directories before reading them. Retained host outcomes are revalidated against the pending
command's allowed result class and exact success evidence even when their envelope and digest are
otherwise canonical. Instance paths are local storage locators, never identity or grants. The embedded
application keeps replay possible after workspaces, standalone releases, and the original
application file are removed.

## Publication and crash semantics

Creation writes a private same-store staging directory, embedded application, revision-zero record,
and HEAD; synchronizes files and directories; renames the directory once; and synchronizes the
store. Before rename, failure is known no instance. After rename, failure is
`artifact_publication_outcome_unknown` and the exact requested instance ID must be inspected before
any retry.

Later transitions first publish an immutable record by synchronized temporary plus no-replace hard
link, then replace HEAD by synchronized temporary plus rename, then synchronize the instance
directory. A pre-link record failure is known no record. A post-link record failure is unknown for
that object, but old HEAD remains authority. A pre-rename HEAD failure preserves old authority. A
post-rename HEAD failure is an unknown state-publication outcome. Exact event-key retry safely
reconciles either state; no new identity is consumed.

Receipts are fully encoded and size-checked before publication. If stdout fails after publication,
the operation is not rolled back; exact event-key replay returns the retained receipt. A host result
is likewise immutable and replayable after response loss. Store lock release and process exit do not
alter committed authority.

Deterministic fault tests inject before and after file write, file sync, immutable link, temporary
cleanup, HEAD rename, directory sync, instance-directory rename, activation visibility, and response
replay boundaries. Outcomes are restricted to known no-change, known complete, explicit unknown, or
corruption rejection.

## Activation grant and production host

An `ActivationGrant` is exact typed policy containing contract version 1, canonical grant name,
bound instance ID, executor kind, one bounded absolute source directory, and one bounded absolute
activation slot. Its canonical JSON is hashed under `lkjscript.activation-grant.v1`; only the name
and digest are retained in semantic records. The caller must resupply the exact descriptor after
restart. A missing or changed descriptor fails—there is no path lookup fallback.

`production` grants authorize the trusted in-process activation executor. The executor accepts only
an explicitly supplied regular application file lexically below the granted source directory after
rejecting every observed symlinked parent component,
decodes and canonically validates it, requires its digest to equal the pending command target,
writes and synchronizes a private same-directory slot candidate, renames it over the one exact slot,
and synchronizes the slot directory. It cannot choose another target, source namespace, slot, or
operation. General file reads/writes, network, child processes, environment, clock, randomness, and
live resource handles remain absent.

The rename is the external visibility point. Failure before it is
`known_failure_before_visibility`. Failure after possible rename is `outcome_unknown`, never an
automatic retry. A durable attempt marker is published before action. If a process disappears after
that marker but before an outcome is recorded, the next execution records `outcome_unknown` without
repeating the activation. Inspection exposes both `host_attempted` and the retained outcome.

Reconciliation reads only the granted slot, validates regular non-symlink exact application bytes,
and records `reconciliation_present`, `reconciliation_absent`, or
`reconciliation_indeterminate`. It performs no semantic transition; resume consumes the result.

## Deterministic fake host

`deterministic_fake` is a distinct grant executor used by exact tests and recovery demonstrations.
It cannot call the production validation, activation, or reconciliation operations. Conversely,
production-bound instances cannot inject fake results.

`fake-outcome` accepts one exact pending command and only a compatible outcome class. Successful
validation/activation and reconciliation-present require evidence equal to the exact target
application digest; every other fake outcome requires empty evidence. Repeating the same outcome
returns the immutable receipt, while a different outcome for the command rejects. The fake mutates
only host-outcome evidence, never semantic state, and remains subject to ordinary resume and replay.

The closed outcome vocabulary and semantic tags are:

| tag | outcome |
|---:|---|
| 1 | `known_success` |
| 2 | `known_failure_before_visibility` |
| 3 | `outcome_unknown` |
| 4 | `reconciliation_present` |
| 5 | `reconciliation_absent` |
| 6 | `reconciliation_indeterminate` |
| 7 | `cancelled_before_action` |
| 8 | `timeout_before_action` |
| 9 | `timeout_after_possible_visibility` |
| 10 | `cleanup_failure` |

No retained executor currently uses a wall clock or asynchronous cancellation. Pending/no attempt,
capability rejection before attempt, and a retained attempt with no result are separately observable
as not attempted, rejected before action, and possible visibility. Timeouts/cancellation/cleanup are
closed fake-host result classes reserved for exact lifecycle testing; they are not claims that the
synchronous production executor implements timing or interruption.

## Bounds

Global maxima are 1 MiB state, 1 MiB event, 4 MiB record payload, 64 MiB retained history, 10,000
transitions, 10,000 records of replay work, 256 history items per query, 64 KiB host evidence and
response, 96-byte event keys, 64-byte grant names, and 4,096-byte paths. Each instance stores an
immutable policy no larger than those global maxima. Counts and aggregate bytes are checked before
corresponding allocation or publication. Policy exhaustion is infrastructure rejection, not a
business value or interpreter trap.

History pages use explicit instance, start revision, and bounded count; there is no ambient current
instance or unbounded dump. Inspection reports exact application, revision, record and state
digests, typed state and bounded response, pending command, attempt/outcome status, grant name and
digest, policy, history counts, tombstone status, and deterministic legal actions. Full state is
bounded by the state policy.

## Creation, deletion, and rebinding

`create` supports validate-only and commit from caller-supplied initial state. Invalid application,
state, policy, grant, path, or destination publishes nothing. Duplicate creation rejects. Application
binding, grant, and instance policy are immutable; changing application meaning requires a new
instance. There is no migration, mutable `latest`, or rebinding route.

Deletion requires exact current revision and no pending command. It atomically replaces HEAD with a
tombstone while retaining application bytes, complete history, receipts, and evidence. The identity
is never reusable. A pending or unknown external responsibility must be resumed or reconciled first;
deletion cannot erase it. There is no compaction or physical purge command in this version.

## Public command contract

Instance command-local JSON contract version 1 is strict and absent from the global workspace
schema catalogue:

```text
instance create
instance validate-event
instance apply-event
instance validate-application
instance execute-activation
instance reconcile-activation
instance fake-outcome
instance validate-resume
instance resume
instance inspect
instance history
instance delete
```

Every command names the store and exact instance/application/command authority required by its
operation. Validate-only and commit share semantic preparation. Host execution and resume are
separate. Output is bounded canonical JSON; free-form diagnostics are non-authoritative.

## Trust boundary and explicit absences

The retained model trusts the local OS account, Rust runtime, executable, dependencies, kernel, and
POSIX-like local filesystem behavior for regular files, hard links, rename, permissions, and sync.
Lexical paths and symlink rejection narrow accidental authority but are not a sandbox and do not
defend against a hostile administrator racing namespace changes. The production executor is
in-process trusted code, not isolated native code.

There is no network, secret store, encryption, multi-user authorization, grant revocation,
automatic retry, timer, scheduler, worker, daemon, database, journal compaction, process capability,
general filesystem capability, opaque continuation, live resource, cross-instance transaction, or
hostile-host isolation.
