# Performance and design evidence

Measurements here are observations, not semantic authority. The current post-migration raw receipts
are [`20260819-semantic-development-acceptance.json`](evidence/20260819-semantic-development-acceptance.json),
[`20260819-semantic-development-functional.json`](evidence/20260819-semantic-development-functional.json),
and [`20260819-semantic-development-representative.json`](evidence/20260819-semantic-development-representative.json).
The predecessor product-campaign receipts remain separately under the `20260819-lkjwork-*` names;
the current campaign summary is
[`20260819-semantic-development.json`](evidence/20260819-semantic-development.json).

## Observation environment

- Linux `7.0.0-29-generic`, x86-64, ZFS container root;
- AMD Ryzen 9 9955HX, 16 cores / 32 logical CPUs, 64 MiB L3;
- 32 GiB RAM;
- rustc `1.96.0` / LLVM `22.1.2`, Cargo `1.96.0`, optimized release binaries;
- sequential samples on a warm host unless a row explicitly says initialization; and
- one unrecorded warm-up followed by five one-shot samples for each retained query/mutation shape.

Page cache was not dropped and these are not cold-machine results. Timing uses monotonic
`perf_counter_ns`; stage counters use Rust monotonic durations. Process RSS, provider tokens, and
provider prices were unavailable.

## Historical lkjstudio revision-48 predecessor workload

The superseded workbench observations are retained only as the direct predecessor baseline in
[`20260819-lkjstudio-workload.json`](evidence/20260819-lkjstudio-workload.json). They used optimized
revision-48 binaries on the warm Linux x86-64 host, with 20 logical CPUs visible to the harness and
without dropping page cache. Each headless row is one complete process-inclusive sample; only
artifact validation and orientation have five samples. The harness did not collect individual-event
latencies, so no event p50/p95 claim is made.

| observation | exact result |
|---|---:|
| artifact validation | 11.075 ms median / 11.563 ms p95 |
| semantic project orientation | 361.340 ms median / 368.737 ms p95 |
| exact function query | 366.422 ms / 69,761 response bytes |
| generated function proposal | 360.316 ms / 189,752 response bytes |
| 10,000 mixed edit/resize/close events | 27,145.253 ms total / 9,999 changed |
| 1,000 growing inserts plus close | 10,962.479 ms total |
| 100-buffer corpus plus close | 287.601 ms total |
| maximum 65,536-scalar paste plus close | 804.825 ms / accepted once |
| 1x1 resize plus close | 18.318 ms |

The mixed total is approximately 2.71 ms per input by division, not a sampled event percentile. The
1,000-growing-insert observation misses a uniformly fast edit-loop ambition and is the retained
representation/rendering reversal gate. The 65,536-scalar paste originally exhausted the old
application-level copy loops; checked sequence slice/concatenate and a target-owned 100,000,000-fuel
policy close that bounded workflow. No provider token/cache/price telemetry was available, and bytes
are not converted into token or monetary estimates.

The isolated fresh-copy receipt is
[`20260819-lkjstudio-fresh-checkout.json`](evidence/20260819-lkjstudio-fresh-checkout.json). With no
copied target directory, the locked release workspace built in 204 seconds using the preinstalled
toolchain and host Cargo registry/source caches. Both semantic projects passed shallow/deep doctor;
all 29 workbench and eight `lkjwork` target cases passed; and two no-overwrite workbench builds were
byte-identical to the checked 161,562-byte artifact. This is an initially absent-target observation,
not a cold-host or empty-dependency-cache claim.

## lkjedit revision-174 product workload

