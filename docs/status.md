# Implemented status

This file records the current checkout. Normative behavior is owned by `docs/spec/`; product use is
owned by `applications/lkjwork/README.md`.

## Semantic development projects

A semantic project is a strict `.lkjscript` authority containing one project marker, one existing
workspace identity, immutable full graph revisions, immutable canonical revision records, and one
HEAD. Project paths are locators only. Explicit selection and bounded ancestor discovery reject
parent traversal, symlinked or nonregular components, malformed markers, nested ambiguity, foreign
workspace bindings, missing closure, and corrupt history.

The public project CLI implements initialization, orientation with known-digest reuse, status,
historical inspect, targeted context, JSON or semantic-document validation/application, bounded log,
record show, arbitrary semantic diff, forward restoration, target list/show/build/test/run, shallow
or deep doctor, no-replace backup, and a correlated foreground session. Ordinary commands discover
the project and do not require a state directory, workspace ID, or revision. Machine responses are
closed version-1 envelopes; `--pretty` is the deterministic bounded human-readable JSON projection.

Every accepted change publishes exactly one revision and one revision record. Validate-only,
malformed, foreign, excessive, stale, invalid, and no-change requests publish nothing. Commit
requests bind the exact project workspace/base, and optional idempotency replays the retained exact
receipt across restart. Restoration validates a candidate and publishes forward; it never rewinds
HEAD or resurrects a tombstoned durable identity.

The current repository stores one canonical full snapshot and one compact record per revision.
Ordinary open checks every retained path and compact record, validates the complete record chain,
then decodes the exact HEAD snapshot plus any snapshots needed by the retained idempotency receipt.
Historical selection decodes and validates the selected snapshot on demand. `doctor --deep` decodes
every snapshot, validates every adjacent identity transition, and recomputes every revision-record
diff fact. Derived indexes and caches are absent. All project operations conservatively hold the one
engine lock; concurrent snapshot readers are not yet exposed.

## First-class build targets

Build targets are durable graph nodes with exact identity, presentation name, kind, and typed
definition. Active kinds are reusable release, application, and native product distribution. Release
targets own the exact root package, metadata, exports, exact target dependencies/imports, and cases.
Application targets own exact release edges, entry/profile mappings, host requirements, policies,
and cases. Product targets select one exact application target. Target cycles, missing/foreign
references, nominal mismatches, incomplete closures, and failing cases reject accepted changes.

`target build` lowers one exact revision deterministically, preflights its response, runs all cases,
and publishes an immutable artifact at an explicit no-overwrite path. Relative output paths resolve
against the command working directory before path validation. Build/test/run never publish a
development revision. There is no arbitrary hook, mutable registry lookup, ambient filesystem input,
or derived build cache.

The former `agent` command, procedural example builders, `release build`, and `app build` were
deleted. The raw `--state … rpc|session` transport remains only as the strict engine-conformance and
embedding boundary; it is not a second project authoring model. Immutable release/application
validate, inspect, test, run, instance, and runtime commands remain distinct distribution/runtime
consumers.

## Migrated lkjwork product

`applications/lkjwork/.lkjscript` is the maintained semantic development authority. Its revision 4
target reproduced the audited pre-migration application byte-for-byte. Revisions 5–7 are the public
CLI dogfood history for the cross-cutting `why TASK` query: rename replaced owners, create the new
types/functions, then update release/application target meaning and cases. Revision 7 has 3,339 graph
nodes, 546 durable identities, three targets, seven passing semantic cases, and no blockers.

`applications/lkjwork/build.py` and generated `bindings.json` are deleted. The checked application
is 167,848 bytes, has file SHA-256
`f9b335db22fbecdacdf7047f8a8e8aa7711d030eccaf3ed42d3eb2783b3cc184`, semantic application digest
`4eb891dc2f400e070d8feaf3ff8aa14e35420010d2ded3ace1a107cec8e45092`, and root release
`67c30ef33a26b53b98c7ded1a89ac6f0f9f961eef103f399bd70c963dca115bf`.

