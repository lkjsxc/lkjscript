# Current architecture

This document maps implemented layers and dependency direction. Normative behavior belongs under
`docs/spec/`; exact executable identities belong in the generated
[contract table](generated/contracts.md).

## Current development path

All current finite graph and command operations converge on Graph 5 authority:

```text
argv / compact records / bounded JSON arguments
                     │
                     ▼
       exhaustive executable registry and typed adapters
                     │
                     ▼
 current project discovery ── rejects predecessor markers
                     │
                     ▼
       GraphRepository / exact RepositoryView
          │ reads                         │ accepted change
          ▼                               ▼
 status / inspect / query       prepare complete candidate
                                 + logical review evidence
                                           │
                                           ▼
                              immutable data, then atomic HEAD
```

`GraphRepository` is the sole normal accepted-authority writer. Public adapters never write raw
storage objects. Names and compact syntax are locators or transport; stable typed keys and the
accepted semantic graph own continuity and meaning.

Closed project creation enters that same authority boundary without first inventing source or a
storage-shaped request:

```text
closed minimal / command / HTTP recipe
            │ typed graph + complete validation
            ▼
 private sibling: canonical Graph 5 repository
            │ HTTP only: typed deployment descriptor + empty generated/
            │ synchronize complete private inventory
            ▼
       one destination visibility rename
            │
            ├─ Graph 5 authority: editable program meaning
            └─ deployment 1: separate operator-editable authority
```

The executable embeds every recipe rule and exact standard package byte. HTTP creation and runtime
admission share generation-neutral structural HTTP types from `platform/http.rs`; an independent
normative field oracle prevents a shared constructor from becoming the only evidence. Project
creation depends on the validated built-in package interface and typed deployment encoder, not on
Axum, resident state, a checkout path, or a prebuilt application artifact.

The command lifecycle continues from the same exact repository view:

```text
GraphRepository / exact accepted revision
          │
          ├─ exact built-in transport selection and interface validation
          │
          ├─ exact-current compiler cache ──┐
          │                                 ├─ compilation manifest + units
          └─ clean normalized compilation ──┘
                                             │
                    exact dependency artifacts + linker
                                             │
                                 strict artifact-10 loader
                                             │
                                  dense NormalizedProgram
                                   ┌─────────┼─────────┐
                                   ▼         ▼         ▼
                              graph tests  artifact  pure command
                              VM/reference  output   VM/reference
```

One `normalized_lifecycle` preparation function owns repository binding, the supported exact
dependency closure, cache selection/recovery, compilation, linking, artifact validation, dense
preparation, and typed observations. `check`, `build`, and `run` do not duplicate compiler or
linker decisions.

## Derived release distribution path

The repository has one tag-driven public binary path for the admitted Linux target:

```text
annotated SemVer tag on origin/main
                  │
                  ▼
read-only build / full verification / exact-candidate acceptance
                  │
                  ▼
deterministic release handoff + byte-bound verifier handoff
                  │
                  ▼
read-only no-checkout release verification + HTTP oracle
                  │
                  ▼
no-checkout publication job with contents:write and release handoff only
                  │
                  ▼
immutable GitHub Release and release attestation
                  │
                  ▼
anonymous exact/latest download + independent transferred HTTP oracle runs
```

`tools/lkjscript-dev` owns typed release preparation and strict archive verification. The hosted
workflow supplies exact runner context and two bounded transient handoffs. The release handoff owns
the archive, checksum, and private release receipt. The separate verifier handoff binds the exact
release-built `lkjscript-dev` bytes; read-only jobs verify those bytes before restoring executable
mode after artifact transport. The pre-publication job checks out no source, safely extracts the
packaged candidate through the first-party release verifier, and runs the same distributed HTTP
oracle used by contributor profiles. The publication job receives only the release handoff and is
the only job with release-write authority. Post-publication verification downloads exact and
latest assets anonymously and runs the transferred oracle independently against both.

