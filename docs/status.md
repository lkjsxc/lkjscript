# Current status

Status date: 2026-08-28 UTC. This file describes implemented checkout reality. Executable-derived
contract and operation catalogs live in [generated/contracts.md](generated/contracts.md) and
[generated/operations.md](generated/operations.md); this file does not duplicate them.

## Current authority and maintained consumers

Graph 5 is the sole current editable program authority. A project root contains a strict
`GraphRepository`: `HEAD`, immutable packs, an object catalog, optional exact package transports,
and private staging/locking state. Accepted meaning is the exact revision and immutable object
closure selected by `HEAD`. No maintained project contains a predecessor `.lkjscript` store.

| Consumer | Exact current identity |
|---|---|
| standard package | repository `repo_c1358d64c351873b51c954b69d1ac988`; package `pkg_10000000000000000000000000000001`; revision `rev_27c3a79c798fe402d114e0000fefa0d628916808062d63d1782a6d9ed5e5aa83`; state `semantic_state_ae45c20a630a5f90f2a40bca7fa486aa3cde16fc05fce4ec51f52bca33cb5da8` |
| `lkjournal` | repository `repo_95f988c5423fe3eb823c329ef0832d51`; package `pkg_20000000000000000000000000000001`; revision `rev_0f660831701b710fc7cd6e5f2c87cd754a944adc4ce77e1aca4649711946b4db`; state `semantic_state_067e2ba593a62c71757d24aaf717ddf28027454bf11b623e292d939120520cd4` |
| built-in standard dependency | package revision `package_revision_4290e78132570943c17a9cd800af0742dfc8c16baa6f471354792dab1d0db981`; transport `package_transport_76566ff6df6024e573d3fc7f868cbc74760170dbd2111805c4c8c30a3a95b154`; artifact manifest `artifact_manifest_1d2b53b867cbe1027d4b537f34ecf93007ea28ce54f28bd6674ebdba0b15fe6e` |

The standard package owns 284 semantic owners, 60 compiler units, and 7 graph tests. Its current
artifact has 572 closure objects and 182,596 bytes. `lkjournal` owns 1,313 semantic owners and one
exact standard dependency; its two-package artifact has 120 compiler units, 2,280 closure objects,
and 685,766 bytes. Its complete dependency closure runs 12 graph tests. Both consumers currently
pass production/reference equality.

Maintained derived assets are:

| Path | Role | SHA-256 |
|---|---|---|
| `packages/standard/generated/standard.lkjp` | exact built-in package transport, 69,811 bytes | `3031ebba737e219964687b04adf7fb0d320289771635209e2053ff322a739623` |
| `packages/standard/generated/standard.lkja` | current artifact-10 standard bundle, 182,596 bytes | `7cc4637334751d36f284cd26a394ba885e570aa2e51a366f8ab91c1aea315436` |
| `applications/lkjournal/generated/lkjournal.lkja` | current artifact-10 application bundle, 685,766 bytes | `80c69d69aec80e49cc0c023ec65eef3106f4a876eff1dc347defb461f3037ccb` |

The built-in transport and artifact are compiled into the executable and strictly cross-checked.
Product verification regenerates maintained owners and compares exact bytes. Service verification
also performs a fresh public `lkjournal` build, requires byte equality with the checked-in bundle,
and stages that one artifact for isolated `serve` and `worker` acceptance.

## Public binary release

[`v0.1.7`](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.7) is the current public and
latest release. Its annotated tag object
`92a6d0b68b8c524d058c51073e8dd28158a9247b` selects commit
`be531d9c77d722559527bbfa3683da67d9055b89`; GitHub reports release `378319314` as immutable,
non-draft, and non-prerelease. It includes the standalone artifact-10 `serve` and `worker`
boundary plus the complete copied-binary HTTP creation, reviewed mutation, build, serve, restart,
and failure workflow. The supported target remains exactly `x86_64-unknown-linux-gnu`. The
executable is ELF64 x86-64, uses `/lib64/ld-linux-x86-64.so.2`, requires `libc.so.6`,
`libgcc_s.so.1`, and `libm.so.6`, and has a measured maximum required symbol version of
`GLIBC_2.38`.

