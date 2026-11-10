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
          ├─ exact immutable source closure admission and interface validation
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

## Local builds and retained installation

Cargo builds the development executable. The [verification command](spec/verification.md)
owns routine, focused and explicitly selected slow checks. The maintained compiler
bootstrap route is recorded in [status](status.md).

`src/release_container` retains strict container and executable-linkage admission;
`platform/installation` owns local runtime installation and selection. Their current
interface is described in [product surface](spec/product-surface.md). Distribution
work follows [the current release policy](release.md).

## Layer ownership

| Layer | Primary code | Owns | Does not own |
|---|---|---|---|
| Executable protocol | `src/bin/lkjscript.rs`, `platform/contract`, `platform/cli.rs`, `platform/control` | closed operations and grammar, compact models, built-in/deployment discovery, response bounds, exit mapping | semantic records or repository layout |
| Current authority | `platform/kernel`, `platform/publication`, `platform/witness`, `platform/storage` | typed meaning graph, exact-interface resource/use meaning, full validation, immutable packs, exact revisions/receipts, one atomic `HEAD` | compiler caches, runtime handles, artifacts, deployment |
| Authored change | `platform/change`, logical-plan control | typed intent, deterministic allocation, bounded extraction/capture/effect closure, impact/test selection, reviewed semantic effects | publication visibility, source text, stored refactor recipes, or derived cache identity |
| Inspection and query | `platform/cli.rs`, `platform/normalized_query`, publication read views | exact-owner summary, complete bounded local-function definition projection, namespace, relation, and bounded local-context reads with stateless logical continuations | mutable cursors, body/query indexes, repair, dependency bodies, source export, or authoring authority |
| Package boundary | `platform/package_interface`, `platform/package_transport`, `platform/builtin_standard`, `platform/builtin_discovery` | exact public interfaces and references, bounded staged owner query/detail, code-complete immutable closure transport, canonical private-body admission, and exact offline export/staging | editable imported repositories, trusted transported bytecode, a general registry, or ambient resolver |
| Compiler/cache | `platform/compiler` | deterministic compiler units, exact manifest, clean/incremental derived cache, linker, artifact bundle | accepted semantic identity |
| Normalized execution | `platform/execution/normalized` | dense runtime indexes, affine local movement, scope/interface/requirement-bound resource entries, VM, canonical reference interpreter, tests, commands, resident HTTP/interactive/worker execution, exact capability bindings | semantic publication or deployment authority |
| Derived output | `platform/owned_output` | bounded synchronized create-new file publication | overwrite or semantic visibility |
| HTTP semantic boundary | `platform/http.rs` | exact structural request/header/query/response types and selector-indexed handler types shared by authoring and runtime admission | listener adaptation, resident state, or application policy |
| Structured-session semantic boundary | `platform/session.rs`, `packages/standard` | canonical event/decision family, exact repeated ordinary state relation, phase and retained-state admission | connections, framing, timers, application policy, or deployment bounds |
| Outbound HTTP boundary | `platform/http_client.rs`, normalized HTTP-client binding | exact endpoint parsing, DNS/address classes, TLS trust, HTTP/1.1 GET, independent limits, cancellation, cleanup, and structural capability codec | graph-selected destination/trust, redirects, retries, proxy, outbound WebSocket, or application response policy |
| Operational data | `platform/data.rs`, normalized data/queue adapters, `platform/queue.rs`, `platform/queue/data.rs` | canonical typed data values, immutable store revisions, exact-base transactions, scans, logical backup/restore, private queue attempt tuples and one durable queue backend | program meaning, public raw lease authority, object bytes, deployment policy, or remote database service |
| Standalone deployment | `platform/deployment.rs`, normalized deployment/adapters | one strict descriptor/schema inventory, starter HTTP defaults, artifact bundle loading, target/grant/preflight binding, adapter ownership, HTTP/interactive/worker lifecycle | project discovery, accepted publication, or application policy |
| Native guide implementation and retained policy fixture | `tools/native-guides/project`, `tools/native-policy/project`, `platform/native_tool.rs` | native capability-reference rendering; byte-classification fixture for public CLI tests; shared strict grant-free embedding | registry/Git/filesystem authority, production policy classification, semantic publication, capability grants, or compiler self-hosting |
| Contributor verification | `tools/check.sh`, `tools/lkjscript-dev` | routine and focused tests, explicit slow controls, standalone semantic probes | product authority |
| Public documentation origin | `tools/lkjscript-site` | Rust-rendered compile-time allowlisted documents, bounded text search, read-only HTTP, source-revision metadata | compiler execution, workspace file access, project mutation, TLS termination, DNS or multi-tenant isolation |
| Local runtime installation | `src/release_container`, `platform/installation` | container validation, local installation and runtime selection | program meaning or compiler authority |

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

