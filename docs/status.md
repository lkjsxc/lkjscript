# Current status

Status date: 2026-09-05 UTC. This file describes implemented checkout reality. Executable-derived
public guides live under [generated](generated), including the
[operation guide](generated/operations.md); this file does not duplicate them.

## Current authority and maintained consumers

The typed meaning graph is the sole current editable program authority. A project root contains a strict
`GraphRepository`: `HEAD`, immutable packs, an object catalog, optional exact package transports,
and private staging/locking state. Accepted meaning is the exact revision and immutable object
closure selected by `HEAD`. No maintained project contains a predecessor `.lkjscript` store.

The sole healthy object catalog is contract 2: one atomic manifest selects at most 32 immutable
content-addressed sorted segments, with at most one segment per level and 64 entries per lookup
block. Healthy repository open and sealing perform no complete old-pack footer scan or full catalog
materialization. Each newly sealed pack set contributes one delta; deterministic equal-level
streaming merges give logarithmic rewrite work. Missing, predecessor, malformed, or derived state
incomplete for the current closure is rebuilt once under the exclusive publication lock from an independent
immutable-footer oracle. Pack contract 1, object-store contract 1, immutable objects, and `HEAD`
remain canonical and unchanged.

| Consumer | Exact current identity |
|---|---|
| standard package | repository `repo_c1358d64c351873b51c954b69d1ac988`; package `pkg_10000000000000000000000000000001`; revision `rev_020a3fd10bc3ccd0f05c8e1bbf12bf259f93ca9dcee084fcaee5528c1e764617`; state `semantic_state_8c6bc5e8a1b83e0018d60223c63bf6b1d9b8999d0ca247b1d8bbaaf600ac9e15` |
| `lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; package `pkg_20000000000000000000000000000001`; revision `rev_0c800bcaf3fb598035b3d29d6bf886dc42f62537569e0aa5124f5fa8c95745a9`; state `semantic_state_11cccd6ab1f48dbcc637737aab3e4740a29a1b3678e94b9e0899b4cc755d4c0c` |
| built-in standard dependency | package revision `package_revision_02f6b51664a610d0a653aa901cef11e39a77bd713c8750defa1aeea5c841faa9`; transport `package_transport_52a7f2a867563eb5367443df9f21518051a4fa0ed527317026651207a1d78124`; artifact manifest `artifact_manifest_4a45508f7083e9e1d913596eb3c7a37e624df08b2bbe2a38db61ee150148b691` |

The standard package owns 550 live semantic owners, 106 compiler units, and 20 graph tests. Its
current artifact has 1,057 closure objects and 328,738 bytes. `lkjournal` owns 2,040 live semantic
owners and one exact standard dependency; its two-package artifact has 196 compiler units, 3,538
closure objects, and 1,062,638 bytes. Its complete dependency closure runs 27 graph tests. Both
consumers currently pass production/reference equality.

Maintained derived assets are:

| Path | Role | SHA-256 |
|---|---|---|
| `packages/standard/generated/standard.lkjp` | code-complete standard source container, 231,436 bytes | `63837ac2cf106c03819eccaa929078015bbf2189dc898d645f957dd669fc1250` |
| `packages/standard/generated/standard.lkja` | current standard artifact bundle, 328,738 bytes | `0cbf9b61a09163c1736da01bd92cd32e5533956593635ab7acfefc156e0a93cb` |
| `applications/lkjournal/generated/lkjournal.lkja` | current application artifact bundle, 1,062,638 bytes | `1a1cf9b5fd7c920e3f6f5a788fc21fa16c35e19238b3f33ea5ccd771fb4311a8` |

The built-in transport and artifact are compiled into the executable and strictly cross-checked.
Product verification regenerates maintained owners and compares exact bytes. Service verification
also performs a fresh public `lkjournal` build, requires byte equality with the checked-in bundle,
and stages that one artifact for isolated `serve` and `worker` acceptance.

## Public binary release

[`v0.1.21`](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.21) is the current public and
supported release. Its annotated tag object
`fd6e87acf987d1d4722845e69d2108de4705ed49` selects exact release-source commit
`6380117363ca2c69d4bf84e512a57d03ce9ea74e`; GitHub reports release `382657212` as immutable,
latest, non-draft, and non-prerelease. The sole current target is
`x86_64-unknown-linux-musl`. The 18,462,944-byte executable is ELF64 x86-64 and has no ELF
interpreter, `DT_NEEDED` runtime library, or GLIBC symbol-version requirement.

| Public asset | Bytes | SHA-256 / GitHub asset digest |
|---|---:|---|
| `lkjscript-x86_64-unknown-linux-musl.tar.gz` | 8,293,246 | `a2b36ab0f856b75b5686ddbc42433a2bbd8ae98e8b5d52675825bd33721fea32` |
| `SHA256SUMS` | 109 | `aee7da6f5e7beb79632aea8947413c9e53ceeb0a654bf407cbeebe0ef308755e` |

Release workflow
[`33862293249`](https://github.com/lkjsxc/lkjscript/actions/runs/33862293249) passed all four jobs on
attempt 1. Tagged source passed 24/24 fresh full gates with zero reuse. Exact target admission
directly inspected static linkage, completed 12-command lifecycles in pinned Alpine 3.22.5/musl 1.2
and Debian 11/glibc 2.31 userlands without candidate network or host-library mounts, and passed
distributed HTTP, outbound HTTPS/TLS/DNS, stateful HTTP, and maintained `lkjournal` service
oracles. The service oracle binds service-receipt contract 9 and proves maintained exact route
ownership, structured interactive sessions, identity-preserving extraction, exact-requirement-bound
affine handoff, the resource-owned queue lifecycle, restart, unchanged semantic authority, and
cleanup. Bounded catalog health and missing/predecessor/incomplete recovery also pass without
promoting derived catalog state to authority.

A no-checkout job verified both handoffs and ran current distributed-receipt contract 4 plus
stateful and outbound application oracles before the write-isolated publication job. Anonymous
exact-tag and `releases/latest` downloads independently passed checksums, GitHub asset digests and
attestations, strict extraction, static inspection, and all three transferred application oracles.
Each independently proved clean/incremental equality, failure and cancellation behavior, unchanged
authority, redaction, and cleanup; exact and latest archive, checksum, candidate, and manifest bytes
were compared only after both behavioral runs. The release consolidates the completed 0.1.17
exact-requirement affine handoff, 0.1.18 identity-preserving `extract.function`, 0.1.19 incremental
catalog, 0.1.20 structured sessions, and 0.1.21 exact HTTP routes without adding graph meaning or
touching deployment state. Exact identities, classifications, resources, negatives, and raw-evidence
pointers are retained in
[`202609041625-v0.1.21-consolidated-release.json`](evidence/202609041625-v0.1.21-consolidated-release.json).

Immutable `v0.1.5` was the first publication attempt for this source generation. Its public bytes
passed independent checksum and command lifecycle verification, but its workflow's final smoke
step compared the binary against a stale pre-cutover discovery digest. The workflow now reads the
product version and capabilities digest from verified release metadata, and recovery used additive
`v0.1.6`; neither `v0.1.5` nor an older tag or asset was changed. The independent `v0.1.7`
publication likewise moved or replaced no predecessor. Release files remain derived distribution
evidence rather than program authority or independent build provenance. Exact evidence is retained
in the historical [v0.1.6 record](evidence/20260828-v0.1.6-public-release.json) and
[v0.1.7 public HTTP record](evidence/202608281451-public-http-release.json).

Immutable `v0.1.8`, release `378668969`, remains an unclosed historical recovery point. Its exact
and latest application oracles passed, but run `33196276783` rejected legitimately distinct
artifacts from separately allocated fresh applications. The workflow defect required changed
source, so recovery advanced additively to v0.1.9 without moving or replacing the v0.1.8 tag,
release, or assets. Its structured evidence remains in
[`202608290000-static-linux-distribution.json`](evidence/202608290000-static-linux-distribution.json).

## Current application lifecycle

Current product source is unreleased `0.1.24`; immutable public latest remains `v0.1.21` at the
exact release-input source commit named above. The source retains the exact-requirement-bound affine
handoff, identity-preserving extraction, incremental catalog, graph-owned structured sessions, and
adds signature-indexed inbound HTTP path captures, code-complete offline package composition,
and constant control space for eligible pure tail execution.
Graph 10, validation witness 6, owner summary 5,
validator 10, package 3, package interface 6, compiler-unit 5, bytecode 3, Artifact 15, resident
runtime 3, function-definition projection 2, service receipt 10, distributed-HTTP receipt 4,
outbound-HTTP receipt 2, stateful-HTTP receipt 6, compact change 14, authored change 11,
logical change plan 5, query 6,
project creation 5, registry 11, CLI 25, deployment 4, HTTP adapter 3, and structured-session 1 are
current. Object catalog 2, semantic-scale receipt 3, object-store 1, pack 1, stream 1, and
HTTP-client adapter 1 retain their independent owners.

The standard package meaning and derived identities let its maintained HTTP and Nostr consumers own
typed exact/pattern route sets. `lkjournal` owns eight exact and three pattern HTTP routes while
preserving its `serve`, worker `work`, interactive `lkjournal-live-1`, shared
first-party data authority, and stable worker split. Existing deployment, data, queue, object,
outbound HTTP, structured-session, and predecessor immutable release formats remain unchanged. The
consolidated release performed no deployment, hosted-data mutation, or live-relay contact.
Graph, package, artifact, deployment, distribution, and operational-data identities remain
separate. The source product retains the same closed
operation names:
`capabilities`, `new`, top-level `data`, `status`, `inspect`, `query`, `change`, exact offline
`package`, `check`, `build`, `run`, and artifact-runtime `serve` and `worker`.
All finite operations use deterministic bounded compact records. Discovery begins with the product
name and product version and reports capabilities digest
`3f938b8992e41959806205e3304fed2652e441c2e17ab7e795a12ed92020f9bd` in current source.

`CapabilityResource<ExactInterface>` values are accepted graph meaning acquired only by an
exact-requirement capability call. Operation parameters canonically distinguish unrestricted,
borrow, and consume. Validation follows language order, retains exact requirement provenance,
permits ordered borrows, one consume, and one exact direct handoff through a final consume parameter
bound to the same task requirement. It rejects fabrication, aliases, use after consume, foreign
authority, branch disagreement, unsupported or recursive function transfer, and aggregate or
durable escape before publication. Compiler units, artifact loading, normalized preparation, the
VM, and the canonical reference interpreter retain and independently recheck moves and bindings.
Resources are task-local, non-equal, non-serializable, droppable, and absent from literals,
constants, tests, caches, queue payloads, object bytes, and operational backups.

An `interactive` target has exactly one function-backed port structurally equal to
`(Option<State>, SessionEvent) -> SessionDecision<State>`. Every repeated `State` is the same
closed ordinary concrete type and cannot retain a stream, capability resource, function, secret,
unresolved parameter, static value, or other live authority. Accepted validation, compiler
lowering, package construction, Artifact 15 loading, and deployment preparation reconstruct that
relation independently. The canonical standard types encode open, message, tick, peer-close, and
shutdown events plus accept, continue, reject, close, and finish decisions; the runtime enforces
their state and phase pairs before installing any next state.

`serve` now selects either an exact HTTP target or an exact interactive target. One structured
parent owns each plaintext RFC 6455 connection, its reader, ordered transition driver, writer,
coalesced tick source, cancellation lineage, bounded byte-accounted mailboxes, state, streams,
permits, and joined children. Each callback remains a finite resident task under the one-hour task
ceiling, while separately bounded idle and total session lifetimes admit 24 hours. Complete output
capacity is reserved before a potentially effectful callback; a failed or invalid transition
installs neither state nor partial output. Public `run` remains pure-command-only, and no live
resource becomes a graph value or result.

Every inbound HTTP target now owns one nonempty finite set of stable exact-or-pattern route owners
and has no universal port. Exact selectors retain literal path bytes; patterns contain 1 through 64
nonempty literal or whole `{capture}` segments, at most 32 unique captures, and no wildcard, tail,
optional, mixed, or regex segment. Each target admits at most 4,096 routes, 4 MiB of aggregate key
bytes, and 65,536 stored pattern segments. Duplicate match languages and incomparable overlap
reject; exact routes and strictly more-specific literal refinements win without authored priority.
The route-set digest binds selector kind, canonical segments, capture order, and port identity.

A pattern's capture names index the backing function's same-named ordered unrestricted `Text`
parameter suffix after `HttpRequest`. Compilation, Artifact 15 loading, deployment preparation,
in-memory execution, and live HTTP independently bind the route, target, component, port, function,
parameters, HTTP types, and requirement closure. The prepared exact index and literal-first trie do
not scan or allocate the complete route set for an ordinary match. Captures preserve raw validated
segment spelling; matching performs no percent or Unicode decoding, normalization, slash collapse,
case folding, implicit method handling, or query selection. Valid unmatched pairs return a fixed
empty 404 without application headers, graph invocation, capability use, or a resident task.
HTTP stop receipts expose the prepared matcher size, resident task totals and peaks, exact
admission/worker permit peaks, zero post-shutdown tasks and permits, and bounded cleanup results;
these remain operational observations rather than program authority.

`lkjournal` preserves route identities `route_79ebd79a701800e82e13dbe560944d55`,
`route_8f26a5cc2dcb7f777ad6d2bbe426ee93`, and `route_a92b27a1757c26506bb0bbac601e9f42` while
selecting `GET /resource/{id}`, `GET /resource/{id}/history`, and
`POST /resource/{id}/update`. Their handlers now receive trailing `id: Text` parameters
`param_ec41b6e339d68388c9df7bbf7c6eac16`, `param_8959bcba1113d7e685ae2dd85802f819`, and
`param_ece943fd633a7a6c2976e6f50609eb21`; identifier syntax remains application
policy. The predecessor `/resource`, `/resource/history`, and `/resource/update` selection paths
and handler `id` query lookup are absent. Other HTTP, worker, interactive, data, queue, object, and
domain behavior remains unchanged.

`data initialize|verify|backup|restore` owns operational lifecycle for one local
`lkjscript-data-store-1` root and `lkjscript-data-backup-1` logical backup. Initialize and restore
publish only absent roots; backup publishes only an absent file; verify walks the complete retained
closure without mutation. Restore reproduces sorted logical schema/key/value/revision facts under a
new physical store identity. These commands never discover or advance semantic `HEAD` and expose no
query shell, repair, overwrite, SQL import, or deployment switch.

`new --template minimal` creates an empty package. Every semantic record in `command`, `http`, and
`nostr-relay-info` is executable-owned typed authored intent using only operations also represented
by the public compact grammar. Recipes and public changes share normalization, allocation,
preparation, logical planning, impact analysis, validation, selected tests, and publication
machinery; no recipe-specific owner or snapshot builder remains. `new` validates the complete
candidate privately and exposes one initial accepted revision in one destination rename.
`new --template command` creates an offline, standard-dependent pure command project with an
application module, implementation, component, port, `main` target, and graph-owned test.
`new --template http`
creates a 21-owner typed meaning graph application with one exact built-in
dependency, task HTTP handler, byte-stream requirement, HTTP port, `serve` target, and stable
`GET /` route owner and status-code test. It atomically includes `service.deployment.json` and an empty `generated/`
directory before the one destination visibility rename; no application artifact is prebuilt.

`new DEST --template nostr-relay-info [--name NAME] --relay-url URL` creates a closed typed
NIP-11 relay-information application. It has one exact `HttpClient.get` requirement plus the
existing inbound byte-stream requirement, one `serve` target, and one exact `GET /relay-info`
route owner. The route handler sends exactly `Accept: application/nostr+json`, preserves a bounded valid
status-200 document byte-for-byte, and maps remote status, media-type, and capability failures to a
local redacted 502. `wss` normalizes to `https`; `ws`/`http` is admitted only for lexical loopback
development. The strict descriptor owns the immutable normalized endpoint, address policy, TLS
trust, and separate request, response, DNS, concurrency, connection, total, and cleanup limits.

The starter descriptor remains separate operator authority. It names
`generated/application.lkja`, target `serve`, one exact byte-stream grant, conservative independent
resource limits, and listener `127.0.0.1:0`. Runtime readiness reports the actual bound loopback
address. The graph's editable response text is changed only through reviewed `change plan` and
`change apply`; build and resident execution do not open or advance project authority.

The closed HTTP recipe remains public behavior. Public `v0.1.21` rejects predecessor PostgreSQL
deployment input and uses first-party data. Immutable `v0.1.10` remains unchanged as the historical
PostgreSQL-backed predecessor generation. The higher-order slice includes the exact
`add.type-parameter`, `expression.function-value`, and `expression.invoke` vocabulary. Function
values are explicit monomorphic references to pure named functions; generic task functions,
lexical closures, partial application, inference, and maps remain unavailable. The current public compact
surface additionally exposes exact `add.dependency`, `create.component`, function-backed
`add.port`, conditionally portless `create.target`, `add.http-route`, and `set.http-route` records
alongside interface/external creation, operation parameters,
dependency replacement, and requirement rebinding. Exact dependency addition is confined to an
already-staged complete immutable source closure; it performs no registry or ambient lookup. The built-in standard exports
graph-owned generic
`list-fold-left<Item, State>`, and the source BBS authors its content-type predicate with a named
header reducer and that fold. Immutable predecessor releases remain unchanged.

Maintained `lkjournal-live-1` accepts only `/live` with a graph-validated bearer session. Its
ordinary state retains the actor and a bounded connection-local subscription map. Strict text JSON
arrays subscribe/replace or unsubscribe identifiers; subscribe returns actor-owned resource
summaries in deterministic graph order and an end marker, while coalesced ticks scan the existing
data authority for newly created or advanced revisions. Binary input and malformed, unknown, or
exhausted application input produce graph-selected errors or close. Independent connections do
not share subscription state, actor filtering remains graph policy, and reconnect reconstructs a
snapshot from durable data rather than restoring runtime cursors. HTTP and worker targets and their
shared data authority remain present.

Exact source-campaign identities, receipts, classifications, and resource observations are retained
in [`202608290721-public-higher-order-generic.json`](evidence/202608290721-public-higher-order-generic.json),
the topology cutover is retained in
[`202609010657-public-graph-topology-authoring.json`](evidence/202609010657-public-graph-topology-authoring.json),
and the current public distribution boundary is retained in
[`202609011758-affine-resource-release.json`](evidence/202609011758-affine-resource-release.json).

`package builtin query owners` and exact `package builtin inspect owner` expose the current public
standard declarations and interface operations with canonical compact references, full signatures,
effects, idempotency, visibility, and revision-bound bounded continuations. Deployment discovery is
generated from the strict descriptor inventory and includes every adapter field and range. Eight
executable-owned generated documents now cover operations, diagnostics, compact change grammar,
local-function definition projection, built-in public interface, deployment schema, and stateful
and Nostr relay-information composition walkthroughs.

`status`, exact owner `inspect`, and normalized owner/name/relation/context `query` read one accepted
revision. Context takes one live local owner, mandatory incoming/outgoing/both direction, and depth
1 through 8. It materializes the complete admitted breadth-first neighborhood, emits owners by
minimum depth and canonical key before canonical relation edges, and pages with a stateless
revision/selector/section/key-bound continuation. Fixed maxima are 4,096 selected local owners,
16,384 unique relations, and 32,768 relation-witness visits; map, store, decode, output, and
continuation bounds remain separate and executable-discovered. Package and foreign endpoints are
reported but not expanded. Success and every failure perform no repository write.

`inspect owner KIND ID --detail definition` succeeds only for one live local pure or task function
with a body. It materializes the complete accepted contract, structural body closure, exact
references, and bound validation facts before rendering deterministic header/contract/preorder-body/
reference/fact pages. Fixed logical maxima are 4,096 body records, 16,384 combined structural and
reference edges, 32,768 fact reads, depth 256, and 8 MiB canonical logical encoding; map, object,
decode, literal-fragment, output, and 320-byte continuation admissions remain independently
discovered. Stateless `icont_` tokens bind repository, package, revision, function, contract,
complete digest, ordering, section, and exclusive record key. Resume recomputes the complete
definition, and output is unknown `change` input.

The maintained `lkjournal` worker entry remains task function
`decl_a914bb78de075ff44a857ac028d704f3`. Its definition digest is
`definition_e5859f5a458aafa079c43186f737f7ad821da2bba2f499c199ef6623863bac61`:
84 logical records, comprising 9 contract, 15 body, 40 reference, and 18 fact records, with 18
structural edges, 40 reference edges, depth 3, and 33,389 logical bytes. It retains claim and
absent/live dispatch, then transfers the live lease exactly once into private task helper
`decl_7f443401f4946c55fa239c5430e8ad93` with no later entry use.

The helper digest is
`definition_4af7237d165ed2c255b89a74f35829593ef8cd21a87397556af5240ead2fb1d4`:
153 logical records, comprising 3 contract, 36 body, 73 reference, and 39 fact records, with 39
structural edges, 73 reference edges, depth 6, and 65,945 logical bytes. Its final consume lease
parameter is bound to exact requirement `req_0cebded5cb056cda5484e39aa40594ad` and owns processing,
`lease-info` borrow, heartbeat consume, renewed-state match, and complete/fail consume. The disjoint
complete-authority oracle agrees on both definitions, the call and parameter-requirement relations,
and absence of post-transfer entry use. Both are below the 40-body-record bound and the 48-record
predecessor.

The formerly largest maintained function `decl_0693166bd7c29bee83d2ead289148f65`
(`update-resource`) retains its identity and now has definition digest
`definition_cece2c65da440eaf44cb6aaf84454d6804df033dae63615d7986aa73598adb49`:
397 logical records, including 18 contract, 93 body, 175 reference, and 99 fact records, with depth
17 and 165,495 logical bytes. Its request is followed by the route-indexed unrestricted `id: Text`
parameter; the predecessor query lookup is absent. Its exact data-only subtree rooted at
`expr_22692186086bc39d6caf2cfe244879c8` retains all 101 movable owner identities under generated
private helper `decl_53936ef7d46ee491d41aef8c37cdffef` (`commit-resource-update`). The helper
captures unrestricted locals `actor`, `id`, `input`, and `entry` in first-use order, owns only the
exact `data` task requirement, and has digest
`definition_6251c53613cb22c91fb0260ef7469257ce527436b88165aa06228cb8d2ccc38c`:
425 logical records, including 5 contract, 101 body, 206 reference, and 106 fact records, with depth
15 and 181,807 logical bytes. The predecessor 192-body-record ownership shape is gone; the 96- and
101-record results are both below its 144-record feasibility threshold. The independently selected
largest maintained definition is now `decl_97e3d3c28142723096e5b121c0205ef2`, with 148 body records
and digest `definition_a0e1d237ade379087dcc5ca832fd67a7a84b15ff64da7eaed2c7a3910687a483`.

`change plan` and `change apply` share typed lowering and a reviewed logical-plan
commitment; apply is the only normal existing-project semantic writer. Exact idempotent apply
reconciliation reprepares the historical logical base without allowing append-only type-object
storage to alter reviewed effects. A stale plan request remains stale.

`extract.function` is the sole graph-native refactor operation. It selects one exact live
nongeneric local function, one proper unaliased structural expression root, and one absent private
same-module helper name. Planning independently bounds and reports the movable closure, free-local
captures and uses, inferred resource-free result, least effect and caller-ordered requirement
subset, optional final consume-only affine provenance, preserved/changed/generated owners, resulting
body counts, impact, tests, and prepared commitment. Apply rederives those facts under the
publication lock. Malformed, stale, conflicting, ambiguous, escaping, recursive, resource-unsafe,
foreign, or exhausted requests leave accepted and operational authority unchanged. There is no
automatic root selection, delete-and-recreate fallback, stored recipe, top-level alias, or
application-specific migrator.

`check`, `build`, and `run` share one normalized preparation path:

```text
project discovery -> exact RepositoryView -> dependency closure
                  -> exact-current cache or clean compilation
                  -> artifact bundle link and strict load
                  -> dense normalized program
