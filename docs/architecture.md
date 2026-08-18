# Architecture

`lkjscript` is a typed semantic programming system. An immutable `Snapshot` owns accepted workspace
meaning; one canonical reusable-release payload owns independently distributed semantic meaning;
one application-v2 artifact owns an exact runnable release graph. JSON, editable documents, context,
review text, compiler IR, memory plans, runtime tags, and caches are proposals, views, or derived
state.

Normative behavior belongs to the [semantic model](spec/semantic-model.md),
[language](spec/language.md), [reusable release](spec/reusable-release.md),
[application](spec/application.md), and [protocol](spec/protocol.md) specifications. This document
owns components, dependency direction, process topology, storage, and trust boundaries.

## Primary paths

Workspace authoring remains:

```text
task-scoped context / editable document / strict JSON
  -> closed typed transaction
  -> Engine under one state-directory authority lock
  -> staged immutable Snapshot and history validation
  -> artifact + HEAD + response preflight
  -> one durable immutable workspace revision
```

Distribution and composition are separate:

```text
exact workspace revision + package + exports + tests + exact dependency artifacts
  -> release closure and canonical workspace-ID erasure
  -> canonical release-local semantic model
  -> encode -> hostile decode -> semantic revalidation -> exact re-encode
  -> exact graph validation and all release tests
  -> validate-only receipt or no-overwrite release publication

explicit exact release artifacts + entry + profile + policy + application cases
  -> exact reachable graph validation (missing/extra/cycle/private rejection)
  -> compiler-private deterministic graph flattening
  -> Core IR compile and independent verification
  -> all release and application cases
  -> canonical application-v2 bundle publication
  -> workspace-free validate / inspect / test / typed run / bytes stream
```

The application embeds every exact release once. A mutable store, resolver, daemon, workspace HEAD,
or network is not on the build or run path.

## Semantic authority and identity

`graph.rs`, `schema.rs`, and `validate.rs` own workspace snapshots and acceptance. Workspace
identity has durable continuity IDs plus revision-bound function-local body IDs. `transaction.rs`
owns proposal normalization and allocation; `diff.rs` owns change facts.

`release/canonical.rs` projects one package closure into a release-local namespace. It assigns all
selected definition IDs before rewriting references, so recursion does not require content hashes
per definition. It canonicalizes modules/definitions and preserves semantic child/body order. The
projection erases workspace/revision IDs, allocator history, tombstones, unrelated declarations,
aliases, and paths.

`release/codec.rs` owns exact `ReleaseId` and `ReleaseContentDigest` domains and canonical release
bytes. `ReleaseId` equality deliberately means complete immutable release equality; public nominal
identity is `(ReleaseId, ReleaseItemId)`. Coordinate, user version, export names, and dependency
slots remain separate metadata/lookup roles.

`release/graph.rs` owns the exact acyclic graph, direct-slot import validation, diamond
deduplication, multi-version coexistence, graph limits, and the sole compiler-private flattening.
Imported proxies redirect to exact dependency items and are omitted from the flattened ownership
tree. Distinct release pairs never collide even when local ordinals and structure match.

## Release and application ownership

The release domain is split by invariant:

| Owner | Responsibility |
|---|---|
| `release/mod.rs` | public release DTOs, identity types, preparation, inspection, tests, limits |
| `release/canonical.rs` | workspace closure, canonical local IDs, remapping, release validation |
| `release/codec.rs` | strict canonical binary envelope and hostile decode |
| `release/graph.rs` | exact dependency graph, proxy signatures, cycles/diamonds, flattening |
| `release/tests.rs` | canonical equality, two consumers, versions, diamond, mutation/publication |

`application.rs` owns application contract 2, exact graph bundle encoding, entry/profile/policy,
public exact nominal values, application cases, inspection, execution, and publication.
`application/tests.rs` owns format rejection, offline typed/stream behavior, nominal values,
missing/corrupt/extra graph objects, 10,000 mutations, and publication edges. The application owner
uses `release::graph`; it does not duplicate release validation or closure logic.

`artifact_io.rs` owns the shared strict regular-file reader and immutable no-overwrite publisher for
release and application artifacts. Workspace persistence remains separately owned by
`persistence.rs` because HEAD history has a different authority transition.

## Engine, storage, and publication

`engine.rs` owns workspace create/open, transactions, queries, compile/run, and release preparation
from an exact revision. Application build is a pure operation over immutable release bytes and does
not open Engine. One `lkjscript.engine.lock` protects a state directory. A competing engine rejects
with `authority_busy`; no mutation is silently retried.

Workspace publication stages and validates the candidate, preflights bytes/response, synchronizes
the revision artifact, atomically advances and synchronizes HEAD, then publishes in memory. An
ambiguous HEAD transition is `commit_outcome_unknown` and poisons the Engine instance.