On disk, immutable typed objects and persistent-map pages are sealed into bounded packs
under the single current schema epoch 1. The catalog is a disposable physical index:
one atomic manifest selects a bounded set of
content-addressed sorted segments, and each segment binds bounded blocks plus exact pack
descriptors. Healthy open and lookup read only the manifest, selected segment metadata, candidate
blocks, and targeted pack footers; sealing adds one delta and performs deterministic streaming
equal-level merges. Neither path enumerates all old packs or materializes and rewrites the complete
catalog.

`HEAD` binds one exact repository, revision record, semantic state, root, witness, and acceptance
evidence. New packs and catalog segments become durable, then the catalog manifest becomes durable,
before the separately atomic `HEAD` visibility change. Missing, foreign-epoch, malformed, stale, or
current-closure-incomplete catalog state is rechecked and rebuilt once from immutable pack footers
under the exclusive repository lock. That independent recovery path does not call the healthy
segment lookup or merge implementation. Missing or inconsistent accepted packs, objects, or
`HEAD` bindings remain canonical corruption and cannot be repaired by catalog bytes.

`PACKAGE-TRANSPORTS/CURRENT` atomically exposes complete validated source selections over the ordinary
immutable object store. Accepted dependencies select one exact logical/semantic revision per package
across the full transitive union. Names and locations never select meaning. Imported sources include
private bodies and current retirements but no editable repository, history bodies, grants, operational
data, or independently trusted compiler units. Direct-dependency/public-interface visibility remains
separate from closure availability. The maintained standard and `lkjournal` use this common path.

## Affine capability-resource path

One exact-interface right remains linked across semantic, derived, and operational boundaries:

```text
CapabilityResource<Interface> + parameter use / optional helper binding
                 │ exact acquiring requirement provenance
                 ▼
 language-order affine validator + disjoint finite oracle
                 │ borrow / consume / one direct acyclic handoff
                 ▼
 compiler-unit 5 / bytecode 3 local loads + exact binding
                 │ Artifact 15 shape/call-graph/route-signature validation
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
whole and transfers its one direct payload only to the selected match arm. One final borrow or
consume parameter may bind the exact task requirement on a directly named helper, including
an imported public function or forwarder. Public signatures retain that original requirement;
private/package-visible dependency members remain hidden. Direct call frames share the task
scope and recheck the handle. Direct and mutual recursive calls apply the same independently
validated body contracts. Borrow preserves the owner, consume transfers it, and neither recursion,
a package boundary nor a host frame creates parallel authority. Imported entry obligations must
be explicit in the port's closed task row and separately granted by deployment.

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
                 Artifact 15 + deployment 4 preparation
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
A malformed or inconsistent cache is reported and clean-rebuilt. If damaged immutable derived
objects already occupy clean keys, lifecycle recovery compiles the admitted source read-only without
overwriting those packs; future preparation may rebuild again. Canonical source corruption cannot
enter this cache-recovery branch. Clean, incremental, and recovered artifact bytes are compared.

## Immutable package composition and project recipes

`packages/standard` owns a code-complete source container and a derived artifact bundle.
`builtin_standard` is an embedded exact source supplier using common admission. It verifies the
derived asset against clean compilation of that exact graph; package identity grants no execution
privilege. All dependencies compile bottom-up from immutable package views through the existing
compiler/linker, without writable shadow projects. A separate closure oracle reconstructs owners,
edges, and interfaces for canonical reference evaluation, independently of production selection and
bytecode. Canonical owners independently supply reference callable/signature, nominal-layout, type,
target, test, and blob access; neutral value-schema access is shared with boundary codecs.
Runtime lists share the neutral immutable 32-way trie/tail carrier in `normalized/list.rs`.
It owns only shape, length, indexed traversal, and immutable handles. Each evaluator independently
owns type/origin/affinity and capture decisions, cancellation, calls, and the cumulative ledger
that admits a carrier reservation before allocation. Calls, locals, environments, and intrinsic
forwarding preserve the same carrier. JSON output explicitly materializes a bounded external array;
typed-data codecs traverse directly. Internal levels never become language depth or serialized
authority. Independent flat-Vec storage and charge oracles cover the shared carrier.

Rank-one composite instantiations absent from stored type objects are derived into disposable
preparation tables. `normalized/prepared_types.rs` follows bytecode calls, while
`normalized/reference_types.rs` independently follows accepted owners. Their unchanged canonical
type identities are never written back to graph meaning or transported as new accepted types.
The compiler test inventory must equal the canonical inventory, and each selected package's tests
run once; target selection stays root-scoped.
Public bounded staged/builtin query and detail expose only public interfaces and exact references;
transport export includes complete private implementations and is not source confidentiality.

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

## Signature-indexed inbound HTTP topology

An HTTP target has no universal handler port. Stable route owners form its sole nonempty finite
dispatch authority:

```text
HTTP target + component ── owns ── method + typed exact/pattern selectors
                                      │ capture order indexes port/function parameters
                                      ▼
                       compiler exact index + segment trie
                                      │ strict artifact/load/preflight binding
                                      ▼