The public release, transient artifacts, archive, manifest, checksum, verifier receipts, asset
digest, and attestation are all derived distribution evidence. None can select or edit Graph 5
meaning, executable contracts, compilation semantics, or deployment data. The root package version
and annotated tag bind release identity; recovery from a published content defect uses a new patch
identity rather than mutation. Immutable `v0.1.7` is the first public release admitted through this
complete path: its pre-publication package run and both independent anonymous exact/latest HTTP
oracles passed in [workflow 33150527883](https://github.com/lkjsxc/lkjscript/actions/runs/33150527883).

## Layer ownership

| Layer | Primary code | Owns | Does not own |
|---|---|---|---|
| Executable protocol | `src/bin/lkjscript.rs`, `platform/contract`, `platform/cli.rs`, `platform/control` | closed operations and grammar, compact models, response bounds, exit mapping | semantic records or repository layout |
| Current authority | `platform/kernel`, `platform/publication`, `platform/witness`, `platform/storage` | typed Graph 5 meaning, full validation, immutable packs, exact revisions/receipts, one atomic `HEAD` | compiler caches, artifacts, deployment |
| Authored change | `platform/change`, logical-plan control | typed intent, allocation, ownership closure, impact/test selection, reviewed semantic effects | publication visibility or derived cache identity |
| Query | `platform/normalized_query`, publication read views | revision-pinned owner, namespace, and relation reads with logical continuations | mutable cursors or repair |
| Package boundary | `platform/package_interface`, `platform/package_transport`, `platform/builtin_standard` | exact public interfaces, closure transport, one validated embedded standard dependency, narrow command/HTTP recipe resolution | a general registry or ambient resolver |
| Compiler/cache | `platform/compiler` | deterministic compiler units, exact manifest, clean/incremental derived cache, linker, artifact 10 | accepted semantic identity |
| Normalized execution | `platform/execution/normalized` | dense runtime indexes, VM, canonical reference interpreter, tests, commands, resident HTTP/worker execution, exact capability bindings | semantic publication or deployment authority |
| Derived output | `platform/owned_output` | bounded synchronized create-new file publication | overwrite or semantic visibility |
| HTTP semantic boundary | `platform/http.rs` | exact structural request/header/query/response and handler types shared by authoring and runtime admission | listener adaptation, resident state, or application policy |
| Standalone deployment | `platform/deployment.rs`, normalized deployment/adapters, representation-neutral database/object/queue engines | strict typed descriptors, starter HTTP defaults, artifact-10 loading, target/grant/preflight binding, adapter ownership, HTTP/worker lifecycle | project discovery, accepted publication, or application policy |
| Contributor verification | `tools/lkjscript-dev` | gate DAG, fingerprints, classifications, logs, receipts, product/service evidence | product authority |
| Release distribution | `tools/lkjscript-dev` release tooling, `.github/workflows/release.yml` | deterministic package validation, transient handoff, immutable publication, anonymous transport verification | program meaning, compiler/runtime authority, or build provenance |

The old `SemanticWorkspace`, predecessor repository writer, drafts, history/diff/merge workflows,
backup/restore, review projection, query indexes, artifact-4 reader/runtime, and predecessor value
representation have no current consumer and are deleted. The source-era parser remains only as an
implementation-disjoint language test oracle; it has no public project or deployment path.

## Authority, identity, and storage

A Graph 5 snapshot owns repository and package identity, package name, typed owners, interned type
objects, exact dependency bindings, namespace and relation witnesses, tests, targets, and
retirements. Stable owner domains remain distinct; a module ID cannot be decoded as a declaration
ID even if its payload bytes coincide. Exact semantic references do not carry mutable module or
declaration names.

Logical semantic state is independent of repository identity and physical map partitioning. The
canonical full reconstruction validates all owner records, types, scopes, effects, capabilities,
relations, components, ports, targets, tests, dependency interfaces, and reachability. Sparse
change preparation and point reads retain that complete reconstruction as an independent oracle.

On disk, immutable typed objects and persistent-map pages are sealed into bounded packs. A catalog
maps content keys to physical pack entries and is rebuildable from verified pack contents. `HEAD`
binds one exact repository, revision record, semantic state, root, witness, and acceptance
evidence. Canonical data is synchronized before the separately atomic `HEAD` visibility change.
Missing disposable staging directories may be recreated; missing or inconsistent accepted packs,
objects, or `HEAD` bindings are corruption.

Package transports stored under `PACKAGE-TRANSPORTS` are exact immutable dependency inputs selected
by accepted dependency records. They are not a second package authoring format. The maintained
standard and `lkjournal` roots contain only this Graph 5 layout.

## Publication and derived cache handoff

`change plan` lowers authored input to one typed request, reads an exact base, allocates stable
identities deterministically, prepares a complete candidate and witness delta, selects validation
and tests, and produces logical review records. `change apply` repeats that path, checks both token
commitments, enters the publication lock, rechecks the base, writes immutable accepted objects,
and changes `HEAD` once.

Before publication, apply may observe an exact base compilation manifest. Only after accepted
authority is visible does it pass the in-memory `PreparedPublication.compiler_units` to
`build_incremental`. The cache writer has its own staging, lock, exact binding validation, and
atomic `CURRENT` head. Incremental failure is a derived-state observation; it never changes the
already accepted response. No compiler-impact journal is durable semantic state.

Lifecycle preparation accepts a cache only when repository, revision, semantic state, compiler
contract, optimization policy, unit closure, and object digests agree. A missing cache clean-builds.
A malformed or inconsistent cache is reported, then clean-built and replaced. Clean and
incremental manifests and artifacts are compared in tests.

## Built-in dependency and project recipes

`packages/standard` owns two generated assets: a package transport for dependency installation and
an artifact-10 bundle for linking/execution. `builtin_standard` embeds both, strictly loads them,
and checks package, semantic revision, logical package revision, interface, and artifact identities
for agreement. Public inspection/export exposes the exact bytes without permitting replacement.

The command recipe constructs typed meaning directly. It resolves the public standard identity
function through the validated built-in interface and stores an exact declaration reference. The
HTTP recipe likewise resolves and signature-checks `StaticText -> Text`, `Text -> Bytes`, and the
exact ByteStream interface and operation policies without hard-coded semantic IDs. It constructs
the application response policy, task handler, component requirement and port, HTTP target, and
test through ordinary Graph 5 initial publication.

The HTTP descriptor is encoded once through deployment-owned types with a fresh nonzero operational
authority revision, loopback listener, one byte-stream grant, and independent bounded resources.
Publishing it beside Graph authority in one complete directory does not merge their identity or
mutation rules. There is no source template, migration reader, path lookup, network fetch, hidden
sidecar, or prebuilt application artifact.

## Artifact and execution boundaries

Artifact contract 10 binds the root repository/package/revision/state, every dependency package
revision, compiler and bytecode contracts, compiler-unit maps, runtime owner metadata, public
interfaces, and exact immutable closure. The decoder rejects predecessor magic, noncanonical
order, duplicates, foreign bindings, missing relocation owners, corrupt objects, trailing input,
and configured count/byte exhaustion before execution.

`NormalizedProgram` maps exact semantic owners and compiler operands to dense process-local
indexes. These indexes and runtime handles are replaceable and never become semantic identity.
Pure commands and graph tests execute once in bytecode and once in the independently implemented
reference interpreter with shared explicit limits; disagreement is failure. Live effects are not
duplicated for differential acceptance.

Build output uses a sibling stage, file synchronization, create-new hard-link visibility, parent
directory synchronization, and cleanup of only its owned stage. Existing files, directories,
symlinks, invalid parents, and byte-limit exhaustion reject without a partial visible artifact.

## Standalone artifact-10 deployment

Resident deployment consumes immutable derived execution and external operational authority
without entering the editable project lifecycle:

```text
strict deployment descriptor + relative artifact-10 bundle
                         │
                         ▼
             strict loader + NormalizedProgram
                         │
              exact target / component / requirements
                         │
       grants + secrets + adapter construction/preflight
                         │
              normalized resident VM exactly once
                  ┌──────┴──────┐
                  ▼             ▼
                serve         worker
                  │             │
                  └──────┬──────┘
                         ▼
       normalized capability codecs at artifact edge
                         │
     representation-neutral PostgreSQL/object/queue engines
```

`PreparedDeployment::load` consumes descriptor, artifact, environment, and named host resources
only. It retains the exact bundle/manifest/root/revision/state identities internally, constructs
adapters in deterministic order, and emits readiness only after validation and required preflight.
Preparation failure shuts down every already-created adapter in reverse order; resident shutdown
stops admission, drains/cancels bounded work, and records adapter cleanup exactly once.

The service harness freshly builds `lkjournal`, requires byte equality with the checked-in bundle,
then stages only a copied binary, the bundle, descriptors, configuration/secrets, a local object
directory, and PostgreSQL coordinates. It snapshots canonical Graph authority before and after the
live HTTP/worker/restart workflow. No deployment path opens or advances accepted `HEAD`.

The independent `distributed_http_application` product gate has no database or container
dependency. It copies one candidate executable to a fresh root outside the checkout, creates and
changes an HTTP project, checks and deterministically builds it, starts and restarts the service,
sends raw loopback HTTP, exercises startup failures, and compares exact accepted-authority
inventories. Its stable receipt binds the verifier and candidate bytes. An explicit absolute
create-new evidence root selects transferred mode, in which the verifier resolves no compile-time
checkout path. Product, service, full, pre-publication package admission, and both anonymous public
asset checks all use this same owner.

## Security and replaceability

All external records, paths, transports, artifacts, cache objects, descriptors, JSON values, and
adapter inputs are hostile boundaries with independent limits. Diagnostics preserve stable class
and code without exposing secrets or large payloads. Runtime resources are scoped and released on
success, failure, cancellation, exhaustion, and shutdown.

The HTTP listener is plaintext and PostgreSQL uses `NoTls`. TLS termination, encrypted graph
storage, hostile-code sandboxing, multi-tenant isolation, artifact signatures, distributed
consensus, JIT/AOT, custom allocation, and a resident authoring daemon are not implemented.
