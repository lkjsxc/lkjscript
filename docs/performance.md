# Current evidence and architecture decisions

This file owns reproduced measurements, selected alternatives, current complexity decisions, and
reversal conditions. Exact structured values are retained in
[`evidence/20260818-reusable-release.json`](evidence/20260818-reusable-release.json). Earlier
evidence files are historical baselines, not current authority.

## Measurement boundary

The campaign started and remains uncommitted on branch `main` at
`e465cc3b7d12353aa4c6fba13dc02de41d381346`, the audited standalone-application baseline. The
pre-existing worktree changes were `AGENTS.md` and the untracked campaign prompt; both were
preserved. The active root policy's Git blob ID was
`6384d5384c373080a02a89c9e96eb85932524cc8`.

The environment is Linux x86-64 with rustc/cargo 1.96.0 and stable Rust edition 2024.
`Cargo.lock` SHA-256 is
`d23b75fc162e485b7149d92f1e3349f3cca39f00420a9fef68f8abea6c405620`.
`/usr/bin/time` is unavailable, so no maximum-RSS result exists. Times are single observations
unless a sample count is stated and are not benchmark distributions. Provider tokens, cached
tokens, reasoning tokens, calls, and prices are unavailable; bytes are not labelled as tokens.

Before edits, formatting, all-target/all-feature Clippy, 218 tests (214 pass, four ignored), release
build, and all six retained examples passed. The baseline had 2,287,367 bytes / 64,768 lines under
`src/` and `tests/`; documentation had 99,572 bytes / 1,903 lines. Release binaries were 7,261,216
bytes for `lkjscript` and 4,252,144 bytes for `lkjscriptd`.

## Selected architecture

One selected workspace package produces one canonical reusable semantic release. Release-local
IDs erase workspace lineage; the exact `ReleaseId` is the domain-separated digest of the complete
canonical payload. Dependencies are exact external release graphs with explicit local proxies.
Graph boundaries survive release and application inspection; application format 2 embeds every
exact release once and privately flattens only for the existing compiler/interpreter.

This is the combined candidate E + H + I + J from the campaign: release content digest plus local
IDs, external exact graph, application-time flattening, and a bundled graph application. Build uses
explicit artifact paths; there is no resolver, lock record, registry, or immutable store. The
application bundle is the offline transfer unit.

### Serious alternatives

| Candidate | Correctness result | Cost/result | Decision and reversal |
|---|---|---|---|
| duplicated application closure | Could run two apps, but cannot independently publish/test one shared authority or prove one diamond nominal identity | smallest format surface; duplicates semantic/test work and does not satisfy the reuse workload | rejected; reconsider only if release graph maintenance repeatedly exceeds duplicated end-to-end work |
| workspace-lineage release | preserves existing IDs | equal semantics from unrelated workspaces produce unequal bytes and leak allocator/history into transfer | rejected; no product consumer requires producer lineage as semantic identity |
| canonical release-local IDs + whole-release digest | passes unrelated-history equality, private visibility, R1/R2, diamond, offline transfer | whole-release changes intentionally churn exact identity; one bounded canonicalizer/codec/graph | selected; reopen if fine-grained updates dominate real workloads |
| definition/SCC content identity | could deduplicate below release | creates a much larger semantic hash surface, recursive-group canonicalization, provenance confusion, and diagnostics cost for no current consumer | boundedly rejected without retained prototype; reopen for measured sub-release sharing |
| assigned release ID + content digest | separates lineage/content | needs an allocator/publication namespace and makes equal offline reconstruction ambiguous; no continuity consumer | rejected; add only with a concrete lineage or external-targeting operation |
| deterministic vendoring | reuses existing workspace tools | mutates consumer authority and creates copy/update/fork/nominal rules while losing one exact graph | rejected; add an explicit adapt/copy operation only for a real editing consumer |
| manifest + immutable store | deduplicates bytes across applications | adds mutable directory/recovery/availability state; retained bundles are only 2.9–6.2 KiB | rejected; prototype after aggregate bundle duplication becomes material |
| flattened semantic application authority | simple runtime | erases reviewable graph boundaries and risks nominal/version confusion | rejected as authority; retained only as a private verified compiler projection |

