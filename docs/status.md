# Implemented status

This file describes implemented reality at the current checkout. Normative behavior belongs under
`docs/spec/`; measurements and reversal evidence belong in `docs/performance.md` and
`docs/evidence/`.

## Product boundary

`lkjscript` is a local typed semantic application platform for exact pure programs and durable
stateful instances. Coding agents can author workspace revisions, publish immutable reusable
releases, build offline application worlds with typed exports/imports, bind instances to exact
grants, apply/replay pure transitions, execute two narrow host interfaces, inspect bounded state and
runtime observations, reconcile unknown visibility, and tombstone identities.

The durable controller and immutable-blob publisher both complete their public lifecycle after
source workspaces and standalone releases are deleted. Each uses production and disjoint fake
adapters across isolated instances, restart, duplicate delivery, stale bases, denied authority,
unknown outcomes, corruption rejection, bounded history, and no identity reuse.

## Active contracts

| boundary | active identity | direct rejection |
|---|---|---|
| workspace logical/JSON protocol | 10 | 9 and older |
| machine contract | `lkjscript-machine-schema-v10` | v9 names |
| workbench | 2 | older roots |
| context packet | 2 | 1 |
| editable document | 1, root `document` | `plan` |
| reusable release | 1, `LKJREL\0\x01` | every other format |
| application world | 4, `LKJAPP\0\x04` | 3 and older |
| durable instance | 2, `LKJINS\0\x02` plus v2 outcome/attempt | 1 and older |
| runtime session | 1 | every other version |
| workspace semantic artifact | 6, `LKJTSM\0\x06` | older successful forms |
| semantic schema | `lkjscript-tsm006` | older schema names |
| workspace HEAD | `LKJHEAD8` | HEAD7 |

Release, application, instance, and runtime DTOs remain command-local owners and are absent from the
global workspace machine catalogue.

## Semantic language and execution

The closed values are `unit`, `bool`, checked `i64`, immutable `bytes`, nominal immutable products,
and nominal immutable sums. Operations cover constants, integer addition/comparison, direct calls,
conditions, counted loops, products, exhaustive sum matching, byte operations, typed holes,
returns, and yields.

Complete selected closure lowers through independently verified Core IR and runs on one
explicit-frame interpreter. Managed immutable bytes retain an allocate-new differential oracle.
There is no bytecode, serialized Core cache, native compiler, or JIT.

## Application worlds

Application format 4 embeds a complete exact release graph, entry/profile/policy, and exact pure
tests. Stateful profiles name exact nominal state, event, command, outcome, and completed/suspended
decision structure. They declare canonical import slots and exact variant routing for
`application_activation_v1` and `immutable_blob_v1`. Interface IDs are derived immutable contract
identities; application bytes contain no grants.

Magic integer commands, fixed digest targets, raw resume tags/evidence arguments, and the
activation-specific stateful ABI are removed. Pure typed and bounded byte-stream invocation remain
host-free.

## Durable instances and adapters

Instance format 2 embeds exact application bytes, full typed state snapshots, canonical grant
bindings, monotonically contiguous revisions, event receipts, typed pending commands, attempt
markers, typed outcomes, and one validated HEAD. Restart validates and reexecutes complete history
without adapters. Validate-only publishes nothing; exact duplicate keys replay, conflicts/stale bases
reject, and tombstones permanently forbid reuse.

Every application import is bound to one immutable instance-specific `HostGrant`. Activation grants
name one source namespace and slot. Blob grants name one private immutable-object namespace with
count/byte limits. Production and deterministic-fake adapters are disjoint. A foreign instance,
slot, interface, descriptor, adapter, command, or outcome rejects before external work.

Activation validates/installs/reconciles one exact application. Immutable blob performs bounded
create-if-absent and exact inspection by content digest. Visibility-capable work records an attempt
first. A crash after attempt but before outcome becomes explicit unknown and is not repeated. Only a
later pure resume can turn retained typed evidence into semantic state.

## Runtime kernel and topology

One topology-neutral runtime kernel now owns operational admission, store opening, adapter
coordination, and stage/resource observations while delegating meaning and durability to application
and instance owners. All one-shot application/instance operations use it.

The retained foreground `runtime session` calls that same kernel over bounded line-delimited
version-1 JSON. It holds one store lock for caller-owned lifetime and remains synchronized after a
malformed line. Every request names exact authority. The workspace authoring line session remains a
separate `Engine` topology.

There is one installed binary and no resident supervisor, socket, auto-spawn, daemon, worker,
scheduler, async runtime, background queue, or multi-client service.

## Resource governance and observations

Semantic owners independently bound application/graph/tests, fuel, frames, cells, values, managed
bytes/objects, state, event, response, evidence, history, replay, and blob objects. Instance and blob
bytes remain durable and grant/policy-accounted.

Runtime policy admits at most 8 MiB request, 32 MiB response, 256 MiB application, one open store,
one active transition, one active host operation, and one compilation. Queue, cache, compiled-unit,
and profile budgets are exactly zero and nonzero requests reject. Inspection exposes current
reservations, counters, all stage observations, supported adapters/topologies, and omissions. RSS and
open files are not claimed as exact logical enforcement.

Telemetry separates startup, request/application decode, canonical re-encode, graph validation,
release tests, flattening, lowering, Core verification, execution, instance open/chain/replay,
transition/state publication, grant/adapter/host/outcome work, queue/cache stages, and response
encoding. It is optional operational evidence, not semantic authority.

## Agent surface

`runtime orientation` gives compact contract versions, exact interface IDs, topology/adapter names,
limits, roots, and expansion commands. Application inspection exposes exact world/import routing;
instance inspection exposes exact revision, command, interface, grant binding, outcome state, legal
actions, bounds, and history counts. Agents author ordinary nominal sums/products rather than magic
tags or a second language.

## Retained choices and explicit absences

Full workspace snapshots, full instance records, embedded exact release graphs, store-wide locking,
synchronous transition-boundary operation, and no cache remain the smallest verified paths for the
current workloads. Repeated instance replay—not interpreter dispatch—is the largest observed runtime
term. The foreground session earns retention through process reduction; a resident supervisor does
not have a current multi-client consumer.

There is no backward compatibility, registry, resolver, mutable dependency store, application
rebinding/migration, grant mutation/revocation, instance compaction/purge, timer, mid-transition
preemption, network, child process, ambient environment, general filesystem interface, secret
store, encryption, multi-user authorization, hostile-host sandbox, database, signature system,
public plugin ABI, or cross-platform support claim.

The supported bootstrap remains stable Rust edition 2024 on Linux x86-64 with no local unsafe Rust.
Durability claims are limited to the documented trusted local POSIX-like filesystem model; formal
verification, power-loss proof, and hostile-administrator isolation are not claimed.
