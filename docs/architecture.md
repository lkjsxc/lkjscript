# Architecture

This file owns current components, dependency direction, topology, trust boundaries, and the trusted
computing base. Observable contracts are normative only in `docs/spec/`; implemented presence and
absence are summarized in `docs/status.md`.

## System shape

```text
agent document / protocol request
              |
              v
      strict typed proposal ----> deterministic validator
              |                           |
              | accepted                  v
              +----------------> immutable workspace revision
                                         |
                                         v
                         canonical reusable release graph
                                         |
                                         v
                         immutable application format 3
                                         |
                              exact bytes embedded at create
                                         v
 event ---> pure transition ---> full instance record + HEAD
                                      |             |
                                      | command     | exact replay/inspect
                                      v             |
                         grant validator + host executor
                                      |
                         immutable typed host outcome
                                      |
                                      +----> pure resume ----> next record
```

There are four independent authority classes:

1. workspace revisions own development continuity and accepted semantic graphs;
2. releases and applications own immutable transferable program meaning;
3. instance records plus HEAD own mutable state continuity and exact history; and
4. host grants own permission to perform one external operation.

Core IR, layouts, ownership plans, compiled functions, query indexes, rendered context, process
state, slot paths, and runtime handles are derived or locators. A host result is retained evidence,
not semantic state. No conversion between these domains is implicit.

## Components and dependency direction

### Semantic workspace and workbench

`schema`, `graph`, `validate`, `transaction`, and `persistence` own accepted workspace meaning and
revision publication. `contract`, `machine`, `protocol`, and `workbench` provide strict typed
proposals and bounded views. `Engine` is the only workspace publication authority.

The global executable contract is retained for workspace RPC, where one compact schema digest and
task-scoped projection have active consumers. Release, application, and instance command records do
not flow back into that catalogue.

### Release and application distribution

`release` projects one immutable workspace revision plus explicit exact dependencies into canonical
workspace-independent meaning. `application` validates a complete explicit release graph, selects
entries and policies, runs exact cases, and embeds the graph in one file. Neither layer may consult
mutable resolver state, workspace HEAD, or an instance.

Stateful applications add only an immutable interface: pure event and resume functions returning a
closed decision product. Capability grants and mutable state remain absent from application bytes.

### Compiler, interpreter, and managed values

`compile`, `core_ir`, `type_layout`, `ownership`, `managed`, and `interpret` form the semantic
execution path. Complete closure lowers to independently verified Core IR. The explicit-frame
interpreter is the correctness oracle and keeps user-controlled depth off the native stack. The
managed immutable-byte plan is a verified acceleration compared differentially with allocate-new
behavior; it never becomes semantic authority.

### Instance store

`instance` owns a store-wide lock, exact embedded application bytes, full canonical state records,
immutable event and host receipts, attempt markers, and one validated HEAD per instance. It reruns
the complete selected chain on every open. State publication is record first, HEAD second.

The store deliberately uses ordinary files instead of a database or journal: current histories and
states are small, immutable objects simplify validation, and unknown host outcomes do not require
transactional coupling to semantic state. Orphan immutable objects from a pre-HEAD crash are not
authority. Compaction is absent.

### Host executors

The production executor is trusted in-process Rust code with one activation-slot operation. It
validates exact pending command, executor-bound grant, source application, target digest, and slot,
then performs a same-directory atomic replace and sync. A durable pre-action marker prevents silent
repeat after a crash. The deterministic fake executor is a disjoint grant class that records only
closed command-compatible results.

The application decides workflow phases and retries. The host cannot alter semantic state, and the
instance runtime cannot claim external success without a typed host result. Unknown external
visibility is a first-class split boundary resolved only by inspection/reconciliation.

## Process topology

The product installs one `lkjscript` binary. Direct workspace commands open one Engine; the
line-delimited session retains one Engine for dependent authoring calls. Release and application
commands are short-lived pure artifact operations. Instance commands are short-lived and serialize
on one store lock. There is no daemon, socket client, worker, server, background scheduler, or
network listener.

The in-process host executor is a code-organization and trust choice, not sandboxing. A separate
worker was rejected because no untrusted native extension or broad capability exists to justify IPC,
descriptor filtering, process lifecycle, and recovery surface.

