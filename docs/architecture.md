# Current architecture

This document maps implemented layers and dependency direction. Normative behavior belongs under
`docs/spec/`; exact internal compatibility identities remain at their typed source owners. Public
capabilities and guides expose stable behavior, the product version, and opaque digests.

## Current development path

All current finite graph and command operations converge on typed meaning authority:

```text
argv / compact records / bounded JSON arguments / offline discovery
                     │
                     ▼
      exhaustive public capabilities and typed adapters
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
                         immutable semantic objects, then atomic HEAD
```

`GraphRepository` is the sole normal accepted-authority writer. Public adapters never write raw
storage objects. Names and compact syntax are locators or transport; stable typed keys and the
accepted semantic graph own continuity and meaning.

Closed project creation enters that same authority boundary without first inventing source or a
storage-shaped request:

```text
closed minimal / command / HTTP / Nostr relay-information recipe
            │ typed graph + complete validation
            ▼
 private sibling: canonical typed meaning graph repository
            │ resident recipes: typed deployment descriptor + empty generated/
            │ synchronize complete private inventory
            ▼
       one destination visibility rename
            │
            ├─ typed meaning authority: editable program meaning
            └─ deployment descriptor: separate operator-editable authority
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
                                 strict artifact bundle loader
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

The repository has one tag-driven path for the sole current release target,
`x86_64-unknown-linux-musl`:

```text
locked source + typed target policy
                  │
                  ├─ host application verifier
                  └─ exact static-musl product candidate
                              │
             fresh source full + target admission
                              │
              deterministic release handoff
                 + byte-bound verifier handoff
                              │
        read-only no-checkout package/static verification
             + transferred distributed/stateful/outbound HTTP
                              │
       no-checkout publication job with contents:write only
                              │
               immutable GitHub Release
                              │
 anonymous exact/latest static + distributed/stateful/outbound acceptance
```

`tools/lkjscript-dev` owns one typed target policy, exact target build and admission, release
preparation, strict archive/static verification, and application-verifier handoff. Target admission
binds source and candidate identities to direct ELF inspection, two pinned networkless userland
command lifecycles, and the distributed HTTP, stateful HTTP, outbound HTTP, and standalone service oracles. A host
build and the source-wide full receipt remain distinct from this exact-candidate evidence.

The hosted workflow supplies exact runner context and two bounded transient handoffs. The release
handoff owns the archive, checksum, and private release receipt. The verifier handoff binds the exact
host `lkjscript-dev` bytes and its release/distributed/stateful/outbound roles. Read-only jobs verify those
bytes before restoring executable mode after artifact transport. The pre-publication job checks out
no source, safely extracts and re-inspects the packaged candidate, then runs all three transferred
application oracles. The publication job receives only the release handoff and is the only job with
release-write authority. Post-publication verification downloads exact and latest assets
anonymously and runs strict static inspection plus all three oracles independently against each.

The public release, transient artifacts, archive, manifest, checksum, receipts, asset digest, and
attestation are derived distribution evidence. None can select or edit typed meaning, executable
behavior, compilation semantics, or deployment data. The root package version and annotated tag
bind the human-facing product snapshot while internal compatibility identities remain independently
owned.
Published content recovers through a new patch rather than mutation.

Immutable `v0.1.12` closes this path at source commit
`8a0141a151a87fe59ccc1ebc738a7e5dd51c6882`. Its exact and latest downloads independently passed
strict package and static inspection plus transferred distributed and first-party-data stateful
HTTP acceptance, including bounded context traversal and the complete ordered-data/durable-queue
cutover.
Immutable `v0.1.8` remains an unclosed historical recovery point: its application checks passed,
but its workflow rejected legitimately distinct fresh-project artifact identities. Recovery
advanced additively through v0.1.9; the v0.1.10 and v0.1.12 publications moved no predecessor tag,
release, or asset. Current source owns product snapshot 0.1.13, CLI 15, project creation 3,
deployment 3, registry 4, and HTTP-client-adapter 1. The immutable public release remains 0.1.12;
data-store, logical-backup, compact-change, and all unrelated identities remain independently owned.

Bounded context remains a read projection of one exact repository view rather than a query store:

```text
exact local root + direction + depth
                 │
                 ▼
 pinned RepositoryView + canonical incoming/outgoing witness ranges
                 │ validate owners, endpoints, edges, revision, admissions
                 ▼
 complete bounded owner-distance map + unique canonical edge set
                 │ owners by (depth, key), then relations by edge key
                 ▼
 stateless pages ── continuation binds view, selector, section, and exclusive key
