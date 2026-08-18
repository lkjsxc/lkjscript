# Architecture

This file owns current components, dependency direction, topology, trust boundaries, and the trusted
computing base. Observable behavior is normative only under `docs/spec/`; implemented presence and
absence are summarized in `docs/status.md`.

## System shape

```text
agent proposal -> semantic Engine -> immutable workspace revision
                                          |
                                          v
                              exact reusable release graph
                                          |
                                          v
                             immutable application world
                               | exports        | imports
                               v                v
                         pure execution    host interfaces
                               |                |
                               +------v---------+
                                  instance store
                            full records + HEAD + outcomes
                                          |
                                          v
                                runtime kernel
                   admission / routing / telemetry / store lock
                         |                         |
                    one-shot CLI          foreground line session
                                          |
                               trusted narrow adapters
```

Authority domains stay separate:

1. workspace revisions own development continuity and accepted semantic graphs;
2. releases own exact reusable semantic units;
3. applications own immutable runnable closure, exports, and interface requirements;
4. instance records plus HEAD own mutable state continuity and history;
5. grants own exact host authority; and
6. host outcomes own retained external evidence.

The runtime kernel is operational composition, not a seventh semantic or durable authority.
Application/interface/grant/adapter/instance identities are distinct. Core IR, layouts, ownership
plans, telemetry, process state, paths, and runtime handles are derived or locators.

## Components and dependency direction

### Semantic workspace and workbench

`schema`, `graph`, `validate`, `transaction`, and `persistence` own workspace meaning and revision
publication. `contract`, `machine`, `protocol`, and `workbench` provide strict typed proposals and
bounded views. `Engine` remains the only workspace publication authority.

The global executable contract serves workspace RPC. Release, application, instance, and runtime
records have command-local owners and do not flow back into that catalogue.

### Release and application worlds

`release` projects one immutable workspace revision and explicit dependencies into canonical
workspace-independent meaning. `application` validates one complete explicit release graph,
selects entries and policy, validates typed stateful worlds/import routing, runs pure tests, and
embeds the graph in one file. Neither consults mutable resolution, workspace HEAD, an instance, or a
grant.

Application-owned nominal sums express commands, outcomes, and completed/suspended decisions. The
application owner maps exact wrapper/request/outcome variants to one closed host interface and
operation. Two built-in immutable interface identities avoid a registry or new artifact domain.

### Execution engine

`compile`, `core_ir`, `type_layout`, `ownership`, `managed`, and `interpret` form the derived
execution engine. Complete closure lowers to independently verified Core IR. The explicit-frame
interpreter keeps user-controlled depth off the native stack and remains the only execution tier.
Managed immutable bytes retain an allocate-new differential oracle.

### Instance store

`instance` owns one store-wide lock, exact embedded application bytes, full canonical state
records, event receipts, immutable grant bindings, attempts, host outcomes, and one validated HEAD
per instance. It revalidates and reexecutes the complete selected chain on every open without host
work. State publication is record first, HEAD second.

Ordinary canonical files remain simpler than a database or compact journal for current histories.
Orphan immutable records from a pre-HEAD crash are not authority. Referenced gaps/corruption reject.
Compaction and grant mutation are absent.

### Host interfaces and adapters

Activation and immutable blob are separate interfaces with different request, evidence, and
authority shapes. Each has one production adapter and an executor-disjoint deterministic fake.

The activation adapter may validate/replace one exact granted application slot. The blob adapter
may create or inspect content-addressed immutable objects only in one granted private namespace and
within grant count/byte limits. Neither exposes arbitrary filesystem operations. Adapters form
typed outcomes but cannot mutate semantic state. Visibility-capable work publishes an attempt first;
unknown visibility is reconciled rather than retried.

### Runtime kernel

`runtime` owns explicit deployment policy, request admission, reuse of one store handle, adapter
coordination, and bounded stage/resource observations. It calls application and instance owners;
it does not duplicate their validators or persistence. `runtime_protocol` owns foreground-session
version 1, and `src/bin/lkjscript/runtime.rs` is the process adapter.