Losing candidates left no format, reader, flag, resolver, store, allocator, compatibility path, or
prototype in the active tree.

## Identity, boundary, and composition decisions

- **Reusable unit:** one existing workspace package is one release root. Modules remain internal
  namespace/containment. A workspace may construct several releases in separate requests.
- **Exact identity:** `ReleaseId` is the digest of all canonical payload content, including
  coordinate and user version. `ReleaseContentDigest` is a separate integrity domain. Provenance,
  signature, authorization, and freshness are absent.
- **Canonical IDs:** modules and definitions are deterministically ordered; member/parameter/body
  order follows accepted semantics. All selected definitions are assigned before references are
  remapped, which handles mutual recursion without definition hashes or graph-isomorphism code.
- **Exports:** one explicit flat symbolic namespace exports functions and nominal declarations.
  Exact consumers ultimately bind item IDs. Dependency proxies cannot be re-exported in format 1.
- **Private implementation:** reachable private content and private tests travel and affect exact
  identity; unrelated private declarations are omitted. Private imports reject.
- **Dependencies:** every slot binds an exact immutable release. Resolution and lock records are
  absent. Direct proxy references cannot escape to transitive/private items.
- **Versions:** coordinate and user version support bounded human review only. R1 and R2 can coexist;
  aliases/slots disambiguate local intent and exact pairs define nominal equality.
- **Diamond:** a `BTreeMap<ReleaseId, DecodedRelease>` owns graph nodes. Equal exact R1 is validated,
  embedded, and flattened once; conflicting bytes and extra inputs reject.
- **Import:** accepted workspace semantics use deliberately bodyless/type-shape proxies plus an
  exact release request binding. Vendoring, copy, fork, and import-into-workspace are absent.
- **Tests:** producer-owned primitive invocation cases remain inside each release; public nominal
  cases live in the application. Every dependency suite runs before dependent publication.
- **Application:** v2 bundles graph authorities and application facts without workspace identity.
  v1 reader/writer/magic success paths were deleted and format 1 rejects.
- **Agent surface:** release/app contracts are command-local and inspections provide bounded exact
  summaries; they do not enlarge the global workspace schema or require a schema dump.

Reversal for the overall architecture: reopen when a representative workload needs independently
updatable sub-release definitions, graph bundles exceed 64 MiB or repeatedly duplicate more than
50% of deployment bytes across at least ten retained applications, explicit file provision causes
measured correction failures, or an authenticity/provenance consumer cannot remain a separate
artifact.

## Representative reusable-release observation

`examples/reusable-release/run.sh` uses only production protocol-v10 transactions and public
release/application commands. One run produced these deterministic artifacts:

| Release | Exact ID | Bytes |
|---|---|---:|
| shared-codec R1 | `cbb4ec4d4362fc24d486fcdbd0fbcd5890a88158940d90aea94950b805854b91` | 1,694 |
| shared-codec R2 | `adfd2e30c8628fe1ed6fa0b0b80a1fda747e8e0124a4ed0c9f1f5feeb2789a5d` | 1,744 |
| consumer-normalizer | `c8d5286c36190cfe6ba59cd5e89063ea824436464763b3e1d1256c5e5a5680b7` | 1,473 |
| consumer-inspector | `05190135b304282509254cc9599631304480a12f2ebc8b9284b7fcce8aec3871` | 889 |
| release-version-coexistence | `602c8019e2ed85eb901699d36217965805380d06f5c8b2396ccff5f34472ff6f` | 1,665 |
| release-diamond | `a39ad1bfd9b3a706a7600a87841e5b0e57f0353c7bd8d36ad4f4bc736992eed7` | 1,111 |

