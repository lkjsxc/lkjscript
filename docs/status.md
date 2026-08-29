# Current status

Status date: 2026-08-29 UTC. This file describes implemented checkout reality. Executable-derived
contract and operation catalogs live in [generated/contracts.md](generated/contracts.md) and
[generated/operations.md](generated/operations.md); this file does not duplicate them.

## Current authority and maintained consumers

Graph 5 is the sole current editable program authority. A project root contains a strict
`GraphRepository`: `HEAD`, immutable packs, an object catalog, optional exact package transports,
and private staging/locking state. Accepted meaning is the exact revision and immutable object
closure selected by `HEAD`. No maintained project contains a predecessor `.lkjscript` store.

| Consumer | Exact current identity |
|---|---|
| standard package | repository `repo_c1358d64c351873b51c954b69d1ac988`; package `pkg_10000000000000000000000000000001`; revision `rev_b7e85425b4d2a15c6e7cbdc2c9128addeaebf24b9cb3dd626f2570ba47da23ee`; state `semantic_state_7590ee186644db29e411c9f919dbbde32daee9382a68f39d7270ff791632c93f` |
| `lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; package `pkg_20000000000000000000000000000001`; revision `rev_5b177805d9e9f6bc81cfdc7d1877d7a9b3d108f93a0bce1594f51b25c13009cf`; state `semantic_state_09c563120fba16b2c47ba7c9fc3d30d50ac107d24ca87ae6b1a7c09d8e779479` |
| built-in standard dependency | package revision `package_revision_b133c038d2997b440d5a6ec3fe9ec326e6c7c2c75259be7499aa234313bd6515`; transport `package_transport_9326e2744a3bfe401ef03750c162d32c1e3d4151a9b384fdd8fb28261601464a`; artifact manifest `artifact_manifest_48e18403aec9c5c74db8c4a0d75633cbe4f38648218c2e58fe5d7d3d1ca267a0` |

The standard package owns 381 semantic owners, 72 compiler units, and 11 graph tests. Its current
artifact has 716 closure objects and 224,984 bytes. `lkjournal` owns 1,313 semantic owners and one
exact standard dependency; its two-package artifact has 132 compiler units, 2,424 closure objects,
and 728,187 bytes. Its complete dependency closure runs 16 graph tests. Both consumers currently
pass production/reference equality.

Maintained derived assets are:

| Path | Role | SHA-256 |
|---|---|---|
| `packages/standard/generated/standard.lkjp` | exact built-in package transport, 77,273 bytes | `b5514baecae1276b8bfc5e551859e0ed351ff8e29a4fcddb66b76ddf5f23479c` |
| `packages/standard/generated/standard.lkja` | current artifact-10 standard bundle, 224,984 bytes | `e30f5c00166bb4b808e5e6557d5043faba492d44818a18e7a53d5113e9366485` |
| `applications/lkjournal/generated/lkjournal.lkja` | current artifact-10 application bundle, 728,187 bytes | `d28232523c319c8bf09d6cb3f54643b0ddd2aaf02d59acf08d741de86093a6cf` |

The built-in transport and artifact are compiled into the executable and strictly cross-checked.
Product verification regenerates maintained owners and compares exact bytes. Service verification
also performs a fresh public `lkjournal` build, requires byte equality with the checked-in bundle,
and stages that one artifact for isolated `serve` and `worker` acceptance.

## Public binary release

[`v0.1.9`](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.9) is the current public and
supported release. Its annotated tag object
`caa78d6851c0f3cfc3d1dfeff78c122ce4780827` selects source commit
`facebf51714b5a604434f49759e315ece0b4c218`; GitHub reports release `378702473` as immutable,
latest, non-draft, and non-prerelease. The sole current target is
`x86_64-unknown-linux-musl`. The 15,923,424-byte executable is ELF64 x86-64 and has no ELF
interpreter, `DT_NEEDED` runtime library, or GLIBC symbol-version requirement.

| Public asset | Bytes | SHA-256 / GitHub asset digest |
|---|---:|---|
| `lkjscript-x86_64-unknown-linux-musl.tar.gz` | 7,197,143 | `eeba53b4fe1fee8af7427dfc147f7a5be4b45bf8b7aff804b2d527132f89f4a0` |
| `SHA256SUMS` | 109 | `ac9f4cbeee75c4d90eb93eccbe0bf71c815a6c0cd07e2a8fe8b1f12e84c776c4` |

Release workflow
[`33200478399`](https://github.com/lkjsxc/lkjscript/actions/runs/33200478399) passed all four jobs on
attempt 1. Tagged source passed 22/22 fresh full gates with zero reuse. Exact target admission
directly inspected static linkage, completed 12-command lifecycles in pinned Alpine 3.22.5/musl 1.2
and Debian 11/glibc 2.31 userlands without candidate network or host-library mounts, and passed the
distributed HTTP, stateful HTTP, and maintained `lkjournal` service oracles. A no-checkout job then
verified the handoffs and passed both transferred application oracles before the write-isolated
publication job ran. Anonymous exact-tag and `releases/latest` downloads independently passed
checksum, attestation, strict extraction, static inspection, distributed HTTP, and stateful HTTP.
Exact and latest candidate and manifest bytes agree; each fresh BBS independently proved its own
clean/incremental artifact equality, persistence, rollback, failure behavior, authority equality,
redaction, and cleanup.

Immutable `v0.1.5` was the first publication attempt for this source generation. Its public bytes
passed independent checksum and command lifecycle verification, but its workflow's final smoke
step compared the binary against a stale pre-cutover registry digest. The workflow now reads the
CLI contract and registry digest from the verified release manifest, and recovery used additive
`v0.1.6`; neither `v0.1.5` nor an older tag or asset was changed. The independent `v0.1.7`
publication likewise moved or replaced no predecessor. Release files remain derived distribution
evidence rather than program authority or independent build provenance. Exact evidence is retained
in the historical [v0.1.6 record](evidence/20260828-v0.1.6-public-release.json) and
[v0.1.7 public HTTP record](evidence/202608281451-public-http-release.json).

Immutable `v0.1.8`, release `378668969`, remains an unclosed historical recovery point. Its exact
and latest application oracles passed, but run `33196276783` rejected legitimately distinct
artifacts from separately allocated fresh applications. The workflow defect required changed
source, so recovery advanced additively to v0.1.9 without moving or replacing the v0.1.8 tag,
release, or assets. Current structured evidence is in
[`202608290000-static-linux-distribution.json`](evidence/202608290000-static-linux-distribution.json).

## Current application lifecycle

Current source has the unreleased product version 0.1.10; immutable public v0.1.9 remains the latest
published binary and does not contain the source-only higher-order cutover. Both retain independent
CLI contract 12. The source executable exposes exactly
`capabilities`, `new`, `status`, `inspect`, `query`, `change`, normalized built-in `package`,
`check`, `build`, `run`, and artifact-runtime `serve` and `worker`.
All finite operations use deterministic bounded compact records. The registry digest is
`9da55649bdf2f20969c8b2633792a7e3ac7782d1198f407a84cb22832283eb41` for the current generated
content.

`new --template minimal` creates an empty package. `new --template command` creates an offline,
standard-dependent pure command project with an application module, implementation, component,
port, `main` target, and graph-owned test. Both recipes are typed executable-owned construction
and publish through the same atomic Graph 5 creation boundary. Project-creation contract 2 adds
`new --template http`, which creates a 20-owner Graph 5 application with one exact built-in
dependency, task HTTP handler, byte-stream requirement, HTTP port, `serve` target, and stable
status-code test. It atomically includes `service.deployment.json` and an empty `generated/`
directory before the one destination visibility rename; no application artifact is prebuilt.

The starter descriptor remains separate operator authority. It names
`generated/application.lkja`, target `serve`, one exact byte-stream grant, conservative independent
resource limits, and listener `127.0.0.1:0`. Runtime readiness reports the actual bound loopback
address. The graph's editable response text is changed only through reviewed `change plan` and
`change apply`; build and resident execution do not open or advance project authority.

The closed HTTP and predecessor stateful recipes are public `v0.1.9` behavior. This source line
advances only compact change contract 6: it retains the task/capability slice and adds the exact
`add.type-parameter`, `expression.function-value`, and `expression.invoke` vocabulary. Function
values are explicit monomorphic references to pure named functions; generic task functions,
capture, partial application, inference, maps, and arbitrary component, interface, external, port,
or target creation remain unavailable. The built-in standard exports Graph-owned generic
`list-fold-left<Item, State>`, and the source BBS authors its content-type predicate with a named
header reducer and that fold. Public v0.1.9 remains unchanged.

Exact source-campaign identities, receipts, classifications, and resource observations are retained
in [`202608290721-public-higher-order-generic.json`](evidence/202608290721-public-higher-order-generic.json).

`package builtin query owners` and exact `package builtin inspect owner` expose the current public
standard declarations and interface operations with canonical compact references, full signatures,
effects, idempotency, visibility, and revision-bound bounded continuations. Deployment discovery is
generated from the strict descriptor inventory and includes every adapter field and range. Seven
executable-owned generated documents now cover contracts, operations, diagnostics, compact change
grammar, built-in public interface, deployment schema, and a stateful HTTP composition walkthrough.

`status`, exact owner `inspect`, and normalized owner/name/relation `query` read one accepted
revision. `change plan` and `change apply` share typed lowering and a reviewed logical-plan
commitment; apply is the only normal existing-project semantic writer. Exact idempotent apply
reconciliation reprepares the historical logical base without allowing append-only type-object
storage to alter reviewed effects. A stale plan request remains stale.

`check`, `build`, and `run` share one normalized preparation path:

```text
project discovery -> exact RepositoryView -> dependency closure
                  -> exact-current cache or clean compilation
                  -> artifact-10 link and strict load
                  -> dense normalized program