```

Package and foreign endpoints stop expansion. Every resumed page reconstructs the bounded logical
result from immutable authority; no query index, frontier file, session, cache write, or mutable
cursor exists.

## Layer ownership

| Layer | Primary code | Owns | Does not own |
|---|---|---|---|
| Executable protocol | `src/bin/lkjscript.rs`, `platform/contract`, `platform/cli.rs`, `platform/control` | closed operations and grammar, compact models, built-in/deployment discovery, response bounds, exit mapping | semantic records or repository layout |
| Current authority | `platform/kernel`, `platform/publication`, `platform/witness`, `platform/storage` | typed meaning graph, full validation, immutable packs, exact revisions/receipts, one atomic `HEAD` | compiler caches, artifacts, deployment |
| Authored change | `platform/change`, logical-plan control | typed intent, allocation, ownership closure, impact/test selection, reviewed semantic effects | publication visibility or derived cache identity |
| Query | `platform/normalized_query`, publication read views | revision-pinned owner, namespace, relation, and bounded local-context reads with logical continuations | mutable cursors, query indexes, repair, or full owner bodies |
| Package boundary | `platform/package_interface`, `platform/package_transport`, `platform/builtin_standard`, `platform/builtin_discovery` | exact public interfaces and references, bounded owner query/detail, closure transport, one validated embedded standard dependency, narrow command/HTTP/Nostr-information recipe resolution | package implementation bodies, a general registry, or ambient resolver |
| Compiler/cache | `platform/compiler` | deterministic compiler units, exact manifest, clean/incremental derived cache, linker, artifact bundle | accepted semantic identity |
| Normalized execution | `platform/execution/normalized` | dense runtime indexes, VM, canonical reference interpreter, tests, commands, resident HTTP/worker execution, exact capability bindings | semantic publication or deployment authority |
| Derived output | `platform/owned_output` | bounded synchronized create-new file publication | overwrite or semantic visibility |
| HTTP semantic boundary | `platform/http.rs` | exact structural request/header/query/response and handler types shared by authoring and runtime admission | listener adaptation, resident state, or application policy |
| Outbound HTTP boundary | `platform/http_client.rs`, normalized HTTP-client binding | exact endpoint parsing, DNS/address classes, TLS trust, HTTP/1.1 GET, independent limits, cancellation, cleanup, and structural capability codec | graph-selected destination/trust, redirects, retries, proxy, WebSocket, or application response policy |
| Operational data | `platform/data.rs`, normalized data adapter, `platform/queue/data.rs` | canonical typed data values, immutable store revisions, exact-base transactions, scans, logical backup/restore, one durable queue backend | program meaning, object bytes, deployment policy, or remote database service |
| Standalone deployment | `platform/deployment.rs`, normalized deployment/adapters | one strict descriptor/schema inventory, starter HTTP defaults, artifact bundle loading, target/grant/preflight binding, adapter ownership, HTTP/worker lifecycle | project discovery, accepted publication, or application policy |
| Contributor verification | `tools/lkjscript-dev` | gate DAG, fingerprints, classifications, logs, receipts, product/service evidence | product authority |
| Release distribution | `tools/lkjscript-dev` release tooling, `.github/workflows/release.yml` | deterministic package validation, transient handoff, immutable publication, anonymous transport verification | program meaning, compiler/runtime authority, or build provenance |

The old `SemanticWorkspace`, predecessor repository writer, drafts, history/diff/merge workflows,
project backup/restore, review projection, query indexes, predecessor artifact reader/runtime, and predecessor value
representation have no current consumer and are deleted. The source-era parser remains only as an
implementation-disjoint language test oracle; it has no public project or deployment path.

## Authority, identity, and storage

A typed meaning graph snapshot owns repository and package identity, package name, typed owners, interned type
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
standard and `lkjournal` roots contain only this typed meaning graph layout.

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
an artifact bundle for linking/execution. `builtin_standard` embeds both, strictly loads them,
and checks package, semantic revision, logical package revision, interface, and artifact identities
for agreement. Public bounded query/detail exposes the implementation-free interface and exact
compact references; inspection/export exposes identities and exact bytes without permitting
replacement.

The command recipe constructs typed meaning directly. It resolves the public standard identity
function through the validated built-in interface and stores an exact declaration reference. The
HTTP recipe likewise resolves and signature-checks `StaticText -> Text`, `Text -> Bytes`, and the
exact ByteStream interface and operation policies without hard-coded semantic IDs. It constructs
the application response policy, task handler, component requirement and port, HTTP target, and
test through ordinary typed meaning graph publication.

The HTTP descriptor is encoded once through deployment-owned types with a fresh nonzero operational
authority revision, loopback listener, one byte-stream grant, and independent bounded resources.
Publishing it beside graph authority in one complete directory does not merge their identity or
mutation rules. There is no source template, migration reader, path lookup, network fetch, hidden
sidecar, or prebuilt application artifact.

The Nostr relay-information recipe reuses that inbound topology and resolves the exact standard
`HttpClient.get` interface/operation plus a pure media-type predicate. Its graph owns route,
ordered `Accept` header, status/media policy, byte-preserving response, and deterministic 502. Its
descriptor adds one exact client grant whose normalized endpoint, address class, TLS trust, and
independent limits remain operator authority. Project creation validates and synchronizes graph,
descriptor, and empty generated directory before one destination rename; readiness performs no
network request.

## Public stateful HTTP authoring path

The starter topology is extended through the ordinary reviewed writer rather than a second recipe
or private graph builder:

```text
copied binary discovery
  ├─ compact operation/type/expression grammar
  ├─ exact built-in declarations/interfaces/operations
  └─ strict deployment adapter schema
                     │
                     ▼
          compact typed authored intent
 requirement + task effect + explicit generic/function values
                     │
            plan / exact-base apply
                     ▼
       one accepted typed meaning graph application revision
                     │
          check + deterministic artifact bundle
                     ▼