The current retained receipts are
[`20260820-lkjedit-workload.json`](evidence/20260820-lkjedit-workload.json) and
[`20260820-lkjedit-campaign.json`](evidence/20260820-lkjedit-campaign.json). Measurements use the
471,096-byte revision-174 artifact at snapshot
`7e037b9e97e2f04cd2243899a30a3721f31faf30c55aa9d7d97f050d22004aa4`, optimized binaries, Linux
7.0.0-29-generic x86-64 with glibc 2.39, Python 3.12.3, 20 visible CPUs, monotonic timing, and warm
host caches that were not dropped. Process RSS and provider token/cache/price telemetry were
unavailable. Headless rows are one complete process-inclusive sample; validation and orientation
use five samples. No per-event percentile is inferred from a total.

| observation | revision-174 result | reproduced predecessor |
|---|---:|---:|
| artifact validation | 31.014 ms median / 32.915 ms p95 | 10.875 / 11.181 ms |
| semantic project orientation | 1,014.191 ms median / 1,020.969 ms p95 / 1,247 bytes | 354.442 / 366.365 ms / 1,239 bytes |
| exact function query | 1,010.180 ms / 17,402 bytes | 354.875 ms / 69,761 bytes |
| generated function proposal | 1,026.689 ms / 5,864 bytes | 357.195 ms / 189,752 bytes |
| 10,000 mixed local events | 27,864.909 ms / completed | 26,998.427 ms |
| 1,000 growing inserts | 3,882.714 ms / completed | 11,055.720 ms |
| 100 tabs / 795 transitions | 2,825.568 ms / completed | no equal predecessor corpus |
| 65,536-scalar paste | 84.220 ms / completed | 799.446 ms |
| 1x1 resize | 52.298 ms / completed | 17.894 ms |

Bulk scalar conversion, bounded sequence repetition, and cell-prefix fitting removed the earlier
application-level scalar loops. Growing insertion improves 64.9 percent and maximum paste improves
89.5 percent on equal corpora; the paste now beats the 250 ms campaign target and no longer exhausts
fuel. Query and proposal responses are 75.1 and 96.9 percent smaller, respectively, although larger
history and application authority make their complete process times slower. The mixed corpus is
3.2 percent slower and still misses the 10-second ambition; growing insert still misses 2 seconds.
Those misses, the 52.3 ms minimum resize, and an observed 977.90-second unoptimized duplicate
layout replay are explicit execution-tier/runtime-boundary reversal gates. They are not hidden by
raising fuel or reported as interaction percentiles.

### Text candidates

Five warm optimized prototype runs used identical 1 MiB ASCII input plus 1,000 two-byte middle
insertions. Flat immutable UTF-8 took 4.246–4.852 ms, a persistent chunk vector took
8.511–8.630 ms, and the selected persistent piece treap took 1.406–1.475 ms. A mutable gap buffer
won repeated end insertion at 0.173–0.199 ms but lost because immutable undo roots and two-view
sharing would require a second owner. The piece treap accepted a 65,536-byte splice in
0.017–0.023 ms before separately measured canonical materialization of 0.248–0.403 ms.

A persistent rope and chunked persistent sequence collapse to the selected bounded-piece-tree
obligations for this narrow surface; retaining separate implementations would only duplicate tree
authority. A classic original/append piece table lost because unbounded edit-piece growth needs a
second compaction policy. Current flat text plus transient builders improved only construction and
left middle splice, immutable undo, and viewport aggregates unresolved. Resident validated runtime
values were retained as an independent boundary optimization; canonical flat materialization remains
the restart/cache-loss oracle. Losing prototypes are not in the repository.

Randomized differential testing uses seed `0x6c6b6a6564697434` for 2,000 splice cases and compares
canonical bytes, scalar/grapheme/line facts, search, and materialization. A frozen 40x120 one-row
terminal change encoded 5,019 full-frame bytes and 172 acknowledged differential bytes, a 96.57
percent reduction. Cache miss reproduces the exact full projection, unchanged frames emit nothing,
and output failure clears acknowledgment. Complete PTY acceptance reconstructs keyboard/mouse
workflows and cleanup through the retained differential route.

### Verification and authoring economy