## Ordering and crash model

Per instance, the canonical observable order is:

1. validate exact base, event/result, application, policy, and output envelope;
2. deterministically evaluate the pure transition;
3. publish and synchronize one immutable full-state record;
4. replace and synchronize HEAD;
5. return the transition receipt;
6. if suspended, separately validate grant and publish an attempt marker where applicable;
7. execute or simulate one command;
8. publish one immutable host outcome;
9. return the host receipt; and
10. accept a later exact resume as a new state transition.

A process crash can lose an unacknowledged response but cannot create a second accepted semantic
transition for the same event key. State and external activation are never described as one atomic
commit. Fault tests classify every retained link/rename/sync visibility boundary as known no-change,
known complete, explicit unknown, or corruption.

The filesystem model assumes a trusted local POSIX-like implementation of regular files, private
permissions, hard-link no-replace, same-directory rename, and file/directory sync. Process-crash
tests and deterministic I/O fault models are covered. Sudden power loss, controller caches,
filesystem firmware defects, and hostile root administration remain outside proof.

## Identity and isolation

Workspace, revision, release, release item, application digest, instance ID, state digest, record
digest, grant digest, command ID, runtime handle, event key, and path remain distinct domains.
Instance ID provides continuity; revision provides order; state digest provides integrity/equality;
record/application digests provide immutable content identity; event key provides instance-local
deduplication; command ID provides pending-result correlation; grant digest binds exact policy.

One store lock prevents concurrent writers. Every record, HEAD, attempt, outcome, command, and grant
revalidates instance and exact application domains. Two instances may use the same application but
cannot substitute event receipts, command results, grants, or state. Storage directories and slot
paths are derived locators and provide no semantic authority.

## Threat model

The attacker controls model-authored requests and any byte/path input accepted at a public boundary.
The local operator and OS administrator are trusted unless a row states otherwise.