| Public asset | Bytes | SHA-256 / GitHub asset digest |
|---|---:|---|
| `lkjscript-x86_64-unknown-linux-gnu.tar.gz` | 6,985,922 | `e62f080b39686603f0401dfd6faea57d00dbf5e37cd51c4f23e7b1b63904f455` |
| `SHA256SUMS` | 108 | `f4db76f640618f97c3d981698f828f09f3730f3e5511eac7886977da5a9d5f1c` |

Release workflow
[`33150527883`](https://github.com/lkjsxc/lkjscript/actions/runs/33150527883) passed all four jobs on
attempt 1. Its exact tagged content passed 21/21 fresh full gates with zero reuse, including
PostgreSQL service and distributed HTTP acceptance. A separate read-only no-checkout job verified
the release and verifier handoffs, safely extracted the package, and passed the transferred HTTP
oracle before publication. Anonymous exact-tag and stable `releases/latest` downloads then matched
the archive and checksum bytes, passed GitHub release and asset attestation, and independently
completed the HTTP oracle twice. The archive holds only the executable, root license, generated
third-party notices, and canonical release manifest under one `lkjscript/` directory.

Immutable `v0.1.5` was the first publication attempt for this source generation. Its public bytes
passed independent checksum and command lifecycle verification, but its workflow's final smoke
step compared the binary against a stale pre-cutover registry digest. The workflow now reads the
CLI contract and registry digest from the verified release manifest, and recovery used additive
`v0.1.6`; neither `v0.1.5` nor an older tag or asset was changed. The independent `v0.1.7`
publication likewise moved or replaced no predecessor. Release files remain derived distribution
evidence rather than program authority or independent build provenance. Exact evidence is retained
in the historical [v0.1.6 record](evidence/20260828-v0.1.6-public-release.json) and the current
[v0.1.7 public HTTP record](evidence/202608281451-public-http-release.json).

## Current-main application lifecycle

Current `main` is package version 0.1.7 with CLI contract 11. It exposes exactly `capabilities`,
`new`, `status`, `inspect`, `query`, `change`, normalized built-in `package`, `check`, `build`,
`run`, and artifact-runtime `serve` and `worker`.
All finite operations use deterministic bounded compact records. The registry digest is
`dd11eb15c9642ba6c64a223b01236ecefcd9323d6ad3711e4b46a6689ebab77d` for the current generated
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

This closed HTTP recipe is public `v0.1.7` behavior. It does not expose generic component,
requirement, port, or target authoring.

`status`, exact owner `inspect`, and normalized owner/name/relation `query` read one accepted
revision. `change plan` and `change apply` share typed lowering and a reviewed logical-plan
commitment; apply is the only normal existing-project semantic writer.

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
  remain private until a complete public workflow exists. The closed HTTP recipe is one complete
  consumer of private topology construction; it does not expose generic component, requirement,
  port, or target records.
- Compact finite output is bounded to 4 MiB and 10,000 records. Query has independent item and byte
  budgets and a bounded revision-bound continuation. Exact limits and diagnostics are executable
  registry data.
- Artifact input/output, compiler units, graph traversal, expression depth, execution stack,
  instruction count, capability work, and adapter resources retain separate checked boundaries.
  These bounds are implementation admissions, not demonstrated scale ceilings.
- No million-owner complete application lifecycle, long-history retention policy, garbage
  collection, live-store packing, artifact signing, encrypted graph storage, or distributed
  publication protocol has been proved.
- The public binary is admitted only for `x86_64-unknown-linux-gnu` with the measured dynamic
  runtime above. Musl/static Linux, older GLIBC, macOS, Windows, arm64, generic Unix, and other
  targets are unproved.
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