The compact product profile passed seven gates in 419,460 ms: format, locked optimized workspace
build, complete 175-snapshot doctor, 12/12 target cases, no-replace artifact build, byte comparison,
and headless plus four PTY groups. Default success was nine lines including the receipt locator. An
injected quick-profile failure preserved its full log and emitted seven bounded diagnostic lines.

The revision-172 user-visible dogfood used one orientation response (1,247 bytes), one function
inspection (694), one task context (64,134), one generated proposal (14,932), one validate-only
receipt (988), one apply receipt (2,731), and one affected-target receipt (16,767). The editable
document was 14,091 bytes. Correction depth was zero: validate published nothing; apply published
one revision and one record. Revisions 173 and 174 then used bounded 7,060-byte or smaller public
documents to cut over four hot functions without a builder. Direct elapsed/call/byte observations
are retained; provider tokens, cache classes, and monetary cost remain unavailable and are not
inferred.

## Semantic-development repository workload

The retained development workload started from the migrated `lkjwork` revision 7 project and used
one public foreground project session to apply 100 alternating package-name changes. Every request
was an accepted exact-base mutation, every mutation produced one revision and record, and restart
occurred after the workload. The final authority is revision 107 with snapshot
`7a5ebff8328f3236d224625afe174fad332f51cd7b8bd884d6849e6a9e981c2b` and record
`a1a02f7891ac017e4bed11f5aac57d5f982d3f895a36f692183fd6deb50c85ce`.

These are single sequential observations from the unoptimized development binary on a warm host;
they are not release-build, cold-cache, physical-write, or statistical latency claims.

| observation | exact result |
|---|---:|
| starting revision-7 authority | 18 files / 1,322,155 bytes |
| ending revision-107 authority | 218 files / 21,580,665 bytes |
| 108 canonical snapshots | 21,460,753 bytes |
| 108 canonical revision records | 119,554 bytes |
| 100 session apply responses | 169,517 bytes |
| 100 accepted applies | 755.428 s elapsed / 739.781 s user / 0.540 s system |
| eager all-history current open, before correction | approximately 114.47 s elapsed |
| current open after lazy historical decode | 3.224 s elapsed / 3.166 s user / 0.010 s system |
| latest five-record log page | 3.159 s elapsed |
| adjacent revision 106-to-107 diff | 4.333 s elapsed |
| current product-target build | 3.691 s elapsed |
| current product-target test, seven cases | 3.688 s elapsed |
| no-replace portable backup | 6.824 s elapsed / 218 files / 21,580,665 bytes |
| complete 108-snapshot deep doctor | 332.369 s elapsed / 325.596 s user / 0.293 s system |

The current-open correction preserves full snapshots but scans historical paths and decodes the
complete compact record chain without decoding unrelated historical graphs. A selected historical
read validates its graph against its record. Deep doctor remains the independent oracle: it decodes
all graphs, validates every adjacent identity/tombstone/allocation transition, and recomputes every
record's semantic diff and entity/target facts. A focused test truncates an unselected old snapshot:
shallow current open succeeds, while historical selection and deep doctor both classify corruption.

### Development-history alternatives

| design | result |
|---|---|
| full canonical snapshot plus compact record per revision | selected: simplest immutable publication, exact arbitrary revision reads, portable copy, and independent reconstruction; shallow loading keeps ordinary service bounded |
| canonical edit journal plus periodic snapshots | rejected for now: reduces retained bytes but adds replay/checkpoint/compaction authority without improving the completed 100-change workflow |
| immutable content-addressed graph objects | rejected for now: requires collision-conflict, reachability, packing, and garbage-collection contracts without a current storage-limit failure |
| Merkle structural sharing | rejected for now: complicates canonical traversal and backup while representation sharing has no semantic consumer |
| append-only packed objects plus index | rejected for now: index rebuilding and interruption-safe pack publication add complexity; current path count and backup remain acceptable |

