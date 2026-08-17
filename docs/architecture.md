# Architecture

`lkjscript` is a typed semantic programming system. The accepted program is an immutable
`Snapshot`; JSON, editable documents, context packets, review text, Core IR, ownership plans, and
runtime values are proposals or derived projections. Only the typed transaction and validator can
publish program meaning.

Normative semantics belong to the [semantic model](spec/semantic-model.md), [language](spec/language.md),
and [protocol](spec/protocol.md) specifications. This document owns components, dependency direction,
process topology, storage shape, and trust boundaries.

## Primary path

```text
task-scoped context packet
    -> bounded editable semantic document
    -> closed typed transaction
    -> topology-neutral Engine under one authority lock
    -> staged immutable Snapshot
    -> semantic and history validation
    -> response, artifact, and HEAD preflight
    -> one durable immutable revision
    -> query/review or dependency-closed Core IR lowering
    -> Core IR verifier
    -> derived ownership plan and independent verifier
    -> explicit-frame managed-value interpreter
```

The primary CLI opens `Engine` directly; users do not manage a background process. A line-delimited
direct session amortizes engine startup for batches. `lkjscriptd` is an optional private Unix-socket
adapter over the same engine, retained for boundary diagnostics rather than as a second semantic
implementation.

## Semantic authority and identities

`graph.rs` owns immutable snapshots. Their current physical representation remains a canonical
ordered map of typed semantic nodes, but that storage shape does not define the product model.
`schema.rs` owns the closed entity and operation vocabulary. `validate.rs` owns whole-model
acceptance and exact history transitions.

Identity has two active strata:

- durable workspace-qualified IDs name continuity-bearing entities, members, parameters, and
  explicit repairable hole anchors;
- function-local IDs name regions, blocks, arguments, ordinary operations, and implied control
  scaffolding only within one function body and exact revision.

The high bit of the current 64-bit serial encoding separates the domains. Local IDs encode the
durable function serial plus a body-local ordinal. Validation rejects cross-function local
references, and local IDs never advance the durable allocator or enter tombstones. A function body
replacement preserves its durable function identity while deterministically rebuilding local terms.
Compiler IDs, runtime handles, packet aliases, draft symbols, and storage digests remain separate
private domains.

`transaction.rs` owns public edit forms, deterministic draft normalization, candidate application,
allocation, and receipt construction. Its large test body is physically separated in
`transaction/tests.rs`; a later ownership split may separate these production concerns when it can
do so without duplicating invariants. `diff.rs` classifies durable entity changes separately from
function-body replacement.

## Editable documents and observations

`workbench/document.rs` owns editable semantic document version 1. The parser is strict, bounded,
location-aware, and uses explicit frames. A document binds schema digest, workspace, base revision,
editable scope, and optional context-packet digest. It normalizes into the same `Transaction` used
by JSON, and its syntax is discarded. Omitted and packet-provided content is not editable.

`workbench/context.rs` composes purpose-specific observations from pure queries. Packet aliases are
valid only with the exact packet digest. A caller may send a known digest and receive a compact
unchanged response, but packets and caches never become authority. `workbench/view.rs` owns bounded,
terminal-safe review output; `workbench/help.rs` projects compact authoring facts from the executable
contract.

`query.rs` owns exact revision-bound scans, pagination, completeness blockers, legal constructors,
dependencies, repair context, and semantic diff composition. Full scans remain the production
implementation and differential oracle because current workloads do not justify persistent indexes.

## Engine and publication

`engine.rs` is the only logical operation dispatcher. It owns workspace create/open, transaction,
query, compile, and run regardless of caller topology. One `lkjscript.engine.lock` protects a state
directory. A competing engine rejects; it does not wait indefinitely or silently retry.

Publication is synchronous:

1. validate the exact base and proposal;
2. construct and validate the candidate snapshot and history transition;
3. preflight the canonical artifact, compact HEAD, and bounded response;
4. write, flush, rename, and sync the revision artifact;
5. write, flush, atomically replace, and sync HEAD;
6. publish in-memory state and acknowledge.

A failure before authoritative HEAD leaves the old head authoritative. If rollback cannot establish
whether the old or new head won, the engine returns `commit_outcome_unknown` and stops accepting
work. Validate-only performs applicable semantic and byte preflight without writing, publishing, or
consuming durable identity.

`persistence.rs` owns path checks, locking, artifact inventory, publication, recovery, idempotency,
and failure injection. `artifact.rs` owns canonical snapshot bytes. The active store keeps a complete
artifact for every revision and decodes contiguous retained history when reopened. Identity
stratification made current body churn sufficiently small that an object store, delta log, or
database did not justify its additional recovery, retention, garbage-collection, and dependency
surface. That decision is explicitly reversible after a larger scaling corpus.

Workspace artifacts are development authority, not distributable package artifacts. There is no
package manifest, dependency resolver, registry, executable cache, or deployment artifact yet.

## Contract and boundaries