The native binary discovers the complete interface from validated application self-description.
`why` returns task ID, phase, archive state, optional manual hold, actionability, and ordered blocker
IDs as application-owned typed data. Missing task is a typed product outcome. Human and JSON output
agree, and filesystem-tree comparison proves successful and missing-task queries publish no instance
revision, event, command, attempt, outcome, checkpoint, manifest, HEAD, or blob.

The complete product still provides task lifecycle/editing, exact DAG dependencies, labels, notes,
attachments, activity/history, list/show/next/summary/context/export/why pure queries, strict JSON,
foreground session, backup/restore, and shallow/deep doctor. The Rust client owns only boundary and
host-adapter duties; task policy remains application meaning.

## Active identities

| boundary | active identity | direct rejected predecessor |
|---|---|---|
| workspace protocol / machine schema | 12 / `lkjscript-machine-schema-v12` | 11 and older |
| semantic project / marker / change / session | 1 / `LKJPROJ1` / 1 / 1 | every other version |
| development revision record | 1 / `LKJREC01` | every other version |
| semantic workbench / context / edit document | 2 / 2 / 1 | context 1, `plan`, every other document version |
| workspace semantic artifact | 8 / `LKJTSM\0\x08` / `lkjscript-tsm008` | format 7 and older |
| workspace HEAD | `LKJHDA10` | `LKJHEAD9` and older |
| build target | 1 | every other version/kind |
| reusable release | contract/format 2 / `LKJREL\0\x02` | format 1 and older |
| application / interface view | contract/format 5 / interface 1 / `LKJAPP\0\x05` | format 4 and older |
| durable instance | contract/format 3 / `LKJINS\0\x03` | format 2 and older |
| runtime session | 2 | every other version |
| lkjwork machine / export | 1 / 1 | every other version |

There are no editions, compatibility readers, aliases, silent migration, or builder fallbacks.

## Language, execution, and product runtime

The language implements `unit`, `bool`, checked `i64`, immutable bytes, validated UTF-8 text,
nominal products, sums, and homogeneous ordered sequences. Text has byte equality and no Unicode
normalization promise. Sequences support empty, length, checked access, append, and replace. One
explicit-frame interpreter is the execution oracle; safe managed bytes/text and immutable `Arc`
sequence elements have exact canonical retained accounting and independent differentials. No local
unsafe Rust exists.

Stateful application format 5 separates mutations and pure queries. Declined/unchanged operations
publish no instance revision; completed/suspended operations publish exactly one. Instance format 3
uses a hash-linked journal, full checkpoints at genesis and every 64 revisions, and a HEAD-bound
current manifest. Missing/corrupt acceleration falls back to full replay without query writes.
`doctor --deep` reexecutes every transition. The representative 2,700-revision product retains
104,745,982 total project bytes below the 256 MiB journal policy.

The only built-in host interface is `immutable_blob_v1`. Applications declare requirements;
instances bind exact grants. Visibility-capable puts record attempts first, and possible visibility
is reconciled without automatic retry. Generic runtime one-shot and caller-owned foreground-session
topologies share one synchronous instance owner; there is no daemon, queue, worker, scheduler,
bytecode, JIT, or native tier.

## Explicit absences and trust

The verified deployment is Linux x86-64 under one trusted local operator and OS account. Native code
and the blob adapter are trusted. Paths remain deployment facts, never semantic identity.

There is no network, cloud sync, multi-user authorization, encryption, signature/provenance,
hostile-native-code sandbox, broad filesystem grant, secret store, child-process interface,
wall-clock semantics, database, persistent project index, build cache, automatic merge, branch,
remote, GUI, production TUI, or cross-platform support claim. Logical resource accounting is not
exact RSS enforcement. Provider token classes and monetary cost are unavailable and are not inferred
from bytes.