validated path spelling ── bounded specificity lookup ── one handler + captures
                                      │ no match
                                      └──────────── fixed empty 404, no task or capability
```

The compiler table and prepared matcher are derived and disposable. They bind route, selector
segments, captures, target, component, port, function parameters, HTTP types, and requirement
closure back to accepted authority and cannot be overridden by a deployment descriptor. Canonical
order binds method, selector kind and segments, capture order, and port identity. Duplicate match
languages and incomparable overlap reject; exact and strictly more-specific literal refinements
win without priority or source order. Runtime matching preserves validated raw segment spelling and
performs no decoding, normalization, implicit method, or query selection. Route identity survives a
supported method/selector/port change, while target inspection exposes exact/pattern counts, the
route-set digest, and maximum specificity chain. Bounded context traversal exposes the ordinary
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
 + target + exact/pattern routes + indexed capture parameters + task effects
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

The graph owns the finite typed route set and each selector-to-port/function binding, plus header/body
admission, strict JSON interpretation, post domain types and validation, response
status/headers/body, space/schema/index policy, canonical typed
encoding, expectations, and transaction boundaries. HTTP/domain functions call a narrow
application-owned persistence layer and contain no filesystem representation. Deployment separately
owns the confined first-party root, namespace, sharing domain, authority revision, data limits,
listener, and runtime limits. Runtime facts are operational authority; they never become graph
meaning or share semantic repository identities.

The request creates each handler with its exact result and requirement closure, and topology
validation checks selector grammar and bounds, overlap/specificity, indexed parameter names/order/
types, port/function agreement, component requirement closure, component/port ownership,
route/target ownership, and HTTP runner shape before
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

The standard also owns `function-compose<A,B,C>(Function(B)->C, Function(A)->B)->Function(A)->C`.
Its private generic graph helper invokes the inner then the outer callable; the public factory binds
those two function values. `ExpressionOperation::Bind` stores only a callee and ordered expression
children. The public compact form, complete definitions, relations, extraction, compiler units, and
package closure follow those ordinary children. No composition intrinsic or second body authority
exists.

Production and canonical-reference execution independently admit a prepared target, fully resolved
type arguments, and an immutable flat runtime prefix. Each new capture is checked before evaluating
the next capture. Environments own immutable values and share collection payloads across invocation;
they retain no lexical frame or mutable substitution map. Capture admission counts environment edges
under depth 256 and charges slots, traversal, and metadata before growth. Ordinary local reads,
argument transfers, and tail invocation use checked roots without descendant readmission. Callable
signatures are leaves for capture safety, allowing generic composition; a stored unconstrained type
parameter remains rejected. Runtime environments have neither semantic equality nor external or
durable encoding.

Function-owned effect parameters and exact finite rows are inline current-schema fields. Pure and
task callable kinds remain distinct; task types retain their exact signature and effect row.
Effect substitution traverses
nested callable and nominal types before ordinary type arguments are installed. Compiler operands
retain exact ordered applications, and strict artifact loading compares executable contents with
canonical owners before preparation. Production closes effect applications under checked work and
storage limits. The canonical reference derives its own closure, signatures and activation rows.

Descriptors carry an exact prepared target, closed ordered type/effect arguments and flat safe
prefix. They carry no grant, adapter, resource, transaction or frame. Every task invocation checks
the current activation's allowance and resolves compatible concrete references to the same checked
canonical grant. Nested activations receive their own declared allowance. Both evaluators preserve
grant accounting and lexical transactions through imported callbacks. HTTP, worker and session
ports use explicit closed task-callable types; the production task-as-pure exception is retired.

The standard range task fold and bound-mapper task map are accepted graph functions. The offline
package owner constructs a separate library and consumer through the copied binary, removes the
producer, and observes configuration results and transactional data through public HTTP. Its
source-bound neutral-grant probes measure logarithmic additional range frames separately from
callback work and indexed list access. This fresh workload is distinct from the maintained standard
and lkjournal encoding/dependency/port cutover.

Nominal types retain ordered parameters and explicit applications in the ordinary current type
record. Applications bind exact declarations and ordered arguments. Applied typed-data layouts
describe all arguments and substituted members, including phantom and inactive types.
Maintained standard and application graphs are materialized through the accepted repository writer
with stable owner identities and exact dependency bindings. Foreign schema epochs reject;
maintained derived artifacts are regenerated through ordinary product owners.

Production preparation closes concrete compiled roots over nominal members and generic calls;
canonical reference preparation derives its closure independently from accepted owners. Layout keys
contain the complete application and preparation origin. Both preparations account work and metadata
under existing limits, check cancellation, and derive separate ordinary/capture/equality facts.
Strict artifact loading cross-checks nominal parameter order, bounds, members and constructor argument
inventory against canonical owners. Canonical data and session shape readers apply their own whole-type
eligibility rules. No application identity or host adapter implements pair, batch or edit policy.

Finite recursive nominal schemas use a syntactic graph of stable declaration/parameter slots.
An occurrence nested inside an application's argument adds an expanding edge; a forwarded
parameter adds a plain edge. An iterative SCC analysis rejects an expanding edge within its own
component and reports its owner/member/argument path and cycle. Function signatures participate
in this syntax relation even though functions remain leaves for ordinary containment and capture
safety. Growth on an acyclic path and reset to closed arguments remain finite.

Private preparation worklists use complete applications, including finite parameter permutations.
Production property analysis propagates disqualifying leaves backwards through exact dependencies;
the canonical reference independently eliminates unsafe candidates. Session state substitution
uses fresh declaration bindings and the same exact-instance distinction. Neither a type back-edge
nor a preparation cache substitutes for admission of separate raw values. Recursive typed-layout
markers remain explicit. Nominal/session proofs bind the current validator
and complete source; persisted acceptance alone cannot establish current admission.

Pure graph tail calls replace the current activation in both execution tiers. Preparation derives
terminal-only continuations once by an iterative linear traversal of strictly loaded Return/Jump
edges, then combines them with exact canonical pure-function signatures. Only process-local
dispatch changes during preparation; stored bytecode, compiler units, graph and artifact records
all use the current schema epoch.
The canonical reference separately follows if/let/sequence/match tail contexts and unwinds lexical
maps and substitutions before its activation loop installs the next callee. Neither tier retains
a per-transfer continuation. Arguments retain their evaluation order and cumulative work/allocation
charges. Task frames, including empty-requirement tasks, and ancestor transactions remain live.
Borrowed-result functions retain their activation on both sides of a call; their
source relationship and transfer cleanup cannot be erased by tail replacement.

## Artifact and execution boundaries

Dynamic owned sequences are one finite structural type containing runtime-variable
numbers of owned children. Products, choices and sequences use the same sealed
composite custody carrier; shape remains independently checked at each semantic
and execution boundary. Indexed reads retain sequence and ancestor guards without
holding a storage mutex across user code. Push moves an element into the vector;
pop preserves the vector in an existing owned-choice/product result envelope.
Allocation-free cleanup traverses arbitrary sequence breadth under the admitted
type-depth bound. This does not introduce recursive nominal ownership, escaping
references or a second editable representation of program meaning.

[Source-tied borrowed results](spec/owned-read-results.md) extend lexical read
custody through pure named function and owned-contract method calls. Canonical
signatures bind one exact borrowed parameter, with methods naming its ordered
position. The caller's explicit borrow-call adopts a sealed result packet with
the selected read token and ancestor guards. Reserved storage precedes custody
detachment; child loans release before guards on success and failure. Provenance
retains parameter identity even when two arguments alias one allocation. The
production VM and canonical reference independently enforce that relationship.

The artifact bundle binds the root repository/package/revision/state, every dependency package
revision, compiler and bytecode compatibility, compiler-unit maps, runtime owner metadata, public
interfaces, and exact immutable closure. The decoder rejects predecessor magic, noncanonical
order, duplicates, foreign bindings, missing relocation owners, corrupt objects, trailing input,
and configured count/byte exhaustion before execution.

`NormalizedProgram` maps exact semantic owners and compiler operands to dense process-local
indexes. Compiler local-load operands preserve unrestricted, borrow, and consume. These indexes and
runtime handles are replaceable and never become semantic identity.
VM and canonical reference locals and operands contain private checked values bound to the exact
preparation. Each evaluator independently admits raw invocation, decoded, host and adapter data
with bounded iterative shape/type/containment and live-authority checks. Production nominal
affinity derives once from strict layouts; the reference derives its own table from canonical
owners. An admitted ordinary aggregate proves all its descendants affine-free, so immutable
sharing, ordinary copies and projections preserve proof without a subtree metadata cache. Direct
handles and affine nominal variants retain distinct ownership; even an empty affine case moves.
Core intrinsics combine checked children or return a checked projection. Arbitrary raw host
results cross admission again. Raw runtime indexes carry a neutral, unpersisted preparation origin;
that origin alone cannot certify a value. Failure destroys unadmitted owned data iteratively and
releases invocation-owned resources; an adapter effect that already occurred remains visible.
Ordinary commands execute once on the VM. Graph-owned checks compare VM results with
the independent reference interpreter. Compiler differential tests create fresh invocations
while sharing immutable probe preparation.

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

Standalone `lkjscript-dev distributed-http` copies an executable into an isolated root,
authors an HTTP project through its public CLI, then checks build, startup, restart,
loopback requests, failure cleanup and unchanged semantic authority.

`lkjscript-dev outbound-http` uses independent local HTTP/TLS fixtures to check endpoint
admission, certificate failures, protocol behavior, cancellation and cleanup.
`lkjscript-dev stateful-http` exercises public BBS authoring and transactional data
behavior, including stale expectations, restart, backup/restore and failed startup.
These remain individually invoked development probes.

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
service observations, records resource samples, and removes its temporary authorities.
Other development probes run without this PostgreSQL oracle.

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

A graph TypeParameter now owns its explicit capture-safe constraint. Constraint edits participate
in interface summaries and dependent validation; they do not qualify for body-only validation.
Compiled signatures and transported interfaces retain the same ordered constraints. Production
preparation reconstructs a disposable safe-type inventory from compiled inputs; canonical reference
preparation independently eliminates unsafe stored closures from accepted layouts. Both use the
completed instantiated composite-type closure. Raw invocation and capture still enforce actual
value provenance, affinity, depth and resource ownership before installation. See the
[capture-safe decision](decisions/20260909-capture-safe-rank-one-abstraction.md) for contract domains
and the standard and non-built-in generic factory witnesses.