Reopen persistence design if a representative maintained project exceeds 256 MiB before 1,000
revisions, ordinary current open exceeds 5 seconds in an optimized build, or deep doctor exceeds 10
minutes at 100 revisions. A candidate must beat full snapshots on the same final graph while retaining
validate/apply parity, exact diff/history, hostile decoding, crash classification, portable backup,
and an independent reconstruction oracle.

## Frozen product corpora

| profile | tasks | core mutation requests | edges | notes | attachments | pure queries | final revision |
|---|---:|---:|---:|---:|---:|---:|---:|
| functional | 25 | 75 | 30 | 50 | 5 | 100 | 85 |
| representative | 500 | 2,500 | 1,000 | 1,000 | 100 | 2,000 | 2,700 |

Both use seed `lkjwork-corpus-v1` and the independent Rust reference-model tests. The fresh-copy
post-migration final semantic digests are functional
`2bb4b8069beb70ef3da69d5efc3229a458158e3b90adbf08f62458fdb85f4602` and representative
`81d4261ee1d74eb4750ecef9c7aabd5af8293da62ed4669a199ae23103d14c7a`.
They deliberately differ from the separately retained predecessor receipts because the exact
application identity and public query interface changed at direct cutover.

The retained stress shape is 2,000 tasks / 10,000 mutations. It was not executed; no stress service
claim is made.

## Representative service results

Times are median / p95 milliseconds. Session rows include all retained corpus samples; one-shot rows
are five samples after warm-up.

| operation | session | one-shot |
|---|---:|---:|
| publishing mutation | 52.6 / 107.3 | — |
| unchanged mutation | — | 47.4 / 49.8 |
| show | 51.6 / 54.0 | 32.0 / 35.2 |
| list (20, priority) | 212.3 / 223.1 | 192.1 / 194.0 |
| next (10) | 65.7 / 69.2 | 44.7 / 45.8 |
| summary | 70.6 / 73.7 | 49.5 / 50.9 |
| context (10 tasks) | 82.9 / 86.6 | 61.7 / 62.5 |
| export page (20) | 59.1 / 61.8 | 35.0 / 37.8 |
| retained history page | 95.2 / 98.3 | 76.2 / 95.5 |

Initialization was 51.4 ms. One complete genesis replay/deep audit of all 2,701 records took
34.176 s and reproduced the exact final state. Attachments, including semantic suspend, host
publication, and resume, were 260.7 / 279.5 ms median/p95. All ordinary query and mutation targets
are met.

Before the product-open correction, representative one-shot application queries took 1.65–1.88 s:
the product locator performed a complete instance inspection before invoking the exact query owner.
Removing that duplicated traversal and using the HEAD-bound current manifest reduced complete
one-shot query medians by 89–98 percent, depending on query shape. This is a complete-workflow
comparison; no interpreter-only speedup is claimed.

## Storage

Representative authority contains 2,701 journal records:

| category | bytes |
|---|---:|
| immutable record chain | 102,841,790 |
| records carrying 64-revision checkpoints | 96,203,455 |
| HEAD-bound current manifest | 1,588,695 |
| application | 167,848 |
| attempts / outcomes | 35,100 / 107,000 |
| blob objects | 5,000 |
| complete project files | 104,745,982 |

The predecessor product campaign separately measured one post-corpus non-checkpoint label mutation;
that historical write-amplification observation remains in its own evidence and is not presented as
a fresh post-migration result. Current storage totals above come directly from the retained
post-migration revision-2,700 receipt. They are logical file-payload observations, not claims about
physical ZFS blocks, device writes, or power-loss amplification.

The current public state is large enough that checkpoints dominate retained bytes, but the complete
project remains below the 256 MiB journal ceiling. There is no semantic-history deletion or
compaction. Normal current operations validate HEAD, current record, manifest/state/index, and exact
accounting without replay. Missing/corrupt acceleration triggers complete chain validation plus
checkpoint reconstruction; deep audit always reexecutes genesis.