```

`check` runs all graph tests through the normalized VM and canonical reference interpreter.
`build` writes artifact contract 10 to an explicit absent path through synchronized create-new
publication. `run` accepts a pure command target, parses a bounded JSON argument array, executes
both tiers once, and rejects disagreement. These operations never advance semantic authority.

The compiler cache under `derived/compiler` is revision-bound disposable state. A valid exact
current cache is reused; absence performs a clean build. Corrupt cache state is reported as
`clean-recovery`, rebuilt, and never selects meaning. After accepted `change apply`, an exact base
cache may be updated incrementally using the in-memory prepared publication. Its outcome is
reported separately as `updated`, `not-available`, `not-attempted-replay`, or `failed`. Derived
failure cannot change the accepted semantic result.

Predecessor project markers receive `project_predecessor_authority` before cache or output work.
The removed `draft`, `history`, general package staging, `review`, `backup`, `restore`, and `doctor`
names are absent from discovery and fail with ordinary `cli_usage`. The old workspace/repository
mutation stack and public check/build/run routing have been deleted. There is no legacy flag,
fallback reader, graph selector, converter, or dual write.

## Runtime boundary

Normalized production and reference execution support pure commands and graph tests. Public `run`
deliberately rejects non-command runners and effectful command entry points. Public `serve` and
`worker` strictly load a standalone artifact-10 bundle, prepare `NormalizedProgram`, resolve the
selected target and exact component requirement closure, and invoke the normalized resident VM.
Their descriptors supply configuration, named secrets, adapter selection, external coordinates,
and topology; deployment preparation never discovers or opens a project repository. Live effects
execute once through production rather than differential replay.

Configuration, secrets, clock, secure randomness, identifiers, password hashing, byte streams,
PostgreSQL, object storage, and durable queue all have exact normalized capability bindings.
PostgreSQL, object, and queue codecs use one representation-neutral host engine per family.
Adapter preparation is all-or-nothing, and repeated shutdown reuses one recorded cleanup outcome.

Runtime call frames carry exact generic type substitutions so the generic `json-decode-or<T>`,
`json-encode<T>`, `list-length<T>`, `list-get<T>`, and `list-fold-left<Item, State>` standard
declarations execute against the concrete Graph 5 runtime layout. Production and canonical-reference implementations remain
disjoint and agree for maintained pure behavior. The stateful BBS uses explicit database,
identifier, wall-clock, and byte-stream requirements; its HTTP/domain layer is isolated from
PostgreSQL coordinates and driver representation behind graph-owned persistence functions.

The HTTP listener is plaintext and PostgreSQL uses `NoTls`. The runtime is not a hostile-code
sandbox or multi-tenant isolation boundary.

## Current limits and unproved properties

- The only released dependency resolver accepts the exact built-in standard package. There is no
  general package manager, ambient filesystem lookup, mutable tag, network registry, or upgrade
  resolver.
- Public `run` is pure-command-only. Outbound HTTP and additional effect families are absent until
  a maintained consumer justifies a dependency-closed capability cutover.
- Context traversal, generic impact query, fuzzy search, and historical query are absent.
- Removed draft/history/review/backup/restore/doctor workflows have not yet been reintroduced on
  Graph 5 authority.
- Public authored change covers the executable-discovered subset; additional typed engine forms
  remain private until a complete public workflow exists. Existing component requirements can be
  extended, existing function contracts changed, and pure functions given explicit type parameters,
  named values, and invocation, but generic component, interface, external, port, and target
  creation remains private.
- Compact finite output is bounded to 4 MiB and 10,000 records. Query has independent item and byte
  budgets and a bounded revision-bound continuation. Exact limits and diagnostics are executable
  registry data.
- Artifact input/output, compiler units, graph traversal, expression depth, execution stack,
  instruction count, capability work, and adapter resources retain separate checked boundaries.
  These bounds are implementation admissions, not demonstrated scale ceilings.
- No million-owner complete application lifecycle, long-history retention policy, garbage
  collection, live-store packing, artifact signing, encrypted graph storage, or distributed
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
the artifact-10 resident cutover is bound by
[evidence/202608272159-artifact10-service-cutover.json](evidence/202608272159-artifact10-service-cutover.json).
The current-main distributed HTTP closure is bound by campaign
[202608281025](campaigns/202608281025.md) and
[its structured evidence](evidence/202608281025-distributed-http-application.json). Its independent,
no-Docker `distributed_http_application` gate is required by product, service, and full profiles
and copies one candidate executable into a fresh root outside the checkout. These campaigns are
summarized in [performance.md](performance.md).

Campaign [202608281817](campaigns/202608281817.md) adds the distinct non-cacheable
`stateful_http_application` gate to service and full profiles. It copies the candidate outside the
checkout, discovers current grammar/interfaces/deployment fields through that copy, authors a
982-record fold-based BBS only through reviewed public changes, requires deterministic clean/incremental
Artifact 10 bytes, and exercises PostgreSQL-backed create/read/update/delete, strict-input,
missing/nonmatching/repeated/reordered content-type, rollback, checksum-divergence, no-readiness,
shutdown, and restart behavior while Graph authority remains byte-identical. The contributor
harness may provision the already supported service and stateful gates from either the pinned
immutable PostgreSQL image or an exact verified local PostgreSQL 16.15 tool root; neither
provisioning path is a product dependency or application helper.
