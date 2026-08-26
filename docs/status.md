# Current status

Status date: 2026-08-26 UTC. This file describes implemented checkout reality. Executable-derived
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
| `applications/lkjournal/frozen-service/lkjournal-artifact-v4.lkja` | read-only artifact-4 service/worker input | `d0a57a74161903a302472cbd8997762434b64cc58bd8ae36577b9ba31d2f96a3` |

The built-in transport and artifact are compiled into the executable and strictly cross-checked.
Product verification regenerates the maintained owners and compares exact bytes. The frozen
service artifact is separately digest-pinned by the service harness and is never compared with or
overwritten by current `build`.

## Released command lifecycle

CLI contract 10 exposes exactly `capabilities`, `new`, `status`, `inspect`, `query`, `change`,
normalized built-in `package`, `check`, `build`, `run`, and artifact-runtime `serve` and `worker`.
All finite operations use deterministic bounded compact records. The registry digest is
`c63d0c4653d6de50e6f375d6da14bfb9101bba5a438aba5c0ae10a9dd27dbc43` for the current generated
content.

`new --template minimal` creates an empty package. `new --template command` creates an offline,
standard-dependent pure command project with an application module, implementation, component,
port, `main` target, and graph-owned test. Both recipes are typed executable-owned construction
and publish through the same atomic Graph 5 creation boundary.

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

Normalized production and reference execution support pure commands and graph tests, plus private
normalized adapter machinery covered by focused tests. The released `run` operation deliberately
rejects non-command runners and effectful command entry points.

`serve` and `worker` remain a separate artifact-runtime boundary. `PreparedDeployment` reads a
strict descriptor, the explicitly referenced frozen artifact-4 bytes, environment/configuration,
and host adapters. It does not discover or open a project repository. This is retained service
behavior, not normalized service compilation or artifact-10 deployment support.

The HTTP listener is plaintext and PostgreSQL uses `NoTls`. The runtime is not a hostile-code
sandbox or multi-tenant isolation boundary.

## Current limits and unproved properties

- The only released dependency resolver accepts the exact built-in standard package. There is no
  general package manager, ambient filesystem lookup, mutable tag, network registry, or upgrade
  resolver.
- Public `run` is pure-command-only. Normalized service, worker, PostgreSQL, queue, object storage,
  and outbound effect deployment remain future dependency-closed cutovers.
- Context traversal, generic impact query, fuzzy search, and historical query are absent.
- Removed draft/history/review/backup/restore/doctor workflows have not yet been reintroduced on
  Graph 5 authority.
- Public authored change covers the executable-discovered subset; additional typed engine forms
  remain private until a complete public workflow exists.
- Compact finite output is bounded to 4 MiB and 10,000 records. Query has independent item and byte
  budgets and a bounded revision-bound continuation. Exact limits and diagnostics are executable
  registry data.
- Artifact input/output, compiler units, graph traversal, expression depth, execution stack,
  instruction count, capability work, and adapter resources retain separate checked boundaries.
  These bounds are implementation admissions, not demonstrated scale ceilings.
- No million-owner complete application lifecycle, long-history retention policy, garbage
  collection, live-store packing, artifact signing, encrypted graph storage, or distributed
  publication protocol has been proved.
- Linux x86-64 with stable Rust is the verified environment. Provider tokens, cached-token counts,
  requests, retries, and monetary cost are unavailable because no direct telemetry exists.

## Verification

The contributor-only `lkjscript-dev check` harness owns gate dependencies, exact input
fingerprints, fresh/reused classification, bounded child logs, required outputs, and receipts.
The final campaign record [202608270014](campaigns/202608270014.md) names the exact focused,
product, full, and service results for this content. Structured lifecycle measurements live in
[evidence/202608270014-normalized-command-lifecycle.json](evidence/202608270014-normalized-command-lifecycle.json)
and are summarized in [performance.md](performance.md).