### History alternatives

| design | result |
|---|---|
| full state in every revision | rejected: representative write amplification and ordinary-open replay were the original dominant product costs |
| hash-linked event/host journal + periodic full checkpoints | selected: one exact chain, bounded current restart, simple deep oracle, and localized corruption |
| content-addressed state object per revision | rejected: adds object reachability/GC/publication obligations without beating the selected corpus result |
| structural-sharing full records | rejected: complicates retained accounting and portable decode while checkpoints already meet the ceiling |
| embedded database | rejected: no query-authority or concurrency consumer repays dependency, transaction, backup, and corruption obligations |

Fixed 64-revision checkpoints were selected over adaptive/user-triggered policies because cadence is
canonical, interruption cannot change semantics, and at most 63 post-checkpoint transitions are
needed on fallback. A retained-byte trigger is a reversal gate if larger states approach the journal
ceiling.

## Semantic design selections

### Text

First-class immutable validated UTF-8 text won over client-only bytes, a nominal wrapper, and a
separate nominal declaration kind. Persistent piece-tree execution now avoids copying unchanged
editor content while canonical flat UTF-8 remains the public and differential oracle. Exact UTF-8
byte equality and no normalization keep the contract small.

### Sequences and operations

Nominal homogeneous immutable sequences won over serialized bytes, recursive cons products,
application-specific fixed arrays, structural generics, and host tables. Nominal identity reuses the
existing release reference model and permits managed indirection without a generic declaration
system. Empty/length/get/append/replace remain the simple construction oracle. Checked slice and
concatenate are retained because they removed editor/render copy loops and closed the maximum-paste
workflow under deterministic fuel. Higher-order operations, mutable builders, maps, sets, and
iterators had no product consumer.

Exact `i64` equality and boolean not/and/or were retained because lifecycle, filtering, pagination,
and dependency paths repeatedly used them. Broader arithmetic, polymorphic equality, traits, and
operator syntax lost for lack of a second domain.

### Values and memory

The safe managed byte store remains for bytes. Text uses a safe persistent UTF-8 piece treap over
immutable `Arc<str>` backing; sequences use immutable `Arc` elements. These won over tracing GC and
mutable arenas because accepted values cannot form pointer cycles and canonical materialization
remains a simple oracle. Representation sharing is invisible and cannot evade limits. No unsafe Rust
was introduced; `unicode-segmentation` 1.13.3 is the one new semantic dependency.

### Responses and queries

Application-owned typed response and query types directly replaced byte responses and read-as-event.
The four decision variants were selected over full-state no-op records and client preflight because
they preserve application policy while publishing zero revisions for domain decline/no-change.
No-publication receipts are not retained; exact retry is bounded by the unchanged base.

First-class pure queries won over client state decode and a second query database. Exact result
digests permit cache-independent unchanged responses. Current queries publish no filesystem facts,
as verified by complete tree comparison and fault tests.

## Packaging, source, CLI, and topology decisions

- A checked semantic development repository plus first-class product target now owns `lkjwork`.
  Embedded validated application bytes remain the installed distribution form, so the product runs
  without development authority; public target build reproduces those bytes.
- The former 255,704-byte Python construction recipe was a useful migration input but lost as
  maintained source. It and its generated binding manifest were deleted. An evolved exact JSON /
  semantic-document proposal surface won over narrow one-command-per-node forms and a conventional
  source language because it already supports proposal-local symbols, multi-owner atomicity,
  incremental function replacement, targets, validate/apply parity, and bounded context.
- Artifact self-description won over generated native bindings. The Rust product resolves and
  validates exported types, fields, variants, and functions from the exact embedded artifact; stale
  hand-maintained IDs cannot silently compile into the client.
- Manual standard-library CLI parsing was retained. No parser dependency was needed to deliver closed
  grammar, help examples, strict machine mode, and bounded errors; JSON-only lost the human product
  requirement.
