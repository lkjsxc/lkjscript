# Implemented status

This file describes the current checkout. Normative behavior belongs to `docs/spec/`; measurements
and decisions belong to `docs/performance.md`; ownership and topology belong to
`docs/architecture.md`.

## Product boundary

`lkjscript` is a local typed semantic programming system for pure deterministic applications. A
coding agent can create and revise a workspace, observe bounded exact context, publish an immutable
workspace-independent reusable semantic release with explicit exports and dependencies, compose an
exact release graph, and build one single-file application that can be validated, inspected,
tested, typed-invoked, and stream-run after all workspaces are removed.

The supported bootstrap is stable Rust edition 2024 on Linux x86-64. The repository forbids local
unsafe Rust. The application runner is not a hostile-host sandbox, registry, package manager,
native executable runtime, or deployment system.

## Active versions

| Contract | Active value | Older forms |
|---|---|---|
| workspace logical protocol / strict JSON | 10 | 9 and older reject |
| machine contract | `lkjscript-machine-schema-v10` | v9 and older reject |
| workbench | 2 | other versions reject |
| context packet | 2 | packet 1 rejects |
| editable semantic document | 1, root `document` | `plan` rejects |
| reusable-release CLI JSON | 1 | other versions reject |
| reusable-release artifact | 1, `LKJREL\0\x01` | other forms reject |
| application CLI JSON | 2 | version 1 and others reject |
| application artifact | 2, `LKJAPP\0\x02` | format 1 and others reject |
| workspace semantic artifact | 6, `LKJTSM\0\x06` | format 5 rejects |
| semantic schema | `lkjscript-tsm006` | older schemas reject |
| workspace HEAD | `LKJHEAD8` | `LKJHEAD7` rejects |

There is no compatibility reader, migration mode, edition split, fallback, or successful old-form
alias.

## Workspace and agent workflow

The primary CLI opens the topology-neutral `Engine` under one state-directory authority lock.
`lkjscript session` retains one Engine for independent line-delimited requests. Agent commands are
`orient`, `create`, `context`, `document`, `validate`, `apply`, `view`, `diff`, and exact-revision
`run`. Context digest reuse returns `unchanged` only after exact reconstruction. Normal work does
not require the full global schema.

One immutable `Snapshot` is workspace authority. Workspace-qualified durable IDs name continuity
for packages/modules, declarations, members, functions, parameters, and explicit hole anchors.
Regions, blocks, binders, ordinary operations, and terminators use revision-bound function-local
IDs. Body replacement preserves function identity and rebuilds local IDs. Names are scoped lookup
and display metadata, never inferred identity.

Workspace persistence stores one full canonical artifact per immutable revision plus compact HEAD.
Restart decodes contiguous history. Validate-only and rejection publish nothing and consume no
durable identity. Queries remain deterministic full scans with differential controls; there is no
semantic index, delta log, object store, compactor, or garbage collector.

## Reusable semantic releases

Release build contract 1 names one exact workspace/revision/package root, explicit export set,
coordinate, user version, exact dependency slots, import proxies, and release cases. It never
infers HEAD. One preparation owner projects the export/test closure, erases workspace identity,
assigns canonical release-local IDs, independently decodes and validates bytes and the complete
graph, and runs every supplied release suite before validate-only success or publication.

`ReleaseId` is the domain-separated 256-bit digest of the complete canonical release payload and is
the exact dependency and nominal domain. `ReleaseContentDigest` uses a separate domain and carries
integrity/equality meaning only. Coordinate and user version are immutable human metadata, not
exact selectors. Provenance and signatures are explicitly absent.

Format 1 supports explicit function, product-type, and sum-type exports; private reachable
implementation; exact acyclic dependencies; bodyless local function/nominal import proxies; and
primitive exact release cases. Consumers cannot reference private targets or undeclared transitive
dependencies. Full signature validation precedes private graph flattening.

Release files are limited to 64 MiB, 100,000 semantic items, 256 exports, 256 dependencies, 4,096
imports, and 256 tests. Graphs are limited to 256 releases, 4,096 edges, depth 64, and 256 MiB
aggregate release bytes. All collections are canonical; unknown, duplicate, malformed, foreign,
noncanonical, truncated, oversized, digest-mismatched, or trailing content rejects.

There is no resolver, lockfile, mutable store, network fetch, registry, range selection, or
automatic latest version. Every graph operation receives explicit exact artifact bytes.

## Application graph and invocation