```

`check` runs all graph tests through the normalized VM and canonical reference interpreter.
`build` writes an artifact bundle to an explicit absent path through synchronized create-new
publication. `run` accepts a pure command target, parses a bounded JSON argument array, executes
both tiers once, and rejects disagreement. These operations never advance semantic authority.

The compiler cache under `derived/compiler` is revision-bound disposable state. A valid exact
current cache is reused; absence performs a clean build. Corrupt cache state is reported as
`clean-recovery`, rebuilt, and never selects meaning. After accepted `change apply`, an exact base
cache may be updated incrementally using the in-memory prepared publication. Its outcome is
reported separately as `updated`, `not-available`, `not-attempted-replay`, or `failed`. Derived
failure cannot change the accepted semantic result.

Predecessor project markers receive `project_predecessor_authority` before cache or output work.
The removed project-scoped `draft`, `history`, predecessor package commands, `review`, `backup`,
`restore`, and `doctor` names are absent and fail with ordinary `cli_usage`; top-level
`data backup|restore` are distinct operational-data operations. The old workspace/repository
mutation stack and public check/build/run routing have been deleted. There is no legacy flag,
fallback reader, graph selector, converter, or dual write.

Contributor command `lkjscript-dev scale` now drives one current semantic-scale workflow through a
copied supported executable. It discovers the public operation grammar and capabilities, constructs
all retained topologies only through compact `change plan` and `change apply`, uses current
`status`, exact owner `inspect`, bounded `query`, `check`, and `build`, and writes one bounded
`lkjscript-semantic-scale-receipt` contract 3. A read-only typed `GraphRepository` inventory is its
formatter-independent semantic oracle. The receipt also binds catalog identity, layout, cumulative
delta/merge/rebuild work, the final healthy session, and an independently reconstructed footer
commitment. Neither oracle has an accepted-authority write path.

One fresh `small-functions` lifecycle admitted 100,100 live owners: 100 modules, 50,000 pure
functions, 50,000 expressions, and 100,000 relations. Fifty-one construction batches plus one
reviewed rename advanced accepted authority 52 times. All 115 copied-binary commands passed;
bounded reads observed the final revision; `check` compiled 50,000 units with differential equality;
and the forced-clean and exact-current builds produced equal 59,377,334-byte artifacts. The typed
oracle agreed with public counts and the campaign removed the temporary project.

One fresh `independent-modules` capacity lifecycle admitted exactly 1,000,000 live modules and no
relations through 1,000 separate 1,000-operation plan/apply batches and 1,000 accepted revisions.
All 2,008 copied-binary commands passed. Final status, exact inspection, bounded owner query, exact
name query, and bounded context query agreed with the typed semantic oracle. The healthy catalog
ended at seven segments and reported zero complete footer scans or full reconstructions across
construction and final reads; the implementation-disjoint footer oracle agreed on all 5,937,309
locations and the logical catalog commitment. The 6,189.686-second run used 19,497,158,083 bytes,
stayed within its declared host envelope, and removed the temporary project. This capacity-only
admission does not run or prove million-owner check, compilation, or build. Exact inputs, receipts,
classifications, observations, and limitations are retained in
[`202609031354-incremental-object-catalog.json`](evidence/202609031354-incremental-object-catalog.json).

## Runtime boundary

Normalized production and reference execution support pure commands and graph tests. Public `run`
deliberately rejects non-command runners and effectful command entry points. Public `serve` and
`worker` strictly load a standalone artifact bundle, prepare `NormalizedProgram`, resolve the
selected target and exact component requirement closure, and invoke the normalized resident VM.
Their descriptors supply configuration, named secrets, adapter selection, external coordinates,
and topology; deployment preparation never discovers or opens a project repository. Live effects
execute once through production rather than differential replay.

Configuration, secrets, clock, secure randomness, identifiers, password hashing, byte streams,
deployment-bound outbound HTTP, first-party ordered data, object storage, and the first-party durable queue all have exact normalized
capability bindings. `data` and `durable_queue_data` may share one confined local root through
separately validated grants and namespaces. Adapter preparation is all-or-nothing, and repeated
shutdown reuses one recorded cleanup outcome.

The current source standard `DurableQueue` exposes exactly nine operations. Claim and heartbeat
return nominal `QueueLeaseState` (`absent` or `live(CapabilityResource<DurableQueue>)`);
`lease-info` borrows and returns only job ID, attempt number, lease-until time, and payload;
heartbeat, complete, and fail consume. Private attempt and worker strings no longer cross the
graph/runtime adapter boundary. Handle capacity is reserved before claim or renewal effects and a
successful renewal commits a fresh handle. Foreign, closed, wrong-kind, wrong-interface,
wrong-requirement, exhausted, duplicate, and stale paths reject or return the exact typed queue
outcome without leaking a right. The physical queue-data and backup formats are unchanged.

Maintained `lkjournal` now claims and matches in its stable entry, then transfers a live lease once
to its private exact-requirement-bound helper. The helper borrows info for application policy,
consumes through heartbeat, matches the renewed state, and consumes through complete or fail. Its
fresh service oracle runs two workers and independently observes expired-lease replacement,
retry/fail, terminal completion, cleared raw transition fields, restart, failed readiness,
backup/absent-root restore, graceful cancellation, and zero task/cleanup leftovers while graph
authority remains byte-identical.

`HttpClient.get` receives only an ordered bounded header list and returns status, ordered headers,
and whole body bytes. One exact deployment grant owns the endpoint and transport policy. Public
destinations require HTTPS; explicit plaintext is loopback-only. Request-time bounded resolution
rejects the entire answer set if any address violates `public_only` or `loopback_only`, and the
client connects only to validated addresses. HTTPS verifies hostname, chain, and validity against
locked WebPKI roots or one named PEM-root secret. It follows no redirect, reads no proxy setting,
injects no credential, decompresses nothing, and automatically retries nothing.

Runtime call frames carry exact generic type substitutions so generic `json-decode-or<T>`,
`json-encode<T>`, `data-encode<T>`, `data-decode-or<T>`, `list-length<T>`, `list-get<T>`, and
`list-fold-left<Item, State>` standard
declarations execute against the concrete typed meaning graph runtime layout. Production and
canonical-reference implementations remain disjoint and agree for maintained pure behavior. The
stateful BBS uses explicit `DataStore`, identifier, wall-clock, and byte-stream requirements. It
stores each post once and atomically maintains a `(created-at, id)` index. `lkjournal` owns explicit
actor, session, resource, immutable-snapshot, object-metadata, lookup, and job spaces/indexes while
object bytes remain in object storage.

The inbound HTTP listener is plaintext, and the local data root is unencrypted trusted-host
storage. Outbound HTTPS authenticates only its exact deployment endpoint under the selected trust
mode; it is not a privacy, browser, DNSSEC, or sandbox boundary. The runtime is not a hostile-code
sandbox or multi-tenant isolation boundary.

## Current limits and unproved properties

- Unreleased source resolves exact immutable code-complete dependency closures, including ordinary
  non-built-in packages. There is no ambient filesystem lookup, mutable tag, network registry,
  package publication, or upgrade resolver. Public release behavior remains separately identified.
- Public `run` is pure-command-only. Arbitrary outbound URLs/methods, request bodies, outbound
  WebSocket clients, redirects, private-network destinations, proxies, client certificates,
  NIP-01 event models/signing, reconnect/backoff, and additional effect families remain absent.
- Source text, recursive referenced-declaration detail, dependency implementation detail queries,
  generic impact query, fuzzy search, multiple-root context, and historical query are absent.
  Definition detail is confined to one admitted live local function; code-complete transport
  separately exports the entire selected current canonical source closure, including private bodies.
- Removed draft/history/review/project-backup/project-restore/doctor workflows have not been
  reintroduced on typed meaning authority. Operational data backup/restore is separately public.
- Public authored change covers the executable-discovered subset; additional typed engine forms
  remain private until a complete public workflow exists. Exact staged code-complete dependencies,
  components, requirements, function-backed ports, and command/HTTP/interactive targets can be
  created. Existing
  function contracts can be changed, and one proper subtree of a nongeneric local function can be
  extracted into a private same-module helper with inferred captures and identity continuity. Pure
  functions support explicit type parameters, named values, invocation, interface/external creation,
  operation parameters, dependency replacement, and requirement rebinding. General move, rebind,
  signature editing, inline, expression-backed ports, dependency removal, general package
  resolution, and additional runner spellings remain private or absent.
- Affine resources are lexical and task-local. They may be dropped, may cross the direct payload of
  one nominal variant, and may move through one exact-requirement-bound final consume parameter on
  a private same-package acyclic task helper. Resource results, borrowed/multiple/nonfinal/public/
  cross-package/generic resource parameters, indirect calls, ports, closures, aggregates, partial
  moves, async tasks, general linear must-use values, and additional resource protocols are absent.
- Compact finite output is bounded to 4 MiB and 10,000 records. Query and function-definition
  inspection have independent item and byte budgets and distinct bounded revision-bound
  continuations. Exact limits and diagnostics are executable capability data.
- Artifact input/output, compiler units, graph traversal, expression depth, execution stack,
  instruction count, capability work, and adapter resources retain separate checked boundaries.
  These bounds are implementation admissions, not demonstrated scale ceilings.
- The first-party data format retains all reachable history and has no compaction, garbage
  collection, destructive repair, replication, consensus, encryption, remote service, or
  million-key admission. Its current support boundary is one local trusted Linux host.
- No million-owner check/compiler/build admission or complete application lifecycle, long-history
  retention policy, graph-store garbage collection, live-store packing, artifact signing,
  encrypted graph storage, or distributed publication protocol has been proved.
- The supported public binary is exactly `x86_64-unknown-linux-musl` with the static linkage and
  pinned Alpine 3.22.5 and Debian 11 userland observations above. A minimum kernel, macOS, Windows,
  arm64, generic Unix, additional Linux targets, and universal portability remain unproved.
- The immutable GitHub release attestation and asset digests prove release identity and integrity;
  they are not code signing, compiler reproducibility, build provenance across runner images, or
  a general supply-chain policy.
- Provider tokens, cached-token counts, requests, and monetary cost are unavailable because no
  direct telemetry exists.

## Verification

Eligible pure graph calls now replace their activation in production and in the independent
canonical reference trampoline. Copied-public runtime-list folds at 0, 1, 256, 4,096, and 8,192
items return the fixed sums; the last three retain peaks of two production and three reference
call frames. Self/mutual recursion, ordered generic instantiation across a removed producer, and
if/let/sequence/match tail contexts use this same mechanism. The unchanged maintained standard
artifact receives the guarantee through current strict preparation.

Bounded-stack probes retain actual continuation, locals, and substitution peaks, reject genuine
non-tail depth and fuel exhaustion, and release execution state after deterministic cancellation.
The standalone transactional HTTP fold commits once, hides a staged write on helper overflow and
cancellation, then succeeds again with the same grants. Empty-requirement task frames remain
ordinary frames. Default budgets, maintained semantic/package/artifact identities, and stored
encodings are unchanged. Exact campaign classifications and source/target boundaries are owned by
[`202609051930-pure-tail-execution.json`](evidence/202609051930-pure-tail-execution.json).

The code-complete offline-package campaign is complete at the copied executable, immutable source,
compiler/reference, maintained consumer, and exact musl candidate boundaries. Its diamond returns
11, stays 11 when D2 is staged, and returns 12 after one reviewed paired replacement. Producer
directories are removed before execution; independently reconstructed producer and transported
inventories agree. Public nominal types, rank-one generic calls, bounded staged-interface paging,
cache recovery, and standalone HTTP passed. That campaign's full verification had 25 fresh gates,
including every product gate; exact-target admission had eight fresh classifications, including the transferred
no-checkout package workflow. The first full run exposed and rejected a concurrent candidate-path
replacement; the corrected dependency ordering retains all gates and exact fingerprints.
Maintained semantic/package/transport and artifact identities above remain unchanged; only the
public source container and operational readiness encoding move. The bounded standard/lkjournal/
diamond admission preflight fits every fixed ceiling without a capacity claim. The terminal record
is assembled before its final fresh revalidation, so no subsequent evidence commit changes those
bound inputs. Current classifications, exact boundaries, deviations and raw pointers are retained in
[`202609050819-offline-packages.json`](evidence/202609050819-offline-packages.json).

Exact identities, oracle boundaries, gate classifications, resource observations, deletion, and
raw-evidence pointers for the current signature-indexed HTTP path-capture slice are owned by
[`202609050018-http-path-captures.json`](evidence/202609050018-http-path-captures.json).

The contributor-only `lkjscript-dev check` harness owns gate dependencies, exact input
fingerprints, fresh/reused classification, bounded child logs, required outputs, and receipts.
The identity-preserving extraction closure uses a public copied-binary plan/apply/reinspect path,
an independently materialized closure/capture/effect/resource oracle, exact moved-owner comparison,
pure production/reference equality, maintained clean/incremental and service proof, and exact static
target admission. Its identities, negative and interruption matrix, gate classifications, resource
observations, cleanup, and raw pointers are owned by
[`202609022319-identity-preserving-function-extraction.json`](evidence/202609022319-identity-preserving-function-extraction.json).
The requirement-bound handoff closure uses public copied-binary plan/apply/inspection, a disjoint
affine call-graph oracle, strict Artifact 12 preparation, the split maintained definition oracle,
and live service/queue-state observation. Its exact identities, negative matrix, gate
classifications, cleanup, and raw pointers are owned by
[`202609021736-affine-task-handoff.json`](evidence/202609021736-affine-task-handoff.json).
The source-only function-definition closure adds a complete-authority oracle disjoint from
production traversal/order/rendering/paging/tokens, copied-candidate maintained worker and largest-
function projection, and a fresh HTTP discover/project/plan/apply/reinspect/check/build/serve
workflow. Its exact definition digests, admissions, negatives, unchanged maintained identities,
gate classifications, target observation, cleanup, and raw-evidence pointers are owned by
[`202609021013-function-definition-projection.json`](evidence/202609021013-function-definition-projection.json).
The current affine queue source closure, including independent semantic-flow, copied-binary
authoring, maintained asset, two-worker service, predecessor-rejection, and exact static-target
observations, is bound by
[`202609011246-affine-durable-queue-lease.json`](evidence/202609011246-affine-durable-queue-lease.json).
Its immutable product publication, exact service-to-public candidate chain, hosted and independently
repeated anonymous exact/latest acceptance, and external-state closure are bound by
[`202609011758-affine-resource-release.json`](evidence/202609011758-affine-resource-release.json).
The current function-definition distribution closure, distributed-receipt v3 cutover, immutable
publication, and hosted plus independently repeated exact/latest projection proof are bound by
[`202609021420-function-definition-release.json`](evidence/202609021420-function-definition-release.json).
The semantic lifecycle campaign record [202608270014](campaigns/202608270014.md) names its exact
focused, product, full, and service results. The public binary campaign record
[202608271521](campaigns/202608271521.md) and its
[structured release evidence](evidence/202608271521-public-binary-release.json) bind the live
tag, workflow, assets, anonymous downloads, and hosted full verification. Lifecycle measurements
remain in
[evidence/202608270014-normalized-command-lifecycle.json](evidence/202608270014-normalized-command-lifecycle.json);
the artifact bundle resident cutover is bound by
[evidence/202608272159-artifact10-service-cutover.json](evidence/202608272159-artifact10-service-cutover.json).
The current-main distributed HTTP closure is bound by campaign
[202608281025](campaigns/202608281025.md) and
[its structured evidence](evidence/202608281025-distributed-http-application.json). Its independent,
no-Docker `distributed_http_application` gate is required by product, service, and full profiles
and copies one candidate executable into a fresh root outside the checkout. These campaigns are
summarized in [performance.md](performance.md).

Campaign [202608281817](campaigns/202608281817.md) introduced the distinct non-cacheable
`stateful_http_application` gate. Campaign [202608300840](campaigns/202608300840.md) advanced its
receipt schema to bind the first-party data authority. The current gate copies the candidate
outside the checkout, creates `minimal`, discovers and stages the exact built-in transport through
that copy, and authors the dependency plus complete component/requirement/function-backed-port/
HTTP-target topology and bounded indexed BBS in one reviewed public request. It requires deterministic
clean/incremental artifact bytes, and exercises create/list/update/delete ordering, stale
expectations, strict input, transaction rollback, schema divergence, failed startup, shutdown,
restart, logical backup, absent-root restore, and byte-identical graph authority without a database
server or container.

The distinct non-cacheable `outbound_http_application` gate is required by product, service, full,
target admission, and future release verification. It copies one candidate outside the checkout,
creates the closed recipe, proves status/query/check and clean/exact-current artifact equality, and
runs it against an implementation-disjoint local raw HTTP/1.1/TLS relay. Its cases cover exact
HTTPS and loopback HTTP, mixed/forbidden address admission, trusted/untrusted/expired/mismatched
certificates, no redirect, forbidden headers, non-200/wrong media type, header/body limits, timeout,
inbound cancellation, malformed protocol, failed startup, recovery, restart, active shutdown, and
complete authority/resource cleanup. It uses no live relay and retains no private key or secret
value.

The maintained `lkjournal` service/worker gate shares an isolated data root, freshly rebuilds the
checked artifact byte-for-byte, and exercises login, actor isolation, resource/history, object
publication/reconciliation, durable claim/completion/stale attempts, restart, failed startup,
backup/restore, and cleanup. PostgreSQL 16.15 remains only in contributor command `data-oracle`,
which compares 416 deterministic neutral facts, public receipts, and three post-warm-up resource
samples for each workload. Exact identities, medians, gate classifications, and raw pointers are
owned by
[`202608300840-first-party-data-engine.json`](evidence/202608300840-first-party-data-engine.json).
