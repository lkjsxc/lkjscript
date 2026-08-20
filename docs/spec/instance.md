# Durable application instances

This specification owns instance identity, typed mutation and query, revision publication, journal
and checkpoint authority, idempotency, grants, host attempts/outcomes, inspection, deletion, and the
durable format. Application profile meaning belongs to
[`application.md`](application.md).

## Identity and policy

Instance contract and format version 3 bind one nonzero 128-bit `InstanceId`, one exact embedded
application-format-9 artifact, one exact application state type, immutable grant bindings, and one
serial revision chain. Deleted instance identities are tombstoned and never reused.

`InstancePolicy` bounds current-state public bytes, event public bytes, total journal bytes,
transitions, and replay work. Global maxima are 16 MiB state, 1 MiB event, 256 MiB journal, 10,000
transitions, and 10,000 replay records. Event keys are nonempty canonical text of at most 96 bytes.
All counts and lengths are checked before corresponding allocation or work.

One store-wide exclusive lock serializes local instance operations. A path locates the store; it is
not instance or grant authority.

## Creation

Creation accepts exact application bytes, initial typed state, immutable grants, and policy. It
validates the application, state type/value/resource bounds, grant/interface bindings, and genesis
record before publication. Validate-only publishes nothing. Commit publishes one private instance
directory containing application bytes, genesis checkpoint record, current manifest, HEAD, and empty
attempt/outcome directories. Directory visibility followed by failed synchronization is reported as
unknown; identity is never silently retried or reused.

Revision zero is the genesis checkpoint. A successful later publishing transition increments the
revision by exactly one.

## Mutation decisions

An event names exact instance, base revision, typed event, mode, and—for commit—an event key. Stale or
future base rejects before application evaluation unless the exact key resolves to a previously
published identical input.

The application decision maps to instance publication exactly:

| decision | revision | state | command | receipt |
|---|---:|---|---|---|
| `declined` | none | none | none | typed response, `published=false` |
| `unchanged` | none | none | none | typed response, `published=false` |
| `completed` | one | exact next state | none | typed response and new digest |
| `suspended` | one | exact next state | one pending command | typed response, state, command |

The entire decision, response type/value, state type/value, command route, policy, next journal bytes,
and receipt encoding are validated before publication. A suspended state and command publish before
host work. At most one command is pending.

No durable receipt is retained for declined or unchanged evaluation. Repeating the exact request is
safe only while its exact base remains current and deterministically reevaluates the application. If
authority advances, the old base rejects as stale; clients must inspect and decide. Published
transitions retain the exact event key/input/response. An exact key/input repeat returns the original
receipt without reexecution, including after restart; reuse with different input rejects.

Validate-only and commit share evaluation. Validate-only consumes no event key and publishes no
record, manifest, HEAD, attempt, outcome, or identity.

## Pure query

`InstanceQueryRequest` names contract 3, exact instance, optional exact revision, and typed query.
The selected application query entry receives `(State, Query)` and must return the exact
application-owned `QueryResult` type.

A query publishes no journal record, state revision, event key, command, attempt, outcome, current
manifest, checkpoint, or HEAD. Its receipt binds application, instance, selected revision, record
digest, state digest, exact typed result, result digest, and `published=false`. The result digest also
binds the exact query input. It is equality evidence, not authority. A product may compare a known
digest only after exact recomputation.

Current-revision queries use the HEAD-bound current manifest described below. An explicitly selected
historical revision validates the retained chain and reconstructs that exact state. Malformed input,
wrong type/domain, excessive input/output, runtime trap, resource exhaustion, and corruption remain
distinct. Output failure cannot roll back anything because no publication occurred.

## Journal, checkpoints, and current manifest

The authoritative history is a contiguous hash-linked immutable record chain selected by HEAD. Each
transition record contains exact instance/application/revision/prior identities, canonical event or
host input, typed response, state digest, public-state byte accounting, immutable grants/policy, and
optional pending command. It contains a full state checkpoint exactly at revision zero and every 64th
revision. Other records contain no full state snapshot.

State digest uses the exact application identity and canonical bounded binary `ApplicationValue`
encoding. Public-state byte accounting uses canonical public JSON length; the two units are not
interchangeable.

