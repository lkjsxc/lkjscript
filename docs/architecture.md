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

Project creation uses the same authored-operation engine without first inventing source or a
storage-shaped request:

```text
minimal / command / HTTP / Nostr relay-information recipe
            │ public-representable typed AuthoredChange operations
            ▼
 normalization + allocation + prepare + logical plan + full validation
            │ exact complete candidate; no recipe-specific owner builder
            ▼
 private sibling: one-initial-revision typed meaning graph repository
            │ resident recipes: typed deployment descriptor + empty generated/
            │ synchronize complete private inventory
            ▼
       one destination visibility rename
            │
            ├─ typed meaning authority: editable program meaning
            └─ deployment descriptor: separate operator-editable authority
```

The executable embeds every recipe rule and exact standard package bytes. Recipe intent and public
compact records converge before semantic allocation and lowering; recipes do not parse a text
round trip, assign semantic IDs, assemble snapshots, or bypass validation. HTTP creation and runtime
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

Immutable `v0.1.16` closes this path at source commit
`b49d78e862b7cbba02c639f06ca4bd2e11db1f2d`. Its exact and latest downloads independently passed
strict package and static inspection plus transferred distributed HTTP, deployment-bound outbound
HTTPS/TLS/DNS, and first-party-data stateful HTTP acceptance. Distributed-receipt contract 3 pages
one complete revision-pinned function definition before and after a reviewed body change, agrees
with direct-file planning and digest reconstruction, and proves malformed/stale continuation and
projection-input rejection without a pre-apply authority change. Each stateful path began with empty
`minimal`, staged the exact built-in transport, and authored the complete dependency/topology/BBS
through the ordinary public writer. The release contains unified graph-native recipe lowering,
the closed NIP-11 relay-information recipe, bounded context traversal, and the complete ordered-data
and durable-queue cutover without contacting a live relay or deploying an application. It also
publishes exact-interface affine resources, unrestricted/borrow/consume parameter use,
compiler/runtime movement checks, the resource-owned queue interface, and complete local-function
definition inspection. Target admission binds the exact service/worker candidate digest and its
independently reconstructed worker definition through the release manifest and accepted exact/latest
public extractions.
Immutable `v0.1.8` remains an unclosed historical recovery point: its application checks passed,
but its workflow rejected legitimately distinct fresh-project artifact identities. Recovery
advanced additively through v0.1.9; the v0.1.10, v0.1.12, v0.1.13, and v0.1.14 publications moved
no predecessor tag, release, or asset; v0.1.15 and v0.1.16 likewise leave every predecessor
unchanged. Current source is unreleased product snapshot 0.1.21; immutable public latest remains
v0.1.16. The source retains requirement-bound affine task handoff, the maintained worker split,
identity-preserving private same-module function extraction, and catalog contract 2. It adds
structured-session contract 1, canonical standard session types, one exact relational
`interactive` target, a bounded RFC 6455 server adapter, and maintained `lkjournal-live-1`. It does
not enter
the distribution path above, alter the
immutable release, or select deployment or operational data.
Deployment 4 owns the strict interactive limits and topology. Project creation 4,
HTTP-client-adapter 1, data-store 1, logical-backup 1, queue-data
1, and all other unchanged identities remain independently owned. Distribution advanced without
advancing semantic `HEAD`, deployment authority, or operational data.

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

Function definition detail is a separate, narrower accepted-meaning projection:

```text
exact local pure/task function + accepted RepositoryView
                         │
                         ▼
 aggregate bounded point reader + structural ownership validation
                         │ complete contract/body/reference/fact closure
                         ▼
 canonical definition records + complete digest/counts
                         │ header, contract, preorder body, references, facts
                         ▼
 stateless pages ── icont_ binds view, function, contract, digest, section, key
```

The aggregate point reader shares one physical admission budget and reveals typed records and bound
facts, never storage representation. Structural children are followed only within the selected
function; named owners and types remain exact reference boundaries. Every page reconstructs and
validates the complete closure. A contributor oracle independently reconstructs full typed
authority and compares owner/fact/relation inventories without production traversal, ordering,
rendering, paging, or token code. No source projection, body index, mutable session, application
dumper, or `change` reader exists.

Function extraction consumes that projection only as derived observation; accepted meaning remains
the graph candidate prepared from typed intent:

```text
exact base + local function + proper expression root + private name
                              │
               independent bounded closure analysis
                              │ captures / effect / requirements / affine provenance
                              ▼
stable moved owners ── reparent under generated helper
original parent slot ── replace with generated direct call and local reads
                              │
                reviewed plan + exact reprepare
                              ▼
               one GraphRepository visibility change
```

