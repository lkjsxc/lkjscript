# Implemented status

This file records the current checkout, not intended future work. Normative behavior is owned under
[`docs/spec/`](spec/); product use is owned by
[`applications/lkjwork/README.md`](../applications/lkjwork/README.md).

## Supported public product

`lkjwork` is an installed local CLI product backed by one exact embedded lkjscript application and
one durable instance per project. It supports:

- private project initialization and parent-directory discovery;
- task create/edit, priority, lifecycle, hold, archive, labels, dependencies, notes, and activity;
- exact DAG cycle rejection and application-derived readiness/actionable ordering;
- pure show/list/next/summary/context/export queries with bounded pages and omission facts;
- immutable file attachments through the built-in blob interface, including unknown-outcome
  reconciliation without repeating a possibly visible put;
- deterministic human output, strict one-shot JSON, and a bounded caller-owned product session;
- exact backup, new-instance restore with explicit grant rebinding, shallow/deep doctor, history,
  and deterministic export version 1.

Task policy, identity allocation, lifecycle, dependencies, readiness, filtering, ordering, context
selection, notes, labels, attachments, typed conflicts, and typed query results live in the embedded
application. The Rust client owns arguments, locator validation, bounded explicit file reads,
terminal-safe rendering, exact host routing, backup transport, and process lifecycle. It has no task
database and does not decode private instance state to answer product queries.

The checked application artifact is 163,670 bytes with digest
`9d5ebe527719aa4c68b471cc10f9113df421385997113a08fbd1a6eae4650c4d`. The public-command build
recipe and generated bindings reproduce and check that artifact. The installed binary validates the
embedded artifact before use; development workspaces and standalone release files are not runtime
dependencies.

## Active contract identities

| boundary | active identity | direct rejected predecessor |
|---|---|---|
| workspace protocol / machine schema | 11 / `lkjscript-machine-schema-v11` | 10 and older |
| semantic workbench / edit document | 2 / 1 | context packet 1 / `plan` root |
| workspace semantic artifact | 7, `LKJTSM\0\x07`, `lkjscript-tsm007` | format 6 and older |
| workspace HEAD | `LKJHEAD9` | `LKJHEAD8` and older |
| reusable release | contract/format 2, `LKJREL\0\x02` | format 1 and older |
| application world | contract/format 5, `LKJAPP\0\x05` | format 4 and older |
| durable instance | contract/format 3, `LKJINS\0\x03` | format 2 and older |
| runtime session | 2 | every other version |
| lkjwork machine / export | 1 / 1 | every other version |

There are no editions, compatibility readers, aliases, migration-only paths, or silent format
fallbacks.

## Language and execution

The language implements `unit`, `bool`, checked `i64`, immutable bytes, validated UTF-8 text,
nominal products, nominal sums, and nominal homogeneous ordered sequences. Text has exact UTF-8 byte
equality and no normalization promise. Sequences support canonical empty, length, checked access,
append, and replace. Public JSON uses strings and ordered arrays; exact release-local nominal identity
is preserved.

The minimal retained algebra adds exact integer equality and boolean not/and/or. One explicit-frame
interpreter remains the execution oracle. Managed bytes/text use the safe generation-checked store;
sequences use safe immutable `Arc` elements with exact canonical retained accounting and an
allocate-new differential. No local unsafe Rust exists.

## Applications and instances

Stateful application format 5 owns exact mutation response, query/query-result, state, event,
command, outcome, and four-variant decision types. The only built-in interface is
`immutable_blob_v1`; the activation interface and its examples were deleted for lack of a current
consumer.

Instance format 3 publishes declined/unchanged decisions without a revision and completed/suspended
decisions with exactly one revision. Published idempotency receipts replay exactly; no-publication
results are reevaluated only while their exact base remains current. Pure query receipts bind exact
application/instance/revision/record/state/input/result facts and publish nothing.

History is an immutable hash-linked event/host journal with full semantic checkpoints at genesis and
every 64 revisions. HEAD binds cumulative history bytes and a current-manifest digest. The bounded
manifest contains current state and the published event-key index, enabling ordinary queries and
mutations without replaying thousands of transitions. Missing or corrupt acceleration falls back to
the full chain without query writes. `doctor --deep` reexecutes every transition and compares every
checkpoint.

The representative 2,700-revision project retains 102,841,790 record bytes and 104,741,804 total
project bytes, below the 256 MiB journal policy. Its HEAD-bound current manifest is 1,588,695 bytes.
The stress profile is retained but was not executed; 10,000 transitions and 256 MiB journal bytes are
the documented hard ceilings, not a claimed stress result.

## Runtime and topology

The topology-neutral runtime contract implements one-shot and foreground line sessions, strict
request correlation, query routing, bounded admission, exact stage observations, and one synchronous
store lock. The generic runtime has no queue, worker, persistent cache, compiled-unit cache, profile,
bytecode, JIT, or native tier.

`lkjwork session` retains one product-local prepared application/current-state object keyed by exact
HEAD. It revalidates HEAD on hits, updates only after publication, and remains correct on miss,
corruption, eviction, or restart. This is disposable acceleration, not authority or a daemon.

## Verification surfaces

The checkout contains:

- independent product reference-model differential tests;
- exact text/sequence/compiler/interpreter/release/application/instance/runtime tests;
- complete product vocabulary, lifecycle, cycle, readiness, pagination, context, export, attachment,
  locator, backup/restore, corruption, restart, idempotency, and session tests;
- deterministic public acceptance and functional/representative workload scripts; and
- structured evidence under `docs/evidence/`.

The retained examples are workspace/release/authoring verification consumers. The old durable
controller and durable blob-publisher drivers were deleted because `lkjwork` now owns the complete
stateful/blob product vertical and focused platform tests retain their invariants.

## Explicit absences and trust boundary

The verified deployment is Linux x86-64 under one trusted local operator and OS account. The client
and immutable-blob adapter are trusted native code. Paths remain locators and deployment facts, not
semantic authority.

There is no network service, cloud sync, account/multi-user model, encryption, signature,
provenance, hostile-administrator isolation, native-code sandbox, broad filesystem grant, secret
store, child process interface, wall-clock semantics, background daemon, scheduler, worker pool,
database, persistent query cache, full-text index, import/migration contract, GUI/TUI, or
cross-platform support claim. Logical resource accounting is not exact RSS enforcement. Provider
tokens and monetary cost are unavailable and are not inferred from bytes.