Release and application publication have no mutable namespace allocator. Their caller selects one
explicit absolute destination. The shared publisher creates a private same-directory file, writes
and synchronizes complete canonical bytes, creates one atomic no-replace hard link, removes the
private name, and synchronizes the directory. Failure before the link is known failure; after the
link it is `artifact_publication_outcome_unknown`. No automatic retry occurs.

Workspace storage retains complete canonical artifacts per revision and compact HEAD. Full scans
remain the query oracle. The reusable workload does not require a content store: application bundles
are 2.9–6.2 KiB in the retained proof and explicit release files are enough for build. There is no
mutable index, store recovery mode, object garbage collection, or lockfile.

## Agent and command boundaries

`protocol.rs` owns workspace logical requests. `contract.rs` owns the manually assembled
machine-schema-v10 catalogue and `machine.rs` owns strict JSON. `workbench/` owns exact context,
documents, review, and compact help. Normal agent work uses task-scoped roots and exact digest reuse,
not a global schema dump.

Release and application commands are deliberately command-local contracts. Their authoritative
Rust types and validators live with their semantic owners; `bin/lkjscript/release.rs` and
`bin/lkjscript/application.rs` only parse options, strict JSON/raw input, map exit classes, and
render bounded output. They do not enter a second catalogue.

The primary CLI opens Engine directly. A line-delimited session amortizes Engine startup across
workspace operations. `lkjscriptd`, `daemon.rs`, and `transport.rs` remain an optional private
socket diagnostic path over the same Engine for the exported framed client and its unique timeout,
correlation, disconnect, shutdown, and lock tests. Release/application work neither requires nor
uses the daemon.

## Compiler and runtime

`compile.rs` discovers the selected flattened entry closure and lowers to private dense Core IDs.
`core_ir.rs` independently verifies type tables, nominal closure, control, results, calls, and
origins. No serialized Core IR is trusted or distributed.

`ownership.rs` derives and independently verifies managed-reference liveness and uniqueness.
`managed.rs` owns checked generation-tagged immutable-byte handles and the allocate-new differential
mode. `interpret.rs` owns public-value validation, explicit frames, flat cells, traps, policy,
cleanup, and materialization. Runtime nominal tags derive from exact flattened release/item pairs;
they are never semantic or artifact identity.

Compilation and execution remain iterative for user-scalable control. Fuel, frames, live cells,
visible bytes, retained backing, objects, allocations, decoded input, and result output are distinct
policies. Release cases and application cases use the same compiler/interpreter oracle.

## Dependency direction

```text
IDs + closed language schema
  -> immutable workspace model + validation
  -> transactions / history / workspace persistence
  -> release canonical model + codec
  -> exact release graph + private flattening
  -> application graph bundle
  -> compiler / Core verifier / ownership / interpreter
  -> JSON, raw stream, terminal, and optional socket adapters
```

Queries and workbench observations depend on workspace authority but cannot mutate it. Codecs depend
on semantic owners and do not become alternate validators. Adapters depend on all relevant owners;
semantic owners do not depend on adapters.

## Trust boundaries

Model output, JSON, documents, packets, workspace/release/application bytes, explicit dependency
sets, filesystem metadata, and public runtime values are untrusted. Structural decode is followed by
semantic validation and exact re-encoding. Unknown, malformed, oversized, duplicate,
noncanonical, foreign-domain, corrupt, truncated, or trailing forms reject before compile/run.

Artifact paths are absolute and lexically canonical; observed symlink parents/inputs and
non-regular inputs reject. The operating system, filesystem, and concurrent directory
administration remain trusted; process separation and these path checks are not a sandbox. The
trusted computing base includes stable Rust, standard library, Cargo dependencies, OS, filesystem,
and CPU. The crate contains no local unsafe Rust and no project build script.

Programs have no ambient host effects or external-resource values. Future effects require explicit
typed authority, acquisition/use/close, timeout, cancellation, partial-action, retry/idempotency,
audit, crash, and cleanup contracts. Immutable-value reclamation remains separate.

## Deliberate absences and reversal gates

- No resolver, lockfile, registry, range, mutable release store, signature, provenance, revocation,
  or trust metadata exists. Add each only for a concrete consumer and keep it outside semantic
  identity.
- No release re-export, release cycle, definition-level content identity, vendoring, or workspace
  import exists. Reopen only for an application the exact external graph cannot express locally.
- No executable cache exists. Prototype one only when measured decode/compile dominates repeated
  application startup and retain semantic compile/interpreter as oracle.
- No semantic index or incremental workspace store exists. Reopen only after scan/restart/retained
  bytes cross recorded thresholds.
- The daemon remains only while the framed client tests are current public value; delete binary,
  client, transport, docs, and tests together when they are not.
- The managed-byte route remains only while representative absolute copy/peak savings justify its
  planner, verifier, handles, and tests.