| Application | Bytes | Nodes / edges / depth |
|---|---:|---:|
| consumer-normalizer | 3,507 | 2 / 1 / 2 |
| consumer-inspector | 2,916 | 2 / 1 / 2 |
| version coexistence | 6,203 | 3 / 2 / 2 |
| exact diamond | 5,523 | 4 / 4 / 3 |

R1 bytes were equal from two unrelated workspace IDs, different durable serial histories, reversed
function/export order, unrelated extra content, and different function-local numbering. R2 has the
same coordinate but different exact ID and behavior. The diamond inspection contains one R1.
Private import, R2-for-R1 nominal value, corrupt dependency, missing dependency, extra dependency,
and release-order-sensitive application output all follow their promised reject/equality behavior.

The complete state directory containing all seven workspaces was removed. Six release artifacts
then validated/inspected/tested, and four applications validated/inspected/tested and rebuilt
byte-identically. Normalizer and diamond stream runs returned `abc`; inspector typed run returned
`3`; coexistence returned an exact R1 nominal value and rejected the structurally identical R2
substitution. No network or ambient store was used.

The workload used 15 authoring RPC calls in one direct session, 56 total processes, nine Engine
opens, 33,370 action bytes, 84,802 observation bytes, 501 diagnostic bytes, and a summed command/RPC
boundary of 273,462,995 ns. These are complete
workflow byte/process observations, not provider tokens or monetary cost.

## Existing application/runtime observation

The updated `binary-canonicalizer` retained the full repair/history/runtime workload and observed:

| Measure | Result |
|---|---:|
| reusable release | 4,979 B, 5 exports, 3 cases |
| application-v2 bundle | 5,568 B, 1 release, 105 flattened items, 6 total cases |
| release command boundary | 5 processes, 3,813 input / 8,017 output bytes |
| application command boundary | 10 processes, 4,447 input / 14,345 output / 150 diagnostic bytes |
| workspace authoring | 43 calls, 5 Engine opens, 56,625 request / 131,567 response bytes |
| deterministic release/application rebuild | pass |
| workspace deletion and offline validate/test/typed/stream | pass |

The managed-byte dense boundary still accepts 1,445 input payload bytes and rejects 1,446 under the
visible-byte policy. The representative 512-octet append control retains 1,024 copied backing bytes
and 513 peak backing bytes versus 131,840 copied and 1,024 peak for allocate-new. The optimization
still materially wins this shared-release application; the allocate-new route remains the oracle.

## Contract, daemon, storage, and build decisions

| Domain | Retained result | Evidence and reversal |
|---|---|---|
| manual workspace catalogue | retained; release/app fields are command-local | schema digest, agent help, packet/document binding, diagnostic root projection, and strict clients still consume it; re-run a complete derive candidate before deleting the 153,227-byte owner |
| optional daemon | retained only for exported framed client tests | no release/app consumer; delete binary/client/transport/tests/docs together when correlation/deadline/disconnect/shutdown/lock coverage moves to direct Engine/session |
| full workspace snapshots | retained | reusable release removes no workspace history and current restart/retained bytes are not dominant; prototype object/delta storage only after two measured thresholds cross |
| deterministic full scans | retained | no shared-release workflow exposed a scan bottleneck; any index remains disposable and differential against scans |
| Core IR cache | absent | compile/run observations do not justify a second verified format; require material gains in at least two of startup, compile, dispatch, or transfer |
| managed bytes | retained | application-scale copy/peak reduction remains material; delete planner/verifier/handles if a broader distribution removes that benefit |
| dependencies/build tooling | unchanged | no crate, feature, build script, proc macro, unsafe Rust, resolver library, database, or network client was added |

One post-cutover optimized LTO build took 1 minute 57 seconds after source changes; this is a single
incremental build observation, not clean-build evidence. Consistent clean/incremental distributions
and maximum RSS remain unavailable.