Movable owners retain identity. Only free-local reads are rebound to generated parameters; the
selected parent relation is replaced at the same evaluation position. Plans and oracle witnesses are
derived and cannot select or advance semantic authority.

## Layer ownership

| Layer | Primary code | Owns | Does not own |
|---|---|---|---|
| Executable protocol | `src/bin/lkjscript.rs`, `platform/contract`, `platform/cli.rs`, `platform/control` | closed operations and grammar, compact models, built-in/deployment discovery, response bounds, exit mapping | semantic records or repository layout |
| Current authority | `platform/kernel`, `platform/publication`, `platform/witness`, `platform/storage` | typed meaning graph, exact-interface resource/use meaning, full validation, immutable packs, exact revisions/receipts, one atomic `HEAD` | compiler caches, runtime handles, artifacts, deployment |
| Authored change | `platform/change`, logical-plan control | typed intent, deterministic allocation, bounded extraction/capture/effect closure, impact/test selection, reviewed semantic effects | publication visibility, source text, stored refactor recipes, or derived cache identity |
| Inspection and query | `platform/cli.rs`, `platform/normalized_query`, publication read views | exact-owner summary, complete bounded local-function definition projection, namespace, relation, and bounded local-context reads with stateless logical continuations | mutable cursors, body/query indexes, repair, dependency bodies, source export, or authoring authority |
| Package boundary | `platform/package_interface`, `platform/package_transport`, `platform/builtin_standard`, `platform/builtin_discovery` | exact public interfaces and references, bounded owner query/detail, closure transport, one validated embedded standard dependency, and exact offline export/staging | package implementation bodies, a general registry, or ambient resolver |
| Compiler/cache | `platform/compiler` | deterministic compiler units, exact manifest, clean/incremental derived cache, linker, artifact bundle | accepted semantic identity |
| Normalized execution | `platform/execution/normalized` | dense runtime indexes, affine local movement, scope/interface/requirement-bound resource entries, VM, canonical reference interpreter, tests, commands, resident HTTP/interactive/worker execution, exact capability bindings | semantic publication or deployment authority |
| Derived output | `platform/owned_output` | bounded synchronized create-new file publication | overwrite or semantic visibility |
| HTTP semantic boundary | `platform/http.rs` | exact structural request/header/query/response and handler types shared by authoring and runtime admission | listener adaptation, resident state, or application policy |
| Structured-session semantic boundary | `platform/session.rs`, `packages/standard` | canonical event/decision family, exact repeated ordinary state relation, phase and retained-state admission | connections, framing, timers, application policy, or deployment bounds |
| Outbound HTTP boundary | `platform/http_client.rs`, normalized HTTP-client binding | exact endpoint parsing, DNS/address classes, TLS trust, HTTP/1.1 GET, independent limits, cancellation, cleanup, and structural capability codec | graph-selected destination/trust, redirects, retries, proxy, outbound WebSocket, or application response policy |
| Operational data | `platform/data.rs`, normalized data/queue adapters, `platform/queue.rs`, `platform/queue/data.rs` | canonical typed data values, immutable store revisions, exact-base transactions, scans, logical backup/restore, private queue attempt tuples and one durable queue backend | program meaning, public raw lease authority, object bytes, deployment policy, or remote database service |
| Standalone deployment | `platform/deployment.rs`, normalized deployment/adapters | one strict descriptor/schema inventory, starter HTTP defaults, artifact bundle loading, target/grant/preflight binding, adapter ownership, HTTP/interactive/worker lifecycle | project discovery, accepted publication, or application policy |
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

On disk, immutable typed objects and persistent-map pages are sealed into bounded pack contract 1
files.
Catalog contract 2 is a disposable physical index: one atomic manifest selects a bounded set of
content-addressed sorted segments, and each segment binds bounded blocks plus exact pack
descriptors. Healthy open and lookup read only the manifest, selected segment metadata, candidate
blocks, and targeted pack footers; sealing adds one delta and performs deterministic streaming
equal-level merges. Neither path enumerates all old packs or materializes and rewrites the complete
catalog.

`HEAD` binds one exact repository, revision record, semantic state, root, witness, and acceptance
evidence. New packs and catalog segments become durable, then the catalog manifest becomes durable,
before the separately atomic `HEAD` visibility change. Missing, predecessor, malformed, stale, or
current-closure-incomplete catalog state is rechecked and rebuilt once from immutable pack footers
under the exclusive repository lock. That independent recovery path does not call the healthy
segment lookup or merge implementation. Missing or inconsistent accepted packs, objects, or
`HEAD` bindings remain canonical corruption and cannot be repaired by catalog bytes.