`protocol.rs` owns closed logical requests and responses. `machine.rs` owns only strict JSON
envelopes/codecs and a small facade. `contract.rs` owns the executable machine-description
catalogue, while `machine_contract.rs` owns its descriptor value model. Agreement tests compare the
catalogue with actual strict codecs and executable samples. The catalogue is now local to its fact
domain, but remains manually assembled; deriving field metadata from authoritative Rust types is a
future evidence gate.

`bin/lkjscript.rs` and `bin/lkjscript/agent.rs` are presentation and lifecycle adapters. The direct
CLI and session call `Engine`. `daemon.rs` and `transport.rs` own only the optional private socket
adapter and framing.

JSON is limited to 8 MiB input and 32 MiB output. Documents are limited to 8 MiB, 32 parser frames,
65,536 items, and 512-byte diagnostics. Context and review outputs are independently limited to
4 MiB. All observable collections use canonical order.

## Compiler and runtime

`compile.rs` discovers the complete selected-entry function and nominal-type closure, lowers typed
bodies, and maps revision-local semantic origins into private dense Core IDs. `core_ir.rs` owns the
target-independent executable contract and independent verifier. Unreachable incomplete code does
not enter lowering; reachable incompleteness rejects.

`ownership.rs` derives managed-reference maps, liveness, cleanup, and uniqueness actions. Its
verifier independently recomputes those facts. `managed.rs` owns checked generation-tagged byte
handles and an allocate-new differential mode. `interpret.rs` owns public-value validation,
explicit frames, flat cells, traps, resource accounting, cleanup, and result materialization.

The production immutable-byte route retains early reclamation and verified unique-left concat reuse
because the retained corpus reduces copied and peak backing bytes from 32 to 23. This is an
implementation strategy, not language-visible ownership. A simpler allocate-new mode remains the
correctness oracle. A second managed value class, values escaping one invocation, cycles, or a loss
of representative benefit reopens the decision.

Compilation and interpretation remain iterative for user-scalable control. Fuel, frames, live
cells, visible managed bytes, retained backing, managed objects, allocations, and result
materialization are distinct policies.

## Source ownership

The intended dependency direction is:

```text
IDs and semantic contracts
    -> immutable model and validation
    -> transactions and diff
    -> engine and publication
    -> queries and agent projections
    -> compiler and runtime
    -> JSON, socket, and terminal adapters
```

Current major owners are:

| Owner | Responsibility |
|---|---|
| `ids.rs`, `schema.rs`, `graph.rs` | identity domains, semantic vocabulary, immutable state |
| `transaction.rs`, `validate.rs`, `diff.rs` | proposal normalization, acceptance, change facts |
| `engine.rs`, `persistence.rs`, `artifact.rs` | dispatch, locking, publication, recovery, bytes |
| `query.rs`, `workbench/` | exact observations, documents, help, and review |
| `contract.rs`, `machine.rs`, `protocol.rs` | executable contract, strict JSON, logical boundary |
| `compile.rs`, `core_ir.rs` | closure, lowering, origins, executable verification |
| `ownership.rs`, `managed.rs`, `interpret.rs` | derived memory plan, managed values, execution |
| `daemon.rs`, `transport.rs`, `bin/` | optional socket and user-facing adapters |

Large invariant suites now live beside their production owners rather than at the end of production
files. `generated_invariant_tests.rs` retains deterministic generated cross-boundary sequences.
`tests/agent_repair_json.rs` remains a broad integration concentration and is a known locality debt.

## Trust boundaries

All model output, JSON, documents, packets, artifacts, filesystem metadata, and public runtime values
are untrusted. Decoders reject unknown, malformed, oversized, noncanonical, foreign-domain, or
trailing data. State paths must be explicit absolute directories with no symbolic-link components;
artifacts and HEAD are bounded regular files with checked canonical names and hashes.

The package forbids local unsafe Rust. The trusted computing base nevertheless includes the Rust
toolchain, standard library, Cargo, resolved dependencies, operating system, filesystem, and CPU.
`blake3`, `fs2`, and `getrandom` include platform or native-facing implementation below the crate's
safe interface. The repository has no project build script and introduced no dependency in this
campaign.

The local engine and optional process boundary are not a sandbox. Programs currently have no host
effects or ambient authority. Future effects require explicit typed permissions, partial-action and
retry rules, cancellation, audit, and deterministic resource cleanup. Ordinary immutable-value
reclamation must remain separate from affine external-resource semantics.

## Deliberate absences and reversal gates

- No package artifact exists; add one only with an exact manifest, dependency, identity, and
  untrusted-decoding contract.
- No semantic index exists; add a narrow derived index only after a scan-dominated workload and keep
  a full-scan differential oracle.
- No incremental store exists; reopen when retained bytes, restart, branching, or package reuse are
  materially constrained by full snapshots.
- No network service, sandbox, branch/merge UI, JIT, general heap, tracing collector, host effect,
  or resource finalizer exists.
- No alternate editable grammar remains. Reopen syntax only through equal-task evidence against
  document version 1.
- The optional daemon may be deleted when socket-boundary coverage no longer justifies its source and
  binary cost.
