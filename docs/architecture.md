# Current architecture

This document maps implemented layers and dependency direction. Normative behavior belongs under
`docs/spec/`; exact executable identities belong in the generated
[contract table](generated/contracts.md).

## Current development path

All released finite graph and command operations converge on Graph 5 authority:

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

## Layer ownership

| Layer | Primary code | Owns | Does not own |
|---|---|---|---|
| Executable protocol | `src/bin/lkjscript.rs`, `platform/contract`, `platform/cli.rs`, `platform/control` | closed operations and grammar, compact models, response bounds, exit mapping | semantic records or repository layout |
| Current authority | `platform/kernel`, `platform/publication`, `platform/witness`, `platform/storage` | typed Graph 5 meaning, full validation, immutable packs, exact revisions/receipts, one atomic `HEAD` | compiler caches, artifacts, deployment |
| Authored change | `platform/change`, logical-plan control | typed intent, allocation, ownership closure, impact/test selection, reviewed semantic effects | publication visibility or derived cache identity |
| Query | `platform/normalized_query`, publication read views | revision-pinned owner, namespace, and relation reads with logical continuations | mutable cursors or repair |
| Package boundary | `platform/package_interface`, `platform/package_transport`, `platform/builtin_standard` | exact public interfaces, closure transport, one validated embedded standard dependency | a general registry or ambient resolver |
| Compiler/cache | `platform/compiler` | deterministic compiler units, exact manifest, clean/incremental derived cache, linker, artifact 10 | accepted semantic identity |
| Normalized execution | `platform/execution/normalized` | dense runtime indexes, VM, canonical reference interpreter, tests and pure commands | semantic publication or live deployment grants |
| Derived output | `platform/owned_output` | bounded synchronized create-new file publication | overwrite or semantic visibility |
| Frozen service runtime | `platform/artifact.rs`, legacy `platform/execution`, deployment/adapters | strict read-only artifact-4 loading and current service/worker host mechanics | editable Graph 4 authority or current builds |
| Contributor verification | `tools/lkjscript-dev` | gate DAG, fingerprints, classifications, logs, receipts, product/service evidence | product authority |

The old `SemanticWorkspace`, predecessor repository writer, drafts, history/diff/merge workflows,
backup/restore, review projection, and query indexes have no current consumer and are deleted. Old
meaning/artifact/runtime types remain only where the frozen artifact-4 service boundary or an
implementation-disjoint test oracle requires them.

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

## Built-in dependency and project recipe

`packages/standard` owns two generated assets: a package transport for dependency installation and
an artifact-10 bundle for linking/execution. `builtin_standard` embeds both, strictly loads them,
and checks package, semantic revision, logical package revision, interface, and artifact identities
for agreement. Public inspection/export exposes the exact bytes without permitting replacement.

The command recipe constructs typed meaning directly. It resolves the public standard identity
function through the validated built-in interface and stores an exact declaration reference. It
then creates the package, application module, private pure function, component, port, target, and
test through ordinary Graph 5 initial publication. There is no source template, migration reader,
path lookup, or network fetch.

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

## Frozen service separation

The retained service path is intentionally outside the current compiler/artifact pipeline:

```text
deployment descriptor
      + frozen-service/lkjournal-artifact-v4.lkja (exact digest)
      + environment and generic host adapters
                         │
                         ▼
             legacy prepared artifact runtime
                         │
                  serve / worker
```

`PreparedDeployment::load` consumes descriptor, artifact, environment, and adapters only. The
service harness copies those inputs to an isolated directory and verifies the frozen artifact
digest before launch. It never opens `applications/lkjournal` as editable authority. A later
campaign must move effectful deployment to artifact 10 before normalized service completion can be
claimed.

## Security and replaceability

All external records, paths, transports, artifacts, cache objects, descriptors, JSON values, and
adapter inputs are hostile boundaries with independent limits. Diagnostics preserve stable class
and code without exposing secrets or large payloads. Runtime resources are scoped and released on
success, failure, cancellation, exhaustion, and shutdown.

The HTTP listener is plaintext and PostgreSQL uses `NoTls`. TLS termination, encrypted graph
storage, hostile-code sandboxing, multi-tenant isolation, artifact signatures, distributed
consensus, JIT/AOT, custom allocation, and a resident authoring daemon are not implemented.