| threat | protected asset / boundary | prevention and detection | recovery / residual risk / evidence |
|---|---|---|---|
| malformed or oversized proposals | memory, semantic authority / JSON and document parsers | closed serde/document grammars, duplicate/unknown/trailing rejection, checked lengths/counts before work | deterministic errors; mutation, exact-limit, and one-over tests; parser/dependency bugs remain possible |
| corrupt workspace bytes | revision history / Engine open | domain/version/digest/chain validation and semantic replay | reject and preserve bytes for operator repair; workspace corruption/restart tests |
| corrupt release or application | distribution meaning / artifact load | strict canonical codecs, complete graph validation, exact re-encode, tests before run | reject before compile; truncation/bit-mutation/old-format suites |
| corrupt instance, state, HEAD, attempt, or outcome | state continuity / store open | strict directory layout plus envelope/domain/digest/type/chain/command-result validation and deterministic transition replay | reject rather than skip; record/outcome corruption, recomputed forged-outcome, and exhaustive envelope mutation tests |
| path traversal or noncanonical path | local files / CLI and grants | bounded absolute lexical grammar; reject dot, parent, empty and repeated components | deterministic denial; hostile administrator can race namespace after validation |
| symlink substitution | source, slot, store / filesystem | `symlink_metadata`, non-symlink regular file/directory requirements, parent-chain checks | reject on observation; a hostile privileged racer remains in TCB |
| hard-link ambiguity | immutable objects and source bytes / filesystem | immutable object name binds digest; conflicting bytes reject; source artifact is decoded and target digest checked | content remains exact; inode provenance is not claimed |
| concurrent directory administration | publication / local host | exclusive store/Engine locks and same-directory operations | external root/admin changes can cause I/O, corruption, or unknown result; no hostile-host isolation |
| stale state writer | current instance HEAD / event boundary | exact base revision plus store-wide exclusive lock | `revision_conflict`, no publication; stale and competing-writer tests |
| duplicate event or response loss | one transition / event receipts | canonical instance-scoped key binds exact retained input and receipt | exact replay, conflict on different bytes; public controller proof |
| capability confusion or overbroad grant | activation namespace / host entry | grant digest binds instance, executor, source directory and one slot; no general file command | `capability_denied`; denied/cross-instance/cross-executor tests; local operator still chooses grant |
| cross-instance access | state, results, grants / every instance codec | instance/domain binding in HEAD, records, commands, attempts, outcomes and grants | reject; two-instance public proof and foreign-domain tests |
| forged success evidence | semantic resume / fake and production results | production derives digest evidence after exact validation; every fake, loaded outcome, and replayed host input requires the command-compatible class and exact target digest | reject malformed/foreign/recomputed incompatible evidence; trusted executable could still lie |
| command-result replay | one pending command / outcome store | derived command ID, immutable one-outcome publication, exact duplicate equality | replay same receipt; conflicting result rejects; resume event key prevents double publication |
| unknown activation outcome | external slot / rename-to-response interval | attempt marker before action; post-visibility failures classify unknown; no automatic repeat | explicit reconciliation present/absent/indeterminate; activation fault matrix and fake-host public proof |
| resource exhaustion | host memory/disk/work / all boundaries | independent state, event, history, replay, command, evidence, output, frame, cell, fuel and byte limits | preallocation rejection; disk capacity and OS accounting remain trusted |
| native stack exhaustion | process / semantic traversal | explicit graph/parser/interpreter work stacks and bounded fixed internal recursion | depth tests; Rust/library stack use outside user-controlled paths remains trusted |
| process crash or response disconnect | publication/result acknowledgement | sync-before-visibility protocol, immutable receipts, explicit unknown classes | restart validates exact authority; directory/record/HEAD/activation fault matrices and duplicate replay |
| power loss | durable files / kernel/filesystem | file and directory sync under documented model | no hardware/firmware proof; an indeterminate or corrupt result is reported, never guessed |
| cleanup failure | temporary files and external action / publication | cleanup is after immutable visibility and maps to explicit unknown where outcome matters | retry reconciles exact object; stale crash staging may require trusted operator cleanup |
| log or terminal injection | operator context / diagnostics | structured bounded JSON, stable codes, terminal-safe semantic renderers | free-form OS errors remain diagnostic only and are never parsed as authority |
| secret leakage | artifacts, state, context / public values | no secret-bearing capability, secret store, ambient environment, or redaction-dependent semantic field | users must not put secrets in ordinary state/events; local filesystem confidentiality only |
| dependency compromise | executable and validators / Cargo supply chain | locked small dependency graph, no local unsafe, no build script, full differential tests | Rust toolchain/dependencies remain TCB; no reproducible supply-chain attestation claim |
| sandbox overclaim | user security decisions / documentation | process and in-process boundaries are explicitly not sandboxes | hostile OS account/admin isolation is absent and stated in README/spec/status |

## Design comparisons and external orientation

The selected boundary follows the useful separation found in durable workflow systems—deterministic
workflow state versus externally fallible activities—without importing a scheduler, service, or
opaque replay runtime. Temporal's current documentation informed this failure distinction:
[Temporal documentation](https://docs.temporal.io/).

Koka's typed-effect material clarified the distinction between effect signatures and actual
authority, but a general effect system would not solve publication or unknown outcomes for the
current application: [Koka book](https://koka-lang.github.io/koka/doc/book.html). The WebAssembly
Component Model informed closed typed interface comparison, not implementation topology:
[Component Model concepts](https://component-model.bytecodealliance.org/design/component-model-concepts.html).

SQLite's atomic-commit description informed the explicit file/directory synchronization and
failure-point audit, but the repository retained its narrow canonical files rather than importing a
database: [SQLite atomic commit](https://www2.sqlite.org/atomiccommit.html).

## Trusted computing base and reversal gates

The TCB is the accepted Rust source, stable Rust toolchain, locked dependencies, standard library,
OS process/filesystem implementation, CPU, and trusted local operator. Application-authored data is
untrusted. The fake host is an oracle, not production authority.

Reconsider the design when a current workload needs independent commands in parallel, live
resources, unattended time-based scheduling, revocable long-lived grants, hostile-code isolation,
state/history compaction, a second host capability that shares a safe narrower abstraction, or
cross-platform filesystem behavior. Any replacement must retain the pure interpreter and exact
state/history differential until direct cutover.