Package transports stored under `PACKAGE-TRANSPORTS` are exact immutable dependency inputs selected
by accepted dependency records. They are not a second package authoring format. The maintained
standard and `lkjournal` roots contain only this typed meaning graph layout.

## Affine capability-resource path

One exact-interface right remains linked across semantic, derived, and operational boundaries:

```text
CapabilityResource<Interface> + parameter use / optional helper binding
                 │ exact acquiring requirement provenance
                 ▼
 language-order affine validator + disjoint finite oracle
                 │ borrow / consume / one direct acyclic handoff
                 ▼
 compiler-unit 4 / bytecode 3 local loads + exact binding
                 │ Artifact 14 shape/call-graph validation
                 ▼
 task resource scope: scope + kind + interface + requirement
                 │ reserve before effect; commit or release
                 ▼
 DurableQueue adapter ── private JobLease tuple ── queue-data-1 engine
```

Accepted meaning owns the resource type, acquiring requirement, and borrow/consume protocol. The
compiler and artifact are derived carriers. A normalized resource entry is the sole live runtime
right; ordinary values cannot recreate it. The queue engine may retain private job, attempt, and
worker fields because operational queue state is a separate authority, but those fields never
re-enter graph values or public adapter signatures. A resource-bearing nominal variant moves as a
whole and transfers its one direct payload only to the selected match arm. One final consume
parameter may bind the exact task requirement on a private same-package helper; direct call frames
share the task scope, recheck the handle, and form an acyclic resource-call graph. No host frame or
function value becomes parallel authority.

Claim and heartbeat reserve scope capacity before performing a possibly visible queue effect.
Empty, stale, failed, or cancelled outcomes release the reservation; success commits a live handle.
Borrow leaves the entry live, while consume removes lexical ownership before either a direct
helper call or adapter call. Failure after helper transfer does not restore the caller. Task
cleanup drops only local entries and never performs an implicit queue transition. This ordering prevents
avoidable post-effect allocation loss without claiming exactly-once work.

## Structured interactive-session path

The interactive path keeps semantic state and live ownership deliberately disjoint:

```text
graph handler: (Option<State>, SessionEvent) -> SessionDecision<State>
                              │ exact closed repeated State relation
                              ▼
                 Artifact 14 + deployment 4 preparation
                              │ validate all session limits before readiness
                              ▼
          one parent session scope owns RFC 6455 connection
       reader ── ordered byte-bounded inbox ── transition driver
                    │ one finite callback at a time
       tick (coalesced)                    reserved outbound capacity
                    │                              │
                    └──────────────── driver ──────┘
                                                   ▼
                              byte-bounded outbox ── writer
```

Axum/Tungstenite owns generic HTTP/1.1 upgrade and RFC 6455 framing. The parent scope owns reader,
driver, writer, tick source, cancellation lineage, task-scoped inbound streams, retained ordinary
state, mailbox/process-buffer permits, and every child join. The graph owns open/path/authentication
policy, message decoding, subscription and data transitions, output values, and close decisions.
No callback receives a connection handle, no child detaches, and no Rust callback stores
application sequence, filter, or subscription state.

Transport events are ordered, while a coalesced tick is admitted only behind events already in the
inbox. Before invoking a potentially effectful callback, the driver reserves enough writer capacity
for the configured maximum transition output. It validates the complete decision, next state, and
batch before enqueue and state installation. Failure or cancellation therefore cannot install
partial state or partial output; prior possibly visible graph effects retain their existing
contract and are not replayed. Peer close, transport faults, overload, timeout, and shutdown stop
new callbacks, cancel and join siblings, close task streams, discard operational state, attempt one
bounded close where valid, and release permits once.

## Publication and derived cache handoff

`change plan` lowers authored input to one typed request, reads an exact base, allocates stable
identities deterministically, prepares a complete candidate and witness delta, selects validation
and tests, and produces logical review records. `change apply` repeats that path, checks both token
commitments, enters the publication lock, rechecks the base, writes immutable accepted objects,
and changes `HEAD` once.

For `extract.function`, both paths materialize the complete selected definition, establish one
structural incoming edge, infer ordered free-local captures and the minimal effect/requirement
contract, and preserve movable identities while replacing one parent edge. The review binds the
base-definition and moved-owner digests, exact owner sets and mappings, resulting body counts, and
optional affine provenance. Conflicts, stale bases, interruption, exhaustion, or derived-cache
failure cannot partially publish the rewrite.

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