HTTP client -> HTTP adapter -> graph handler/domain policy
                             -> graph persistence functions
                             -> exact DataStore requirement
                             -> first-party ordered data adapter
```

The graph owns route selection, header/body admission, strict JSON interpretation, post domain
types and validation, response status/headers/body, space/schema/index policy, canonical typed
encoding, expectations, and transaction boundaries. HTTP/domain functions call a narrow
application-owned persistence layer and contain no filesystem representation. Deployment separately
owns the confined first-party root, namespace, sharing domain, authority revision, data limits,
listener, and runtime limits. Runtime facts are operational authority; they never become graph
meaning or share semantic repository identities.

`SetFunctionContract` updates the starter handler's exact result/effect requirement closure while
preserving its identity and parameter. Generic runtime call frames carry concrete graph type
substitutions for standard JSON/list declarations in both production and reference tiers. An
idempotent apply retry reopens the accepted request's exact parent and hides only the child-added
physical type objects, preserving logical reprepare even though immutable storage grows.

The built-in standard now exports graph-owned
`list-fold-left<Item, State>(List<Item>, State, Function(State, Item) -> State) -> State`. Its
private index helper uses ordinary direct recursion, `list-length`, `list-get`, and general
`invoke`; there is no fold intrinsic, opcode, or host callback. The BBS passes its private
`(Bool, Header) -> Bool` reducer as a named function value, so application header policy flows into
the standard dependency at runtime without reversing package dependency direction. Public compact
records lower through the same authored-intent codec, validator, compiler, VM, and reference path
as direct graph construction.

## Artifact and execution boundaries

The artifact bundle binds the root repository/package/revision/state, every dependency package
revision, compiler and bytecode compatibility, compiler-unit maps, runtime owner metadata, public
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

## Standalone artifact bundle deployment

Resident deployment consumes immutable derived execution and external operational authority
without entering the editable project lifecycle:

```text
strict deployment descriptor + relative artifact bundle
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
       exact-endpoint outbound HTTP
       + first-party ordered data + object capability
       + first-party durable-queue namespace