Application contract 2 names an exact root release, exact exported entry, typed or bytes-stream
profile, policy, and nonempty application cases. Build accepts every release through repeated
`--release FILE`, validates exactly the reachable graph, rejects missing and unrelated objects,
privately flattens exact identities, verifies Core IR, and runs all release and application cases.
It opens no workspace and performs no resolution.

Format 2 embeds every exact release once in strict ID order. It contains one graph, entry, profile,
policy, and application suite but no workspace/revision IDs, paths, caches, resolver state, Core IR,
runtime handles, provenance, or signatures. Its 256-MiB decoder checks all release and graph limits,
requires exact re-encoding, and rejects application format 1 directly.

Typed public nominal values carry exact `(ReleaseId, ReleaseItemId)` types and member targets.
Structurally identical R1/R2 values do not unify; a shared diamond R1 does. Bytes-stream remains a
pure bounded `bytes -> bytes` adapter. Both use the same explicit-frame interpreter oracle.

Release and application publication share the no-overwrite artifact owner: absolute canonical
non-symlink paths, private mode-0600 temporary file, complete write, file sync, atomic hard link,
temporary cleanup, and directory sync. Before-link failure is known failure. Any after-link failure
is `artifact_publication_outcome_unknown` and is never silently retried.

## Public command families

```text
release build --state DIR [--dependency FILE ...] (--validate-only | --output FILE)
release validate|inspect|test --artifact FILE [--dependency FILE ... for test]

app build --release FILE [--release FILE ...] (--validate-only | --output FILE)
app validate|inspect|test --artifact FILE
app run|stream --artifact FILE
```

Release inspection exposes exact identity, content digest, metadata, export signatures/member IDs,
dependencies, tests, counts, and limits. Application inspection exposes the exact graph, graph
digest, entry/profile/policy, test digests, flattened item count, and aggregate limits. Receipts are
bounded and do not echo large requests.

## Language, compiler, and runtime

Implemented semantics include `unit`, `bool`, checked `i64`, immutable `bytes`, immutable nominal
products and sums, calls, conditions, counted loops, exhaustive lazy matching, exact
construction/projection, returns/yields, and typed holes. Compilation lowers only one complete
entry closure and independently verifies target-neutral Core IR. Compiler IDs remain private.

The explicit-frame interpreter bounds user depth without native recursion. Immutable bytes use
generation-checked handles and an independently verified ownership plan; allocate-new execution is
the differential oracle. Fuel, frames, cells, visible bytes, retained backing, objects,
allocations, input, and result materialization are separate policies. Cleanup is tested after
success and failure.

No serialized Core IR, bytecode, JIT, AOT, native image, executable cache, host effect, external
resource, concurrency, time, randomness, filesystem access, or network access exists in language
semantics.

## Process topology and contract ownership

`lkjscriptd` remains an optional private Unix-socket diagnostic adapter because the exported framed
`Client` and tests still consume correlation, deadlines, disconnect, shutdown, and competing-lock
behavior. It is not required for authoring, release, application build, or execution and is not a
sandbox.

The global protocol-v10 machine catalogue remains manually owned in `contract.rs` because schema
digest binding, workbench help, context/document contracts, root projections, and diagnostic clients
consume it. Release and application DTOs deliberately use their Rust types/codecs and command-local
help instead of duplicating them into the catalogue. No generator, IDL, or second catalogue is
retained.

## Retained public examples

- `reusable-release` proves `shared-codec`, two independent consumers, R1/R2 coexistence, an exact
  diamond, private rejection, nominal rejection, corruption/missing/extra rejection, complete state
  deletion, and offline byte-identical application rebuild through production commands.
- `binary-canonicalizer` exercises the larger authoring/repair/history/runtime workload, publishes
  its reusable release, builds application format 2, deletes state, and validates/tests/runs the
  retained artifacts.
- `job-policy`, `named-data`, `release-channel`, `release-manifest`, and `agent-maintenance` retain
  broader language, history, identity, repair, review, and diagnostic coverage.

## Exact absences and evidence limits

The checkout has no online or local resolver, lockfile, registry/index, mutable release store,
dependency range, re-export, release cycle, provenance artifact, signature, attestation,
authorization, revocation, transparency log, remote build, deployment, sandbox, or compatibility
path. Release tests support primitive invocation values only; public nominal composition is tested
at application level. Copy, vendor, fork, and import-into-workspace operations are not implemented.

The mutation corpora are deterministic rather than coverage-guided. No fresh provider trial,
provider token/pricing telemetry, cross-platform run, model checker, new long-running fuzz target,
sanitizer run, or hostile concurrent-directory-administration proof was completed. Measurements are
reported as bytes and processes, never inferred tokens. Current reproduced evidence is in
[performance.md](performance.md).