Every nonempty recipe is a typed list of operations from the same authored model exposed by compact
change grammar. It resolves declarations and interfaces through the validated built-in inventory,
then uses normal normalization, deterministic allocation, authored preparation, logical planning,
impact/test selection, full validation, and publication. The command recipe references the exact
public standard identity function. HTTP resolves and signature-checks `StaticText -> Text`,
`Text -> Bytes`, and the ByteStream interface and operation policy. No recipe assigns semantic IDs,
inserts owner records, assembles snapshots, or retains a private topology path.

The HTTP descriptor is encoded once through deployment-owned types with a fresh nonzero operational
authority revision, loopback listener, one byte-stream grant, and independent bounded resources.
Publishing it beside graph authority in one complete directory does not merge their identity or
mutation rules. There is no source template, migration reader, path lookup, network fetch, hidden
sidecar, or prebuilt application artifact.

The Nostr relay-information recipe reuses that inbound topology and resolves the exact standard
`HttpClient.get` interface/operation plus a pure media-type predicate. Its graph owns exact
`GET /relay-info` route membership,
ordered `Accept` header, status/media policy, byte-preserving response, and deterministic 502. Its
descriptor adds one exact client grant whose normalized endpoint, address class, TLS trust, and
independent limits remain operator authority. Project creation validates and synchronizes graph,
descriptor, and empty generated directory before one destination rename; readiness performs no
network request.

## Exact inbound HTTP topology

An HTTP target has no universal handler port. Stable route owners form its sole nonempty finite
dispatch authority:

```text
HTTP target + component ── owns ── exact (method, path) route owners
                                      │ each names one component port
                                      ▼
                         canonical compiler route table
                                      │ strict artifact/load/preflight binding
                                      ▼
validated transport request ── one exact lookup ── selected handler at most once
                                      │ no match
                                      └──────────── fixed empty 404, no task or capability
```

The compiler table and prepared lookup index are derived and disposable. They bind route, target,
component, port, function shape, and requirement closure back to accepted authority and cannot be
overridden by a deployment descriptor. Canonical order is unsigned method bytes then path bytes;
the runtime uses the adapter's validated path spelling without normalization or an implicit method.
Route identity survives a supported method/path/port change, while target inspection exposes only
the canonical route-set digest and count. Bounded context traversal exposes the ordinary
route/target, route/port, port/function, component, and effect relations rather than a second route
projection.

## Public stateful HTTP authoring path

The complete topology is authored through the ordinary reviewed writer from `minimal`, rather than
inherited from a private graph builder:

```text
copied binary discovery
  ├─ compact operation/type/expression grammar
  ├─ exact built-in declarations/interfaces/operations
  └─ strict deployment adapter schema
                     │
                     ▼
 export and stage exact immutable built-in transport
                     │ semantic HEAD unchanged
                     ▼
          one compact typed authored request
 dependency + component + requirements + function-backed HTTP ports
 + target + exact routes + task effects + explicit generic/function values
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

The graph owns the finite exact route set and each route-to-port binding, plus header/body
admission, strict JSON interpretation, post domain types and validation, response
status/headers/body, space/schema/index policy, canonical typed
encoding, expectations, and transaction boundaries. HTTP/domain functions call a narrow
application-owned persistence layer and contain no filesystem representation. Deployment separately
owns the confined first-party root, namespace, sharing domain, authority revision, data limits,
listener, and runtime limits. Runtime facts are operational authority; they never become graph
meaning or share semantic repository identities.

The request creates each handler with its exact result and requirement closure, and topology
validation checks route keys, duplicate absence, port/function agreement, component requirement
closure, component/port ownership, route/target ownership, and HTTP runner shape before
publication. Generic runtime call frames carry concrete graph type
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
indexes. Compiler local-load operands preserve unrestricted, borrow, and consume. These indexes and
runtime handles are replaceable and never become semantic identity.
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
          serve: HTTP or      worker
            interactive
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
data root, and a local object directory. It injects two create-new operational fixtures through the
unchanged queue-data contract, runs two maintained worker processes, and independently scans the
bounded primary state after shutdown. This proves retry/fail and expired-lease replacement alongside
normal completion, renewal-path execution, restart, failed readiness, backup/restore, and cleanup.
It snapshots canonical graph authority before and after the live workflow. No deployment path opens
or advances accepted semantic `HEAD`.

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
copies the candidate to a fresh root. It creates `minimal`, uses only executable discovery and
public package export/staging to construct one bounded dependency/topology/BBS request and `data`
deployment, plans and applies through the public CLI, compares clean and
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