Current operational capacity is synchronous: one store, transition, host operation, and compilation;
zero queue, cache, compiled-unit, and profile bytes. Full application and instance validation still
occurs for each request. There is no stale HEAD, grant, or authorization reuse.

## Process topology

The product installs one `lkjscript` binary:

- workspace one-shot commands open one `Engine`;
- the workspace line session retains one `Engine` for dependent authoring requests;
- release commands are one-shot pure artifact operations;
- application/instance one-shot commands construct the runtime kernel; and
- the foreground runtime line session retains that same kernel and one exact store lock.

Each session request names exact authority and has an independent publication boundary. The caller
owns startup, EOF, and shutdown. There is no daemon, socket supervisor/client, auto-spawn, worker,
background scheduler, multiplexed client, or network listener. Foreground reuse materially reduces
complete-workflow process count; no multi-client consumer justifies socket lifecycle/authentication
surface.

An in-process adapter is a code/trust choice, not isolation. A separate worker was rejected because
the retained trusted adapters do not justify IPC, descriptor transfer, lifecycle, and new unknown
outcomes.

## Ordering and crash model

Per instance, the observable order is:

1. decode, validate exact request/authority, and reserve operational capacity;
2. load and replay exact application/instance authority;
3. evaluate and validate one pure transition;
4. preflight the bounded receipt;
5. publish/synchronize one immutable full-state record and then HEAD;
6. return the transition receipt;
7. separately validate exact command/grant/adapter;
8. publish an attempt before visibility-capable action;
9. execute the trusted adapter and publish one immutable typed outcome;
10. return the host receipt; and
11. accept a later pure resume as another state transition.

Output failure cannot roll back published authority. Exact event/host idempotency returns retained
receipts. State and host visibility are not one atomic commit. Fault tests classify retained
write/link/rename/sync boundaries as known no-change, known complete, explicit unknown, or
corruption rejection.

The filesystem model assumes trusted local POSIX-like regular files, private directories,
hard-link no-replace, same-directory rename, and file/directory sync. Sudden-power-loss hardware
behavior and hostile administrator races are outside proof.

## Resource ownership

Semantic owners enforce fuel, frames, cells, managed bytes/objects, arguments/results, application
graph/tests, state, event, response, evidence, history, replay, and blob content. Instance/grant
owners retain state/history/object bytes across process exit.

The runtime policy separately admits request/response/application bytes and active store,
transition, host-operation, and compilation slots. Stage telemetry distinguishes decode, validation,
flattening, lowering, verification, execution, replay, publication, and adapter work. Zero-valued
queue/cache/profile/compiled categories are explicit. RSS and open files are observations omitted
from exact enforcement, not silently presented as logical accounting.

## Threat and trust boundary

Model-authored JSON/document/application values, artifacts, instance files, caches if ever added,
paths, grants, and adapter evidence are hostile input. Closed serde/codecs, checked lengths/counts,
canonical re-encoding, domain-separated digests, nominal validation, exact base revisions,
instance/interface/grant binding, private directories, symlink rejection, immutable publication,
and deterministic replay reject malformed or substituted authority.

Unknown activation/blob visibility remains explicit after an attempt. Corrupt semantic/instance
authority rejects; disposable runtime observations may be lost. Paths locate resources but do not
authorize them. Peer process identity is not an application grant. No multi-user, hostile-host,
secret, network, generated-native-code, or sandbox claim is made.

The TCB is accepted safe Rust source, stable Rust toolchain, locked dependencies, standard library,
CPU, trusted local operator, and OS/filesystem implementation. There is no local unsafe Rust or
build script. Production adapters are trusted; deterministic fakes are test oracles, not production
authority.

## Reversal gates

Reopen interface artifacts only for an independent distribution/binding consumer. Reopen grant
revision for a current rotation/revocation need. Reopen per-instance locking or scheduling only for
measured independent concurrency. Reopen a supervisor for demonstrated multi-client/aggregate
admission not served by the foreground session. Reopen cache/tiering only when complete-workflow
stage evidence crosses the recorded threshold. Until direct cutover, retain full replay and the
explicit-frame interpreter as independent oracles.