## External design comparison

Primary sources were refreshed on 2026-08-18. They informed dimensions and failure cases, not an
ecosystem copy:

- [Unison's big idea](https://www.unison-lang.org/docs/the-big-idea/) separates names from
  definition identity; lkjscript keeps that lesson but chooses one bounded release identity instead
  of definition hashes.
- [Nix store objects](https://nix.dev/manual/nix/latest/store/store-object/) distinguish immutable
  store objects and closures; lkjscript needs no ambient store because explicit files and embedded
  graph bundles close the current offline workload.
- [Cargo resolution](https://doc.rust-lang.org/cargo/reference/resolver.html) distinguishes human
  dependency intent, exact selected packages, lockfiles, and multi-version graphs; lkjscript begins
  after selection and encodes only exact release IDs.
- The [WebAssembly Component Model explainer](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md)
  reinforces explicit imports/exports and composition boundaries; lkjscript has no ABI, host
  interface, or component linker surface.
- [OCI descriptors](https://github.com/opencontainers/image-spec/blob/main/descriptor.md) separate
  media type, digest, size, and graph transfer; lkjscript uses strict domain-specific release bytes
  rather than a general blob manifest.
- [TUF](https://theupdateframework.github.io/specification/latest/),
  [in-toto](https://in-toto.io/), and
  [SLSA provenance](https://slsa.dev/spec/v1.2/provenance) separate content from freshness, roles,
  authorization, and build evidence; those domains are explicitly absent rather than overloaded
  onto `ReleaseId`.
- [Git objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) and
  [IPFS CIDs](https://docs.ipfs.tech/concepts/content-addressing/) demonstrate immutable graph
  addressing and mutable names, but lkjscript does not inherit their object graph, transport,
  multicodec, or trust model.
- [Leroy's applicative-functor work](https://caml.inria.fr/pub/papers/xleroy-applicative_functors-toplas.pdf)
  highlights nominal identity and repeated application. lkjscript uses exact release/item pairs and
  rejects release cycles instead of implementing a general module calculus.

## Adversarial and tool evidence

- Both release and application decoders reject every truncation point and 10,000 deterministic
  one-bit mutations. This is deterministic mutation, not coverage-guided fuzzing.
- Release coordinate, version, name, slot, and declared-payload policies accept their exact
  boundaries and reject one-over. Exact graph node, edge, depth, and aggregate-byte arithmetic
  policies likewise cover boundary, one-over, and byte-count overflow. Equal duplicate exact
  bytes deduplicate; conflicting bytes for one ID, self-dependencies, and two-release cycles reject.
- Release tests cover unrelated-history canonical equality, private rejection, two consumers,
  multi-version nominal distinction, exact diamond deduplication, no-overwrite publication,
  symlink/non-regular paths, and all before/after write-sync-link-cleanup-directory-sync outcomes.
- Application tests cover old-v1 rejection, graph permutation, missing/extra/corrupt releases,
  exact nominal values, offline typed/stream execution, no-overwrite, and the same publication
  transitions.
- Nightly Miri 0.1.0 passed the exact graph limit/duplicate/cycle test in 573.83 seconds and the
  exact application nominal-value test in 166.12 seconds. Existing Core-IR verification, runtime
  differential, and workspace mutation smoke also remain applicable.
- `cargo-fuzz`, model checking, cross-platform execution, a sanitizer run, and provider telemetry
  were unavailable or not run. No claim of formal verification follows.

## Next evidence gate

The highest-value next gate is weak-model/application-authoring economy for the now-complete release
surface: freeze equal create/bind/update/inspect/corrupt tasks, compare the current exact
command-local JSON plus task-scoped inspection against one bounded typed action candidate, and
measure semantic success, corrections, action/observation bytes, processes, Engine opens, files
opened, and actual provider telemetry when exposed. Delete the candidate if it does not improve the
complete task.