HEAD selects exact instance, revision, record digest, current-manifest digest, cumulative authoritative
journal bytes, and tombstone state. The current manifest contains exact current state plus a bounded
contiguous event-key-to-record index. Its envelope digest is bound by HEAD. The index makes published
idempotency replay one immutable-record lookup and makes cumulative policy checks independent of a
full journal scan.

The current manifest is replaceable acceleration, not independent semantic history. Missing,
mismatched, truncated, or corrupt manifest input falls back to the HEAD-selected complete record chain
and latest checkpoint. A pure query does not repair or republish it. Interruption after a new manifest
but before new HEAD leaves old HEAD authoritative and the manifest ignored.

Ordinary current-state open validates the private directory, HEAD, exact application, HEAD-bound
manifest envelope and state digest, current record, its immediate prior object, current checkpoint
when applicable, event-key index, grant/policy shape, and cumulative count/byte bounds. It does not
claim complete-history audit. Missing or unusable acceleration causes complete chain validation and
checkpoint reconstruction.

`inspect --deep` reads every HEAD-selected record from genesis, checks canonical envelopes, links,
identities, event-key uniqueness, cumulative bytes, grants, policy, checkpoint cadence, and every
checkpoint, then reexecutes every transition through the application and compares final state. It is
the independent audit oracle. A disagreement rejects; no record or checkpoint is guessed or repaired.
Orphan records/manifests after interrupted pre-HEAD publication are nonauthority.

## Host grants and immutable blobs

A `HostGrant` binds contract 3, canonical grant name, exact instance and import slot, exact built-in
interface, adapter kind, and bounded descriptor. Instance creation stores only a digest-bound
`GrantBinding` in semantic records. Applications declare requirements and never contain grants.

The retained descriptor is `immutable_blob { namespace, maximum_objects, maximum_bytes }`. Namespace
paths are absolute, canonical, bounded, non-symlink local deployment locators. Production and
deterministic-fake adapters are distinct exact kinds. The only adapter input is `none`; blob content is
the application command request, not broad filesystem authority.

A visibility-capable put records an exact attempt before host action. Outcomes are immutable and bind
instance, application, command, interface, grant, adapter, operation, class, typed application outcome,
and evidence. Known success/already-present, known previsibility failure, possible visibility, and
reconciliation remain disjoint. Restart with an attempt but no outcome materializes unknown evidence;
the put is never repeated automatically. Resume consumes one exact compatible outcome and publishes
at most one next revision.

Blob names are domain-separated content digests and objects publish no-replace. Digest equality claims
content equality only—not provenance, authorization, freshness, or signature. Missing, foreign,
symlinked, nonregular, oversized, or digest-mismatched objects reject.

## Inspection, history, and deletion

Ordinary inspection returns exact current state, response, command/outcome/attempt status, grants,
policy, HEAD revision/digests, cumulative history records/bytes, checkpoint fact, validation scope, and
legal next actions. Bounded history pages expose retained revision/digest/key/status/command facts and
do not execute application queries. History inspection publishes nothing.

Deletion requires exact current base, no pending command, and publishes only a tombstone HEAD.
Retained files remain inspectable; identity cannot be recreated.

## Format version 3

The sole successful durable encodings use:

- records `LKJINS\0\x03`;
- outcomes `LKJOUT\0\x03`;
- attempts `LKJATT\0\x03`;
- canonical HEAD `LKJIHEAD`; and
- canonical current manifest `LKJICUR\0`.

Every envelope carries format 3, checked little-endian payload length, strict payload, and a
domain-separated digest. Records/HEAD/outcomes/attempts use closed canonical JSON payloads; current
state uses the closed binary application-value/index payload. Decoders reject wrong magic/version,
unknown or duplicate fields/tags, invalid UTF-8, noncanonical IDs/order, malformed length, foreign
domains, excessive values, truncation, digest mismatch, and trailing bytes.

Format 2 and older successful records and their byte-response/query-less assumptions reject directly.
There is no compatibility reader, migration, edition, fallback parser, no-op full-state publication,
activation adapter, database, compaction root, mutable state index, or persistent opaque query cache.