- Parent locator discovery with explicit override won over directory-name identity and global
  registries. Exact operation validation remains in the instance owner.
- A caller-owned product session was retained: one process handles roughly 4,700 representative
  operations instead of one process per request, comfortably exceeding the 50 percent process gate.
  A daemon, worker pool, and socket supervisor have no consumer.
- One session-local exact-HEAD application/current-state cache was retained because repeated query
  medians meet the product ceiling and miss/restart remain correct. No persistent cache was built.
- The activation interface, durable-controller driver, and standalone blob-publisher driver were
  deleted. The product supplies the current stateful/blob consumer and focused tests retain failure
  invariants.

## Agent-cost observation

The dogfood `why` feature used three accepted public project changes and no rejected correction
proposal: an identity-preserving rename revision, an atomic type/function creation revision, and a
query/target/case cutover revision. The three canonical change inputs totalled 13,796 bytes (387,
9,420, and 3,989 bytes). The following bounded one-shot observations at revision 7 were retained:

| observation | observed bytes |
|---|---:|
| orientation | 1,228 |
| unchanged orientation | 252 |
| one function summary | 658 |
| targeted refactor context | 53,811 |
| latest four revision records | 2,397 |
| migration-to-dogfood diff | 3,334 |
| exact `query_entry` function projection | 33,133 |
| `query_entry` function + callees + targets | 35,762 |
| packet-free `query_entry` proposal document | 11,846 |
| unchanged exact function projection | 391 |

The proposal row measures the extracted editable document; its enclosing project response was
12,550 bytes. The three new semantic-query/proposal rows were measured on the same checked revision
7 authority after the public query/proposal implementation. Bytes are not converted to tokens or
money.

The feature required three apply processes/engine opens, one target build, and one target test; its
semantic proposal correction depth was zero. The predecessor baseline was a 255,704-byte,
4,869-line procedural source that emitted one 3,232-item transaction and separately constructed
release/application requests and generated bindings. Exact baseline files-opened, request/response,
token, and provider-price telemetry were not exposed, so no invented byte-to-token or monetary
savings claim is made. The exact outcome claim is narrower: ongoing graph maintenance no longer
requires reading, modifying, or executing that builder, and unchanged orientation is 252 bytes.

The current product-runtime benchmark is separate: one product session handled 2,000 query
observations using 730,308 request bytes and 7,717,826 response bytes. That measures installed
application interaction, not semantic development authoring.

## Build observations and reversal gates

The predecessor product campaign observed a 130-second optimized workspace rebuild and a
132.84-second isolated-target build on a warm host; those measurements predate the semantic-project
cutover and remain historical baseline. The verified campaign tree was copied before its final
local commit without `.git`, `target`, or local caches and built with an initially absent isolated
Cargo target directory on the same warm host. The locked release build took 184.784 seconds elapsed
(201.487 user, 3.441 system).
The resulting `lkjscript` binary is 13,263,608 bytes with SHA-256
`634ba9c4647b6fa2d0c16768d3f5ebe8fe9aa0dd78d1b22d41f133304cefc679`; `lkjwork` is
3,553,056 bytes with SHA-256
`b786ec1b22aff29a44e55178b0f0ba313d2168499ac9803272a207ce7ca5927b`.
This is an empty-target, warm-host build, not a cold compiler/download measurement.

Reopen design only when a complete current consumer crosses a gate:

- checkpoint cadence or compaction: representative store approaches 256 MiB or fallback/deep service
  becomes unacceptable;
- persistent cache: restart benefit improves complete workload by at least 20 percent and repays its
  hostile format/lifecycle;
- bytecode/native tier: execution dominates and complete work improves at least 30 percent;
- daemon/concurrency: a real multi-client or unattended workflow cannot use the foreground session;
- database/index: accepted query or concurrency needs cannot be met by bounded application scans;
- broader text/collections/search: a complete product workflow requires exact new semantics.
