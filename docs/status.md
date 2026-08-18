# Implemented status

This file describes implemented reality at the current checkout. Normative behavior belongs under
`docs/spec/`; measurements and reversal evidence belong in `docs/performance.md` and
`docs/evidence/`.

## Product boundary

`lkjscript` is a local typed semantic programming system for exact pure programs and durable
stateful application instances. Coding agents can author semantic workspaces, publish immutable
reusable releases, build transferable applications, create isolated instances, validate/apply typed
events, execute one narrow granted activation action, resume from typed results, inspect bounded
state/history, reconcile unknown visibility, and tombstone instances.

The durable release-controller example completes that lifecycle through public commands after its
source workspace and standalone release are deleted. It uses the production activation executor for
one instance and the exact deterministic fake executor for a second failure/recovery lifecycle.

## Active contracts

| boundary | active identity | legacy behavior |
|---|---|---|
| workspace logical/JSON protocol | 10 | 9 and older reject |
| machine contract | `lkjscript-machine-schema-v10` | v9 names reject |
| workbench | 2 | older roots reject |
| context packet | 2 | 1 rejects |
| editable document | 1, root `document` | `plan` rejects |
| reusable-release command/artifact | 1, `LKJREL\0\x01` | all other formats reject |
| application command/artifact | 3, `LKJAPP\0\x03` | format/contract 2 and older reject |
| instance command/artifacts | 1, `LKJINS\0\x01` and related v1 envelopes | no legacy form |
| workspace semantic artifact | 6, `LKJTSM\0\x06` | older successful forms reject |
| semantic schema | `lkjscript-tsm006` | older schema names reject |
| workspace HEAD | `LKJHEAD8` | HEAD7 rejects |

Release, application, and instance DTOs are command-local owners and deliberately absent from the
global workspace machine catalogue.

## Semantic language and execution

The closed value set is `unit`, `bool`, checked `i64`, immutable `bytes`, nominal immutable
products, and nominal immutable sums. Operations cover constants, integer addition/comparison,
direct calls, conditions, counted loops, products, exhaustive sum matching, byte operations, typed
holes, returns, and yields. No new language primitive was needed for durable state: applications use
nominal states/events and a four-field nominal decision product.

Complete selected closure lowers through verified Core IR and runs on one explicit-frame
interpreter. User-controlled graph and execution depth do not recurse on the native stack. Managed
immutable bytes remain the production memory route with an allocate-new differential oracle.
Expected workflow errors are ordinary semantic variants; traps, corruption, resource failure,
authority denial, and unknown publication remain distinct.

## Immutable distribution

Reusable release format 1 is workspace-independent canonical semantic authority with exact exports,
dependencies, tests, nominal identities, and explicit absence of provenance/signatures. Application
format 3 embeds one complete exact release graph, entry/profile/policy, and exact cases. It supports
typed, bounded bytes-stream, and stateful profiles. Every load independently decodes, validates,
re-encodes, compiles, and tests as applicable. No resolver, mutable store, or source workspace is
consulted.

The stateful profile declares pure event and resume functions. It can request validation,
activation, or reconciliation as ordinary typed data; it cannot perform host work directly.

## Durable instances

Instance format 1 binds one caller-selected 128-bit continuity identity to exact embedded
application bytes, typed full-state snapshots, monotonically contiguous revisions, an immutable
policy, and one immutable grant digest. Revision zero is caller-supplied validated initial state.
Every committed event or resume publishes exactly one next full-state record and then HEAD.
Validate-only predicts the same transition and publishes nothing.

Committed operations require bounded instance-scoped event keys. Exact duplicate delivery replays
the retained receipt; different input under the same key rejects. Stale bases reject. Restart
revalidates and reexecutes the complete selected record chain without host actions. History is
retained without compaction and queried by bounded revision ranges. Semantic no-op events publish a
revision. Tombstone deletion retains all authority/evidence and permanently forbids identity reuse.

State publication precedes host execution. One pending command may suspend an instance. Host
outcomes are immutable typed records; only resume can publish resulting semantic state. Attempt
markers make a missing acknowledgement conservative: after any possible activation attempt, the
runtime records unknown rather than repeating the action. Reconciliation produces present, absent,
or indeterminate evidence and remains separate from semantic transition.

## Host authority

The only production host capability is one activation grant bound to instance ID, exact executor,
one source directory, and one slot. It validates an exact application artifact and can atomically
replace only that slot. Paths locate resources but do not grant authority by themselves. Symlink and
non-regular forms reject; the in-process executor and local OS/filesystem remain trusted and are not
a sandbox.

The deterministic fake executor is a distinct grant class. It accepts only command-compatible exact
outcomes and digest evidence and cannot call production host operations. Production grants cannot
inject fake outcomes. This closes application-owned success, known-failure, unknown,
reconciliation, timeout/cancellation class, retry, and terminal tests without external effects.

## Public topology and authoring

One `lkjscript` binary provides direct RPC, line-delimited workspace session, agent workbench,
release, application, and instance command families. The prior `lkjscriptd` binary, socket client,
framing transport, exports, tests, and documentation are deleted because no product consumer
remained. A process boundary is not presented as isolation.

Normal authoring uses task-scoped context packets and one editable semantic document. Application
and instance inspection expose exact bounded interfaces; ordinary instance operation does not
require the global machine schema or source inspection.

## Retained implementation choices

- Full workspace snapshots/scans remain the simplest verified workspace authority.
- Full instance state records plus HEAD remain the simplest verified instance authority; the
  current controller history does not justify a journal/database or compaction.
- Exact release graphs remain embedded in applications because this closes offline transfer and
  current bundle sizes are small.
- Managed immutable bytes remain because the existing differential workload still demonstrates a
  copy/peak-byte benefit and the stateful workload exposes no contrary dominant cost.
- The large global contract owner remains for workspace RPC consumers, while release, application,
  and instance contracts use strict command-local serde owners. No generator or proc-macro was added.
- Safe stable Rust 2024 remains the only implementation language; the crate has no local unsafe
  block and no build script.

## Explicit absences

There is no backward compatibility, application rebinding/migration, instance compaction or purge,
grant revocation, automatic retry, timer/scheduler, network, child process, ambient environment,
general filesystem API, live external-resource value, secret store, state encryption, multi-user
authorization, hostile-host sandbox, worker, daemon, database, registry, resolver, package manager,
bytecode, serialized Core IR, JIT, AOT/native application backend, tracing collector, signature
system, public plugin ABI, or cross-platform support claim.

The supported verified bootstrap remains stable Rust on Linux x86-64. Filesystem durability claims
are limited to the documented trusted local POSIX-like model; formal verification, power-loss proof,
and hostile-administrator isolation are not claimed.
