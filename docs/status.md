# Current status

Status date: 2026-08-31 UTC. This file describes implemented checkout reality. Executable-derived
public guides live under [generated](generated), including the
[operation guide](generated/operations.md); this file does not duplicate them.

## Current authority and maintained consumers

The typed meaning graph is the sole current editable program authority. A project root contains a strict
`GraphRepository`: `HEAD`, immutable packs, an object catalog, optional exact package transports,
and private staging/locking state. Accepted meaning is the exact revision and immutable object
closure selected by `HEAD`. No maintained project contains a predecessor `.lkjscript` store.

| Consumer | Exact current identity |
|---|---|
| standard package | repository `repo_c1358d64c351873b51c954b69d1ac988`; package `pkg_10000000000000000000000000000001`; revision `rev_805f3ec96b7907e8ad6ac63d53bdab63aee81d671a39b4ff6cd35604f97bd44f`; state `semantic_state_7d089c26bd03b9bfd72638895bc78baa77f3dd74e100efdfd18b4a731942f281` |
| `lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; package `pkg_20000000000000000000000000000001`; revision `rev_1eb8825f79e542273c7df08e98720878b503f51c3cdbd91f5954d081f07aa203`; state `semantic_state_e2e3834a0a3f0f415f01eb6dc8ee9c8ff0e0c6e60725e51ad68202a9c576fb2a` |
| built-in standard dependency | package revision `package_revision_48cfe9aceb123da0d8a6fc30b034e89599cdc8235cf6b72085d2225b281cefc9`; transport `package_transport_629e6035e974334ecb79431860bb0678d4946aaf67817baa53f5d15d89a816d6`; artifact manifest `artifact_manifest_025419b2248bacf0c01cbc9ad6928aecc4aed4364c5dc7262814e19f361f42e8` |

The standard package owns 427 live semantic owners, 81 compiler units, and 13 graph tests. Its
current artifact has 808 closure objects and 253,276 bytes. `lkjournal` owns 1,559 live semantic
owners and one exact standard dependency; its two-package artifact has 141 compiler units, 2,763
closure objects, and 822,515 bytes. Its complete dependency closure runs 18 graph tests. Both
consumers currently pass production/reference equality.

Maintained derived assets are:

| Path | Role | SHA-256 |
|---|---|---|
| `packages/standard/generated/standard.lkjp` | exact built-in package transport, 88,618 bytes | `b863a95f62cf05fc8a9d6abe9c728c987c0d9b88b3d9d3d98cfda117a9140350` |
| `packages/standard/generated/standard.lkja` | current standard artifact bundle, 253,276 bytes | `1cfeb8545e045e86682950c1ba7fa2b63346ba349e4ca70d39645dce6a78a1a7` |
| `applications/lkjournal/generated/lkjournal.lkja` | current application artifact bundle, 822,515 bytes | `1e55c2053261b5a3d15e6e9d38c8de94cea6f263bd92658521f3fc57c70f6b8f` |

The built-in transport and artifact are compiled into the executable and strictly cross-checked.
Product verification regenerates maintained owners and compares exact bytes. Service verification
also performs a fresh public `lkjournal` build, requires byte equality with the checked-in bundle,
and stages that one artifact for isolated `serve` and `worker` acceptance.

## Public binary release

[`v0.1.12`](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.12) is the current public and
supported release. Its annotated tag object
`d6d81e2398ced78023a8ea62d69e03f6b9f5d4da` selects source commit
`8a0141a151a87fe59ccc1ebc738a7e5dd51c6882`; GitHub reports release `379320112` as immutable,
latest, non-draft, and non-prerelease. The sole current target is
`x86_64-unknown-linux-musl`. The 15,767,776-byte executable is ELF64 x86-64 and has no ELF
interpreter, `DT_NEEDED` runtime library, or GLIBC symbol-version requirement.

| Public asset | Bytes | SHA-256 / GitHub asset digest |
|---|---:|---|
| `lkjscript-x86_64-unknown-linux-musl.tar.gz` | 7,113,264 | `96d63b4cdcd598258635e4391c8e596b39f168c46d22fb4b380f0a3bff30efb4` |
| `SHA256SUMS` | 109 | `a1b83b4e8e41dca07ce11713ddb7b7ba0b07866ada1b186cd2b8fbe392971091` |

Release workflow
[`33318722126`](https://github.com/lkjsxc/lkjscript/actions/runs/33318722126) passed all four jobs on
attempt 1. Tagged source passed 23/23 fresh full gates with zero reuse. Exact target admission
directly inspected static linkage, completed 12-command lifecycles in pinned Alpine 3.22.5/musl 1.2
and Debian 11/glibc 2.31 userlands without candidate network or host-library mounts, and passed the
distributed HTTP, stateful HTTP, and maintained `lkjournal` service oracles. A no-checkout job then
verified the handoffs and passed both transferred application oracles before the write-isolated
publication job ran. Anonymous exact-tag and `releases/latest` downloads independently passed
checksum, attestation, strict extraction, static inspection, distributed HTTP, and stateful HTTP.
Exact and latest candidate and manifest bytes agree; each fresh BBS independently proved its own
clean/incremental artifact equality, persistence, rollback, failure behavior, authority equality,
redaction, and cleanup.

The public binary contains the explicit higher-order generic authoring, single-product-version
surface, bounded context traversal, and complete first-party ordered-data/durable-queue cutovers.
Exact identities, classifications, resource observations, and raw-evidence pointers are retained in
[`202608302224-public-product-release.json`](evidence/202608302224-public-product-release.json).

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

Current source owns product snapshot 0.1.13 and the deployment-bound outbound HTTPS/NIP-11 slice.
Immutable public latest remains 0.1.12 and does not contain that slice; no 0.1.13 tag, release,
asset, workflow dispatch, or deployment exists. Source advances CLI 15, project creation 3,
deployment 3, registry 4, and the separately owned `http_client_adapter` 1 because their observable
contracts changed. Compact-change and unrelated graph, data, queue, object, artifact, compiler, and
runtime contract numbers remain independently owned. Graph, package, artifact, deployment, and
operational-data identities remain separate.
The source and distributed executable expose exactly
`capabilities`, `new`, top-level `data`, `status`, `inspect`, `query`, `change`, normalized built-in
`package`, `check`, `build`, `run`, and artifact-runtime `serve` and `worker`.
All finite operations use deterministic bounded compact records. Discovery begins with the product
name and product version and reports capabilities digest
`17ea689fc7e54fbb9f78c42882c739d1bcd36eb4f4153882ccf7d0aa9dd2402d` computed by the executable.

`data initialize|verify|backup|restore` owns operational lifecycle for one local
`lkjscript-data-store-1` root and `lkjscript-data-backup-1` logical backup. Initialize and restore
publish only absent roots; backup publishes only an absent file; verify walks the complete retained
closure without mutation. Restore reproduces sorted logical schema/key/value/revision facts under a
new physical store identity. These commands never discover or advance semantic `HEAD` and expose no
query shell, repair, overwrite, SQL import, or deployment switch.

`new --template minimal` creates an empty package. `new --template command` creates an offline,
standard-dependent pure command project with an application module, implementation, component,
port, `main` target, and graph-owned test. Both recipes are typed executable-owned construction
and publish through the same atomic typed meaning graph creation boundary. `new --template http`
creates a 20-owner typed meaning graph application with one exact built-in
dependency, task HTTP handler, byte-stream requirement, HTTP port, `serve` target, and stable
status-code test. It atomically includes `service.deployment.json` and an empty `generated/`
directory before the one destination visibility rename; no application artifact is prebuilt.

`new DEST --template nostr-relay-info [--name NAME] --relay-url URL` creates a closed typed
NIP-11 relay-information application. It has one exact `HttpClient.get` requirement plus the
existing inbound byte-stream requirement, one `serve` target, and two graph-owned tests. The graph
exposes `GET /relay-info`, sends exactly `Accept: application/nostr+json`, preserves a bounded valid
status-200 document byte-for-byte, and maps remote status, media-type, and capability failures to a
local redacted 502. `wss` normalizes to `https`; `ws`/`http` is admitted only for lexical loopback
development. The strict descriptor owns the immutable normalized endpoint, address policy, TLS
trust, and separate request, response, DNS, concurrency, connection, total, and cleanup limits.

The starter descriptor remains separate operator authority. It names
`generated/application.lkja`, target `serve`, one exact byte-stream grant, conservative independent
resource limits, and listener `127.0.0.1:0`. Runtime readiness reports the actual bound loopback
address. The graph's editable response text is changed only through reviewed `change plan` and
`change apply`; build and resident execution do not open or advance project authority.

The closed HTTP recipe remains public behavior. Public `v0.1.12` rejects predecessor PostgreSQL
deployment input and uses first-party data. Immutable `v0.1.10` remains unchanged as the historical
PostgreSQL-backed predecessor generation. The higher-order slice includes the exact
`add.type-parameter`, `expression.function-value`, and `expression.invoke` vocabulary. Function
values are explicit monomorphic references to pure named functions; generic task functions,
capture, partial application, inference, maps, and arbitrary component, port, or target creation
remain unavailable. The current compact-change surface additionally exposes exact
interface/external creation, operation parameters, dependency replacement, and requirement
rebinding required for reviewed dependency-closed cutovers. The built-in standard exports
graph-owned generic
`list-fold-left<Item, State>`, and the source BBS authors its content-type predicate with a named
header reducer and that fold. Immutable predecessor releases remain unchanged.

Exact source-campaign identities, receipts, classifications, and resource observations are retained
in [`202608290721-public-higher-order-generic.json`](evidence/202608290721-public-higher-order-generic.json),
and the current public distribution boundary is retained in
[`202608302224-public-product-release.json`](evidence/202608302224-public-product-release.json).

`package builtin query owners` and exact `package builtin inspect owner` expose the current public
standard declarations and interface operations with canonical compact references, full signatures,
effects, idempotency, visibility, and revision-bound bounded continuations. Deployment discovery is
generated from the strict descriptor inventory and includes every adapter field and range. Seven
executable-owned generated documents now cover operations, diagnostics, compact change
grammar, built-in public interface, deployment schema, and stateful and Nostr relay-information
composition walkthroughs.

`status`, exact owner `inspect`, and normalized owner/name/relation/context `query` read one accepted
revision. Context takes one live local owner, mandatory incoming/outgoing/both direction, and depth
1 through 8. It materializes the complete admitted breadth-first neighborhood, emits owners by
minimum depth and canonical key before canonical relation edges, and pages with a stateless
revision/selector/section/key-bound continuation. Fixed maxima are 4,096 selected local owners,
16,384 unique relations, and 32,768 relation-witness visits; map, store, decode, output, and
continuation bounds remain separate and executable-discovered. Package and foreign endpoints are
reported but not expanded. Success and every failure perform no repository write.

`change plan` and `change apply` share typed lowering and a reviewed logical-plan
commitment; apply is the only normal existing-project semantic writer. Exact idempotent apply
reconciliation reprepares the historical logical base without allowing append-only type-object
storage to alter reviewed effects. A stale plan request remains stale.

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
The removed project-scoped `draft`, `history`, general package staging, `review`, `backup`,
`restore`, and `doctor` names are absent and fail with ordinary `cli_usage`; top-level
`data backup|restore` are distinct operational-data operations. The old workspace/repository
mutation stack and public check/build/run routing have been deleted. There is no legacy flag,
fallback reader, graph selector, converter, or dual write.

Contributor command `lkjscript-dev scale` now drives one current semantic-scale workflow through a
copied supported executable. It discovers the public operation grammar and capabilities, constructs
all retained topologies only through compact `change plan` and `change apply`, uses current
`status`, exact owner `inspect`, bounded `query`, `check`, and `build`, and writes one bounded
`lkjscript-semantic-scale-receipt` contract 2. A read-only typed `GraphRepository` inventory is its
formatter-independent oracle; it has no accepted-authority write path.

One fresh `small-functions` lifecycle admitted 100,100 live owners: 100 modules, 50,000 pure
functions, 50,000 expressions, and 100,000 relations. Fifty-one construction batches plus one
reviewed rename advanced accepted authority 52 times. All 115 copied-binary commands passed;
bounded reads observed the final revision; `check` compiled 50,000 units with differential equality;
and the forced-clean and exact-current builds produced equal 59,377,334-byte artifacts. The typed
oracle agreed with public counts and the campaign removed the temporary project.

The separately bounded one-million-module capacity attempt is classified `environment_limit`, not
admission. Its one-hour allocation completed 275 reviewed 1,000-module batches. The partial typed
oracle found exactly 275,000 live modules at the last accepted revision, after which the next apply
timed out without a reported accepted update; the temporary project was removed. Exact inputs,
receipts, classifications, observations, and limitations are retained in
[`202608311331-current-semantic-scale.json`](evidence/202608311331-current-semantic-scale.json).

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

- The only released dependency resolver accepts the exact built-in standard package. There is no
  general package manager, ambient filesystem lookup, mutable tag, network registry, or upgrade
  resolver.
- Public `run` is pure-command-only. Arbitrary outbound URLs/methods, request bodies, streaming,
  redirects, private-network destinations, proxies, client certificates, WebSocket, NIP-01 event
  models/signing, and additional effect families remain absent.
- Full owner-body projection, generic impact query, fuzzy search, multiple-root context, and
  historical query are absent.
- Removed draft/history/review/project-backup/project-restore/doctor workflows have not been
  reintroduced on typed meaning authority. Operational data backup/restore is separately public.
- Public authored change covers the executable-discovered subset; additional typed engine forms
  remain private until a complete public workflow exists. Existing component requirements can be
  extended, existing function contracts changed, and pure functions given explicit type parameters,
  named values, invocation, interface/external creation, operation parameters, dependency
  replacement, and requirement rebinding, but generic component, port, and target creation remains
  private.
- Compact finite output is bounded to 4 MiB and 10,000 records. Query has independent item and byte
  budgets and a bounded revision-bound continuation. Exact limits and diagnostics are executable
  registry data.
- Artifact input/output, compiler units, graph traversal, expression depth, execution stack,
  instruction count, capability work, and adapter resources retain separate checked boundaries.
  These bounds are implementation admissions, not demonstrated scale ceilings.
- The first-party data format retains all reachable history and has no compaction, garbage
  collection, destructive repair, replication, consensus, encryption, remote service, or
  million-key admission. Its current support boundary is one local trusted Linux host.
- No million-owner admission or complete application lifecycle, long-history retention policy, graph-store
  garbage collection, live-store packing, artifact signing, encrypted graph storage, or distributed
  publication protocol has been proved.
- The supported public binary is exactly `x86_64-unknown-linux-musl` with the static linkage and
  pinned Alpine 3.22.5 and Debian 11 userland observations above. A minimum kernel, macOS, Windows,
  arm64, generic Unix, additional Linux targets, and universal portability remain unproved.
- The immutable GitHub release attestation and asset digests prove release identity and integrity;
  they are not code signing, compiler reproducibility, build provenance across runner images, or
  a general supply-chain policy.
- Provider tokens, cached-token counts, requests, and monetary cost are unavailable because no
  direct telemetry exists.

## Verification

The contributor-only `lkjscript-dev check` harness owns gate dependencies, exact input
fingerprints, fresh/reused classification, bounded child logs, required outputs, and receipts.
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
`stateful_http_application` gate. Campaign [202608300840](campaigns/202608300840.md) advances its
receipt schema to bind the first-party data authority. The gate copies the candidate
outside the checkout, discovers current grammar/interfaces/deployment fields through that copy,
authors a bounded indexed BBS only through reviewed public changes, requires deterministic
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