```

`PreparedDeployment::load` consumes descriptor, artifact, environment, and named host resources
only. It retains the exact bundle/manifest/root/revision/state identities internally, constructs
adapters in deterministic order, and emits readiness only after validation and required preflight.
Preparation failure shuts down every already-created adapter in reverse order; resident shutdown
stops admission, drains/cancels bounded work, and records adapter cleanup exactly once.

The service harness freshly builds `lkjournal`, requires byte equality with the checked-in bundle,
then stages only a copied binary, the bundle, descriptors, configuration/secrets, one shared local
data root, and a local object directory. It snapshots canonical graph authority before and after
the live HTTP/worker/restart/backup/restore workflow. No deployment path opens or advances accepted
semantic `HEAD`.

The independent `distributed_http_application` product gate has no database or container
dependency. It copies one candidate executable to a fresh root outside the checkout, creates and
changes an HTTP project, checks and deterministically builds it, starts and restarts the service,
sends raw loopback HTTP, exercises startup failures, and compares exact accepted-authority
inventories. Its stable receipt binds the verifier and candidate bytes. An explicit absolute
create-new evidence root selects transferred mode, in which the verifier resolves no compile-time
checkout path. Product/full verification and the target-admission, pre-publication, and
anonymous exact/latest paths all use this same owner.

The distinct `outbound_http_application` owner copies the candidate into a fresh root, creates the
closed relay-information recipe, exercises discovery/status/query/check/build/serve, and compares
clean and exact-current artifacts. An independent raw HTTP/1.1/TLS oracle uses deterministic
certificate fixtures and records exact wire requests while sharing no production endpoint parser,
HTTP parser, response generator, or application assertions. It proves public/loopback address
admission, mixed-answer rejection, TLS chain/hostname/validity failures, no redirect/retry,
forbidden headers, response limits, timeout, inbound cancellation, malformed protocol, failed
startup, restart, active shutdown, unchanged semantic authority, and complete cleanup without a
live relay. Product/service/full, target admission, and future transferred release paths use this
same non-cacheable owner.

The separate `stateful_http_application` owner has the same contributor/transferred context seam and
copies the candidate to a fresh root. It uses executable discovery to construct a bounded BBS
request and `data` deployment, plans and applies through the public CLI, compares clean and
incremental artifacts, initializes an isolated store, then drives real BBS HTTP behavior through one
`lkjscript serve` process. Each post has one primary fact and one `(created-at, id)` index fact;
listing resolves both in one snapshot and create/update/delete maintain them in one transaction. It
proves header and malformed-input behavior, stale expectations and rollback, schema divergence,
restart persistence, backup/absent-root restore, corrupt/absent-root failed startup without
readiness, and unchanged semantic authority. Its current receipt binds verifier/source/copied
candidate identities, context, optional checkout, data authority, and complete cleanup. Service/full
verification and target-admission, pre-publication, and anonymous exact/latest paths share this
owner. The `lkjournal` service oracle remains a separate maintained workload, and distributed HTTP
remains the faster stateless oracle.

The data authority itself is an independent deployment branch:

```text
strict data or durable_queue_data grant
               │ validate confined root / namespace / limits
               ▼
     lkjscript-data-store-1 physical identity
               │ immutable complete revisions
               │ cross-process lock + exact-base recheck
               ▼
        durable atomic operational HEAD
          ├─ application spaces/indexes
          └─ internal durable-queue namespace
```

Readers pin immutable revisions. A commit synchronizes its complete revision before one head
visibility change. Public logical backup pins a head and excludes page/object layout; restore creates
an equivalent absent root with a new physical identity. This operational head has no dependency on
and no write path to `GraphRepository::HEAD`.

Contributor-only `lkjscript-dev scale` stays on the public product dependency direction for every
measured effect: it copies one candidate, discovers its operation grammar, and invokes project
creation, reviewed changes, reads, checks, and builds through that executable. Its separate
read-only `platform::contributor::semantic_inventory` entry opens an exact `RepositoryView`, fully
validates the selected accepted graph, and reports deterministic owner/relation counts and digests
without calling the compact-result formatter or exposing a graph writer. Raw command output,
requests, temporary projects, and artifacts remain derived campaign data; the bounded scale receipt
is evidence rather than program authority.

Contributor-only `lkjscript-dev data-oracle` is outside that product dependency direction. It owns
the PostgreSQL client and exact isolated PostgreSQL 16.15 container, constructs and exports neutral
facts, invokes the first-party store only through its independent import side, binds copied BBS and
service receipts, records resource samples, and removes its temporary authorities. Product,
service, static-target, transferred, and release-candidate paths neither link that client nor start
a database server.

The release command lifecycle also copies the exact candidate into a fresh private root, creates a
command project through that binary, and isolates a complete `lkjournal` authority copy. Candidate
queries resolve `module service`, traverse incoming, outgoing, and both-direction contexts at
multiple depths, resume multiple pages while changing item and byte limits, and compare complete
owner-distance maps and relation sets with independently orchestrated one-hop public reads. Full
in-process reconstruction plus the canonical relation extractor remains the implementation-disjoint
semantic oracle. Complete inventories before and after success and selected failures must agree.

## Security and replaceability

All external records, paths, transports, artifacts, cache objects, descriptors, JSON values, and
adapter inputs are hostile boundaries with independent limits. Diagnostics preserve stable class
and code without exposing secrets or large payloads. Runtime resources are scoped and released on
success, failure, cancellation, exhaustion, and shutdown.

The inbound HTTP listener is plaintext, and first-party data is unencrypted local trusted-host
storage. Outbound HTTPS authenticates one deployment endpoint under locked public roots or one
named PEM root; it provides no browser trust UI, insecure switch, client certificate, privacy
layer, DNSSEC, private-network mode, or sandbox. Inbound TLS termination, encrypted graph or data
storage, hostile-code sandboxing, multi-tenant isolation, artifact signatures, replication,
distributed consensus, online data compaction, JIT/AOT, custom allocation, and a resident authoring
daemon are not implemented.
